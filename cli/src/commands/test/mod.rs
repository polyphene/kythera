mod gas_report;

use crate::utils::search::search_files;
use clap::ArgAction;
use colored::Colorize;
use kythera_lib::{
    ApplyRet, ExecutionEvent, MethodType, TestResult, TestResultType, Tester, WasmActor,
};
use std::error::Error;
use std::{
    path::PathBuf,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    thread,
};

use self::gas_report::GasReport;

/// Kythera test command cli arguments.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Actor files dir.
    path: PathBuf,

    /// Verbosity of the traces.
    ///
    /// Pass multiple times to increase the verbosity (e.g. -v, -vv, -vvv).
    ///
    /// Verbosity levels:
    /// - 2: Print execution traces for failing tests
    /// - 3: Print execution traces for all tests
    #[clap(long, short, verbatim_doc_comment, action = ArgAction::Count)]
    pub verbosity: u8,

    /// Print gas reports.
    #[clap(long)]
    gas_report: bool,
}

/// Kythera cli test command.
pub fn test(args: &Args) -> anyhow::Result<()> {
    let test_targets = search_files(&args.path)?;
    let mut gas_report = GasReport::default();
    let mut tester = Tester::new();

    // Iterate through target actors and respective tests.
    for test_target in test_targets {
        log::info!("\tRunning Tests for Actor : {}", test_target.actor.name());
        let constructor = test_target.actor.abi().constructor().cloned();

        match (
            tester.deploy_target_actor(test_target.actor.clone()),
            constructor,
        ) {
            // Target actor constructor should also be accounted in gas costs.
            (Ok(Some(ret)), Some(constructor)) if args.gas_report => {
                gas_report.analyze_method(
                    tester
                        .deployed_actor()
                        .expect("Deployed actor should be available"),
                    constructor,
                    ret.msg_receipt.gas_used,
                );
            }
            (Err(err), _) => {
                log::error!("\nError: {}", err);
                if let Some(source) = err.source() {
                    log::error!("Caused by: {}", source)
                }
                continue;
            }
            _ => {}
        }

        // Filter the [`Method`]s to be test, `MethodType::Test` `MethodType::TestFail`.
        let populated_tests = test_target
            .tests
            .iter()
            .filter(|test| {
                test.abi().methods().iter().any(|method| {
                    matches!(method.r#type(), MethodType::Test | MethodType::TestFail)
                })
            })
            .collect::<Vec<&WasmActor>>();

        log::info!("\t\tTesting {} test files\n", populated_tests.len());

        // Iterate through test actors.
        for test in populated_tests {
            // Create two channels, one for streaming the result,
            // and another for synchronization when the streaming is over.
            let (sync_tx, sync_rx) = sync_channel(1);
            let (stream_tx, stream_rx) = sync_channel(10);

            let verbosity = args.verbosity;
            thread::spawn(move || stream_results(stream_rx, sync_tx, verbosity));

            match tester.test(test, Some(stream_tx)) {
                Ok(results) => {
                    if args.gas_report {
                        let deployed = tester
                            .deployed_actor()
                            .expect("Deployed actor should be available");
                        gas_report.analyze_results(deployed, &results);
                    }
                }
                Err(err) => {
                    log::error!("\nError: {}", err);
                    if let Some(source) = err.source() {
                        log::error!("Caused by: {}", source)
                    }
                }
            };

            sync_rx
                .recv()
                .expect("Should be able to sync the end of streaming results");
        }
    }

    if args.gas_report {
        log::info!("\nGas report");
        for table in gas_report.finalize() {
            log::info!("{table}");
        }
    }

    Ok(())
}

/// Stream the results received from `Tester::test, so that users see the result of each test as
/// soon as it finishes.
fn stream_results(
    stream: Receiver<(WasmActor, TestResult)>,
    sync_tx: SyncSender<()>,
    verbosity: u8,
) {
    let mut tests_failed = vec![];
    let mut tests_passed = vec![];
    // Default failed will be shown for test actors that returned errors on setup.
    let mut result = "FAILED".bright_red();
    for (_actor, test_result) in stream {
        log::info!("{test_result}");
        match test_result.ret() {
            TestResultType::Passed(apply_ret) | TestResultType::Failed(apply_ret) => {
                log::info!("(gas consumption: {})", apply_ret.msg_receipt.gas_used);
                // 'vvv', prints all traces.
                if verbosity == 3 {
                    print_verbose_traces(apply_ret);
                }
                if test_result.passed() {
                    tests_passed.push(test_result);
                } else {
                    // 'vv', prints failing traces.
                    if verbosity == 2 {
                        print_verbose_traces(apply_ret);
                    }
                    tests_failed.push(test_result);
                }
            }
            TestResultType::Erred(_) => {
                tests_failed.push(test_result);
            }
        }
    }

    // Optimist mindset that if we got returned values and some of them are passing then all
    // are passing.
    if !tests_passed.is_empty() {
        result = "ok".green();
    }

    // After each and every test result has been printed,
    // we print the sum of failed and passed tests.
    if !tests_failed.is_empty() {
        result = "FAILED".bright_red();
        log::error!("\nfailures:");
        for f in tests_failed.iter() {
            log::error!("test {}", f.method());
            match (f.method().r#type(), f.ret()) {
                (_, TestResultType::Erred(err)) => {
                    log::error!("\nError: {err}");
                }
                (MethodType::Test, TestResultType::Failed(apply_ret)) => {
                    let info = apply_ret
                        .failure_info
                        .as_ref()
                        .expect("Failure info should be available");
                    log::error!("failed: {info}");
                }
                (MethodType::TestFail, TestResultType::Failed(_)) => {
                    log::error!("failed: test exited with exit code 0");
                }
                (_, TestResultType::Passed(_)) => panic!("Test should have failed"),
                _ => panic!("Failed tests should be of type test or test fail"),
            }
        }
    }

    log::info!(
        "\ntest result: {result}. {} passed; {} failed\n",
        tests_passed.len(),
        tests_failed.len()
    );
    sync_tx
        .send(())
        .expect("Should be able to sync finish streaming results");
}

/// Print the traces and gas consumptions of each test.
fn print_verbose_traces(apply_ret: &ApplyRet) {
    for trace in apply_ret.exec_trace.iter() {
        match trace {
            // OnChainReturnValue doesn't have costs.
            ExecutionEvent::GasCharge(gas_charge) if gas_charge.name == "OnChainReturnValue" => {}
            ExecutionEvent::GasCharge(gas_charge) => {
                log::info!("├─ [<Gas Charge>] {}", gas_charge.name);
                log::info!("│   └─ ← {}", gas_charge.compute_gas);
            }
            kythera_lib::ExecutionEvent::Call {
                from, to, method, ..
            } => {
                log::info!("├─ [<Call>] from {from} to {to} method: {method}");
            }
            ExecutionEvent::CallReturn(exit_code, _) => {
                log::info!("└─ ← {exit_code}");
            }
            ExecutionEvent::CallError(syscal_error) => {
                log::info!("├─ [<Syscall Error>] {syscal_error}");
            }
            // non_exhaustive enum.
            _ => {}
        }
    }
}

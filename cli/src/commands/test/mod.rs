use crate::utils::search::search_files;
use clap::{ArgAction, Args};
use colored::Colorize;
use kythera_lib::{
    ApplyRet, ExecutionEvent, MethodType, TestResult, TestResultType, Tester, WasmActor,
};
use std::{
    path::PathBuf,
    sync::mpsc::{channel, sync_channel, Receiver, SyncSender},
    thread,
};

/// Kythera test command cli arguments.
#[derive(Args, Debug)]
pub(crate) struct TestArgs {
    /// Actor files dir.
    path: PathBuf,

    /// Verbosity of the traces.
    ///
    /// Pass multiple times to increase the verbosity (e.g. -v, -vv, -vvv).
    ///
    /// Verbosity levels:
    /// - 1: Print logs for all tests
    /// - 2: Print execution traces for failing tests
    /// - 3: Print execution traces for all tests, and setup traces for failing tests
    #[clap(long, short, verbatim_doc_comment, action = ArgAction::Count)]
    pub verbosity: u8,
}

/// Kythera cli test command.
pub(crate) fn test(args: &TestArgs) -> anyhow::Result<()> {
    let tests = search_files(&args.path)?;
    for test in tests {
        // Create two channels, one for streaming the result,
        // and another for synchronization when the streaming is over.
        let (sync_tx, sync_rx) = sync_channel(1);
        let (stream_tx, stream_rx) = channel();

        log::info!("  Running Tests for Actor : {}", test.actor.name());
        let mut tester = Tester::new();
        // TODO Need to test if this error. If that's the case then continue
        match tester.deploy_target_actor(test.actor) {
            Err(err) => {
                dbg!(&err);
                return Err(err.into());
            }
            _ => {}
        };
        let verbosity = args.verbosity;
        thread::spawn(move || stream_results(stream_rx, sync_tx, verbosity));
        dbg!("---------------------------------------------");
        // TODO Need to test if this error. If that's the case then continue. Might also be sent to stream result, to check
        let _results = tester.test(&test.tests, Some(stream_tx))?;
        dbg!("BAAAAAAAAAAAAAAAAAAAAAAAABO");
        sync_rx
            .recv()
            .expect("Should be able to sync the end of streaming results");
    }
    Ok(())
}

/// Stream the results received from `Tester::test,
/// so that users see the result of each test as soon as it finishes.
fn stream_results(
    stream: Receiver<(WasmActor, TestResult)>,
    sync_tx: SyncSender<()>,
    verbosity: u8,
) {
    let mut tests_failed = vec![];
    let mut tests_passed = vec![];
    let mut result = "ok".green();
    for (_actor, test_result) in stream {
        log::info!("{test_result}");
        match test_result.ret() {
            TestResultType::Passed(apply_ret) | TestResultType::Failed(apply_ret) => {
                log::info!("(gas consumption: {})", apply_ret.msg_receipt.gas_used);
                if verbosity >= 2 {
                    print_verbose_traces(apply_ret);
                }
                if test_result.passed() {
                    tests_passed.push(test_result);
                } else {
                    tests_failed.push(test_result);
                }
            }
            TestResultType::Erred(_) => {
                tests_failed.push(test_result);
            }
        }
    }

    // After each and every test result has been printed,
    // we print the sum of failed and passed tests.
    if !tests_failed.is_empty() {
        result = "FAILED".bright_red();
        log::info!("\nfailures:");
        for f in tests_failed.iter() {
            log::info!("test {}", f.method());
            match (f.method().r#type(), f.ret()) {
                (_, TestResultType::Erred(err)) => {
                    dbg!("BOOOOOOOBA");
                    log::info!("{}", f.method());
                    log::info!("\nError: {err}");
                }
                (MethodType::Test, TestResultType::Failed(apply_ret)) => {
                    let info = apply_ret
                        .failure_info
                        .as_ref()
                        .expect("Failure info should be available");
                    log::info!("failed: {info}");
                }
                (MethodType::TestFail, TestResultType::Failed(_)) => {
                    log::info!("failed: test exited with exit code 0");
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

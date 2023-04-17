use crate::utils::search::search_files;
use clap::{ArgAction, Args};
use colored::Colorize;
use kythera_lib::{ApplyRet, ExecutionEvent, TestResult, TestResultType, Tester, WasmActor};
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
        let mut tester = Tester::new();
        tester.deploy_target_actor(test.actor)?;
        let verbosity = args.verbosity;
        thread::spawn(move || stream_results(stream_rx, sync_tx, verbosity));
        let _results = tester.test(&test.tests, Some(stream_tx))?;
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
                if verbosity >= 1 {
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
            match f.ret() {
                TestResultType::Erred(err) => {
                    log::info!("Error: {err}");
                }
                TestResultType::Failed(apply_ret) => {
                    let info = apply_ret
                        .failure_info
                        .as_ref()
                        .expect("Failure info should be available");
                    log::info!("failed: {info}");
                }
                TestResultType::Passed(_) => panic!("Test should have failed"),
            }
        }
    }
    log::info!(
        "test result: {result}. {} passed; {} failed",
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

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Write};

    use assert_cmd::Command;
    use kythera_lib::{to_vec, Abi, Method};
    use kythera_test_actors::wasm_bin::{BASIC_TEST_ACTOR_BINARY, FAILED_TEST_ACTOR_BINARY};
    use predicates::str::contains;
    use tempfile::{tempdir, TempDir};

    const TARGET_WAT: &str = r#"
        ;; Mock invoke function
            (module
                (func (export "invoke") (param $x i32) (result i32)
                    (i32.const 1)
                )
            )
        "#;

    fn create_target_and_test_actors(actor: &[u8], test_actor_abi: &Abi) -> TempDir {
        let dir = tempdir().unwrap();
        let dir_path = dir.path();

        let mut target_actor_file = File::create(dir_path.join("Target.wasm")).unwrap();
        target_actor_file
            .write_all(&wat::parse_str(TARGET_WAT).unwrap())
            .unwrap();
        target_actor_file.flush().unwrap();

        let mut target_actor_abi_file = File::create(dir_path.join("Target.cbor")).unwrap();
        target_actor_abi_file
            .write_all(
                &to_vec(&Abi {
                    constructor: None,
                    set_up: None,
                    methods: vec![],
                })
                .unwrap(),
            )
            .unwrap();
        target_actor_abi_file.flush().unwrap();

        let mut test_actor_file = File::create(dir_path.join("Target.t.wasm")).unwrap();
        test_actor_file.write_all(&Vec::from(actor)).unwrap();
        test_actor_file.flush().unwrap();

        let mut test_actor_abi_file = File::create(dir_path.join("Target.t.cbor")).unwrap();
        test_actor_abi_file
            .write_all(&to_vec(test_actor_abi).unwrap())
            .unwrap();
        test_actor_abi_file.flush().unwrap();

        dir
    }

    #[test]
    fn outputs_passed_tests() {
        let dir = create_target_and_test_actors(
            BASIC_TEST_ACTOR_BINARY,
            &Abi {
                constructor: None,
                set_up: None,
                methods: vec![
                    Method::new_from_name("TestOne").unwrap(),
                    Method::new_from_name("TestTwo").unwrap(),
                ],
            },
        );
        let mut cmd = Command::cargo_bin("kythera").unwrap();
        cmd.args(["test", &dir.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(contains("Running Tests for Actor : Target.wasm"))
            .stdout(contains("Testing 1 test files"))
            .stdout(contains("testing Target.t.wasm 2 methods"))
            .stdout(contains("test 3948827889 - TestOne ... ok"))
            .stdout(contains("test 891686990 - TestTwo ... ok"))
            .stdout(contains("test result: ok. 2 passed; 0 failed"));
    }

    #[test]
    fn outputs_error_tests() {
        let dir = create_target_and_test_actors(
            BASIC_TEST_ACTOR_BINARY,
            &Abi {
                constructor: None,
                set_up: None,
                methods: vec![
                    Method::new_from_name("NonExistentTest").unwrap(),
                    Method::new_from_name("TestTwo").unwrap(),
                ],
            },
        );
        let mut cmd = Command::cargo_bin("kythera").unwrap();
        cmd.args(["test", &dir.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(contains("Running Tests for Actor : Target.wasm"))
            .stdout(contains("Testing 1 test files"))
            .stdout(contains("testing Target.t.wasm 2 methods"))
            .stdout(contains("test 4080437639 - NonExistentTest ... FAILED"))
            .stdout(contains("test 891686990 - TestTwo ... ok"))
            .stdout(contains("failures:"))
            .stdout(contains("test 4080437639 - NonExistentTest"))
            .stdout(contains("failed: message failed with backtrace:"))
            .stdout(contains(
                "(method 4080437639) -- Unknown method number (22)",
            ))
            .stdout(contains("test result: FAILED. 1 passed; 1 failed"));
    }
    #[test]
    fn outputs_failed_tests() {
        let dir = create_target_and_test_actors(
            FAILED_TEST_ACTOR_BINARY,
            &Abi {
                constructor: None,
                set_up: None,
                methods: vec![Method::new_from_name("TestFailed").unwrap()],
            },
        );
        let mut cmd = Command::cargo_bin("kythera").unwrap();
        cmd.args(["test", &dir.path().to_str().unwrap()])
            .assert()
            .success()
            .stdout(contains("Running Tests for Actor : Target.wasm"))
            .stdout(contains("Testing 1 test files"))
            .stdout(contains("testing Target.t.wasm 1 methods"))
            .stdout(contains("test 1857827781 - TestFailed ... FAILED"))
            .stdout(contains("failures:"))
            .stdout(contains("test 1857827781 - TestFailed"))
            .stdout(contains("failed: message failed with backtrace:"))
            .stdout(contains(
                "(method 1857827781) -- panicked at \'assertion failed: `(left == right)`",
            ))
            .stdout(contains("left: `2`,"))
            .stdout(contains(
                "right: `3`\', test_actors/actors/failed_test_actor/src/actor.rs:41:5 (24)",
            ))
            .stdout(contains("test result: FAILED. 0 passed; 1 failed"));
    }
}

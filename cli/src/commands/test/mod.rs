use crate::utils::search::search_files;
use clap::Args;
use kythera_lib::{TestResult, TestResultType, Tester, WasmActor};
use std::{
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    thread,
};

/// Kythera test command cli arguments.
#[derive(Args, Debug)]
pub(crate) struct TestArgs {
    /// Actor files dir.
    path: PathBuf,
}

/// Kythera cli test command.
pub(crate) fn test(args: &TestArgs) -> anyhow::Result<()> {
    let tests = search_files(&args.path)?;
    for test in tests {
        let (tx, rx) = channel();
        let mut tester = Tester::new();
        println!("Running Tests for Actor : {}", test.actor);
        tester.deploy_target_actor(test.actor)?;
        println!("running {} tests", test.tests.len());
        thread::spawn(move || stream_results(rx));
        let _results = tester.test(&test.tests, Some(tx))?;
    }
    Ok(())
}

fn stream_results(stream: Receiver<(WasmActor, TestResult)>) {
    let mut tests_failed = vec![];
    let mut tests_passed = vec![];
    let mut result = "ok";
    for (_actor, test_result) in stream {
        println!("{test_result}");
        if !test_result.passed() {
            result = "FAILED";
            tests_failed.push(test_result);
        } else {
            tests_passed.push(test_result);
        }
    }
    if !tests_failed.is_empty() {
        println!("failures:");
        for f in tests_failed.iter() {
            println!("test {}", f.method());
            match f.ret() {
                TestResultType::Erred(err) => {
                    println!("Error: {err}");
                }
                TestResultType::Failed(apply_ret) => {
                    let info = apply_ret
                        .failure_info
                        .as_ref()
                        .expect("Failure info should be available");
                    println!("failed: {info}");
                }
                TestResultType::Passed(_) => panic!("Test should have failed"),
            }
        }
    }
    println!(
        "test result: {result}. {} passed; {} failed",
        tests_passed.len(),
        tests_failed.len()
    );
}

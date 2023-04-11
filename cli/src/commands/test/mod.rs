use crate::utils::search::search_files;
use clap::Args;
use kythera_lib::Tester;
use std::path::PathBuf;

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
        let mut tester = Tester::new();
        tester.deploy_target_actor(test.actor)?;
        tester.test(&test.tests, None)?;
    }
    Ok(())
}

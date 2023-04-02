// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::path::PathBuf;

use clap::{Args, Parser};
use kythera_lib::Tester;

mod search;

/// Kythera, a Toolset for Filecoin Virtual Machine Native Actor development, testing and deployment.
#[derive(Parser, Debug)]
#[command(version)]
enum MainArgs {
    #[clap(visible_alias = "t")]
    Test(TestArgs),
}

/// Run an Actor tests.
#[derive(Args, Debug)]
struct TestArgs {
    /// Actor files dir.
    path: PathBuf,
}

/// Test
fn test(args: TestArgs) -> anyhow::Result<()> {
    let tests = search::search_files(&args.path)?;
    for test in tests {
        let mut tester = Tester::new();
        tester.deploy_target_actor(test.actor)?;
        tester.test(&test.tests, None)?;
    }
    Ok(())
}
fn main() -> anyhow::Result<()> {
    let args = MainArgs::parse();
    match args {
        MainArgs::Test(args) => test(args)?,
    };

    Ok(())
}

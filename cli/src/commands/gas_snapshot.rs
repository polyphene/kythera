// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use kythera_lib::Tester;
use serde::{Deserialize, Serialize};

use crate::utils::search::search_files;

/// Kythera gas_snapshot command cli arguments.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Actor files dir.
    #[clap(long)]
    path: PathBuf,

    /// Output a diff against a pre-existing snapshot.
    ///
    /// By default, the comparison is done with .gas-snapshot.
    #[clap(long, conflicts_with = "check")]
    diff: Option<Option<PathBuf>>,

    /// Compare against a pre-existing snapshot, exiting with code 1 if they do not match.
    ///
    /// Outputs a diff if the snapshots do not match.
    ///
    /// By default, the comparison is done with .gas-snapshot.
    #[clap(long, conflicts_with = "diff")]
    check: Option<Option<PathBuf>>,
}

/// Method name with its cost.
#[derive(Debug, Deserialize, Serialize)]
pub struct MethodCost {
    name: String,
    cost: u64,
    passed: bool,
}

/// Kythera cli test command.
pub fn snapshot(args: &Args) -> Result<()> {
    log::info!("Generating gas snapshot");
    let methods = generate(&args.path)?;

    if let Some(path) = &args.diff {
        let path = path
            .as_deref()
            .unwrap_or_else(|| Path::new(".gas-snapshot"));
        return diff(&methods, path);
    }

    if let Some(path) = &args.check {
        let path = path
            .as_deref()
            .unwrap_or_else(|| Path::new(".gas-snapshot"));
        if check(&methods, path)? {
            std::process::exit(0)
        } else {
            std::process::exit(1)
        }
    }

    let file = File::create(".gas-snapshot")?;
    let mut wtr = csv::Writer::from_writer(file);
    // we need to serialze each method instead of a Vec of them for readibility.
    // see https://github.com/BurntSushi/rust-csv/issues/221#issuecomment-767653324
    for method in methods {
        wtr.serialize(method)?;
    }
    wtr.flush()?;

    Ok(())
}

/// Output a diff between the `[MethodCost]`s from the [`TestResult`]s and the gas snapshot
// provided in the input path.
fn diff(methods: &[MethodCost], path: &Path) -> Result<()> {
    let file = File::open(path).context("Could not open diff file")?;
    let mut rdr = csv::Reader::from_reader(file);
    let former = rdr
        .deserialize::<MethodCost>()
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|c| (c.name.clone(), c))
        .collect::<HashMap<_, _>>();
    let mut total = 0;

    for method in methods {
        if let Some(c) = former.get(&method.name) {
            log::info!(
                "{} : gas used: {} % ",
                method.name,
                method.cost as f64 / c.cost as f64 * 100f64
            );
            total += method.cost - c.cost;
        }
    }
    log::info!("Total gas dif: {total}");
    Ok(())
}

/// Compare the `[MethodCost]`s from the [`TestResult`]s and the gas snapshot
/// provided in the input path. Returns true if inputs are the same.
fn check(methods: &[MethodCost], path: &Path) -> Result<bool> {
    let mut equal = true;
    let file = File::open(path).context("Could not open check file")?;
    let mut rdr = csv::Reader::from_reader(file);
    let former = rdr
        .deserialize::<MethodCost>()
        .into_iter()
        .filter_map(|r| r.ok())
        .map(|c| (c.name.clone(), c))
        .collect::<HashMap<_, _>>();

    for method in methods {
        match former.get(&method.name) {
            Some(c) => {
                log::info!(
                    "{} : gas used: {} % ",
                    method.name,
                    method.cost as f64 / c.cost as f64 * 100f64
                );
            }
            None => {
                log::error!(
                    "No matching snapshot entry found for \"{}\" in snapshot file",
                    method.name
                );
                equal = false;
            }
        }
    }

    Ok(equal)
}

/// Generate on the provided path a csv with the gas cost of the list of [`TestResult`]s.
fn generate(path: &Path) -> Result<Vec<MethodCost>> {
    let mut costs = vec![];
    let test_files = search_files(path)?;
    for test_file in test_files {
        let mut tester = Tester::new();
        let actor_name = test_file.actor.name().to_string();
        tester.deploy_target_actor(test_file.actor)?;
        for test in test_file.tests {
            let test_results = tester.test(&test, None)?;
            let mut passed;
            for result in test_results {
                let ret = match result.ret() {
                    kythera_lib::TestResultType::Passed(ret) => {
                        passed = true;
                        ret
                    }
                    kythera_lib::TestResultType::Failed(ret) => {
                        passed = false;
                        ret
                    }
                    kythera_lib::TestResultType::Erred(_) => continue,
                };
                let name = format!("{}::{}", actor_name, result.method().name());
                let cost = ret.msg_receipt.gas_used;
                costs.push(MethodCost { name, cost, passed });
            }
        }
    }
    Ok(costs)
}

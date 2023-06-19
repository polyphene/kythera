// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::builder::ValueHint;
use colored::Colorize;
use kythera_lib::Tester;
use serde::{Deserialize, Serialize};

use crate::utils::search::search_files;

/// Kythera gas_snapshot command cli arguments.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Actor files dir.
    path: PathBuf,

    /// Output file for the snapshot.
    #[clap(
        long,
        default_value = ".gas-snapshot",
        value_hint = ValueHint::FilePath,
        value_name = "FILE",
    )]
    snap: PathBuf,

    /// Output a diff against a pre-existing snapshot.
    ///
    /// By default, the comparison is done with .gas-snapshot.
    #[clap(long, conflicts_with = "snap", value_hint = ValueHint::FilePath)]
    diff: Option<Option<PathBuf>>,

    /// Compare against a pre-existing snapshot, exiting with code 1 if they do not match.
    ///
    /// Outputs a diff if the snapshots do not match.
    ///
    /// By default, the comparison is done with .gas-snapshot.
    #[clap(long, conflicts_with = "diff", value_hint = ValueHint::FilePath)]
    check: Option<Option<PathBuf>>,
}

/// Method name with its cost.
#[derive(Debug, Deserialize, Serialize)]
pub struct MethodCost {
    pub name: String,
    pub cost: u64,
    pub passed: bool,
}

/// Kythera cli test command.
pub fn snapshot(args: &Args) -> Result<()> {
    let methods = generate(&args.path)?;
    log::info!("\nGenerating gas snapshot");
    if let Some(path) = args.diff.as_ref().or(args.check.as_ref()) {
        let check = args.check.is_some();

        let path = path
            .as_deref()
            .unwrap_or_else(|| &Path::new(".gas-snapshot"));

        let equal = diff(&methods, path, check)?;
        if check && !equal {
            std::process::exit(1)
        }
        std::process::exit(0)
    }

    let file = File::create(&args.snap).context("Could not create snapshot file")?;
    let mut wtr = csv::Writer::from_writer(file);
    // we need to serialize each method instead of a Vec of them for readability.
    // see https://github.com/BurntSushi/rust-csv/issues/221#issuecomment-767653324
    for method in methods {
        wtr.serialize(method)?;
    }
    wtr.flush()?;

    Ok(())
}

/// Output a diff between the `[MethodCost]`s from the [`TestResult`]s and the gas snapshot
/// provided in the input path. If `check` is true prints the methods not present in the gas snapshot.
/// Returns true if the the inputs are the same.
fn diff(methods: &[MethodCost], path: &Path, check: bool) -> Result<bool> {
    let file = File::open(path).context("Could not open diff file")?;
    let mut rdr = csv::Reader::from_reader(file);
    let former = rdr
        .deserialize::<MethodCost>()
        .filter_map(|r| r.ok())
        .map(|c| (c.name.clone(), c))
        .collect::<HashMap<_, _>>();
    let mut total = 0;

    for method in methods {
        match (former.get(&method.name), check) {
            (Some(c), _) => {
                let new = method.cost;
                let old = c.cost;
                match old.cmp(&new) {
                    std::cmp::Ordering::Equal => {
                        log::info!("{}: gas used is the same: {}", method.name, new);
                    }
                    std::cmp::Ordering::Less => {
                        let mut more = (new - old) as f64 / old as f64 * 100.0;
                        if more >= 1 as f64 {
                            more = more.round() as f64;
                        }
                        log::info!("{}: gas used is {}% more: {new}", method.name, more);
                    }
                    std::cmp::Ordering::Greater => {
                        let mut less = (old - new) as f64 / old as f64 * 100.0;
                        if less >= 1 as f64 {
                            less = less.round() as f64;
                        }

                        log::info!("{}: gas used is {}% less: {new}", method.name, less);
                    }
                }

                total += c.cost as i64 - method.cost as i64;
            }
            (None, true) => {
                let message = format!(
                    "No matching snapshot entry found for \"{}\" in snapshot file",
                    method.name
                )
                .red();
                log::error!("{}", message);
                return Ok(false);
            }
            (None, false) => {}
        }
    }
    log::info!("Total gas diff: {total}");
    Ok(total == 0)
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

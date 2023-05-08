// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

mod commands;
mod utils;

use commands::{gas_snapshot, test};
use env_logger::Target;
use log::LevelFilter;

use std::io::Write;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Kythera")]
#[command(author, version, about, arg_required_else_help(true))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(visible_alias = "t")]
    Test(test::Args),
    Snapshot(gas_snapshot::Args),
}

fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter(Some("kythera_lib"), LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .target(Target::Stdout)
        .init();

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Test(args)) => test::test(args)?,
        Some(Commands::Snapshot(args)) => gas_snapshot::snapshot(args)?,
        // Help is printed via `arg_required_else_help` in the `Cli` derive `command`.
        None => {}
    }
    Ok(())
}

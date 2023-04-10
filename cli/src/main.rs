// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

mod commands;
mod utils;

use commands::test;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Kythera")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(visible_alias = "t")]
    Test(test::TestArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Test(args)) => test::test(args)?,
        None => {}
    }
    Ok(())
}

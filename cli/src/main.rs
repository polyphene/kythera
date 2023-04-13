// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

mod commands;
mod utils;

use commands::test;
use env_logger::Target;
use log::LevelFilter;

use std::io::Write;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Kythera")]
#[command(author, version, about)]
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
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter(Some("kythera_lib"), LevelFilter::Info)
        .format(|buf, record| writeln!(buf, "{}", record.args()))
        .target(Target::Stdout)
        .init();

    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Test(args)) => test::test(args)?,
        None => {}
    }
    Ok(())
}

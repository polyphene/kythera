use crate::utils::repo::helpers::absolute_path;
use crate::CliContext;
use clap::Subcommand;
use std::fs;

// Tmp sub command to be removed TODO
#[derive(Subcommand)]
pub(crate) enum TmpSubCommands {
    PrintConfig {},
}

pub(crate) fn print_context(context: CliContext) -> () {
    println!(
        "actors_bin_dir: {}",
        absolute_path(context.actors_bin_dir)
            .unwrap()
            .to_string_lossy()
    )
}

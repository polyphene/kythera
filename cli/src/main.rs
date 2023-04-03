mod commands;
mod utils;

use commands::test;
use commands::tmp;

use utils::context::CliContext;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "Kythera")]
#[command(bin_name = "kythera")]
#[command(author = "Polyphene")]
#[command(
    about = "Kythera is a Toolset for Filecoin Virtual Machine Native Actor development, testing and deployment."
)]
#[command(version)] // Read from `Cargo.toml`
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(visible_alias = "t")]
    Test(test::TestArgs),
    /// does temporary testing things
    Tmp {
        #[command(subcommand)]
        command: Option<tmp::TmpSubCommands>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Test(args)) => test::test(args)?,
        Some(Commands::Tmp {
            command: sub_command,
        }) => {
            match &sub_command {
                Some(tmp::TmpSubCommands::PrintConfig {}) => {
                    let context = CliContext::new()?;
                    tmp::print_context(context)
                }
                None => {}
            };
        }
        None => {}
    }
    Ok(())
}

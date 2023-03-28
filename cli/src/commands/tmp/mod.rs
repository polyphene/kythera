use clap::Subcommand;

// Tmp sub command to be removed TODO
#[derive(Subcommand)]
pub(crate) enum TmpSubCommands {
    PrintConfig {}
}

pub(crate) fn print_config() -> anyhow::Result<()> {
    Ok(())
}
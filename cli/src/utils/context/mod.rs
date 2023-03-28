use std::env;
use std::fs::File;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_yaml;
use anyhow::{anyhow, Context};
use crate::utils::constants::CONFIG_FILE;


#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("failed to open configuration file")]
    FailedToOpenConfFile,
    #[error("invalid configuration file")]
    InvalidConfFile,
}

/// Context structure helping accessing the repository area in a consistent way throughout the CLI
/// commands.
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CliContext {
    pub(crate) actors_bin_dir: PathBuf,
}

impl CliContext {
    /// Public function helping to initialize a [ Context ] object.
    pub(crate) fn new() -> anyhow::Result<Self> {
        let root_path = env::current_dir()?;
        let config_file_path = root_path.join(CONFIG_FILE);

        if config_file_path.exists() {
            let config_file =
                File::open(&config_file_path).context(Error::FailedToOpenConfFile)?;
            serde_yaml::from_reader(config_file)
                .context(Error::InvalidConfFile)
        } else {
            Ok(CliContext {
                actors_bin_dir: root_path.join("Todo")
            })
        }
    }
}

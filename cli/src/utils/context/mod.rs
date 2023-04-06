use std::env;
use std::fs::File;
use std::path::PathBuf;

use optional_struct::*;

use crate::utils::repo::helpers::to_relative_path_to_project_root;
use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Name of the configuration file.
const CONFIG_FILE: &str = "kythera.config.yml";

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("failed to open configuration file")]
    FailedToOpenConfFile,
    #[error("invalid configuration file")]
    InvalidConfFile,
    #[error("error with actors_bin_dir")]
    FailedToGetActorsBinDirAsStr,
}

/// Context structure helping accessing the repository area in a consistent way throughout the CLI
/// commands.
#[optional_struct]
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone)]
pub(crate) struct CliContext {
    pub(crate) actors_bin_dir: PathBuf,
}

impl CliContext {
    /// Public function helping to initialize a [ Context ] object.
    pub(crate) fn new() -> anyhow::Result<Self> {
        let root_path = env::current_dir()?;
        let config_file_path = root_path.join(CONFIG_FILE);
        // fetch config from configuration file if it exists
        let config = if config_file_path.exists() {
            let config_file = File::open(&config_file_path).context(Error::FailedToOpenConfFile)?;
            serde_yaml::from_reader(config_file).context(Error::InvalidConfFile)?
        } else {
            OptionalCliContext::default()
        };
        // create context object from fetched configuration and default values
        let context = CliContext {
            actors_bin_dir: config
                .actors_bin_dir
                .unwrap_or_else(|| root_path.join("artifacts")),
        };
        // secure context by checking that targeted paths are part of the project
        let actors_bin_dir_str = context
            .actors_bin_dir
            .to_str()
            .context(Error::FailedToGetActorsBinDirAsStr)
            .unwrap();
        to_relative_path_to_project_root(actors_bin_dir_str)?;
        Ok(context)
    }
}

// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

/// Kythera testing-framework errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Actor not loaded")]
    MissingActor,
    #[error("Could not set Actor: {name} on the BlockStore")]
    SettingActor {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Sync + Send>,
    },
    #[error("Tester error: {msg}")]
    Tester {
        msg: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Sync + Send>>,
    },
}

/// Helper trait for adding custom messages to inner Fvm errors.
pub trait WrapFVMError<T> {
    /// Wrap the source `Error` with an `Error::Tester`.
    fn tester_err(self, msg: &str) -> Result<T, Error>;

    /// Wrap the source `Error` with an `Error::SettingActor`.
    fn setting_err(self, msg: &str) -> Result<T, Error>;
}

impl<T, E> WrapFVMError<T> for Result<T, E>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn tester_err(self, msg: &str) -> Result<T, Error> {
        self.map_err(|err| Error::Tester {
            msg: msg.into(),
            source: Some(err.into()),
        })
    }

    fn setting_err(self, name: &str) -> Result<T, Error> {
        self.map_err(|err| Error::SettingActor {
            name: name.into(),
            source: err.into(),
        })
    }
}

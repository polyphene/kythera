// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

/// Kythera common errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not generate method number for `{name}`")]
    MethodNumberGeneration {
        name: String,
        #[source]
        source: Box<dyn std::error::Error + Sync + Send>,
    },
}

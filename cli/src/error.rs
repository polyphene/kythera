// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

/// Kythera cli errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to convert abi path to string")]
    FailedConversion,
}

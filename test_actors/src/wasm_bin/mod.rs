// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
// constants for wasm build artifacts

macro_rules! wasm_bin {
    ($x: expr) => {
        concat!(
            env!("OUT_DIR"),
            "/bundle/wasm32-unknown-unknown/wasm/",
            $x,
            ".wasm"
        )
    };
}

// Integration test actors.
pub const BASIC_TEST_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("basic_test_actor"));
pub const CONSTRUCTOR_TEST_ACTOR_BINARY: &[u8] =
    include_bytes!(wasm_bin!("constructor_test_actor"));
pub const SETUP_TEST_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("setup_test_actor"));

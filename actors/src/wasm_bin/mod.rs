// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
// Constants for wasm build artifacts.

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

pub const CHEATCODES_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("cheatcodes_actor"));

#[cfg(feature = "testing")]
pub mod test_actors {
    // Integration tests actors.
    pub const BASIC_TEST_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("basic_test_actor"));
    pub const BASIC_TARGET_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("basic_target_actor"));
    pub const BUILTINS_TEST_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("builtins_test_actor"));
    pub const CHEATCODES_TEST_ACTOR_BINARY: &[u8] =
        include_bytes!(wasm_bin!("cheatcodes_test_actor"));
    pub const FAIL_TEST_ACTOR_BINARY: &[u8] = include_bytes!(wasm_bin!("fail_test_actor"));
}

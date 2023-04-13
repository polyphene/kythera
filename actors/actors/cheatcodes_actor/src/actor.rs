// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

/// Deserialize message parameters into given struct.
pub fn deserialize_params<D: DeserializeOwned>(params: u32) -> D {
    let params = fvm_sdk::message::params_raw(params)
        .expect("Could not get message parameters")
        .expect("Expected message parameters but got none");

    let params = RawBytes::new(params.data);

    params
        .deserialize()
        .expect("Should be able to deserialize message params into arguments of called method")
}

#[no_mangle]
fn invoke(input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Warp" => {
                // Ensure that the message params can be deserialized.
                let new_timestamp: u64 = deserialize_params(input);

                Warp(new_timestamp);

                NO_DATA_BLOCK_ID
            },
            _ => {
                fvm_sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

/// Warp the machine context to a given timestamp. Aside the handling of the message in the Kernel,
/// there is currently nNo additional logic is currently needed in the execution.
#[allow(non_snake_case)]
fn Warp(_new_timestamp: u64) {}

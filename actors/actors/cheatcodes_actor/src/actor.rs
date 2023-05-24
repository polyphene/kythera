// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::address::Address;
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
            "Epoch" => {
                // Ensure that the message params can be deserialized.
                let new_epoch: i64 = deserialize_params(input);

                Epoch(new_epoch);

                NO_DATA_BLOCK_ID
            },
            "Fee" => {
                // Ensure that the message params can be deserialized.
                let (lo, hi): (u64, u64) = deserialize_params(input);

                Fee(
                    fvm_shared::sys::TokenAmount {
                        lo,
                        hi
                    }
                );

                NO_DATA_BLOCK_ID
            },
            "ChainId" => {
                // Ensure that the message params can be deserialized.
                let new_chain_id: u64 = deserialize_params(input);

                ChainId(new_chain_id);

                NO_DATA_BLOCK_ID
            },
            "Prank" => {
                // Ensure that the message params can be deserialized.
                let new_caller: Address = deserialize_params(input);

                Prank(new_caller);

                NO_DATA_BLOCK_ID
            },
            "Trick" => {
                // Ensure that the message params can be deserialized.
                let new_origin: Address = deserialize_params(input);

                Trick(new_origin);

                NO_DATA_BLOCK_ID
            },
            "Log" => {
                let message: String = deserialize_params(input);

                Log(message);

                NO_DATA_BLOCK_ID

            }
            _ => {
                fvm_sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

/// Warp the machine context to a given timestamp.
#[allow(non_snake_case)]
fn Warp(_new_timestamp: u64) {}

/// Update the machine context to a given epoch.
#[allow(non_snake_case)]
fn Epoch(_new_epoch: i64) {}

/// Update the base fee that's in effect when the machine runs.
#[allow(non_snake_case)]
fn Fee(_new_fee: fvm_shared::sys::TokenAmount) {}

/// Set a new chain Id in the machine context.
#[allow(non_snake_case)]
fn ChainId(_new_chain_id: u64) {}

/// Prank the call manager to set a pre-determined caller for the next message sent.
#[allow(non_snake_case)]
fn Prank(_new_caller: Address) {}

/// Trick the call manager to set a pre-determined origin for the next message sent.
#[allow(non_snake_case)]
fn Trick(_new_origin: Address) {}

/// Log a message from the test actor on Stdout.
#[allow(non_snake_case)]
fn Log(_message: String) {}

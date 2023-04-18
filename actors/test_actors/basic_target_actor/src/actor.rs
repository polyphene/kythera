// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk as sdk;
use fvm_shared::error::ExitCode;
use sdk::sys::ErrorNumber;
use serde::ser;
use thiserror::Error;

#[derive(Error, Debug)]
enum IpldError {
    #[error("ipld encoding error: {0}")]
    Encoding(#[from] fvm_ipld_encoding::Error),
    #[error("ipld blockstore error: {0}")]
    Blockstore(#[from] ErrorNumber),
}

fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
where
    T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Caller" => {
                let mc_caller: u64 = unsafe { fvm_sdk::sys::vm::message_context().unwrap().caller };

                return_ipld(&mc_caller).unwrap()
            },
            "Origin" => {
                let mc_origin: u64 = unsafe { fvm_sdk::sys::vm::message_context().unwrap().origin };

                return_ipld(&mc_origin).unwrap()
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

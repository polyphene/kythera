// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

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

// TODO use helix frc42_dispatch when their dependencies are up to date.
#[no_mangle]
fn invoke(_input: u32) -> u32 {
    let method_num = sdk::message::method_number();
    match method_num {
        3948827889 => return_ipld(TestOne()).unwrap(),
        891686990 => return_ipld(TestTwo()).unwrap(),
        _ => {
            sdk::vm::abort(
                ExitCode::USR_UNHANDLED_MESSAGE.value(),
                Some("Unknown method number"),
            );
        }
    }
}

#[allow(non_snake_case)]
fn TestOne() -> &'static str {
    "TestOne"
}

#[allow(non_snake_case)]
fn TestTwo() -> &'static str {
    "TestTwo"
}

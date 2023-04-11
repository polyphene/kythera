// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use fvm_ipld_encoding::ipld_block::IpldBlock;
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::address::Address;
use fvm_shared::bigint::Zero;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::sys::SendFlags;

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("{info}")),
        )
    }));

    let method_num = sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestWarp" => {
                TestWarp();

                NO_DATA_BLOCK_ID
            },
            _ => {
                sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

// Checks that all relevant builtins are deployed at a correct actor Id in Kythera
#[allow(non_snake_case)]
fn TestWarp() {
    let timestamp = fvm_sdk::network::tipset_timestamp();

    assert_eq!(timestamp, 0u64);

    let new_timestamp = 10000u64;
    let bytes = fvm_ipld_encoding::to_vec(&new_timestamp).unwrap();

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        112632689,
        Some(IpldBlock::serialize(DAG_CBOR, bytes.as_slice()).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let timestamp = fvm_sdk::network::tipset_timestamp();

    assert_eq!(timestamp, new_timestamp);
}

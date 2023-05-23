// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::{match_method, method_hash};
use fvm_ipld_encoding::ipld_block::IpldBlock;
use fvm_ipld_encoding::DAG_CBOR;
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::address::Address;
use fvm_shared::bigint::Zero;
use fvm_shared::econ::TokenAmount;
use fvm_shared::error::ExitCode;
use fvm_shared::sys::SendFlags;
use paste::paste;

macro_rules! declare_match_method {
    (
        $input:expr, $($name:literal => $func:path,)*
    ) => {
        let method_num = sdk::message::method_number();
        match_method! {
            method_num,
            {
                $($name => {
                    $func($input);
                    NO_DATA_BLOCK_ID
                }),*
                _ => {
                    sdk::vm::abort(
                        ExitCode::USR_UNHANDLED_MESSAGE.value(),
                        Some("Unknown method number"),
                    );
                },
            }
        }
    };
}

macro_rules! declare_tests_fail {
    ($($method:literal),*) => {
        $(
            paste! {
                #[allow(non_snake_case)]
                fn [<TestFailDeserialization $method>](_input: u32) {
                    let new_timestamp = String::from("timestamp");

                    fvm_sdk::send::send(
                        &Address::new_id(98),
                        method_hash!($method),
                        Some(IpldBlock::serialize(DAG_CBOR, &new_timestamp).unwrap()),
                        TokenAmount::zero(),
                        None,
                        SendFlags::empty(),
                    )
                    .unwrap();
                }
                #[allow(non_snake_case)]
                fn [<TestFailNoParameters $method>](_input: u32) {
                    fvm_sdk::send::send(
                        &Address::new_id(98),
                        method_hash!($method),
                        None,
                        TokenAmount::zero(),
                        None,
                        SendFlags::empty(),
                    )
                    .unwrap();
                }
            }
        )*
    };
}

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
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("{info}")),
        )
    }));

    declare_match_method! {
        input,
        "TestFailDeserializationWarp" => TestFailDeserializationWarp,
        "TestFailNoParametersWarp" => TestFailNoParametersWarp,
        "TestWarp" => TestWarp,
        "TestFailDeserializationEpoch" => TestFailDeserializationEpoch,
        "TestFailNoParametersEpoch" => TestFailNoParametersEpoch,
        "TestEpoch" => TestEpoch,
        "TestFailDeserializationFee" => TestFailDeserializationFee,
        "TestFailNoParametersFee" => TestFailNoParametersFee,
        "TestFee" => TestFee,
        "TestFailDeserializationChainId" => TestFailDeserializationChainId,
        "TestFailNoParametersChainId" => TestFailNoParametersChainId,
        "TestChainId" => TestChainId,
        "TestFailDeserializationPrank" => TestFailDeserializationPrank,
        "TestFailNoParametersPrank" => TestFailNoParametersPrank,
        "TestFailAddressTypePrank" => TestFailAddressTypePrank,
        "TestPrank" => TestPrank,
        "TestFailDeserializationTrick" => TestFailDeserializationTrick,
        "TestFailNoParametersTrick" => TestFailNoParametersTrick,
        "TestFailAddressTypeTrick" => TestFailAddressTypeTrick,
        "TestTrick" => TestTrick,
        "TestLog" => TestLog,
    }
}

// Checks Warp cheatcode happy path.
#[allow(non_snake_case)]
fn TestWarp(_input: u32) {
    let timestamp = fvm_sdk::network::tipset_timestamp();

    assert_eq!(timestamp, 0u64);

    let new_timestamp = 10000u64;

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Warp"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_timestamp).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let nc_timestamp = unsafe { fvm_sdk::sys::network::context().unwrap().timestamp };

    assert_eq!(new_timestamp, nc_timestamp);
}

// Checks Epoch cheatcode happy path.
#[allow(non_snake_case)]
fn TestEpoch(_input: u32) {
    let epoch = fvm_sdk::network::curr_epoch();

    assert_eq!(epoch, 0i64);

    let new_epoch = 10000i64;

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Epoch"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_epoch).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let nc_epoch = unsafe { fvm_sdk::sys::network::context().unwrap().epoch };

    assert_eq!(new_epoch, nc_epoch);
}

// Checks Fee cheatcode happy path.
#[allow(non_snake_case)]
fn TestFee(_input: u32) {
    let base_fee_sys =
        fvm_shared::sys::TokenAmount::try_from(fvm_sdk::network::base_fee()).unwrap();
    let lo = base_fee_sys.lo;
    let hi = base_fee_sys.hi;

    assert_eq!(lo, 100u64);
    assert_eq!(hi, 0u64);

    let new_base_fee = (200u64, 200u64);

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Fee"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_base_fee).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let nc_base_fee = unsafe {
        let base_fee = fvm_sdk::sys::network::context().unwrap().base_fee;
        (base_fee.lo, base_fee.hi)
    };

    assert_eq!(new_base_fee, nc_base_fee);
}

// Checks ChainId cheatcode happy path.
#[allow(non_snake_case)]
fn TestChainId(_input: u32) {
    let chain_id = fvm_sdk::network::chain_id();

    assert_eq!(u64::from(chain_id), 1312u64);

    let new_chain_id = 1500u64;

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("ChainId"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_chain_id).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let nc_chain_id = unsafe { fvm_sdk::sys::network::context().unwrap().chain_id };

    assert_eq!(new_chain_id, nc_chain_id);
}

// Checks Prank cheatcode happy path.
#[allow(non_snake_case)]
fn TestPrank(input: u32) {
    let target_actor_id: u64 = deserialize_params(input);

    let new_caller = Address::new_id(1);

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Prank"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_caller).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("Caller"),
        None,
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let caller: u64 = RawBytes::new(
        res.return_data
            .expect("Should be able to get Caller from target actor")
            .data,
    )
    .deserialize()
    .unwrap();

    assert_eq!(new_caller.id().unwrap(), caller);
}

// Checks Prank with a wrong address type.
#[allow(non_snake_case)]
fn TestFailAddressTypePrank(_input: u32) {
    let new_caller = Address::new_actor(b"WrongType");

    fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Prank"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_caller).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();
}

// Checks Trick cheatcode happy path.
#[allow(non_snake_case)]
fn TestTrick(input: u32) {
    let target_actor_id: u64 = deserialize_params(input);

    let new_origin = Address::new_id(1);

    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Trick"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_origin).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("Origin"),
        None,
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let origin: u64 = RawBytes::new(
        res.return_data
            .expect("Should be able to get Origin from target actor")
            .data,
    )
    .deserialize()
    .unwrap();

    assert_eq!(new_origin.id().unwrap(), origin);
}

// Checks Trick with a wrong address type.
#[allow(non_snake_case)]
fn TestFailAddressTypeTrick(_input: u32) {
    let new_origin = Address::new_actor(b"WrongType");

    fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Trick"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_origin).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();
}

// Checks Warp cheatcode happy path.
#[allow(non_snake_case)]
fn TestLog(_input: u32) {
    let params = ("Info", "hello from actor");
    let res = fvm_sdk::send::send(
        &Address::new_id(98),
        method_hash!("Log"),
        Some(IpldBlock::serialize(DAG_CBOR, &params).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
    .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);
}

declare_tests_fail!("Warp", "Epoch", "Fee", "ChainId", "Prank", "Trick");

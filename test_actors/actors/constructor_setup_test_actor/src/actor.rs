// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::{multihash::Code, Cid};
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

#[derive(Serialize_tuple, Deserialize_tuple)]
struct TestConstructorSetupActorState {
    value: u32,
}

impl TestConstructorSetupActorState {
    pub fn load(cid: &Cid) -> Self {
        let data = sdk::ipld::get(cid).unwrap();
        fvm_ipld_encoding::from_slice::<Self>(&data).unwrap()
    }

    pub fn save(&self) -> Cid {
        let serialized = fvm_ipld_encoding::to_vec(self).unwrap();
        let block = Block {
            codec: DAG_CBOR,
            data: serialized,
        };
        sdk::ipld::put(
            Code::Blake2b256.into(),
            32,
            block.codec,
            block.data.as_ref(),
        )
        .unwrap()
    }
}

// TODO use helix frc42_dispatch when their dependencies are up to date.
#[no_mangle]
fn invoke(_input: u32) -> u32 {
    let method_num = sdk::message::method_number();
    match method_num {
        1 => Constructor(),
        3556852554 => Setup(),
        3654954405 => TestConstructorSetup(),
        method => {
            sdk::vm::abort(
                ExitCode::USR_UNHANDLED_MESSAGE.value(),
                Some(&format!("Unknown method number: {method}")),
            );
        }
    }

    NO_DATA_BLOCK_ID
}

#[allow(non_snake_case)]
fn Constructor() {
    let state = TestConstructorSetupActorState { value: 1 };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn Setup() {
    let state = TestConstructorSetupActorState::load(&sdk::sself::root().unwrap());
    let value = state.value + 1;
    let state = TestConstructorSetupActorState { value };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn TestConstructorSetup() {
    let state = TestConstructorSetupActorState::load(&sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 2u32 {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("value is different was not called {value}")),
        )
    }
}

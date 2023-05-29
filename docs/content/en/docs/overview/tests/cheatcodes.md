---
title: "Cheatcodes"
description: ""
lead: ""
date: 2020-10-06T08:48:57+00:00
lastmod: 2020-10-06T08:48:57+00:00
draft: false
images: []
menu:
  docs:
    parent: "tests"
weight: 222
toc: true
---

Most of the time, simply testing your actors outputs isn't enough. To manipulate the state of the blockchain, 
as well as test for specific edge cases, Kythera is shipped with a set of cheatcodes.

Cheatcodes allow you to change the block number, your identity, and more. They are invoked by calling specific functions
on a specially designated actor ID: `98`.

Let's write an actor with a state that can only be updated by a selected actor ID, `250`.
```rust
// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;
use sdk::sys::ErrorNumber;
use serde::ser;
use thiserror::Error;

// Actor's state.
#[derive(Serialize_tuple, Deserialize_tuple)]
struct DemoActorState {
    value: u32,
}

impl DemoActorState {
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

// IPLD Utils.

#[derive(Error, Debug)]
enum IpldError {
    #[error("ipld encoding error: {0}")]
    Encoding(#[from] fvm_ipld_encoding::Error),
    #[error("ipld blockstore error: {0}")]
    Blockstore(#[from] ErrorNumber),
}

// Util to save value in Block and return the block ID
fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
    where
        T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
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
        sdk::vm::abort(ExitCode::FIRST_USER_EXIT_CODE, Some(&format!("{info}")))
    }));

    let method_num = fvm_sdk::message::method_number();

    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();

                NO_DATA_BLOCK_ID
            },
            "Read" => {
                Read()
            },
            "Write" => {
                let write_value: u32 = deserialize_params(input);
                let caller: u64 = unsafe { fvm_sdk::sys::vm::message_context().unwrap().caller };

                assert_eq!(caller, 250u64, "Only actor ID 250 can alter this value");

                Write(write_value);

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

#[allow(non_snake_case)]
fn Constructor() {
    let state = DemoActorState { value: 1 };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn Read() -> u32 {
    let state = DemoActorState::load(&sdk::sself::root().unwrap());

    return_ipld(&state.value).unwrap()
}

#[allow(non_snake_case)]
fn Write(value: u32) {
    let mut state = DemoActorState::load(&sdk::sself::root().unwrap());
    state.value = value;
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

We can now write a basic test case against it:
```rust
// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use frc42_dispatch::method_hash;
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

// IPLD Utils.

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

    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestWrite" => {
                TestWrite(input);

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


#[allow(non_snake_case)]
fn TestWrite(input: u32) {
    let target_actor_id: u64 = deserialize_params(input);

    // Try to write
    let new_value = 15u32;
    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("Write"),
        Some(IpldBlock::serialize(DAG_CBOR, &new_value).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    // Read
    let res = fvm_sdk::send::send(
        &Address::new_id(target_actor_id),
        method_hash!("Read"),
        None,
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    assert_eq!(res.exit_code, ExitCode::OK);

    let value: u32 = RawBytes::new(
        res.return_data
            .expect("Should be able to read target actor")
            .data,
    )
        .deserialize()
        .unwrap();

    assert_eq!(value, new_value);
}
```

If we run `kythera test` now, we will see that the test fails as the actor expects the send actor ID to be `250`:
```shell
$ kythera test ./artifacts

	Running Tests for Actor : Locked.wasm
		Testing 1 test files

Locked.t.wasm: testing 1 tests
test TestWrite ... FAILED
(gas consumption: 2558341)

failures:
test TestWrite
failed: message failed with backtrace:
00: f0104 (method 3427243882) -- panicked at 'assertion failed: `(left == right)`
  left: `ExitCode { value: 16 }`,
 right: `ExitCode { value: 0 }`', tests/locked-test/src/actor.rs:78:5 (24)
01: f0103 (method 254162321) -- panicked at 'assertion failed: `(left == right)`
  left: `104`,
 right: `250`: Only actor ID 250 can alter this value', actors/locked/src/actor.rs:109:17 (16)


test result: FAILED. 0 passed; 1 failed
```

Let's add a step in the code to call our cheatcodes actor to prank the machine in thinking that we are actually the actor
at ID `250`:

```rust
// ...

#[allow(non_snake_case)]
fn TestWrite(input: u32) {
    let target_actor_id: u64 = deserialize_params(input);

    // Prank machine with actor ID 250
    let new_caller = Address::new_id(250);

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

    // ...
}
```

Now, if we run `kythera test` again:
```shell
$ kythera test ./artifacts

	Running Tests for Actor : Locked.wasm
		Testing 1 test files

Locked.t.wasm: testing 1 tests
test TestWrite ... ok
(gas consumption: 5796750)

test result: ok. 1 passed; 0 failed
```

> 📚 **Reference**
>
> See the [Cheatcodes](/docs/reference/cheatcodes/) Reference for a complete overview of all the available cheatcodes.
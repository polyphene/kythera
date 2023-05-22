---
title: "Writing tests"
description: ""
lead: ""
date: 2020-10-06T08:48:57+00:00
lastmod: 2020-10-06T08:48:57+00:00
draft: false
images: []
menu:
  docs:
    parent: "overview"
weight: 221
toc: true
---

Tests ran in Kythera are executed at the Wasm level. This effectively means that tests can be written in any language that supports
compilation to this target. However, it is likely that when developing native actors a developer leverages the same language
for both application actors and test actors.

In this section, we'll go over the basic knowledge necessary to start implementing test actors in a Rust project.

Let's take a look at a basic test:
```rust
// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

/**************************************************
 * Actor's state
 **************************************************/

#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    value: u32,
}

impl ActorState {
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

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::exit(
            ExitCode::USR_ASSERTION_FAILED.value(),
            None,
            Some(&format!("{info}")),
        )
    }));

    let method_num = sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            "Setup" => {
                Setup();
                NO_DATA_BLOCK_ID
            },
            "TestStateValue" => {
                TestStateValue();
                NO_DATA_BLOCK_ID
            },
            "TestFailStateValue" => {
                TestFailStateValue();
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

#[allow(non_snake_case)]
fn Constructor() {
    let state = ActorState { value: 1 };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn Setup() {
    let mut state = ActorState::load(&sdk::sself::root().unwrap());
    state.value += 1;
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn TestStateValue() {
    let state = ActorState::load(&sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 2u32 {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("expected value to be 2, got: {value}")),
        )
    }
}

#[allow(non_snake_case)]
fn TestFailStateValue() {
    let state = ActorState::load(&sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 4u32 {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("value properly not set to 4")),
        )
    }
}

```

Kythera uses the following keywords in tests:
- **`SetUp`**: An optional function invoked after the actor **`Constructor`** and before each test case is run.
```rust
#[allow(non_snake_case)]
fn Setup() {
    let mut state = ActorState::load(&sdk::sself::root().unwrap());
    state.value += 1;
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```
- **`Test`**: Functions prefixed with test are run as a test case.
```rust
#[allow(non_snake_case)]
fn TestStateValue() {
    let state = ActorState::load(&sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 2u32 {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("expected value to be 2, got: {value}")),
        )
    }
}
```
- **`TestFail`**: The inverse of the **`Test`** prefix - if the function does not return an `ExitCode::Ok`, the test fails.
```rust
#[allow(non_snake_case)]
fn TestFailStateValue() {
    let state = ActorState::load(&sdk::sself::root().unwrap());
    let value = state.value;
    if state.value != 4u32 {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("value properly not set to 4")),
        )
    }
}
```

Tests are deployed to the next highest actor Id available in the machine context, to ensure no overlap with external forked state.
If an actor is interacted with within the tests the default sender will be the actor Id associated to the test actor.
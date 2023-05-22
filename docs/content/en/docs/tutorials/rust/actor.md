---
title: "Create an actor"
description: ""
lead: ""
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
contributors: []
menu:
  docs:
    parent: "rust"
weight: 422
toc: true
---

As we previously mentioned, all of our native actors will have to be created in the `actors` folder.

An actor takes the shape of a Rust crate. In the starter you cloned you should already be able to find one crate, `hello-world`.

## Configuring the actor crate

To proceed, we will copy the crate to make it ours:
```shell
$ cp -r actors/hello-world/ actors/basic-actor
$ cd actors/basic-actor/ && ls -la

total 36
  .
  ..
  Cargo.lock
  Cargo.toml
  src
```

First, let's have a look to the `Cargo.toml`:
```toml
[package]
name = "hello-world"
version = "0.1.0"
edition = "2021"

[target.'cfg(target_arch = "wasm32")'.dependencies]
cid = { version = "0.8.5", default-features = false }
frc42_dispatch = "3.1.0"
fvm_sdk = {  version = "3.0.0" }
fvm_shared = {  version = "3.1.0" }
fvm_ipld_blockstore = "0.1.1"
fvm_ipld_encoding = {  version = "0.3.3" }
serde = { version = "1.0.136", features = ["derive"] }
serde_tuple = { version = "0.5.0" }
thiserror = { version = "1.0.31" }

[lib]
crate-type = ["cdylib"]
```

Two things to note:
- Dependencies are specified under `target.'cfg(target_arch = "wasm32")'`. This is because our crate will end up being compiled
to Wasm, so we can focus on specifying dependencies for this target. 
- We are specifying `crate-type = ["cdylib"]`. This is mandatory to be able to properly compile to the `wasm32` target.

To ensure that there are no overlapping, let's change the crate name to `basic-actor`:
```toml
[package]
name = "basic-actor"
version = "0.1.0"
edition = "2021"
```

## Actor's development

For this tutorial, we will create an actor that stores a value updated through the call of an `Add()` method. It
should also be possible to read the current value through a `Read()` method.

### Layout 

Our main logic for the actor will be located in `src/actor.rs`. The content of the file copied from `hello-world` is the 
following:
```rust
use cid::{multihash::Code, Cid};
use frc42_dispatch::match_method;
use fvm_ipld_blockstore::Block;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_sdk::sys::ErrorNumber;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;
use serde::ser;
use thiserror::Error;

// Actor's state.
#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    who_am_i: String,
}

impl ActorState {
    pub fn load(cid: &Cid) -> Self {
        let data = fvm_sdk::ipld::get(cid).unwrap();
        fvm_ipld_encoding::from_slice::<Self>(&data).unwrap()
    }

    pub fn save(&self) -> Cid {
        let serialized = fvm_ipld_encoding::to_vec(self).unwrap();
        let block = Block {
            codec: DAG_CBOR,
            data: serialized,
        };
        fvm_sdk::ipld::put(
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

fn return_ipld<T>(value: &T) -> std::result::Result<u32, IpldError>
where
    T: ser::Serialize + ?Sized,
{
    let bytes = fvm_ipld_encoding::to_vec(value)?;
    Ok(fvm_sdk::ipld::put_block(DAG_CBOR, bytes.as_slice())?)
}

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
                NO_DATA_BLOCK_ID
            },
            "HelloWorld" => {
                HelloWorld()
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

// `Constructor` for the actor, called at every instantiation.
#[allow(non_snake_case)]
fn Constructor() {
    let state = ActorState {
        who_am_i: String::from("Basic Target Actor"),
    };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}

#[allow(non_snake_case)]
fn HelloWorld() -> u32 {
    let state = ActorState::load(&fvm_sdk::sself::root().unwrap());

    return_ipld(&state.who_am_i).unwrap()
}
```

There are a few methods that are interesting to note:
- `ActorState::load()` & `ActorState::save()`: Method implemented over the structure representing the inner state of an actor.
They are used to read and write the current state.
- `return_ipld()`: Method used to format a payload into an IPLD block and pass its ID as a return value for the received message.
- `invoke()`: Method that serves as the main entry point for our actor. It contains a `match` where each variant represents
a call to an inner method of our actor.
- `Constructor()`: Method called at instantiation time of our actor in the machine.

### Storage: ActorState

To add both `Add()` and `Read()` to our actor we need to create the related methods and add their variant to our
match in `invoke()`. But first, let's change the state associated to our actor.

We need to change our state to store a `counter` instead of the `who_am_i` property.
```rust
#[derive(Serialize_tuple, Deserialize_tuple)]
struct ActorState {
    value: u64,
}

impl ActorState {
    pub fn load(cid: &Cid) -> Self {
        let data = fvm_sdk::ipld::get(cid).unwrap();
        fvm_ipld_encoding::from_slice::<Self>(&data).unwrap()
    }

    pub fn save(&self) -> Cid {
        let serialized = fvm_ipld_encoding::to_vec(self).unwrap();
        let block = Block {
            codec: DAG_CBOR,
            data: serialized,
        };
        fvm_sdk::ipld::put(
            Code::Blake2b256.into(),
            32,
            block.codec,
            block.data.as_ref(),
        )
        .unwrap()
    }
}
```

 Then, update the `Constructor()` to properly initialize the state:
```rust
// `Constructor` for the actor, called at every instantiation.
#[allow(non_snake_case)]
fn Constructor() {
    let state = ActorState {
        value: 0,
    };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

### View method: Read

Let's remove the `HelloWorld` variant from our invoke match and the inner method `HelloWorld()` from `src/actor.rs`:
```rust
// ...

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "Constructor" => {
                Constructor();
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

// `Constructor` for the actor, called at every instantiation.
#[allow(non_snake_case)]
fn Constructor() {
    let state = ActorState {
        who_am_i: String::from("Basic Target Actor"),
    };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

Then, add the match variant and the method:
```rust
// ...

#[no_mangle]
fn invoke(_input: u32) -> u32 {
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
            _ => {
                fvm_sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

// ...

// `Read` returns the current value of our state value.
#[allow(non_snake_case)]
fn Read() -> u32 {
    let state = ActorState::load(&fvm_sdk::sself::root().unwrap());

    return_ipld(&state.value).unwrap()
}
```

By leveraging the `return_ipld()`, we can return the value of `ActorState::value` to the caller.

### Write method: Add

Finally, let's add our `Add()` method to update the state of the actor. We are aiming to update the value of 
`ActorState::value` everytime the method is called by adding the passed value to the current state value.

The first thing we need is a way to fetch the arguments passed to the message received:
```rust
use fvm_ipld_encoding::{de::DeserializeOwned, RawBytes};

// ...

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
```

`deserialize_params` takes the `u32` value passed to the `invoke()` method and deserialize it in the type expected by the
receiving variable.

We can now implement our `Add()` method and its `invoke()` match variant:
```rust
#[no_mangle]
fn invoke(input: u32) -> u32 {
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
            "Add" => {
                let to_add: u64 = deserialize_params(input);
                Add(to_add);
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

// ...

// `Add` will add the passed value to the current state.
#[allow(non_snake_case)]
fn Add(to_add: u64) {
    // Load the current state.
    let mut state = ActorState::load(&fvm_sdk::sself::root().unwrap());
    
    // Add message value.
    state.value += to_add;

    // Save updated state.
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

With this method implemented, the interface of our actor corresponds to our intended design. However, to ensure that things 
works as intended we need to add tests to our project. This is what we will focus on during the next step. 


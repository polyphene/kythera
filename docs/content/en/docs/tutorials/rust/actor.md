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

To proceed, we can use the script at the root of the repository to create a new actor and its test:
```shell
$ ./create-actor.sh basic-actor
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
name = "basic-actor"
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

## Actor's development

For this tutorial, we will create an actor that stores a value updated through the call of an `Add()` method. It
should also be possible to read the current value through a `Read()` method.

### Layout 

Our main logic for the actor will be located in `src/actor.rs`. We also have some utilities located in `src/utils.rs`.

In `utils.rs`, there are two methods that are interesting to note:
- `ActorState::load()` & `ActorState::save()`: Method implemented over the structure representing the inner state of an actor.
They are used to read and write the current state.
- `return_ipld()`: Method used to format a payload into an IPLD block and pass its ID as a return value for the received message.

In `actor.rs`
- -`invoke()`: Method that serves as the main entry point for our actor. It contains a `match` where each variant represents
a call to an inner method of our actor.
- `Constructor()`: Method called at instantiation time of our actor in the machine.

### Storage: ActorState

To add both `Add()` and `Read()` to our actor we need to create the related methods and add their variant to our
match in `invoke()`. But first, let's change the state associated to our actor.

We need to change our state to store a `counter` instead of the `who_am_i` property. Let's update `utils.rs`:
```rust
#[derive(Serialize_tuple, Deserialize_tuple)]
pub struct ActorState {
    value: u64,
}
```

 Then, update the `Constructor()` in `actor.rs` to properly initialize the state:
```rust
// `Constructor` for the actor, called at every instantiation.
#[allow(non_snake_case)]
fn Constructor() {
    let state = crate::utils::ActorState {
        value: 0,
    };
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

### View method: Read

Let's add the match variant and the method to read the current state:
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
    let state = crate::utils::ActorState::load(&fvm_sdk::sself::root().unwrap());

    crate::utils::return_ipld(&state.value).unwrap()
}
```

By leveraging the `return_ipld()`, we can return the value of `ActorState::value` to the caller.

### Write method: Add

Finally, let's add our `Add()` method to update the state of the actor. We are aiming to update the value of 
`ActorState::value` everytime the method is called by adding the passed value to the current state value.

For this, we will use `deserialize_params` from `utils.rs`. It takes the `u32` value passed to the `invoke()` method and 
deserialize it in the type expected by the receiving variable.

We can now implement our `Add()` method and its `invoke()` match variant in `actor.rs`:
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
                let to_add: u64 = crate::utils::deserialize_params(input);
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
    let mut state = crate::utils::ActorState::load(&fvm_sdk::sself::root().unwrap());
    
    // Add message value.
    state.value += to_add;

    // Save updated state.
    let cid = state.save();
    fvm_sdk::sself::set_root(&cid).unwrap();
}
```

With this method implemented, the interface of our actor corresponds to our intended design. However, to ensure that things 
works as intended we need to add tests to our project. This is what we will focus on during the next step. 


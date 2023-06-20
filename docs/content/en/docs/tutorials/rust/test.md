---
title: "Test our actor"
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
weight: 423
toc: true
---

In this final step of our basic Rust tutorial we will go over the creation of tests for our `basic-actor`.

## Configuring the test actor crate

If we move to the `tests` folder from the root of the repository, we can see that a `basic-actor-test` has been created.

This is the crate that we will use to write test for our `basic-actor`.

## Test Actor's development

To demonstrate the capabilities of Kythera we will create two tests: `TestAdd()` and `TestFailAddArgumentType()`. They should 
respectively:
- `TestAdd()`: Should test that our `Add()` method happy path works as intended.
- `TestFailAddArgumentType()`: Should assert that a message sent with a wrong argument type will return a non 0 exit code.

> 🗒️ **Note**
> 
> The core layout of a test actor is the same as the one from our actor. The only difference is that in a test actor a new 
dedicated method can be added, `Setup()`. This setup method is called before each test of the test actor is invoked.

### Test method: TestAdd

We can add our `TestAdd()` method to assert that our native actor works properly. To send a message to it, we can
leverage the argument passed in message sent to our test actor. It contains the actor ID at which the actor was deployed.

```rust
// ...

#[no_mangle]
fn invoke(input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        fvm_sdk::vm::exit(
            ExitCode::USR_ASSERTION_FAILED.value(),
            None,
            Some(&format!("{info}")),
        )
    }));

    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestAdd" => {
                TestAdd(input);
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

// Tests that the actor properly adds a passed value to its state.
#[allow(non_snake_case)]
fn TestAdd(input: u32) {
    // Get basic actor ID.
    let basic_actor_id: u64 = crate::utils::deserialize_params(input);

    // Value to add to state.
    let to_add = 10000u64;

    // Send message to add value to actor's state.
    let res = fvm_sdk::send::send(
        &Address::new_id(basic_actor_id),
        method_hash!("Add"),
        Some(IpldBlock::serialize(DAG_CBOR, &to_add).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    // Assert message went through.
    assert_eq!(res.exit_code, ExitCode::OK);

    // read current state of the actor.
    let res = fvm_sdk::send::send(
        &Address::new_id(basic_actor_id),
        method_hash!("Read"),
        None,
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    // Assert message went through.
    assert_eq!(res.exit_code, ExitCode::OK);

    // Deserialize value.
    let value: u64 = RawBytes::new(
        res.return_data
            .expect("Should be able to get result from HelloWorld of target actor")
            .data,
    )
        .deserialize()
        .unwrap();

    // Assert the current state value is the same as we set it to be, as we added to 0.
    assert_eq!(value, to_add);
}
```
### Test Fail method: TestFailAddArgumentType

The last test method we will be adding differs from the previous one. In this case, Kythera expects the transaction to end 
with an exit non-equal to 0. In other words, something should go wrong in the transaction.

In our case, we want to ensure that if we pass a message payload that contains arguments of a non-supported type our actor
should fail.

Let's add the `TestFailAddArgumentType()` method and its `invoke()` match variant:
```rust
#[no_mangle]
fn invoke(input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        fvm_sdk::vm::exit(
            ExitCode::USR_ASSERTION_FAILED.value(),
            None,
            Some(&format!("{info}")),
        )
    }));

    let method_num = fvm_sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestAdd" => {
                TestAdd(input);
                NO_DATA_BLOCK_ID
            },
            "TestFailAddArgumentType" => {
                TestFailAddArgumentType(input);
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

// Tests that the actor fails if it receives a payload of a non supported type for its method.
#[allow(non_snake_case)]
fn TestFailAddArgumentType(input: u32) {
    // Get basic actor ID.
    let basic_actor_id: u64 = deserialize_params(input);

    // Value to add to state.
    let to_add: Option<&str> = None;

    // Send message with a wrongly typed payload.
    let res = fvm_sdk::send::send(
        &Address::new_id(basic_actor_id),
        method_hash!("Add"),
        Some(IpldBlock::serialize(DAG_CBOR, &to_add).unwrap()),
        TokenAmount::zero(),
        None,
        SendFlags::empty(),
    )
        .unwrap();

    // This assertion should panic as our message must have failed
    assert_eq!(res.exit_code, ExitCode::OK);
}
```

## Run tests

Now that everything is set up for our actors to be compiled and tested, let's try it!

```shell
$ cargo build

    Finished dev [unoptimized + debuginfo] target(s) in 1.89s
    
$ kythera test ./artifacts

	Running Tests for Actor : BasicActor.wasm
		Testing 1 test files

BasicActor.t.wasm: testing 2 tests
test TestAdd ... ok
(gas consumption: 4639960)
test TestFailAddArgumentType ... ok
(gas consumption: 2500193)

test result: ok. 2 passed; 0 failed
```

It's a success! Our actor works as intended, and we could even ensure it through tests. This is the end of this basic tutorial
on how to start your own Rust native actors project using Kythera. 

If you have any comment or recommendation on how to evolve this tutorial to make it even better for newcomers, do not hesitate
to refer to our [contributing section](/docs/appendix/contributing/)!

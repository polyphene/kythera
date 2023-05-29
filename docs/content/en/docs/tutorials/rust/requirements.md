---
title: "Requirements"
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
weight: 421
toc: true
---

Before creating our own actor along with its test we need to ensure that we have properly set the development environment.

## Cloning starter project

To initialize the development environment, clone our dedicated Rust template:
```shell
$ git clone https://github.com/polyphene/kythera-rs-starter.git
```

Let's see how the project layout looks like:
```shell
$ cd kythera-rs-starter
$ tree . -d -L 1

.
├── actors
├── build-helper
└── tests

3 directories
```

Each folder has its own use:
- `build-helper`: crate dedicated to the generation of artifacts for Kythera to run over the Rust project
- `actors`: folder that will contain all of our project actors
- `tests`: folder that will contain all the tests for our actors

## Build

Let's ensure that this environment works properly by running `cargo build`:
```shell
$ cargo build
    ...
    Finished dev [unoptimized + debuginfo] target(s) in 2m 10s
```

We can notice that the layout for the project has changed:
Let's see how the project layout looks like:
```shell
$ cd kythera-rs-starter
$ tree . -d -L 1

.
├── actors
├── artifacts
├── build-helper
├── target
└── tests

5 directories
```

This new `artifacts` folder contains all the necessary files for Kythera to run the tests:
- `.wasm` & `.t.wasm`: files that contains the Wasm bytecode for our actors and their tests
- `.cbor` & `.t.cbor`: CBOR encoded files that contains a description of the interface exposed by a related Wasm file

## Test

We can then run `kythera test`:
```shell
$ kythera test ./artifacts

	Running Tests for Actor : HelloWorld.wasm
		Testing 1 test files

HelloWorld.t.wasm: testing 2 tests
test TestConstructorSetup ... ok
(gas consumption: 1519287)
test TestMethodParameter ... ok
(gas consumption: 2706032)

test result: ok. 2 passed; 0 failed
```

As we are sure that our environment is ready we can now create our own actor.
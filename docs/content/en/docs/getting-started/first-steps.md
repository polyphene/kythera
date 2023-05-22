---
title: "First Steps with Kythera"
date: 2023-04-03T11:01:56+02:00
lastmod: 2023-04-03T11:01:56+02:00
draft: false
images: []
menu:
    docs:
        parent: "getting-started"
weight: 130
toc: true
---

This section provide a quick example of how to use the `kythera` command line tool. We demonstrate how to start up a Rust-based
native actors project, compile and test it.

> 🗒️ **Note**
> 
> Future base project for Assembly Script and Golang will soon be created to assist developers more familiar with 
those languages.

To start the development clone our dedicated Rust template:
```shell
git clone https://github.com/polyphene/kythera-rs-starter.git
```

Let's see how the project layout looks like:
```shell
$ cd kythera-rs-starter
$ tree . -d -L 1
.
├── actors
├── build-helper
├── target
└── tests

4 directories
```

Now, we can simply build necessary assets through a simple `cargo build`:
```shell
$ cargo build
    ...
    Finished dev [unoptimized + debuginfo] target(s) in 2m 10s
```

Once the source actors are compiled we can now run our tests:
```shell
$kythera test 
	Running Tests for Actor : HelloWorld.wasm
		Testing 1 test files

HelloWorld.t.wasm: testing 2 tests
test TestConstructorSetup ... ok
(gas consumption: 1519287)
test TestMethodParameter ... ok
(gas consumption: 2706032)

test result: ok. 2 passed; 0 failed
```

> 💡 **Tip**
> 
> You can always print help for any subcommand (or their subcommands) by adding --help at the end.
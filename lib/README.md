# `kythera-lib`

`kythera-lib` is the core implementation for the Kythera FVM.

## Usage

An example of how to leverage `kythera-lib`:

```rust
use kythera_common::abi::{Abi, Method};
use kythera_lib::{TestResultType, Tester, WasmActor};
use std::fs;

fn main() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Get actor bin
    let content = fs::read_to_string("path/to/your/wasm/bin")
        .expect("Should have been able to read the file");
    let target_wasm_bin = wat::parse_str(content).unwrap();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("HelloWorld.wasm"),
        target_wasm_bin,
        Abi {
            constructor: Some(Method::new_from_name("Constructor").unwrap()),
            set_up: None,
            methods: vec![Method::new_from_name("HelloWorld").unwrap()],
        },
    );

    // Get test actor bin
    let content = fs::read_to_string("path/to/your/wasm/bin")
        .expect("Should have been able to read the file");
    let test_wasm_bin = wat::parse_str(content).unwrap();

    // Set test actor
    let test_abi = Abi {
        constructor: Some(Method::new_from_name("Constructor").unwrap()),
        set_up: Some(Method::new_from_name("Setup").unwrap()),
        methods: vec![
            Method::new_from_name("TestConstructorSetup").unwrap(),
            Method::new_from_name("TestMethodParameter").unwrap(),
            Method::new_from_name("TestFailed").unwrap(),
        ],
    };

    let test_actor = WasmActor::new(String::from("HelloWorld.t.wasm"), test_wasm_bin, test_abi);

    match tester.test(&test_actor.clone(), None) {
        Err(_) => {
            panic!("Could not run test when testing Tester flow")
        }
        Ok(test_res) => {
            // Handle test results
        }
    }
}
```
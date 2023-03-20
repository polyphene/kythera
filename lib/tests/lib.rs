use basic_test_actor::WASM_BINARY as BASIC_BINARY;

use kythera_common::abi::{Method, ABI};
use kythera_lib::{Tester, WasmActor};

const TARGET_WAT: &str = r#"
;; Mock invoke function
(module
  (func (export "invoke") (param $x i32) (result i32)
    (i32.const 1)
  )
)
"#;

#[test]
fn test_tester() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    let target_wasm_bin = wat::parse_str(TARGET_WAT).unwrap();
    let target_abi = ABI { methods: vec![] };
    let target_actor = WasmActor::new(String::from("Target"), target_wasm_bin, target_abi);

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(BASIC_BINARY.unwrap());
    let test_abi = ABI {
        methods: vec![
            Method {
                number: 3948827889,
                name: String::from("TestOne"),
            },
            Method {
                number: 891686990,
                name: String::from("TestTwo"),
            },
        ],
    };
    let test_actor = WasmActor::new(String::from("Test"), test_wasm_bin, test_abi);

    match tester.deploy_main_actor(target_actor.clone()) {
        Err(_) => {
            panic!("Could not set main actor when testing Tester")
        }
        _ => {}
    }

    match tester.test(target_actor, test_actor) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        _ => {}
    }
}

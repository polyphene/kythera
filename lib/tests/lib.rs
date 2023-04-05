use fvm_shared::error::ExitCode;
use kythera_common::abi::{Abi, Method, MethodType};
use kythera_lib::{TestResultType, Tester, WasmActor};
use kythera_test_actors::wasm_bin::BUILTIN_TEST_ACTOR_BINARY;

const TARGET_WAT: &str = r#"
        ;; Mock invoke function
            (module
                (func (export "invoke") (param $x i32) (result i32)
                    (i32.const 1)
                )
            )
        "#;

#[test]
fn test_builtin_deployed() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    let target_wasm_bin = wat::parse_str(TARGET_WAT).unwrap();
    let target_abi = Abi { methods: vec![] };
    let target_actor = WasmActor::new(String::from("Target"), target_wasm_bin, target_abi);

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(BUILTIN_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        methods: vec![Method::new_from_name("TestBuiltinsDeployed").unwrap()],
    };
    let test_actor = WasmActor::new(
        String::from("Builtins Deployed Test"),
        test_wasm_bin,
        test_abi,
    );

    match tester.deploy_target_actor(target_actor) {
        Err(_) => {
            panic!("Could not set target Actor when testing if builtins are properly deployed")
        }
        _ => {}
    }

    // Run test
    match tester.test(&[test_actor.clone()], None) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        Ok(test_res) => {
            assert_eq!(test_res.len(), 1usize);
            assert_eq!(test_res[0].results.as_ref().unwrap().len(), 1usize);
            assert_eq!(test_res[0].test_actor, &test_actor);

            test_res[0]
                .results
                .as_ref()
                .unwrap()
                .iter()
                .for_each(|result| match (result.method().r#type(), result.ret()) {
                    (MethodType::Test, TestResultType::Passed(apply_ret)) => {
                        assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK);
                    }
                    _ => panic!("test against basic test actor should pass"),
                })
        }
    }
}

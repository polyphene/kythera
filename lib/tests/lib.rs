use fvm_shared::error::ExitCode;
use kythera_actors::wasm_bin::test_actors::{
    BASIC_TARGET_ACTOR_BINARY, BASIC_TEST_ACTOR_BINARY, BUILTINS_TEST_ACTOR_BINARY,
    CHEATCODES_TEST_ACTOR_BINARY, CONSTRUCTOR_SETUP_TEST_ACTOR_BINARY,
};
use kythera_common::abi::{Abi, Method, MethodType};
use kythera_fvm::executor::ApplyFailure::MessageBacktrace;
use kythera_lib::{TestResultType, Tester, WasmActor};

const TARGET_WAT: &str = r#"
        ;; Mock invoke function
            (module
                (func (export "invoke") (param $x i32) (result i32)
                    (i32.const 1)
                )
            )
        "#;

fn set_target_actor(tester: &mut Tester, name: String, binary: Vec<u8>, abi: Abi) {
    let target_actor = WasmActor::new(name, binary, abi);

    tester
        .deploy_target_actor(&target_actor)
        .expect("Could not set target Actor when testing if builtins are properly deployed");
}

#[test]
fn test_tester_test() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target"),
        wat::parse_str(TARGET_WAT).unwrap(),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(BASIC_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: None,
        set_up: None,
        methods: vec![
            Method::new_from_name("TestOne").unwrap(),
            Method::new_from_name("TestTwo").unwrap(),
        ],
    };
    let test_actor = WasmActor::new(String::from("Basic"), test_wasm_bin, test_abi);

    match tester.test(&[test_actor.clone()], None) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        Ok(test_res) => {
            assert_eq!(test_res.len(), 1usize);
            assert_eq!(test_res[0].results.as_ref().unwrap().len(), 2usize);
            assert_eq!(test_res[0].test_actor, &test_actor);

            test_res[0]
                .results
                .as_ref()
                .unwrap()
                .iter()
                .enumerate()
                .for_each(
                    |(i, result)| match (result.method().r#type(), result.ret()) {
                        (MethodType::Test, TestResultType::Passed(apply_ret)) => {
                            assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK);
                            let ret_value: String =
                                apply_ret.msg_receipt.return_data.deserialize().unwrap();
                            if i == 0usize {
                                assert_eq!(ret_value, String::from("TestOne"))
                            } else {
                                assert_eq!(ret_value, String::from("TestTwo"))
                            }
                        }
                        _ => panic!("test against basic test actor should pass"),
                    },
                )
        }
    }
}

#[test]
fn test_builtin_deployed() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target"),
        wat::parse_str(TARGET_WAT).unwrap(),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(BUILTINS_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: None,
        set_up: None,
        methods: vec![Method::new_from_name("TestBuiltinsDeployed").unwrap()],
    };
    let test_actor = WasmActor::new(
        String::from("Builtins Deployed Test"),
        test_wasm_bin,
        test_abi,
    );

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

#[test]
fn test_constructor_and_set_up_called() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target"),
        wat::parse_str(TARGET_WAT).unwrap(),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(CONSTRUCTOR_SETUP_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: Some(Method::new_from_name("Constructor").unwrap()),
        set_up: Some(Method::new_from_name("Setup").unwrap()),
        methods: vec![Method::new_from_name("TestConstructorSetup").unwrap()],
    };
    let test_actor = WasmActor::new(String::from("Constructor Test"), test_wasm_bin, test_abi);

    match tester.test(&[test_actor.clone()], None) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        Ok(test_res) => {
            assert_eq!(test_res.len(), 1);
            assert_eq!(test_res[0].results.as_ref().unwrap().len(), 1);
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
                    apply_ret => {
                        panic!("test against basic test actor should pass: {apply_ret:?}")
                    }
                })
        }
    }
}

macro_rules! generate_match_assert {
        ($apply_failure:expr, $result:expr, $($test_name:expr => $test_message:expr),*) => {{
        match $apply_failure {
            MessageBacktrace(backtrace) => {
                match $result.method().name() {
                    $($test_name => {
                        assert!(backtrace.to_string().contains($test_message));
                    })*
                    _ => {}
                }
            },
            _ => {
                panic!("Failure should be a MessageBacktrace variant for failing tests");
            }
        }
    }};
}

#[test]
fn test_cheatcodes() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target"),
        Vec::from(BASIC_TARGET_ACTOR_BINARY),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![
                Method::new_from_name("Caller").unwrap(),
                Method::new_from_name("Origin").unwrap(),
            ],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(CHEATCODES_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: None,
        set_up: None,
        methods: vec![
            Method::new_from_name("TestWarp").unwrap(),
            Method::new_from_name("TestFailDeserializationWarp").unwrap(),
            Method::new_from_name("TestFailNoParametersWarp").unwrap(),
            Method::new_from_name("TestEpoch").unwrap(),
            Method::new_from_name("TestFailDeserializationEpoch").unwrap(),
            Method::new_from_name("TestFailNoParametersEpoch").unwrap(),
            Method::new_from_name("TestFee").unwrap(),
            Method::new_from_name("TestFailDeserializationFee").unwrap(),
            Method::new_from_name("TestFailNoParametersFee").unwrap(),
            Method::new_from_name("TestChainId").unwrap(),
            Method::new_from_name("TestFailDeserializationChainId").unwrap(),
            Method::new_from_name("TestFailNoParametersChainId").unwrap(),
            Method::new_from_name("TestPrank").unwrap(),
            Method::new_from_name("TestFailDeserializationPrank").unwrap(),
            Method::new_from_name("TestFailNoParametersPrank").unwrap(),
            Method::new_from_name("TestFailAddressTypePrank").unwrap(),
            Method::new_from_name("TestTrick").unwrap(),
            Method::new_from_name("TestFailDeserializationTrick").unwrap(),
            Method::new_from_name("TestFailNoParametersTrick").unwrap(),
            Method::new_from_name("TestFailAddressTypeTrick").unwrap(),
        ],
    };
    let test_actor = WasmActor::new(String::from("Cheatcodes Tests"), test_wasm_bin, test_abi);

    match tester.test(&[test_actor.clone()], None) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        Ok(test_res) => test_res[0]
            .results
            .as_ref()
            .unwrap()
            .iter()
            .for_each(|result| match (result.method().r#type(), result.ret()) {
                (MethodType::TestFail, TestResultType::Failed(apply_ret)) => {
                    assert_eq!(
                        apply_ret.msg_receipt.exit_code,
                        ExitCode::SYS_ASSERTION_FAILED
                    );
                    let apply_failure = apply_ret.failure_info.clone().unwrap();

                    generate_match_assert!(
                        apply_failure,
                        result,
                        "TestFailDeserializationWarp" => "Could not deserialize parameters for Warp cheatcode",
                        "TestFailNoParametersWarp" => "No parameters provided for Warp cheatcode",
                        "TestFailDeserializationEpoch" => "Could not deserialize parameters for Epoch cheatcode",
                        "TestFailNoParametersEpoch" => "No parameters provided for Epoch cheatcode",
                        "TestFailDeserializationFee" => "Could not deserialize parameters for Fee cheatcode",
                        "TestFailNoParametersFee" => "No parameters provided for Fee cheatcode",
                        "TestFailDeserializationChainId" => "Could not deserialize parameters for ChainId cheatcode",
                        "TestFailNoParametersChainId" => "No parameters provided for ChainId cheatcode",
                        "TestFailDeserializationPrank" => "Could not deserialize parameters for Prank cheatcode",
                        "TestFailNoParametersPrank" => "No parameters provided for Prank cheatcode",
                        "TestFailAddressTypePrank" => "Address parameter for Prank should have a valid ActorID",
                        "TestFailDeserializationTrick" => "Could not deserialize parameters for Trick cheatcode",
                        "TestFailNoParametersTrick" => "No parameters provided for Trick cheatcode",
                        "TestFailAddressTypeTrick" => "Address parameter for Trick should have a valid ActorID"
                    );
                }
                (MethodType::Test, TestResultType::Passed(apply_ret)) => {
                    assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK);
                }
                apply_ret => {
                    panic!("test against cheatcodes test actor should be valid: {apply_ret:?}")
                }
            }),
    }
}

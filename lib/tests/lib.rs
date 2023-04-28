use fvm_ipld_encoding::from_slice;
use fvm_shared::error::ExitCode;
use kythera_actors::wasm_bin::test_actors::{
    BASIC_TARGET_ACTOR_BINARY, BASIC_TEST_ACTOR_BINARY, BUILTINS_TEST_ACTOR_BINARY,
    CHEATCODES_TEST_ACTOR_BINARY, FAIL_TEST_ACTOR_BINARY,
};
use kythera_common::abi::{Abi, Method, MethodType};
use kythera_fvm::executor::ApplyFailure::MessageBacktrace;
use kythera_lib::error::Error;
use kythera_lib::{TestResultType, Tester, WasmActor};

fn set_target_actor(tester: &mut Tester, name: String, binary: Vec<u8>, abi: Abi) {
    let target_actor = WasmActor::new(name, binary, abi);

    tester
        .deploy_target_actor(target_actor)
        .expect("Could not set target Actor when testing if builtins are properly deployed");
}

#[test]
fn test_deploy_non_valid_target_actor() {
    // Instantiate tester
    let mut tester = Tester::new();

    let target_actor = WasmActor::new(
        String::from("Target.wasm"),
        vec![1, 2, 3],
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![Method::new_from_name("Hello").unwrap()],
        },
    );

    match tester.deploy_target_actor(target_actor).err().unwrap() {
        Error::Tester { source, .. } => {
            assert!(matches!(source.unwrap().as_ref(), &Error::Validator { .. }));
        }
        _ => {
            panic!("Error should be triggered by non valid wasm bin");
        }
    }
}

#[test]
fn test_deploy_non_valid_test_actor() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target.wasm"),
        Vec::from(BASIC_TARGET_ACTOR_BINARY),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = vec![1, 2, 3];
    let test_abi = Abi {
        constructor: None,
        set_up: None,
        methods: vec![Method::new_from_name("TestBuiltinsDeployed").unwrap()],
    };
    let test_actor = WasmActor::new(String::from("Target.t.wasm"), test_wasm_bin, test_abi);

    // Run test
    match tester.test(&test_actor.clone(), None).err().unwrap() {
        Error::Tester { source, .. } => {
            assert!(matches!(source.unwrap().as_ref(), &Error::Validator { .. }));
        }
        _ => {
            panic!("Error should be triggered by non valid wasm bin");
        }
    }
}

#[test]
fn test_failing_target_actor_constructor() {
    // Instantiate tester
    let mut tester = Tester::new();

    let target_actor = WasmActor::new(
        String::from("Target.wasm"),
        Vec::from(FAIL_TEST_ACTOR_BINARY),
        Abi {
            constructor: Some(Method::new_from_name("Constructor").unwrap()),
            set_up: None,
            methods: vec![],
        },
    );

    let res = tester.deploy_target_actor(target_actor);
    if !matches!(res.err().unwrap(), Error::Constructor { .. }) {
        panic!("Error should be triggered by error on Constructor execution");
    }
}

#[test]
fn test_failing_test_actor_constructor_setup() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target.wasm"),
        Vec::from(BASIC_TARGET_ACTOR_BINARY),
        Abi {
            constructor: None,
            set_up: None,
            methods: vec![],
        },
    );

    // Set failing constructor test actor
    let test_wasm_bin: Vec<u8> = Vec::from(FAIL_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: Some(Method::new_from_name("Constructor").unwrap()),
        set_up: None,
        methods: vec![Method::new_from_name("TestBuiltinsDeployed").unwrap()],
    };
    let constructor_test_actor =
        WasmActor::new(String::from("Constructor.t.wasm"), test_wasm_bin, test_abi);

    // Set failing setup test actor
    let test_wasm_bin: Vec<u8> = Vec::from(FAIL_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: None,
        set_up: Some(Method::new_from_name("Setup").unwrap()),
        methods: vec![Method::new_from_name("TestBuiltinsDeployed").unwrap()],
    };
    let setup_test_actor = WasmActor::new(String::from("Setup.t.wasm"), test_wasm_bin, test_abi);

    // Run test
    for test_actor in &[constructor_test_actor.clone(), setup_test_actor.clone()] {
        match tester.test(&test_actor, None) {
            Err(err) => {
                if test_actor.name().contains("Constructor") {
                    if !matches!(err, Error::Constructor { .. }) {
                        panic!("Error should be triggered by error on Constructor execution");
                    }
                } else {
                    if !matches!(err, Error::Setup { .. }) {
                        panic!("Error should be triggered by error on Constructor execution");
                    }
                }
            }
            Ok(_) => {
                panic!("Test should return error on non constructor for target actor");
            }
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
        String::from("Target.wasm"),
        Vec::from(BASIC_TARGET_ACTOR_BINARY),
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
    let test_actor = WasmActor::new(String::from("Target.t.wasm"), test_wasm_bin, test_abi);

    // Run test
    match tester.test(&test_actor.clone(), None) {
        Err(_) => {
            panic!("Could not run test when testing Tester for builtins")
        }
        Ok(test_res) => {
            assert_eq!(test_res.len(), 1usize);

            test_res
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
fn test_tester_flow() {
    // Instantiate tester
    let mut tester = Tester::new();

    // Set target actor
    set_target_actor(
        &mut tester,
        String::from("Target.wasm"),
        Vec::from(BASIC_TARGET_ACTOR_BINARY),
        Abi {
            constructor: Some(Method::new_from_name("Constructor").unwrap()),
            set_up: None,
            methods: vec![
                Method::new_from_name("HelloWorld").unwrap(),
                Method::new_from_name("Caller").unwrap(),
                Method::new_from_name("Origin").unwrap(),
            ],
        },
    );

    // Set test actor
    let test_wasm_bin: Vec<u8> = Vec::from(BASIC_TEST_ACTOR_BINARY);
    let test_abi = Abi {
        constructor: Some(Method::new_from_name("Constructor").unwrap()),
        set_up: Some(Method::new_from_name("Setup").unwrap()),
        methods: vec![
            Method::new_from_name("TestConstructorSetup").unwrap(),
            Method::new_from_name("TestMethodParameter").unwrap(),
            Method::new_from_name("TestFailed").unwrap(),
            Method::new_from_name("TestFailFailed").unwrap(),
            Method::new_from_name("TestFailSuccess").unwrap(),
        ],
    };
    let test_actor = WasmActor::new(String::from("Target.t.wasm"), test_wasm_bin, test_abi);

    match tester.test(&test_actor.clone(), None) {
        Err(_) => {
            panic!("Could not run test when testing Tester flow")
        }
        Ok(test_res) => {
            assert_eq!(test_res.len(), 5);
            test_res
                .iter()
                .for_each(|result| match result.method().name() {
                    "TestConstructorSetup" => match &result.ret() {
                        TestResultType::Passed(apply_ret) => {
                            assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK)
                        }
                        _ => panic!("TestConstructorSetup should be passing with ExitCode::OK"),
                    },
                    "TestMethodParameter" => {
                        match &result.ret() {
                            TestResultType::Passed(apply_ret) => {
                                assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK);

                                let returned_target_id: u64 =
                                    from_slice(apply_ret.msg_receipt.return_data.bytes()).unwrap();
                                // All target actors are at actor Id 103 based on the current tester
                                // flow.
                                assert_eq!(returned_target_id, 103);
                            }
                            _ => panic!("TestMethodParameter should be passing with ExitCode::OK"),
                        }
                    }
                    "TestFailed" => match &result.ret() {
                        TestResultType::Failed(apply_ret) => {
                            assert_eq!(
                                apply_ret.msg_receipt.exit_code,
                                ExitCode::USR_ASSERTION_FAILED
                            );
                        }
                        _ => panic!("TestFailed should be passing with ExitCode::OK"),
                    },
                    "TestFailFailed" => match &result.ret() {
                        TestResultType::Failed(apply_ret) => {
                            assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK)
                        }
                        _ => panic!("TestFailFailed should be failing with ExitCode::OK"),
                    },
                    "TestFailSuccess" => match &result.ret() {
                        TestResultType::Passed(apply_ret) => {
                            assert_ne!(apply_ret.msg_receipt.exit_code, ExitCode::OK)
                        }
                        _ => panic!("TestFailSuccess should be passing with non ExitCode::OK"),
                    },
                    name => panic!("Test case not handled for: {}", name),
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
        String::from("Target.wasm"),
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
    let test_actor = WasmActor::new(String::from("Target.t.wasm"), test_wasm_bin, test_abi);

    match tester.test(&test_actor.clone(), None) {
        Err(_) => {
            panic!("Could not run test when testing Tester")
        }
        Ok(test_res) => test_res
            .iter()
            .for_each(|result| match (result.method().r#type(), result.ret()) {
                (MethodType::TestFail, TestResultType::Passed(apply_ret)) => {
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

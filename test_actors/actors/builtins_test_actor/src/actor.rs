// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use fil_actors_runtime_v10::runtime::builtins::Type;
use fil_actors_runtime_v10::{
    BURNT_FUNDS_ACTOR_ADDR, CRON_ACTOR_ADDR, DATACAP_TOKEN_ACTOR_ADDR, DATACAP_TOKEN_ACTOR_ID,
    INIT_ACTOR_ADDR, REWARD_ACTOR_ADDR, STORAGE_MARKET_ACTOR_ADDR, STORAGE_POWER_ACTOR_ADDR,
    SYSTEM_ACTOR_ADDR, VERIFIED_REGISTRY_ACTOR_ADDR,
};
use frc42_dispatch::match_method;
use fvm_sdk as sdk;
use fvm_sdk::actor::{get_actor_code_cid, get_builtin_actor_type};
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

// TODO use helix frc42_dispatch when their dependencies are up to date.
#[no_mangle]
fn invoke(_input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::abort(
            ExitCode::USR_ASSERTION_FAILED.value(),
            Some(&format!("{info}")),
        )
    }));

    let method_num = sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestBuiltinsDeployed" => {
                TestBuiltinsDeployed();

                NO_DATA_BLOCK_ID
            },
            _ => {
                sdk::vm::abort(
                    ExitCode::USR_UNHANDLED_MESSAGE.value(),
                    Some("Unknown method number"),
                );
            }
        }
    )
}

// Checks that all relevant builtins are deployed at a correct actor Id in Kythera
#[allow(non_snake_case)]
fn TestBuiltinsDeployed() {
    // Test system actor deployment.
    let code_cid = get_actor_code_cid(&SYSTEM_ACTOR_ADDR)
        .unwrap_or_else(|| panic!("Should get an code CID at address: {}", &SYSTEM_ACTOR_ADDR));
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::System as i32);

    // Test init actor deployment.
    let code_cid = get_actor_code_cid(&INIT_ACTOR_ADDR)
        .unwrap_or_else(|| panic!("Should get an code CID at address: {}", &INIT_ACTOR_ADDR));
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Init as i32);

    // Test reward actor deployment.
    let code_cid = get_actor_code_cid(&REWARD_ACTOR_ADDR)
        .unwrap_or_else(|| panic!("Should get an code CID at address: {}", &REWARD_ACTOR_ADDR));
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Reward as i32);

    // Test cron actor deployment.
    let code_cid = get_actor_code_cid(&CRON_ACTOR_ADDR)
        .unwrap_or_else(|| panic!("Should get an code CID at address: {}", &CRON_ACTOR_ADDR));
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Cron as i32);

    // Test power actor deployment.
    let code_cid = get_actor_code_cid(&STORAGE_POWER_ACTOR_ADDR).unwrap_or_else(|| {
        panic!(
            "Should get an code CID at address: {}",
            &STORAGE_POWER_ACTOR_ADDR
        )
    });
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Power as i32);

    // Test market actor deployment.
    let code_cid = get_actor_code_cid(&STORAGE_MARKET_ACTOR_ADDR).unwrap_or_else(|| {
        panic!(
            "Should get an code CID at address: {}",
            &STORAGE_MARKET_ACTOR_ADDR
        )
    });
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Market as i32);

    // Test verified registry actor deployment.
    let code_cid = get_actor_code_cid(&VERIFIED_REGISTRY_ACTOR_ADDR).unwrap_or_else(|| {
        panic!(
            "Should get an code CID at address: {}",
            &VERIFIED_REGISTRY_ACTOR_ADDR
        )
    });
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::VerifiedRegistry as i32);

    // Test datacap actor deployment.
    let code_cid = get_actor_code_cid(&DATACAP_TOKEN_ACTOR_ADDR).unwrap_or_else(|| {
        panic!(
            "Should get an code CID at address: {}",
            &DATACAP_TOKEN_ACTOR_ID
        )
    });
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::DataCap as i32);

    // Test burnt funds actor deployment.
    let code_cid = get_actor_code_cid(&BURNT_FUNDS_ACTOR_ADDR).unwrap_or_else(|| {
        panic!(
            "Should get an code CID at address: {}",
            &BURNT_FUNDS_ACTOR_ADDR
        )
    });
    let actor_type = get_builtin_actor_type(&code_cid)
        .unwrap_or_else(|| panic!("Should get a builtin actor type for CID: {}", &code_cid));
    assert_eq!(actor_type, Type::Account as i32);
}

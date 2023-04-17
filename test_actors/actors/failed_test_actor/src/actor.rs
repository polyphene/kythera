// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use frc42_dispatch::match_method;
use fvm_sdk as sdk;
use fvm_sdk::NO_DATA_BLOCK_ID;
use fvm_shared::error::ExitCode;

#[no_mangle]
fn invoke(_input: u32) -> u32 {
    std::panic::set_hook(Box::new(|info| {
        sdk::vm::exit(
            ExitCode::USR_ASSERTION_FAILED.value(),
            None,
            Some(&format!("{info}")),
        )
    }));

    let method_num = sdk::message::method_number();
    match_method!(
        method_num,
        {
            "TestFailed" => {
                TestFailed();

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
fn TestFailed() {
    assert_eq!(1 + 1, 3);
}

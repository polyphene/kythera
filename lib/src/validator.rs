use crate::error::{Error, WrapFVMError};
use rayon::prelude::*;
use wasmparser::{FuncValidatorAllocations, Parser, ValidPayload, Validator, WasmFeatures};

/// Utility to validate a wasm file. The code is a replica of the _validate_ command from the wasm-tools
/// repository: https://github.com/bytecodealliance/wasm-tools/blob/e5293d587f463e67d42ca26b151cd7afdc0a5e17/src/bin/wasm-tools/validate.rs#L52-L83.
pub(crate) fn validate_wasm_bin(wasm_bin: &[u8]) -> Result<(), Error> {
    // Generate Fvm Wasm Features
    let features = WasmFeatures {
        simd: false,
        ..Default::default()
    };

    let mut validator = Validator::new_with_features(features);
    let mut functions_to_validate = Vec::new();

    for payload in Parser::new(0).parse_all(wasm_bin) {
        match validator
            .payload(&payload.validator_err("Could not parse wasm bin module")?)
            .validator_err("Wasm bin module is not valid")?
        {
            ValidPayload::Ok | ValidPayload::Parser(_) | ValidPayload::End(_) => {}
            ValidPayload::Func(validator, body) => functions_to_validate.push((validator, body)),
        }
    }

    functions_to_validate.into_par_iter().try_for_each_init(
        FuncValidatorAllocations::default,
        |allocs, (to_validate, body)| -> Result<_, Error> {
            let mut validator = to_validate.into_validator(std::mem::take(allocs));
            validator
                .validate(&body)
                .validator_err("Failed to validate method")?;
            *allocs = validator.into_allocations();
            Ok(())
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kythera_actors::wasm_bin::test_actors::BUILTINS_TEST_ACTOR_BINARY;

    #[test]
    fn test_wasm_bin_validation() {
        let wasm_bin = Vec::from(BUILTINS_TEST_ACTOR_BINARY);
        assert!(validate_wasm_bin(&wasm_bin).is_ok())
    }

    #[test]
    fn test_fail_validate_wasm_bin() {
        let wasm_bin = vec![1, 2, 3];
        let res = validate_wasm_bin(&wasm_bin);
        assert!(res.is_err());
        assert!(res
            .err()
            .unwrap()
            .to_string()
            .contains("Could not parse wasm bin module"))
    }
}

// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use frc42_dispatch::hash::MethodResolver;

use crate::abi::blake2b::Blake2bHasher;
use crate::error;

mod blake2b;

/// `ABI` is the structure we use internally to deal with Actor Binary Interface. It contains all
/// exposed [`Methods`] from a given actor.
struct ABI {
    methods: Vec<Methods>,
}

/// Method number indicator for calling actor methods.
pub type MethodNum = u64;

// `Methods` describes an exposed method from an actor entrypoint.
struct Methods {
    number: MethodNum,
    name: String,
}

/// `derive_method_num` will return the method number for a given method name based on the FRC-042:
/// https://github.com/filecoin-project/FIPs/blob/master/FRCs/frc-0042.md
fn derive_method_num(name: String) -> Result<MethodNum, error::Error> {
    let resolver = MethodResolver::new(Blake2bHasher {});

    return match resolver.method_number(name.as_str()) {
        Ok(method_number) => Ok(method_number),
        Err(err) => {
            println!("{}", err.to_string());
            return Err(error::Error::MethodNumberGeneration {
                name,
                source: err.into(),
            });
        }
    };
}

#[cfg(test)]
mod test {
    use crate::abi::derive_method_num;

    #[test]
    fn test_method_derivation() {
        let method_name = String::from("TestTransfer");

        match derive_method_num(method_name.clone()) {
            Ok(method_num) => {
                assert_eq!(method_num, 3760293944);
            }
            Err(_) => {
                panic!("derive_method_num failed for {}", method_name);
            }
        }
    }

    #[test]
    fn test_fail_method_derivation() {
        // Using function with lower case as first character in method name to fail the test
        let method_name = String::from("test_transfer");

        match derive_method_num(method_name.clone()) {
            Ok(method_num) => {
                panic!("derive_method_num success for {}", method_name);
            }
            Err(err) => {
                assert_eq!(
                    format!("Could not generate method number for `{}`", method_name),
                    err.to_string()
                )
            }
        }
    }
}

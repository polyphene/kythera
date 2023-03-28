// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use frc42_dispatch::hash::MethodResolver;

use crate::abi::blake2b::Blake2bHasher;
use crate::error;

mod blake2b;

/// Split a PascalCase string into a vector of its components.
/// If the string is not PascalCase function returns an empty [`Vec`].
pub fn pascal_case_split(s: &str) -> Vec<&str> {
    let mut split = vec![];
    // Work with indices to avoid allocations.
    let mut chars = s.char_indices();

    // Check if first character is capitalized.
    let mut beg = match chars.next() {
        Some((i, c)) if c.is_uppercase() => i,
        _ => return split,
    };

    // Iterate the rest of the characters.
    for (i, c) in chars {
        if c.is_uppercase() || c.is_numeric() {
            split.push(&s[beg..i]);
            beg = i;
        }
    }

    // Push the last word, this word is not covered by the iterator
    // as it doesn't know when it's last element.
    split.push(&s[beg..s.len()]);
    split
}

/// `Abi` is the structure we use internally to deal with Actor Binary Interface. It contains all
/// exposed [`Method`] from a given actor.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Abi {
    pub methods: Vec<Method>,
}

/// Method number indicator for calling actor methods.
pub type MethodNum = u64;

/// `Methods` describes an exposed method from an actor entrypoint.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Method {
    pub number: MethodNum,
    pub name: String,
}

/// `derive_method_num` will return the method number for a given method name based on the FRC-042:
/// https://github.com/filecoin-project/FIPs/blob/master/FRCs/frc-0042.md
pub fn derive_method_num(name: &str) -> Result<MethodNum, error::Error> {
    let resolver = MethodResolver::new(Blake2bHasher {});

    match resolver.method_number(name) {
        Ok(method_number) => Ok(method_number),
        Err(err) => Err(error::Error::MethodNumberGeneration {
            name: name.into(),
            source: err.into(),
        }),
    }
}

#[cfg(test)]
mod test {
    use super::{derive_method_num, pascal_case_split};

    #[test]
    fn test_method_derivation() {
        let method_name = String::from("TestTransfer");

        match derive_method_num(&method_name) {
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

        match derive_method_num(&method_name) {
            Ok(_) => {
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

    #[test]
    fn test_pascal_case() {
        assert_eq!(pascal_case_split("TestOne"), vec!["Test", "One"]);
        assert_eq!(
            pascal_case_split("TestFailWithMultipleWords"),
            vec!["Test", "Fail", "With", "Multiple", "Words"]
        );
        assert_eq!(pascal_case_split("Test1"), vec!["Test", "1"]);
        assert_eq!(pascal_case_split("testOne"), Vec::<&str>::new());
    }
}

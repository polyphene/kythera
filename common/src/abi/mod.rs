// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use frc42_dispatch::hash::MethodResolver;
use fvm_ipld_encoding::tuple::{Deserialize_tuple, Serialize_tuple};
use serde::de::SeqAccess;

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
#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Abi {
    pub methods: Vec<Method>,
}

/// Method number indicator for calling actor methods.
pub type MethodNum = u64;

/// `Methods` describes an exposed method from an actor entrypoint.
#[derive(Serialize_tuple, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Method {
    #[serde(skip_serializing)]
    pub number: MethodNum,
    pub name: String,
}

// Implement custom deserialization method for [`Method`] as we expect the bytes to be deserialized to only contain
// the `name` and not the `number` property that is generated at deserialization time.
impl<'de> serde::de::Deserialize<'de> for Method {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct MethodVisitor;

        impl<'de> serde::de::Visitor<'de> for MethodVisitor {
            type Value = Method;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Method")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let name = seq
                    .next_element::<String>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                let number = match derive_method_num(&name) {
                    Ok(number) => number,
                    Err(_) => {
                        return Err(serde::de::Error::custom(format_args!(
                            "Could not derive method number for: {}",
                            &name
                        )))
                    }
                };

                Ok(Self::Value { number, name })
            }
        }

        deserializer.deserialize_seq(MethodVisitor)
    }
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
    use crate::abi::{Abi, Method};

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

    #[test]
    fn test_tuple_serde() {
        // Test assets
        let test_transfer_name = String::from("TestTransfer");
        let test_transfer_fail_name = String::from("TestFailTransfer");

        let abi = Abi {
            methods: vec![
                Method {
                    number: derive_method_num(&test_transfer_name).unwrap(),
                    name: test_transfer_name,
                },
                Method {
                    number: derive_method_num(&test_transfer_fail_name).unwrap(),
                    name: test_transfer_fail_name,
                },
            ],
        };
        let serialized_abi: Vec<u8> = vec![
            129, 130, 129, 108, 84, 101, 115, 116, 84, 114, 97, 110, 115, 102, 101, 114, 129, 112,
            84, 101, 115, 116, 70, 97, 105, 108, 84, 114, 97, 110, 115, 102, 101, 114,
        ];

        // Serialize
        let abi_vec = crate::to_vec(&abi).unwrap();
        assert_eq!(abi_vec, serialized_abi);

        // Deserialize
        let deserialized_abi: Abi = crate::from_slice(&serialized_abi).unwrap();
        assert_eq!(deserialized_abi, abi);
    }

    #[test]
    fn test_fail_tuple_serde() {
        // Test assets
        let test_transfer_name = String::from("TestTransfer");
        let test_transfer_fail_name = String::from("testFailTransfer");

        let abi = Abi {
            methods: vec![
                Method {
                    number: derive_method_num(&test_transfer_name).unwrap(),
                    name: test_transfer_name,
                },
                Method {
                    number: 3280706483,
                    name: test_transfer_fail_name,
                },
            ],
        };

        let serialized_abi: Vec<u8> = vec![
            129, 130, 129, 108, 84, 101, 115, 116, 84, 114, 97, 110, 115, 102, 101, 114, 129, 112,
            116, 101, 115, 116, 70, 97, 105, 108, 84, 114, 97, 110, 115, 102, 101, 114,
        ];

        // Serialize
        let abi_vec = crate::to_vec(&abi).unwrap();
        assert_eq!(abi_vec, serialized_abi);

        // Deserialize
        match crate::from_slice::<Abi>(&serialized_abi) {
            Ok(_) => panic!("Deserialization should fail"),
            Err(err) => {
                dbg!(&err);
                assert!(err
                    .to_string()
                    .contains("Could not derive method number for: testFailTransfer"));
            }
        };
    }
}

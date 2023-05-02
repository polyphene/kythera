// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use std::fmt;

use anyhow::Result;
use frc42_dispatch::hash::MethodResolver;
use serde::de::SeqAccess;

use crate::abi::blake2b::Blake2bHasher;
use crate::error::{self, Error};

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
/// exposed [`Method`] from a given Actor.
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Abi {
    pub constructor: Option<Method>,
    pub set_up: Option<Method>,
    pub methods: Vec<Method>,
}

impl Abi {
    /// Get the `Constructor` method on the Abi if it exists.
    pub fn constructor(&self) -> Option<&Method> {
        self.constructor.as_ref()
    }

    /// Get the `Setup` method on the Abi if it exists.
    pub fn set_up(&self) -> Option<&Method> {
        self.set_up.as_ref()
    }

    pub fn methods(&self) -> &[Method] {
        &self.methods
    }
}

/// Custom implementation of [`Serialize`] so that we join `Constructor` and `Setup`
/// into the rest of the methods.
impl serde::Serialize for Abi {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = vec![];
        if let Some(constructor) = &self.constructor {
            s.push(vec![constructor.name()]);
        }
        if let Some(constructor) = &self.set_up {
            s.push(vec![constructor.name()]);
        }
        let methods = self.methods.iter().map(|m| vec![m.name()]);
        s.extend(methods);

        serde::Serialize::serialize(&vec![s], serializer)
    }
}

/// Custom implementation of [`Deserialize`] so that we check for `Constructor` and `Setup`
/// existence and assert there's only one of each.
impl<'de> serde::Deserialize<'de> for Abi {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AbiVisitor;

        impl<'de> serde::de::Visitor<'de> for AbiVisitor {
            type Value = Abi;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Abi")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut constructor = None;
                let mut set_up = None;

                let mut methods = vec![];
                let seq_methods = seq
                    .next_element::<Vec<Method>>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;

                // TODO: Can't we parse each method sequentially instead? and not have to
                // iterate again all over here?
                for method in seq_methods {
                    match (constructor.is_some(), set_up.is_some(), &method.r#type()) {
                        (false, _, MethodType::Constructor) => constructor = Some(method),
                        (_, false, MethodType::Setup) => set_up = Some(method),
                        (true, _, MethodType::Constructor) | (_, true, MethodType::Setup) => {
                            return Err(serde::de::Error::custom(
                                "Abi can only have one Constructor and one SetUp function",
                            ))
                        }
                        (_, _, _) => methods.push(method),
                    }
                }

                Ok(Abi {
                    constructor,
                    set_up,
                    methods,
                })
            }
        }
        deserializer.deserialize_seq(AbiVisitor)
    }
}

/// Method number indicator for calling Actor methods.
pub type MethodNum = u64;

/// [`Method`] describes an exposed method from an actor entrypoint.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Method {
    number: MethodNum,
    name: String,
    r#type: MethodType,
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Type of the [`Abi`] [`Method`] of the test Actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MethodType {
    Constructor,
    Entrypoint,
    Setup,
    Test,
    TestFail,
}

impl Method {
    /// Get the [`Method`] number.
    pub fn number(&self) -> MethodNum {
        self.number
    }

    /// Get the [`Method`] number.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the [`Method`] type.
    pub fn r#type(&self) -> MethodType {
        self.r#type
    }
}

impl Method {
    pub fn new_from_name(name: &str) -> Result<Self, Error> {
        let number = derive_method_num(name)?;
        let name = name.to_string();

        let split = pascal_case_split(&name);
        let r#type = match &split[..] {
            ["Constructor", ..] => MethodType::Constructor,
            ["Setup", ..] => MethodType::Setup,
            ["Test", "Fail", ..] => MethodType::TestFail,
            ["Test", ..] => MethodType::Test,
            _ => MethodType::Entrypoint,
        };

        Ok(Method {
            number,
            name,
            r#type,
        })
    }
}

/// Implement custom deserialization method for [`Method`] as we expect the bytes to be deserialized to only contain
/// the `name` and not the `number` property that is generated at deserialization time.
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

                Self::Value::new_from_name(&name).map_err(|_| {
                    serde::de::Error::custom(format!("Couldn't deserialize method: {}", &name))
                })
            }
        }

        deserializer.deserialize_seq(MethodVisitor)
    }
}

/// `derive_method_num` will return the method number for a given method name based on the FRC-042:
/// https://github.com/filecoin-project/FIPs/blob/master/FRCs/frc-0042.md .
pub fn derive_method_num(name: &str) -> Result<MethodNum, error::Error> {
    let resolver = MethodResolver::new(Blake2bHasher {});

    match resolver.method_number(name) {
        Ok(method_number) => Ok(method_number),
        Err(err) => Err(Error::MethodNumberGeneration {
            name: name.into(),
            source: err.into(),
        }),
    }
}

#[cfg(test)]
mod test {
    use super::{derive_method_num, pascal_case_split};
    use crate::abi::{Abi, Method, MethodType};

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
        // Using function with lower case as first character in method name to fail the test.
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
        // Test assets.
        let test_transfer_name = String::from("TestTransfer");
        let test_transfer_fail_name = String::from("TestFailTransfer");

        let abi = Abi {
            constructor: None,
            set_up: None,
            methods: vec![
                Method {
                    number: derive_method_num(&test_transfer_name).unwrap(),
                    name: test_transfer_name,
                    r#type: MethodType::Test,
                },
                Method {
                    number: derive_method_num(&test_transfer_fail_name).unwrap(),
                    name: test_transfer_fail_name,
                    r#type: MethodType::TestFail,
                },
            ],
        };

        let serialized_abi: Vec<u8> = vec![
            129, 130, 129, 108, 84, 101, 115, 116, 84, 114, 97, 110, 115, 102, 101, 114, 129, 112,
            84, 101, 115, 116, 70, 97, 105, 108, 84, 114, 97, 110, 115, 102, 101, 114,
        ];

        // Serialize.
        let abi_vec = crate::to_vec(&abi).unwrap();
        assert_eq!(abi_vec, serialized_abi);

        // Deserialize.
        let deserialized_abi: Abi = crate::from_slice(&serialized_abi).unwrap();
        assert_eq!(deserialized_abi, abi);
    }

    #[test]
    fn test_fail_tuple_serde() {
        // Test assets.
        let test_transfer_name = String::from("TestTransfer");
        let test_transfer_fail_name = String::from("testFailTransfer");

        let abi = Abi {
            constructor: None,
            set_up: None,
            methods: vec![
                Method {
                    number: derive_method_num(&test_transfer_name).unwrap(),
                    name: test_transfer_name,
                    r#type: MethodType::Test,
                },
                Method {
                    number: 3280706483,
                    name: test_transfer_fail_name,
                    r#type: MethodType::TestFail,
                },
            ],
        };

        let serialized_abi: Vec<u8> = vec![
            129, 130, 129, 108, 84, 101, 115, 116, 84, 114, 97, 110, 115, 102, 101, 114, 129, 112,
            116, 101, 115, 116, 70, 97, 105, 108, 84, 114, 97, 110, 115, 102, 101, 114,
        ];

        // Serialize.
        let abi_vec = crate::to_vec(&abi).unwrap();
        assert_eq!(abi_vec, serialized_abi);

        // Deserialize.
        match crate::from_slice::<Abi>(&serialized_abi) {
            Ok(_) => panic!("Deserialization should fail"),
            Err(err) => {
                assert!(err
                    .to_string()
                    .contains("Couldn't deserialize method: testFailTransfer"));
            }
        };
    }

    #[test]
    fn test_method_constructor() {
        assert_eq!(
            Method::new_from_name("TestOne").unwrap().r#type,
            MethodType::Test
        );
        assert_eq!(
            Method::new_from_name("TestFailOne").unwrap().r#type,
            MethodType::TestFail
        );
        assert_eq!(
            Method::new_from_name("Constructor").unwrap().r#type,
            MethodType::Constructor
        );
        assert_eq!(
            Method::new_from_name("Setup").unwrap().r#type,
            MethodType::Setup
        );

        assert!(Method::new_from_name("testOne").is_err());
        assert!(Method::new_from_name("").is_err());
    }
}

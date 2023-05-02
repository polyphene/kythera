// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

#[cfg(feature = "colors")]
use colored::Colorize;

pub use kythera_common::{
    abi::{pascal_case_split, Abi, Method, MethodType},
    from_slice, to_vec,
};

pub use kythera_fvm::{
    executor::{ApplyRet, KytheraExecutor},
    trace::ExecutionEvent,
    Account, Address, Payload,
};

use core::fmt;
use std::sync::mpsc::SyncSender;

use fvm_ipld_encoding::RawBytes;
use fvm_shared::{bigint::Zero, econ::TokenAmount, error::ExitCode};

use crate::validator::validate_wasm_bin;
use error::Error;
use state_tree::{BuiltInActors, StateTree};

pub mod error;
mod state_tree;
mod validator;

/// Main interface to test `Actor`s with Kythera.
pub struct Tester {
    // Builtin actors root Cid used in the Machine
    builtin_actors: BuiltInActors,
    // State tree constructed before instantiating the Machine
    state_tree: StateTree,
    // Account used for testing.
    account: Account,
    // The Target Actor to be tested.
    target_actor: Option<DeployedActor>,
    // The Method message sequence number.
    sequence: u64,
}

/// WebAssembly Actor.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmActor {
    name: String,
    bytecode: Vec<u8>,
    abi: Abi,
}

impl WasmActor {
    /// Create a new WebAssembly Actor.
    pub fn new(name: String, bytecode: Vec<u8>, abi: Abi) -> Self {
        Self {
            name,
            bytecode,
            abi,
        }
    }

    /// Get the WebAssembly Actor name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the WebAssembly Actor bytecode.
    pub fn code(&self) -> &[u8] {
        &self.bytecode
    }

    /// Get the Actor Abi.
    pub fn abi(&self) -> &Abi {
        &self.abi
    }

    /// Convert into a [`DeployedActor`].
    pub fn deploy(self, address: Address) -> DeployedActor {
        DeployedActor {
            name: self.name,
            bytecode: self.bytecode,
            abi: self.abi,
            address: address,
        }
    }
}

impl fmt::Display for WasmActor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// An Actor that has been deployed into a `BlockStore`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeployedActor {
    name: String,
    bytecode: Vec<u8>,
    abi: Abi,
    address: Address,
}

impl DeployedActor {
    /// Get the WebAssembly Actor name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the WebAssembly Actor bytecode.
    pub fn code(&self) -> &[u8] {
        &self.bytecode
    }

    /// Get the Actor Abi.
    pub fn abi(&self) -> &Abi {
        &self.abi
    }

    /// Get the Actor [`Address`].
    pub fn address(&self) -> &Address {
        &self.address
    }
}

/// Outcome of the test.
#[derive(Clone, Debug)]
pub enum TestResultType {
    Passed(ApplyRet),
    Failed(ApplyRet),
    Erred(String),
}

/// Output of running a [`Method`] of an Actor test.
#[derive(Clone, Debug)]
pub struct TestResult {
    method: Method,
    ret: TestResultType,
}

impl TestResult {
    /// Check if the [`TestResult`] passed.
    pub fn passed(&self) -> bool {
        matches!(self.ret, TestResultType::Passed(_))
    }
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "test {} ... ", self.method(),)?;
        if self.passed() {
            let ok = "ok";
            #[cfg(feature = "colors")]
            let ok = ok.green();
            write!(f, "{ok}")
        } else {
            let failed = "FAILED";
            #[cfg(feature = "colors")]
            let failed = failed.bright_red();
            write!(f, "{failed}")
        }
    }
}

impl TestResult {
    /// Get the [`Method`] tested.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get the [`ApplyRet`] of the test.
    pub fn ret(&self) -> &TestResultType {
        &self.ret
    }
}

/// Output of testing a list of Tests and its [`Method`]s for a target Actor.
#[derive(Debug)]
pub struct TestActorResults<'a> {
    pub test_actor: &'a WasmActor,
    pub results: Result<Vec<TestResult>, Error>,
}

impl Tester {
    /// Create a new Kythera Tester.
    pub fn new() -> Self {
        let mut state_tree = StateTree::new();

        let builtin_actors = state_tree.load_builtin_actors();
        state_tree.load_kythera_actors();
        let account = state_tree.create_account(*builtin_actors.manifest.get_account_code());

        Self {
            builtin_actors,
            state_tree,
            account,
            target_actor: None,
            sequence: 0,
        }
    }

    /// Retrieve the Deployed target Actor.
    pub fn deployed_actor(&self) -> Option<&DeployedActor> {
        self.target_actor.as_ref()
    }

    /// Deploy the target Actor file into the `StateTree`.
    pub fn deploy_target_actor(&mut self, actor: WasmActor) -> Result<(), Error> {
        // Validate wasm bin.
        if let Err(err) = validate_wasm_bin(actor.code()) {
            return Err(Error::Tester {
                msg: format!("Non valid target actor wasm file: {}", actor.name()),
                source: Some(Box::from(err)),
            });
        }

        // Set actor bin.
        let address = self
            .state_tree
            .deploy_actor_from_bin(&actor, TokenAmount::zero())?;

        let address_id = match address.id() {
            Ok(id) => {
                RawBytes::new(to_vec(&id).expect("Should be able to serialize target actor ID"))
            }
            Err(_) => panic!("Actor Id should be valid"),
        };
        // Instantiate executor.
        let root = self.state_tree.flush();
        let blockstore = self.state_tree.store().clone();
        let mut executor = KytheraExecutor::new(
            blockstore,
            root,
            self.builtin_actors.root,
            self.account.1,
            address_id,
        );

        // Run the constructor if it exists.
        if let Some(constructor) = actor.abi().constructor() {
            let sequence = self.state_tree.actor_sequence(self.account.0)?;

            match executor.execute_method(address, constructor.number(), sequence) {
                Ok(apply_ret) => {
                    if apply_ret.msg_receipt.exit_code != ExitCode::OK {
                        let source = apply_ret.failure_info.map(|f| f.to_string().into());
                        return Err(Error::Constructor {
                            name: actor.name().to_string(),
                            source,
                        });
                    }
                }
                Err(err) => {
                    return Err(Error::Constructor {
                        name: actor.name().to_string(),
                        source: Some(err.into()),
                    });
                }
            }
        }

        // Update owned state tree
        let (root, blockstore) = executor.into_store();
        self.state_tree.override_inner(blockstore, root).unwrap();

        self.target_actor = Some(actor.deploy(address));

        Ok(())
    }

    // Get and increment the next Actor sequence.
    pub fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = sequence + 1;
        sequence
    }

    /// Test an Actor on a `MemoryBlockstore`.
    pub fn test(
        &mut self,
        test_actor: &WasmActor,
        stream_results: Option<SyncSender<(WasmActor, TestResult)>>,
    ) -> Result<Vec<TestResult>, Error> {
        // Get target actor Id to pass it to test methods.
        let target = self
            .target_actor
            .as_ref()
            .cloned()
            .ok_or(Error::MissingActor {
                msg: "Main Actor not loaded".to_string(),
            })?;

        let target_id = match target.address.id() {
            Ok(id) => {
                RawBytes::new(to_vec(&id).expect("Should be able to serialize target actor ID"))
            }
            Err(_) => panic!("Actor Id should be valid"),
        };

        // Iterate over all test actors
        log::info!(
            "{}: testing {} tests",
            test_actor.name(),
            test_actor.abi().methods().len()
        );

        // Validate actor bin.
        if let Err(err) = validate_wasm_bin(test_actor.code()) {
            return Err(Error::Tester {
                msg: format!("Non valid test actor wasm file: {}", test_actor.name),
                source: Some(Box::from(err)),
            });
        }

        // Deploy test actor
        let test_address = match self
            .state_tree
            .deploy_actor_from_bin(test_actor, TokenAmount::zero())
        {
            // Properly deployed, get address
            Ok(test_address) => test_address,
            // Error on deployment, return error as part of [`TestResults`]
            Err(err) => return Err(err),
        };

        // Instantiate executor.
        let root = self.state_tree.flush();
        let blockstore = self.state_tree.store().clone();
        let mut executor = KytheraExecutor::new(
            blockstore,
            root,
            self.builtin_actors.root,
            self.account.1,
            target_id.clone(),
        );

        let mut sequence = self.state_tree.actor_sequence(self.account.0)?;

        // Run the constructor if it exists.
        if let Some(constructor) = test_actor.abi().constructor() {
            match executor.execute_method(test_address, constructor.number(), sequence) {
                Ok(apply_ret) => {
                    if apply_ret.msg_receipt.exit_code != ExitCode::OK {
                        let source = apply_ret.failure_info.map(|f| f.to_string().into());
                        return Err(Error::Constructor {
                            name: test_actor.name().to_string(),
                            source,
                        });
                    }
                }
                Err(err) => {
                    return Err(Error::Constructor {
                        name: test_actor.name().to_string(),
                        source: Some(err.into()),
                    })
                }
            }
            sequence += 1;
        }

        // Run Setup if it exists.
        if let Some(set_up) = test_actor.abi().set_up() {
            match executor.execute_method(test_address, set_up.number(), sequence) {
                Ok(apply_ret) => {
                    if apply_ret.msg_receipt.exit_code != ExitCode::OK {
                        let source = apply_ret.failure_info.map(|f| f.to_string().into());
                        return Err(Error::Setup {
                            name: test_actor.name().to_string(),
                            source,
                        });
                    }
                }
                Err(err) => {
                    return Err(Error::Setup {
                        name: test_actor.name().to_string(),
                        source: Some(err.into()),
                    })
                }
            }
        }

        // Update owned state tree
        let (root, blockstore) = executor.into_store();
        self.state_tree.override_inner(blockstore, root).unwrap();

        // Increment the sequence for the methods tests.
        let sequence = self.state_tree.actor_sequence(self.account.0)?;

        // TODO concurrent testing
        // We'll be able to use thread to do concurrent testing once we set the Engine Pool with more than
        // one possible concurrent engine.
        // The following steps will not end up in a result. Either we could finalize message
        // handling and we return the related ApplyRet or we return nothing.
        Ok(test_actor
            .abi
            .methods
            .iter()
            .map(|method| {
                let root = self.state_tree.flush();
                let blockstore = self.state_tree.store().clone();
                // TODO is it possible to impl `Clone` for `DefaultExecutor`
                // and submit PR upstream to implement with it?
                let mut executor = KytheraExecutor::new(
                    blockstore,
                    root,
                    self.builtin_actors.root,
                    self.account.1,
                    target_id.clone(),
                );

                log::debug!(
                    "Testing test {}.{}() for Actor {}",
                    test_actor.name,
                    method.name(),
                    target.name
                );
                let message = executor.execute_method(test_address, method.number(), sequence);

                let ret = match message {
                    Ok(apply_ret) => match (method.r#type(), apply_ret.msg_receipt.exit_code) {
                        (MethodType::Test, ExitCode::OK) => TestResultType::Passed(apply_ret),
                        (MethodType::TestFail, exit_code) => {
                            if exit_code == ExitCode::OK {
                                TestResultType::Failed(apply_ret)
                            } else {
                                TestResultType::Passed(apply_ret)
                            }
                        }
                        _ => TestResultType::Failed(apply_ret),
                    },
                    Err(err) => TestResultType::Erred(err.to_string()),
                };

                let result = TestResult {
                    method: method.clone(),
                    ret,
                };
                if let Some(ref sender) = stream_results {
                    if let Err(err) = sender.send((test_actor.clone(), result.clone())) {
                        log::error!("Could not Stream the Result: {err}");
                    }
                }
                result
            })
            .collect())
    }
}

impl Default for Tester {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fvm_ipld_blockstore::Blockstore;

    #[test]
    fn test_tester_instantiation() {
        // Get state_tree loaded with builtins
        let mut state_tree = StateTree::new();
        let builtins_actors = state_tree.load_builtin_actors();

        // Instantiate tester
        let tester = Tester::new();

        // Testing that we got proper CIDs for our revision for builtin actors and that they are
        // set in the state tree
        assert_eq!(tester.builtin_actors.root, builtins_actors.root);
        assert!(tester
            .state_tree
            .store()
            .has(&builtins_actors.root)
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_system_code(),
            *builtins_actors.manifest.get_system_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(builtins_actors.manifest.get_system_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_init_code(),
            *builtins_actors.manifest.get_init_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(builtins_actors.manifest.get_init_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_account_code(),
            *builtins_actors.manifest.get_account_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(builtins_actors.manifest.get_account_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_placeholder_code(),
            *builtins_actors.manifest.get_placeholder_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(builtins_actors.manifest.get_placeholder_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_eam_code(),
            *builtins_actors.manifest.get_eam_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(builtins_actors.manifest.get_eam_code())
            .unwrap());

        // Expect actor Id to be 102 as we deployed verified registry signer & multisig previously
        assert_eq!(tester.account.0, 102);

        assert!(tester.target_actor.is_none());
    }
}

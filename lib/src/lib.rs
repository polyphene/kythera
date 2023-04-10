// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

pub use kythera_common::{
    abi::{pascal_case_split, Abi, Method, MethodType},
    from_slice, to_vec,
};

use kythera_fvm::{
    executor::{ApplyRet, KytheraExecutor},
    Account,
};
use std::sync::mpsc::Sender;

use fvm_shared::{address::Address, bigint::Zero, econ::TokenAmount, error::ExitCode};

use error::Error;
use state_tree::{BuiltInActors, StateTree};

mod error;
mod state_tree;

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
}

/// An Actor that has been deployed into a `BlockStore`.
#[derive(Debug, Clone)]
struct DeployedActor {
    name: String,
    address: Address,
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
pub struct TestResult<'a> {
    method: &'a Method,
    ret: TestResultType,
}

impl<'a> TestResult<'a> {
    /// Get the [`Method`] tested.
    pub fn method(&self) -> &Method {
        self.method
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
    pub results: Result<Vec<TestResult<'a>>, Error>,
}

impl Tester {
    /// Create a new Kythera Tester.
    pub fn new() -> Self {
        let mut state_tree = StateTree::new();

        let builtin_actors = state_tree.load_builtin_actors();
        let account = state_tree.create_account(*builtin_actors.manifest.get_account_code());

        Self {
            builtin_actors,
            state_tree,
            account,
            target_actor: None,
            sequence: 0,
        }
    }

    /// Deploy the target Actor file into the `StateTree`.
    pub fn deploy_target_actor(&mut self, actor: WasmActor) -> Result<(), Error> {
        let address = self
            .state_tree
            .deploy_actor_from_bin(&actor, TokenAmount::zero())?;
        self.target_actor = Some(DeployedActor {
            name: actor.name,
            address,
        });

        Ok(())
    }

    // Get and increment the next Actor sequence.
    pub fn next_sequence(&mut self) -> u64 {
        let sequence = self.sequence;
        self.sequence = sequence + 1;
        sequence
    }

    /// Test an Actor on a `MemoryBlockstore`.
    pub fn test<'a>(
        &mut self,
        test_actors: &'a [WasmActor],
        stream_results: Option<Sender<(&'a WasmActor, TestResult<'a>)>>,
    ) -> Result<Vec<TestActorResults<'a>>, Error> {
        let target = self
            .target_actor
            .as_ref()
            .cloned()
            .ok_or(Error::MissingActor)?;

        let target_id = match target.address.id() {
            Ok(id) => id.to_ne_bytes().to_vec(),
            Err(_) => panic!("Actor Id should be valid"),
        };

        // Iterate over all test actors
        Ok(test_actors
            .iter()
            .map(|test_actor| {
                // Deploy test actor
                let test_address = match self
                    .state_tree
                    .deploy_actor_from_bin(test_actor, TokenAmount::zero())
                {
                    // Properly deployed, get address
                    Ok(test_address) => test_address,
                    // Error on deployment, return error as part of [`TestResults`]
                    Err(err) => {
                        return TestActorResults {
                            test_actor,
                            results: Err(err),
                        }
                    }
                };

                log::info!("Testing Actor {}", target.name);

                let root = self.state_tree.flush();
                let blockstore = self.state_tree.store().clone();
                let mut executor = KytheraExecutor::new(
                    blockstore,
                    root,
                    self.builtin_actors.root,
                    self.account.1,
                    test_address,
                    target_id.clone().into(),
                );

                // Run the constructor if it exists.
                if let Some(constructor) = test_actor.abi().constructor() {
                    match executor.execute_method(constructor.number(), self.next_sequence()) {
                        Ok(apply_ret) => {
                            if apply_ret.msg_receipt.exit_code != ExitCode::OK {
                                let source = apply_ret.failure_info.map(|f| f.to_string().into());
                                return TestActorResults {
                                    test_actor,
                                    results: Err(Error::ConstructorError { source }),
                                };
                            }
                        }
                        Err(err) => {
                            return TestActorResults {
                                test_actor,
                                results: Err(Error::ConstructorError {
                                    source: Some(err.into()),
                                }),
                            }
                        }
                    }
                }

                // Run Setup if it exists.
                if let Some(set_up) = test_actor.abi().set_up() {
                    match executor.execute_method(set_up.number(), self.next_sequence()) {
                        Ok(apply_ret) => {
                            if apply_ret.msg_receipt.exit_code != ExitCode::OK {
                                let source = apply_ret.failure_info.map(|f| f.to_string().into());
                                return TestActorResults {
                                    test_actor,
                                    results: Err(Error::SetupError { source }),
                                };
                            }
                        }
                        Err(err) => {
                            return TestActorResults {
                                test_actor,
                                results: Err(Error::SetupError {
                                    source: Some(err.into()),
                                }),
                            }
                        }
                    }
                }

                let (root, blockstore) = executor.into_store();

                // Increment the sequence for the methods tests.
                let sequence = self.next_sequence();

                // TODO concurrent testing
                // We'll be able to use thread to do concurrent testing once we set the Engine Pool with more than
                // one possible concurrent engine.
                // The following steps will not end up in a result. Either we could finalize message
                // handling and we return the related ApplyRet or we return nothing.
                TestActorResults {
                    test_actor,
                    results: Ok(
                        test_actor
                            .abi
                            .methods
                            .iter()
                            .map(|method| {
                                // TODO is it possible to impl `Clone` for `DefaultExecutor`
                                // and submit PR upstream to implement with it?
                                let mut executor = KytheraExecutor::new(
                                    blockstore.clone(),
                                    root,
                                    self.builtin_actors.root,
                                    self.account.1,
                                    test_address,
                                    target_id.clone().into(),
                                );

                                log::info!(
                                    "Testing test {}.{}() for Actor {}",
                                    test_actor.name,
                                    method.name(),
                                    target.name
                                );
                                let message = executor.execute_method(method.number(), sequence);

                                let ret = match message {
                                    Ok(apply_ret) => {
                                        match (method.r#type(), apply_ret.msg_receipt.exit_code) {
                                            (MethodType::Test, ExitCode::OK)
                                            | (
                                                MethodType::TestFail,
                                                ExitCode::USR_ASSERTION_FAILED,
                                            ) => TestResultType::Passed(apply_ret),
                                            _ => TestResultType::Failed(apply_ret),
                                        }
                                    }
                                    Err(err) => TestResultType::Erred(err.to_string()),
                                };

                                let result = TestResult { method, ret };
                                if let Some(ref sender) = stream_results {
                                    if let Err(err) = sender.send((test_actor, result.clone())) {
                                        log::error!("Could not Stream the Result: {err}");
                                    }
                                }
                                result
                            })
                            .collect(),
                    ),
                }
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
            .has(&*builtins_actors.manifest.get_system_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_init_code(),
            *builtins_actors.manifest.get_init_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(&*builtins_actors.manifest.get_init_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_account_code(),
            *builtins_actors.manifest.get_account_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(&*builtins_actors.manifest.get_account_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_placeholder_code(),
            *builtins_actors.manifest.get_placeholder_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(&*builtins_actors.manifest.get_placeholder_code())
            .unwrap());

        assert_eq!(
            *tester.builtin_actors.manifest.get_eam_code(),
            *builtins_actors.manifest.get_eam_code()
        );
        assert!(tester
            .state_tree
            .store()
            .has(&*builtins_actors.manifest.get_eam_code())
            .unwrap());

        // Expect actor Id to be 102 as we deployed verified registry signer & multisig previously
        assert_eq!(tester.account.0, 102);

        assert!(tester.target_actor.is_none());
    }
}

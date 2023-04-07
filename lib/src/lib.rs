use std::sync::mpsc::Sender;

// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
use cid::Cid;

pub use kythera_common::{
    abi::{pascal_case_split, Abi, Method, MethodType},
    from_slice, to_vec,
};

use kythera_fvm::{
    engine::EnginePool,
    executor::{ApplyKind, ApplyRet, Executor, KytheraExecutor},
    externs::FakeExterns,
    machine::{KytheraMachine, NetworkConfig},
    Account,
};

use fvm_ipld_blockstore::Blockstore;
use fvm_shared::{
    address::Address, bigint::Zero, econ::TokenAmount, error::ExitCode, message::Message,
    version::NetworkVersion,
};

use error::Error;
use state_tree::{BuiltInActors, StateTree};

mod error;
mod state_tree;

const NETWORK_VERSION: NetworkVersion = NetworkVersion::V18;
const DEFAULT_BASE_FEE: u64 = 100;

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
        }
    }

    /// Create a new `Executor` to test the provided test Actor.
    fn new_executor<B: Blockstore + 'static>(
        blockstore: B,
        state_root: Cid,
        builtin_actors: Cid,
    ) -> KytheraExecutor<B, FakeExterns> {
        let mut nc = NetworkConfig::new(NETWORK_VERSION);
        nc.override_actors(builtin_actors);
        nc.enable_actor_debugging();

        let mut mc = nc.for_epoch(0, 0, state_root);
        mc.set_base_fee(TokenAmount::from_atto(DEFAULT_BASE_FEE))
            .enable_tracing();

        let code_cids = vec![];

        let engine = EnginePool::new_default((&mc.network.clone()).into())
            .expect("Should be able to start EnginePool");
        engine
            .acquire()
            .preload(&blockstore, &code_cids)
            .expect("Should be able to preload Executor");

        let machine = KytheraMachine::new(&mc, blockstore, FakeExterns::new())
            .expect("Should be able to start KytheraMachine");

        KytheraExecutor::new(engine, machine).expect("Should be able to start Executor")
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

                let root = self.state_tree.flush();

                log::info!("Testing Actor {}", target.name);

                TestActorResults {
                    test_actor,
                    results: Ok(
                        test_actor
                            .abi
                            .methods
                            .iter()
                            .map(|method| {
                                let blockstore = self.state_tree.store().clone();

                                let mut executor =
                                    Self::new_executor(blockstore, root, self.builtin_actors.root);

                                let message = Message {
                                    from: self.account.1,
                                    to: test_address,
                                    gas_limit: 1000000000,
                                    method_num: method.number(),
                                    params: target_id.clone().into(),
                                    ..Message::default()
                                };

                                log::info!(
                                    "Testing test {}.{}() for Actor {}",
                                    test_actor.name,
                                    method.name(),
                                    target.name
                                );
                                let message =
                                    executor.execute_message(message, ApplyKind::Explicit, 100);

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
    use fvm_shared::error::ExitCode;
    use kythera_test_actors::wasm_bin::BASIC_TEST_ACTOR_BINARY;

    use super::*;
    use kythera_common::abi::{Abi, Method};

    const TARGET_WAT: &str = r#"
        ;; Mock invoke function
            (module
                (func (export "invoke") (param $x i32) (result i32)
                    (i32.const 1)
                )
            )
        "#;

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

        assert_eq!(tester.account.0, 100);

        assert!(tester.target_actor.is_none());
    }

    #[test]
    fn test_tester_test() {
        // Instantiate tester
        let mut tester = Tester::new();

        // Set target actor
        let target_wasm_bin = wat::parse_str(TARGET_WAT).unwrap();
        let target_abi = Abi { methods: vec![] };
        let target_actor = WasmActor::new(String::from("Target"), target_wasm_bin, target_abi);

        // Set test actor
        let test_wasm_bin: Vec<u8> = Vec::from(BASIC_TEST_ACTOR_BINARY);
        let test_abi = Abi {
            methods: vec![
                Method::new_from_name("TestOne").unwrap(),
                Method::new_from_name("TestTwo").unwrap(),
            ],
        };
        let test_actor = WasmActor::new(String::from("Basic"), test_wasm_bin, test_abi);

        match tester.deploy_target_actor(target_actor) {
            Err(_) => {
                panic!("Could not set target Actor when testing Tester")
            }
            _ => {}
        }

        match tester.test(&[test_actor.clone()], None) {
            Err(_) => {
                panic!("Could not run test when testing Tester")
            }
            Ok(test_res) => {
                assert_eq!(test_res.len(), 1usize);
                assert_eq!(test_res[0].results.as_ref().unwrap().len(), 2usize);
                assert_eq!(test_res[0].test_actor, &test_actor);

                test_res[0]
                    .results
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .for_each(
                        |(i, result)| match (result.method().r#type(), result.ret()) {
                            (MethodType::Test, TestResultType::Passed(apply_ret)) => {
                                assert_eq!(apply_ret.msg_receipt.exit_code, ExitCode::OK);
                                let ret_value: String =
                                    apply_ret.msg_receipt.return_data.deserialize().unwrap();
                                if i == 0usize {
                                    assert_eq!(ret_value, String::from("TestOne"))
                                } else {
                                    assert_eq!(ret_value, String::from("TestTwo"))
                                }
                            }
                            _ => panic!("test against basic test actor should pass"),
                        },
                    )
            }
        }
    }
}

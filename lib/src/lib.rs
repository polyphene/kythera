// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
use cid::Cid;

pub use kythera_common::abi::{pascal_case_split, Abi};
use kythera_fvm::{
    engine::EnginePool,
    executor::{ApplyKind, ApplyRet, Executor, KytheraExecutor},
    externs::FakeExterns,
    machine::{KytheraMachine, NetworkConfig},
    Account,
};

use fvm_ipld_blockstore::Blockstore;
use fvm_shared::{
    address::Address, bigint::Zero, econ::TokenAmount, message::Message, version::NetworkVersion,
};

use crate::error::WrapFVMError;
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
    // TODO: parse the Abi methods from the bytecode instead of receiving it via constructor.
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
}

/// An Actor that has been deployed into a `BlockStore`.
#[derive(Debug, Clone)]
struct DeployedActor {
    name: String,
    address: Address,
}

/// An Actor that has been deployed into a `BlockStore`.
#[derive(Debug)]
pub struct TestResults {
    pub test_actor: WasmActor,
    pub results: Result<Vec<Result<ApplyRet, Error>>, Error>,
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
    pub fn test(&mut self, test_actors: &[WasmActor]) -> Result<Vec<TestResults>, Error> {
        // TODO: Should we clone the `StateTree` before each test run,
        // and make our `Tester` stateless?
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
                        return TestResults {
                            test_actor: test_actor.clone(),
                            results: Err(err),
                        }
                    }
                };

                let root = self.state_tree.flush();

                log::info!("Testing Actor {}", target.name);

                // TODO concurrent testing
                // We'll be able to use thread to do concurrent testing once we set the Engine Pool with more than
                // one possible concurrent engine.
                // The following steps will not end up in a result. Either we could finalize message
                // handling and we return the related ApplyRet or we return nothing.
                TestResults {
                    test_actor: test_actor.clone(),
                    results: Ok(test_actor
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
                                method_num: method.number,
                                params: target_id.clone().into(),
                                ..Message::default()
                            };

                            log::info!(
                                "Testing test {}.{}() for Actor {}",
                                test_actor.name,
                                method.name,
                                target.name
                            );
                            let apply_ret = executor
                                .execute_message(message, ApplyKind::Explicit, 100)
                                .tester_err("Couldn't execute message")?;
                            Ok(apply_ret)
                        })
                        .collect()),
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
                Method {
                    number: 3948827889,
                    name: String::from("TestOne"),
                },
                Method {
                    number: 891686990,
                    name: String::from("TestTwo"),
                },
            ],
        };
        let test_actor = WasmActor::new(String::from("Basic"), test_wasm_bin, test_abi);

        match tester.deploy_target_actor(target_actor) {
            Err(_) => {
                panic!("Could not set target Actor when testing Tester")
            }
            _ => {}
        }

        match tester.test(&[test_actor.clone()]) {
            Err(_) => {
                panic!("Could not run test when testing Tester")
            }
            Ok(test_res) => {
                assert_eq!(test_res.len(), 1usize);
                assert_eq!(test_res[0].results.as_ref().unwrap().len(), 2usize);
                assert_eq!(test_res[0].test_actor, test_actor);

                test_res[0]
                    .results
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .for_each(|(i, option_apply_ret)| match option_apply_ret {
                        Ok(apply_ret) => {
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
                    })
            }
        }
    }
}

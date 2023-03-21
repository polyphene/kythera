// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
use cid::Cid;

use kythera_common::abi::ABI;
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
    abi: ABI,
}

impl WasmActor {
    /// Create a new WebAssembly Actor.
    pub fn new(name: String, bytecode: Vec<u8>, abi: ABI) -> Self {
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

    /// Deploy the main Actor file into the `StateTree`.
    pub fn deploy_main_actor(&mut self, name: String, actor: WasmActor) -> Result<(), Error> {
        let address = self
            .state_tree
            .deploy_actor_from_bin(&actor, TokenAmount::zero())?;
        self.target_actor = Some(DeployedActor { name, address });

        Ok(())
    }

    /// Test an Actor on a `MemoryBlockstore`.
    pub fn test(&mut self, test_actor: &WasmActor) -> Result<Vec<Option<ApplyRet>>, Error> {
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

        let test_address = self
            .state_tree
            .deploy_actor_from_bin(&test_actor, TokenAmount::zero())?;

        let root = self.state_tree.flush();

        log::info!("Testing Actor {}", target.name);

        // TODO concurrent testing
        // We'll be able to use thread to do concurrent testing once we set the Engine Pool with more than
        // one possible concurrent engine.
        Ok(test_actor
            .abi
            .methods
            .iter()
            .map(|method| {
                let blockstore = self.state_tree.store().clone();

                let mut executor = Self::new_executor(blockstore, root, self.builtin_actors.root);

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
                match executor.execute_message(message, ApplyKind::Explicit, 100) {
                    Err(err) => {
                        log::info!(
                            "Error while testing {}.{}() for Actor: {}",
                            test_actor.name,
                            method.name,
                            target.name
                        );
                        log::info!("{}", err.to_string());
                        None
                    }
                    Ok(apply_ret) => {
                        log::info!(
                            "Could test  {}.{}() for Actor: {}",
                            test_actor.name,
                            method.name,
                            target.name
                        );

                        Some(apply_ret)
                    }
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

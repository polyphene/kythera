// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

mod error;
mod state_tree;

use crate::error::{Error, WrapFVMError};
use crate::state_tree::StateTree;

use cid::Cid;
use fvm_ipld_blockstore::Blockstore;
use fvm_shared::bigint::Zero;
use fvm_shared::version::NetworkVersion;
use fvm_shared::{address::Address, econ::TokenAmount, message::Message};
use kythera_fvm::executor::{ApplyKind, Executor};
use kythera_fvm::{
    engine::EnginePool,
    executor::KytheraExecutor,
    externs::FakeExterns,
    machine::{KytheraMachine, NetworkConfig},
    Account,
};
use state_tree::BuiltInActors;

// TODO: document purpose.
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WasmActor {
    name: String,
    bytecode: Vec<u8>,
}

impl WasmActor {
    /// Create a new WebAssembly Actor.
    pub fn new(name: String, bytecode: Vec<u8>) -> Self {
        Self { name, bytecode }
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
        let mut nc = NetworkConfig::new(NetworkVersion::V18);
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

    /// Test an Actor on a `MemoryBlockstore`.
    pub fn test(&mut self, tests: &[WasmActor]) -> Result<(), Error> {
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

        log::info!("testing Actor {}", target.name);

        for test in tests {
            log::info!("testing test {} to Actor {}", test.name, target.name);

            let test_address = self
                .state_tree
                .deploy_actor_from_bin(&test, TokenAmount::zero())?;

            let root = self.state_tree.flush();

            let blockstore = self.state_tree.store().clone();

            let mut executor = Self::new_executor(blockstore, root, self.builtin_actors.root);

            let message = Message {
                from: self.account.1,
                to: test_address,
                gas_limit: 1000000000,
                method_num: 1,
                params: target_id.clone().into(),
                ..Message::default()
            };

            executor
                .execute_message(message, ApplyKind::Explicit, 100)
                .tester_err(&format!("Could not test the Actor: {}", target.name))
                .map(|_| ())?;
        }
        Ok(())
    }
}

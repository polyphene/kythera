// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use cid::Cid;
use fvm_ipld_blockstore::MemoryBlockstore;
use fvm_ipld_encoding::RawBytes;
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use fvm_shared::message::Message;
use fvm_shared::version::NetworkVersion;
use fvm_shared::MethodNum;
use kythera_fvm::engine::EnginePool;
use kythera_fvm::executor::{ApplyRet, Executor as _};
use kythera_fvm::machine::NetworkConfig;
use kythera_fvm::{
    executor::{ApplyKind, KytheraExecutor},
    externs::FakeExterns,
    machine::{KytheraMachine, Machine},
};

use crate::error::{Error, WrapFVMError};

const NETWORK_VERSION: NetworkVersion = NetworkVersion::V18;
const DEFAULT_BASE_FEE: u64 = 100;

/// Wrapper around `fvm` Executor with sane defaults.
pub struct Executor {
    inner: KytheraExecutor<MemoryBlockstore, FakeExterns>,
    account_address: Address,
    test_address: Address,
    target_actor_id: RawBytes,
}

impl Executor {
    /// Create a new `Executor`.
    pub fn new(
        blockstore: MemoryBlockstore,
        state_root: Cid,
        builtin_actors: Cid,
        account_address: Address,
        test_address: Address,
        target_actor_id: RawBytes,
    ) -> Executor {
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

        Self {
            inner: KytheraExecutor::new(engine, machine).expect("Should be able to start Executor"),
            account_address,
            test_address,
            target_actor_id,
        }
    }

    /// Execute the provided method.
    pub fn execute_method(
        &mut self,
        method_num: MethodNum,
        sequence: u64,
    ) -> Result<ApplyRet, Error> {
        let message = Message {
            from: self.account_address,
            to: self.test_address,
            gas_limit: 1000000000,
            method_num,
            params: self.target_actor_id.clone().into(),
            sequence,
            ..Message::default()
        };

        self.inner
            .execute_message(message, ApplyKind::Explicit, 100)
            .tester_err("Could not execute message")
    }

    /// Convert the executor back into a [`Blockstore`].
    pub fn into_store(mut self) -> (Cid, MemoryBlockstore) {
        let root = self
            .inner
            .flush()
            .expect("Should be able to flush Executor");

        let blockstore = self
            .inner
            .into_machine()
            .expect("Machine should exist at this point")
            .into_store()
            .into_inner();
        (root, blockstore)
    }
}

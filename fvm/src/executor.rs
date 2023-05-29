// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::externs::FakeExterns;
use crate::machine::KytheraMachine;
use cid::Cid;
use fvm::engine::EnginePool;
use fvm::executor::DefaultExecutor;

use crate::kernel::KytheraKernel;
use crate::utils::KYTHERA_NETWORK_ID;
pub use fvm::executor::Executor as _;
pub use fvm::executor::{ApplyFailure, ApplyKind, ApplyRet};
use fvm::machine::{DefaultMachine, Machine, NetworkConfig};
use fvm_ipld_blockstore::{Buffered, MemoryBlockstore};
use fvm_ipld_encoding::RawBytes;
use fvm_shared::address::Address;
use fvm_shared::chainid::ChainID;
use fvm_shared::econ::TokenAmount;
use fvm_shared::message::Message;
use fvm_shared::version::NetworkVersion;
use fvm_shared::MethodNum;

const NETWORK_VERSION: NetworkVersion = NetworkVersion::V18;
const DEFAULT_BASE_FEE: u64 = 100;

/// Wrapper around `fvm` Executor with sane defaults.
pub struct KytheraExecutor {
    inner: DefaultExecutor<KytheraKernel>,
    account_address: Address,
    target_actor_id: RawBytes,
}

impl KytheraExecutor {
    /// Create a new `Executor`.
    pub fn new(
        blockstore: MemoryBlockstore,
        state_root: Cid,
        builtin_actors: Cid,
        account_address: Address,
        target_actor_id: RawBytes,
    ) -> Self {
        let mut nc = NetworkConfig::new(NETWORK_VERSION);
        nc.override_actors(builtin_actors);
        nc.enable_actor_debugging();
        // If chain Id is 0 (invalid value) we set our default
        if nc.chain_id == ChainID::from(0) {
            nc.chain_id = ChainID::from(KYTHERA_NETWORK_ID)
        }

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

        let machine = KytheraMachine::<DefaultMachine<MemoryBlockstore, FakeExterns>>::new(
            mc,
            blockstore,
            FakeExterns::new(),
        )
        .expect("Should be able to start KytheraMachine");

        Self {
            inner: DefaultExecutor::new(engine, machine).expect("Should be able to start Executor"),
            account_address,
            target_actor_id,
        }
    }

    /// Execute the provided method.
    pub fn execute_method(
        &mut self,
        to: Address,
        method_num: MethodNum,
        sequence: u64,
    ) -> Result<ApplyRet, anyhow::Error> {
        let message = Message {
            from: self.account_address,
            to,
            gas_limit: 1000000000,
            method_num,
            params: self.target_actor_id.clone(),
            sequence,
            version: 0,
            value: TokenAmount::default(),
            gas_fee_cap: TokenAmount::default(),
            gas_premium: TokenAmount::default(),
        };

        self.inner
            .execute_message(message, ApplyKind::Explicit, 100)
    }

    /// Convert the executor back into a [`Blockstore`].
    pub fn into_store(mut self) -> (Cid, MemoryBlockstore) {
        let root = self
            .inner
            .flush()
            .expect("Should be able to flush Executor");

        let mut machine: KytheraMachine = self
            .inner
            .into_machine()
            .expect("Machine should exist at this point");

        machine.state_tree_mut();

        let buff_blockstore = machine.into_store();
        buff_blockstore
            .flush(&root)
            .expect("Should be able to flush Buffered Blockstore");

        let memory_blockstore = buff_blockstore.into_inner();

        (root, memory_blockstore)
    }
}

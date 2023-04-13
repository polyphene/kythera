use crate::context::OverrideContext;
use crate::externs::FakeExterns;
use crate::machine::KytheraMachine;
use anyhow::anyhow;
use cid::Cid;
use fvm::call_manager::{CallManager, DefaultCallManager, FinishRet, InvocationResult};
use fvm::engine::Engine;
use fvm::gas::{Gas, GasTracker};
use fvm::kernel::{Block, ExecutionError};
use fvm::machine::Machine;
use fvm::state_tree::ActorState;
use fvm::{DefaultKernel, Kernel};
use fvm_ipld_blockstore::MemoryBlockstore;
use fvm_ipld_encoding::from_slice;
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use fvm_shared::event::StampedEvent;
use fvm_shared::{ActorID, MethodNum};

pub struct KytheraCallManager {
    inner: DefaultCallManager<KytheraMachine<MemoryBlockstore, FakeExterns>>,
    override_context: OverrideContext,
}

impl CallManager for KytheraCallManager {
    type Machine = KytheraMachine<MemoryBlockstore, FakeExterns>;

    fn new(
        machine: Self::Machine,
        engine: Engine,
        gas_limit: u64,
        origin: ActorID,
        origin_address: Address,
        receiver: Option<ActorID>,
        receiver_address: Address,
        nonce: u64,
        gas_premium: TokenAmount,
    ) -> Self {
        Self {
            inner: DefaultCallManager::new(
                machine,
                engine,
                gas_limit,
                origin,
                origin_address,
                receiver,
                receiver_address,
                nonce,
                gas_premium,
            ),
            override_context: OverrideContext::default(),
        }
    }

    fn send<K: Kernel<CallManager = Self>>(
        &mut self,
        from: ActorID,
        to: Address,
        method: MethodNum,
        params: Option<Block>,
        value: &TokenAmount,
        gas_limit: Option<Gas>,
        read_only: bool,
    ) -> fvm::kernel::Result<InvocationResult> {
        if &to == &Address::new_id(98) {
            if method == *crate::utils::WARP_NUM {
                if !params.is_none() {
                    let new_timestamp: u64 = from_slice(params.clone().unwrap().data()).unwrap();
                    dbg!(new_timestamp);
                }
            }
        }
        // TODO Having call manager require the Kernel to refer to the same structure prevent us from doing this
        self.inner.send::<DefaultKernel<DefaultCallManager<KytheraMachine<MemoryBlockstore, FakeExterns>>>>(
            from, to, method, params, value, gas_limit, read_only,
        )
    }

    fn with_transaction(
        &mut self,
        _f: impl FnOnce(&mut Self) -> fvm::kernel::Result<InvocationResult>,
    ) -> fvm::kernel::Result<InvocationResult> {
        // TODO having the callback function refering to this structure prevent us from passing it to the inner call manager
        Err(ExecutionError::Fatal(anyhow!("aa")))
    }

    fn finish(self) -> (fvm::kernel::Result<FinishRet>, Self::Machine) {
        self.inner.finish()
    }

    fn machine(&self) -> &Self::Machine {
        self.inner.machine()
    }

    fn machine_mut(&mut self) -> &mut Self::Machine {
        self.inner.machine_mut()
    }

    fn engine(&self) -> &Engine {
        self.inner.engine()
    }

    fn gas_tracker(&self) -> &GasTracker {
        self.inner.gas_tracker()
    }

    fn gas_premium(&self) -> &TokenAmount {
        self.inner.gas_premium()
    }

    fn origin(&self) -> ActorID {
        self.inner.origin()
    }

    fn next_actor_address(&self) -> Address {
        self.inner.next_actor_address()
    }

    fn create_actor(
        &mut self,
        code_id: Cid,
        actor_id: ActorID,
        delegated_address: Option<Address>,
    ) -> fvm::kernel::Result<()> {
        self.inner
            .create_actor(code_id, actor_id, delegated_address)
    }

    fn resolve_address(&self, address: &Address) -> fvm::kernel::Result<Option<ActorID>> {
        self.inner.resolve_address(address)
    }

    fn set_actor(&mut self, id: ActorID, state: ActorState) -> fvm::kernel::Result<()> {
        self.inner.set_actor(id, state)
    }

    fn get_actor(&self, id: ActorID) -> fvm::kernel::Result<Option<ActorState>> {
        self.inner.get_actor(id)
    }

    fn delete_actor(&mut self, id: ActorID) -> fvm::kernel::Result<()> {
        self.inner.delete_actor(id)
    }

    fn transfer(
        &mut self,
        from: ActorID,
        to: ActorID,
        value: &TokenAmount,
    ) -> fvm::kernel::Result<()> {
        self.inner.transfer(from, to, value)
    }

    fn nonce(&self) -> u64 {
        self.inner.nonce()
    }

    fn invocation_count(&self) -> u64 {
        self.inner.invocation_count()
    }

    fn limiter_mut(&mut self) -> &mut <Self::Machine as Machine>::Limiter {
        self.inner.limiter_mut()
    }

    fn append_event(&mut self, evt: StampedEvent) {
        self.inner.append_event(evt)
    }
}

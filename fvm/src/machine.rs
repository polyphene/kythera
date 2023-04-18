use crate::context::OverrideContext;
use crate::externs::FakeExterns;
use fvm::machine::MachineContext;
pub use fvm::machine::{DefaultMachine, Machine, Manifest, NetworkConfig};
use fvm::state_tree::StateTree;
use fvm_ipld_blockstore::MemoryBlockstore;

pub struct KytheraMachine<M = DefaultMachine<MemoryBlockstore, FakeExterns>> {
    inner: M,
    pub(crate) override_context: OverrideContext,
}

impl<M> KytheraMachine<M>
where
    M: Machine,
{
    pub fn new(
        context: MachineContext,
        blockstore: MemoryBlockstore,
        externs: FakeExterns,
    ) -> anyhow::Result<KytheraMachine<DefaultMachine<MemoryBlockstore, FakeExterns>>> {
        let machine = DefaultMachine::new(&context, blockstore, externs)?;
        Ok(KytheraMachine {
            inner: machine,
            override_context: OverrideContext::default(),
        })
    }

    pub fn override_context(&self) -> &OverrideContext {
        &self.override_context
    }
}

impl<M> Machine for KytheraMachine<M>
where
    M: Machine,
{
    type Blockstore = M::Blockstore;
    type Externs = M::Externs;
    type Limiter = M::Limiter;

    fn blockstore(&self) -> &Self::Blockstore {
        self.inner.blockstore()
    }

    fn context(&self) -> &MachineContext {
        self.inner.context()
    }

    fn externs(&self) -> &Self::Externs {
        self.inner.externs()
    }

    fn builtin_actors(&self) -> &Manifest {
        self.inner.builtin_actors()
    }

    fn state_tree(&self) -> &StateTree<Self::Blockstore> {
        self.inner.state_tree()
    }

    fn state_tree_mut(&mut self) -> &mut StateTree<Self::Blockstore> {
        self.inner.state_tree_mut()
    }

    fn into_store(self) -> Self::Blockstore {
        self.inner.into_store()
    }

    fn machine_id(&self) -> &str {
        self.inner.machine_id()
    }

    fn new_limiter(&self) -> Self::Limiter {
        self.inner.new_limiter()
    }
}

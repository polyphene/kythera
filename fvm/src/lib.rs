// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

pub use fvm::{account_actor, init_actor, system_actor};
use fvm_shared::address::Address;
use fvm_shared::ActorID;

pub mod engine {
    pub use fvm::engine::EnginePool;
}

pub mod executor {
    use crate::machine::KytheraMachine;
    use fvm::call_manager::DefaultCallManager;
    use fvm::executor::DefaultExecutor;
    use fvm::DefaultKernel;

    pub use fvm::executor::Executor;
    pub use fvm::executor::{ApplyFailure, ApplyKind, ApplyRet};
    pub type KytheraExecutor<B, E> =
        DefaultExecutor<DefaultKernel<DefaultCallManager<KytheraMachine<B, E>>>>;
}

pub mod machine {
    pub use fvm::machine::{DefaultMachine as KytheraMachine, Machine, Manifest, NetworkConfig};
}

pub mod state_tree {
    pub use fvm::state_tree::ActorState;
    pub use fvm::state_tree::StateTree;
}

pub type Account = (ActorID, Address);

pub mod externs;

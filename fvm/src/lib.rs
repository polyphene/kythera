// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

pub use fvm::{account_actor, init_actor, system_actor};
use fvm_shared::address::Address;
use fvm_shared::ActorID;

pub mod engine {
    pub use fvm::engine::EnginePool;
}

pub mod executor;
pub mod machine {
    pub use fvm::machine::{DefaultMachine as KytheraMachine, Machine, Manifest, NetworkConfig};
}

pub mod state_tree {
    pub use fvm::state_tree::ActorState;
    pub use fvm::state_tree::StateTree;
}

pub type Account = (ActorID, Address);

mod call_manager;
mod context;
pub mod externs;
mod kernel;
pub(crate) mod utils;

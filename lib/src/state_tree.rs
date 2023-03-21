// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use crate::{
    error::{Error, WrapFVMError},
    WasmActor,
};

use cid::{multihash::Code, Cid};
use futures::executor::block_on;
use fvm_ipld_blockstore::{Block, Blockstore, MemoryBlockstore};
use fvm_ipld_car::load_car_unchecked;
use fvm_ipld_encoding::{serde::Serialize, CborStore};
use fvm_shared::{address::Address, econ::TokenAmount, state::StateTreeVersion, ActorID, IPLD_RAW};
use kythera_fvm::{
    account_actor, init_actor, machine::Manifest, state_tree::ActorState, system_actor, Account,
};
use libsecp256k1::{PublicKey, SecretKey};
use rand::SeedableRng;

const EAM_ACTOR_ID: ActorID = 10;
const STATE_TREE_VERSION: StateTreeVersion = StateTreeVersion::V5;

/// Built-in Actors that are deployed to the testing `StateTree`.
pub struct BuiltInActors {
    pub root: Cid,
    pub manifest: Manifest,
}

/// Test `StateTree`
pub struct StateTree {
    // The inner `StateTree`.
    inner: kythera_fvm::state_tree::StateTree<MemoryBlockstore>,
}

impl StateTree {
    /// Create a new Testing `StateTree`.
    pub fn new() -> Self {
        let bs = MemoryBlockstore::default();
        let inner = kythera_fvm::state_tree::StateTree::new(bs, STATE_TREE_VERSION)
            .expect("Should be able to put the Version in the StateTree");

        Self { inner }
    }

    pub fn flush(&mut self) -> cid::CidGeneric<64> {
        self.inner
            .flush()
            .expect("Should be able to acquire the root CID by flushing")
    }
    /// Retrieve the inner `BlockStore`.
    pub fn store(&self) -> &MemoryBlockstore {
        self.inner.store()
    }
    /// set actor on the `Blockstore`.
    /// And activate them on the `StateTree`.
    fn set_actor<S: Serialize>(
        &mut self,
        name: &str,
        state: S,
        code_cid: Cid,
        id: ActorID,
        sequence: u64,
        balance: TokenAmount,
    ) -> Result<(), Error> {
        let state_cid = self
            .inner
            .store()
            .put_cbor(&state, Code::Blake2b256)
            .setting_err(name)?;

        let actor_state = ActorState {
            code: code_cid,
            state: state_cid,
            sequence,
            balance,
            delegated_address: None,
        };

        log::trace!("Setting Actor {} on the BlockStore", name);
        self.inner.set_actor(id, actor_state);
        Ok(())
    }

    /// Load the built-in actors into the `Blockstore`.
    /// And activate them on the `StateTree`.
    pub fn load_builtin_actors(&mut self) -> BuiltInActors {
        let blockstore = self.inner.store();
        // Load the built-in Actors
        let builtin_actors =
            block_on(async { load_car_unchecked(blockstore, actors_v10::BUNDLE_CAR).await })
                .expect("Should be able to import built-in Actors")[0];

        let (version, root) = blockstore
            .get_cbor::<(u32, Cid)>(&builtin_actors)
            .expect("Should be able to decode the built-in Actor CBOR")
            .expect("There should be manifest information for built-in Actor Cid");

        let manifest = Manifest::load(blockstore, &root, version)
            .expect("Should be able to load built-in Actor manifest");

        // deploy built-in Actors on the StateTree.
        let init_state = init_actor::State::new_test(&blockstore);
        let sys_state = system_actor::State {
            builtin_actors: root,
        };

        self.set_actor(
            "System Actor",
            sys_state,
            *manifest.get_system_code(),
            system_actor::SYSTEM_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the system Actor");

        self.set_actor(
            "Init Actor",
            init_state,
            *manifest.get_init_code(),
            init_actor::INIT_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the Init Actor");

        self.set_actor(
            "Eam Actor",
            &[(); 0],
            *manifest.get_eam_code(),
            // EAM Actor Id.
            EAM_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the Eam Actor");

        BuiltInActors {
            root: builtin_actors,
            manifest,
        }
    }

    /// Creates new accounts in the testing context
    /// Inserts the account in the state tree, all with the provided balance, returning it and its public key address.
    pub fn create_account(&mut self, accounts_code_cid: Cid) -> Account {
        let rng = &mut rand_chacha::ChaCha8Rng::seed_from_u64(8);

        let priv_key = SecretKey::random(rng);

        let pub_key = PublicKey::from_secret_key(&priv_key);
        let pub_key_addr =
            Address::new_secp256k1(&pub_key.serialize()).expect("PublicKey length should be valid");

        let assigned_addr = self
            .inner
            .register_new_address(&pub_key_addr)
            .expect("Should be able to register an account public key on the StateTree");

        let state = account_actor::State {
            address: pub_key_addr,
        };

        let cid = self
            .inner
            .store()
            .put_cbor(&state, Code::Blake2b256)
            .expect("Should be able to put the Actor State as CBOR");

        let actor_state = ActorState {
            code: accounts_code_cid,
            state: cid,
            sequence: 0,
            balance: TokenAmount::from_atto(10000),
            delegated_address: None,
        };

        self.inner.set_actor(assigned_addr, actor_state);
        (assigned_addr, pub_key_addr)
    }

    /// Deploy a new Actor at a given address, provided with a given token balance
    /// and returns the CodeCID of the installed actor
    pub fn deploy_actor_from_bin(
        &mut self,
        actor: &WasmActor,
        balance: TokenAmount,
    ) -> Result<Address, Error> {
        let actor_id = rand::random();
        let actor_address = Address::new_id(actor_id);

        // Put the WASM code into the blockstore.
        log::debug!("Deploying Actor {} code", actor.name);
        let code_cid = self
            .inner
            .store()
            .put(
                Code::Blake2b256,
                &Block {
                    codec: IPLD_RAW,
                    data: &actor.bytecode,
                },
            )
            .setting_err(&actor.name)?;

        // Set the Actor State on the `BlockStore`.
        self.set_actor(&actor.name, &[(); 0], code_cid, actor_id, 0, balance)?;

        Ok(actor_address)
    }
}

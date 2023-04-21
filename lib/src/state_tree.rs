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
use fvm_shared::{
    address::Address, econ::TokenAmount, state::StateTreeVersion, ActorID, HAMT_BIT_WIDTH, IPLD_RAW,
};
use kythera_fvm::{account_actor, machine::Manifest, state_tree::ActorState, Account};
use libsecp256k1::{PublicKey, SecretKey};
use rand::SeedableRng;

use fil_actors_runtime_v10::runtime::builtins::Type;
use fil_actors_runtime_v10::{
    make_empty_map, BURNT_FUNDS_ACTOR_ADDR, BURNT_FUNDS_ACTOR_ID, CRON_ACTOR_ID,
    DATACAP_TOKEN_ACTOR_ID, INIT_ACTOR_ID, REWARD_ACTOR_ID, STORAGE_MARKET_ACTOR_ADDR,
    STORAGE_MARKET_ACTOR_ID, STORAGE_POWER_ACTOR_ADDR, STORAGE_POWER_ACTOR_ID, SYSTEM_ACTOR_ID,
    VERIFIED_REGISTRY_ACTOR_ADDR, VERIFIED_REGISTRY_ACTOR_ID,
};
use fvm_shared::bigint::Zero;
use fvm_shared::sector::StoragePower;
use kythera_actors::wasm_bin::CHEATCODES_ACTOR_BINARY;
use kythera_common::abi::Abi;

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
        // Load the built-in Actors
        let builtin_actors = block_on(async {
            load_car_unchecked(self.inner.store(), actors_v10::BUNDLE_CAR).await
        })
        .expect("Should be able to import built-in Actors")[0];

        let (version, root) = self
            .inner
            .store()
            .get_cbor::<(u32, Cid)>(&builtin_actors)
            .expect("Should be able to decode the built-in Actor CBOR")
            .expect("There should be manifest information for built-in Actor Cid");

        let manifest = Manifest::load(self.inner.store(), &root, version)
            .expect("Should be able to load built-in Actor manifest");

        // Set system actor.
        let sys_state = fil_actor_system_v10::State {
            builtin_actors: root,
        };
        self.set_actor(
            "System Actor",
            sys_state,
            *manifest
                .code_by_id(Type::System as u32)
                .expect("Should be able to get System Actor code from manifest"),
            SYSTEM_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the System Actor");

        // Set init actor.
        let init_state = fil_actor_init_v10::State::new(self.inner.store(), "test".to_string())
            .expect("Should be able to initialize Init Actor state");
        self.set_actor(
            "Init Actor",
            init_state,
            *manifest
                .code_by_id(Type::Init as u32)
                .expect("Should be able to get Init Actor code from manifest"),
            INIT_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Init Actor");

        // Set reward actor.
        let reward_state = fil_actor_reward_v10::State::new(StoragePower::zero());
        self.set_actor(
            "Reward Actor",
            reward_state,
            *manifest
                .code_by_id(Type::Reward as u32)
                .expect("Should be able to get Reward Actor code from manifest"),
            REWARD_ACTOR_ID,
            0,
            TokenAmount::from_whole(1_100_000_000),
        )
        .expect("Should be able to set the Reward Actor");

        // Set cron actor.
        let cron_state = fil_actor_cron_v10::State {
            entries: vec![
                fil_actor_cron_v10::Entry {
                    receiver: STORAGE_POWER_ACTOR_ADDR,
                    method_num: fil_actor_power_v10::Method::OnEpochTickEnd as u64,
                },
                fil_actor_cron_v10::Entry {
                    receiver: STORAGE_MARKET_ACTOR_ADDR,
                    method_num: fil_actor_market_v10::Method::CronTick as u64,
                },
            ],
        };
        self.set_actor(
            "Cron Actor",
            cron_state,
            *manifest
                .code_by_id(Type::Cron as u32)
                .expect("Should be able to get Cron Actor code from manifest"),
            CRON_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Cron Actor");

        // Set power actor.
        let power_state = fil_actor_power_v10::State::new(self.inner.store())
            .expect("Should be able to initialize Power Actor state");
        self.set_actor(
            "Storage Power Actor",
            power_state,
            *manifest
                .code_by_id(Type::Power as u32)
                .expect("Should be able to get Storage Power Actor code from manifest"),
            STORAGE_POWER_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Storage Power Actor");

        // Set market actor.
        let market_state = fil_actor_market_v10::State::new(self.inner.store())
            .expect("Should be able to initialize Market Actor state");
        self.set_actor(
            "Storage Market Actor",
            market_state,
            *manifest
                .code_by_id(Type::Market as u32)
                .expect("Should be able to get Storage Market Actor code from manifest"),
            STORAGE_MARKET_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Storage Market Actor");

        // Deploy multisig and a signer to act as verified registry root.
        // Initialize signer address.
        let verified_reg_signer_address =
            Address::new_bls(&[200; fvm_shared::address::BLS_PUB_LEN])
                .expect("Should be able to generate verified registry multisig signer address");
        let verified_reg_signer_id = self
            .inner
            .register_new_address(&verified_reg_signer_address)
            .expect("Should be able to register verified registry multisig signer address");
        // Initialize signer state.
        let verified_reg_signer_state = fil_actor_account_v10::State {
            address: verified_reg_signer_address,
        };

        // Set signer actor.
        self.set_actor(
            "Verified Registry Signer",
            verified_reg_signer_state,
            *manifest
                .code_by_id(Type::Account as u32)
                .expect("Should be able to get Account Actor code from manifest"),
            verified_reg_signer_id,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Verified Registry Signer");

        // Initialize verified registry root address.
        let empty_root = make_empty_map::<_, ()>(self.inner.store(), HAMT_BIT_WIDTH)
            .flush()
            .expect("Should be able to generate an empty root CID");
        let verified_reg_root_address = Address::new_actor(b"VerifiedRegistryRoot");
        let verified_reg_root_id = self
            .inner
            .register_new_address(&verified_reg_root_address)
            .expect("Should be able to register verified registry multisig root address");
        // Initialize verified registry root state.
        let verified_reg_root_state = fil_actor_multisig_v10::State {
            signers: vec![Address::new_id(verified_reg_signer_id)],
            num_approvals_threshold: 1,
            next_tx_id: Default::default(),
            initial_balance: Default::default(),
            start_epoch: 0,
            unlock_duration: 0,
            pending_txs: empty_root,
        };
        // Set verified registry root.
        self.set_actor(
            "Verified Registry Root",
            verified_reg_root_state,
            *manifest
                .code_by_id(Type::Multisig as u32)
                .expect("Should be able to get Multisig Actor code from manifest"),
            verified_reg_root_id,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Verified Registry Root");

        // Set verified registry itself.
        let verified_reg_state = fil_actor_verifreg_v10::State::new(
            self.inner.store(),
            Address::new_id(verified_reg_root_id),
        )
        .expect("Should be able to initialize Verified Registry Actor state");
        self.set_actor(
            "Verified Registry Actor",
            verified_reg_state,
            *manifest
                .code_by_id(Type::VerifiedRegistry as u32)
                .expect("Should be able to get Verified Registry Actor code from manifest"),
            VERIFIED_REGISTRY_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Verified Registry Actor");

        // Set datacap actor.
        let datacap_state =
            fil_actor_datacap_v10::State::new(self.inner.store(), VERIFIED_REGISTRY_ACTOR_ADDR)
                .expect("Should be able to initialize Datacap Actor state");
        self.set_actor(
            "Datacap Actor",
            datacap_state,
            *manifest
                .code_by_id(Type::DataCap as u32)
                .expect("Should be able to get Datacap Actor code from manifest"),
            DATACAP_TOKEN_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Datacap Actor");

        // Set burnt funds actor.
        let burnt_state = fil_actor_account_v10::State {
            address: BURNT_FUNDS_ACTOR_ADDR,
        };
        self.set_actor(
            "Burnt Funds Actor",
            burnt_state,
            *manifest
                .code_by_id(Type::Account as u32)
                .expect("Should be able to get Burnt Funds Actor code from manifest"),
            BURNT_FUNDS_ACTOR_ID,
            0,
            TokenAmount::zero(),
        )
        .expect("Should be able to set the Burnt Funds Actor");

        BuiltInActors {
            root: builtin_actors,
            manifest,
        }
    }

    /// Load Kythera utilities' actors.
    pub fn load_kythera_actors(&mut self) {
        // Deploy cheatcodes actor.
        let cheatcodes_actor = WasmActor::new(
            String::from("Cheatcodes"),
            CHEATCODES_ACTOR_BINARY.to_vec(),
            Abi::default(),
        );

        self.deploy_actor_from_bin_at_address(
            &Address::new_id(98u64),
            &cheatcodes_actor,
            TokenAmount::zero(),
        )
        .expect("Should be able to load cheatcodes actor");
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

    fn deploy_actor_from_bin_at_address(
        &mut self,
        address: &Address,
        actor: &WasmActor,
        balance: TokenAmount,
    ) -> Result<(), Error> {
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
        self.set_actor(
            &actor.name,
            [(); 0],
            code_cid,
            address
                .id()
                .expect("Should be able to get actor Id from address"),
            0,
            balance,
        )
    }

    /// Deploy a new Actor at a given address, provided with a given token balance
    /// and returns the CodeCID of the installed actor
    pub fn deploy_actor_from_bin(
        &mut self,
        actor: &WasmActor,
        balance: TokenAmount,
    ) -> Result<Address, Error> {
        let actor_address = Address::new_actor(actor.name.as_bytes());
        let actor_id = self
            .inner
            .register_new_address(&actor_address)
            .expect("Should be able to register verified registry multisig root address");
        let actor_address_id = Address::new_id(actor_id);
        self.deploy_actor_from_bin_at_address(&actor_address_id, actor, balance)?;
        Ok(actor_address_id)
    }
}

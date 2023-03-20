// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

mod error;

use cid::{multihash::Code, Cid};
use fil_builtin_actors_bundle::BUNDLE_CAR;
use futures::executor::block_on;
use fvm::{
    engine::EnginePool,
    executor::{ApplyKind, DefaultExecutor, Executor},
    init_actor,
    machine::{DefaultMachine, Manifest, NetworkConfig},
    state_tree::{ActorState, StateTree},
    system_actor,
};
use fvm_integration_tests::{
    dummy::DummyExterns,
    tester::{Account, IntegrationExecutor},
};
use fvm_ipld_blockstore::{Block, Blockstore, MemoryBlockstore};
use fvm_ipld_car::load_car_unchecked;
use fvm_ipld_encoding::{serde::Serialize, CborStore};
use libsecp256k1::{PublicKey, SecretKey};
use rand::SeedableRng;

use error::{Error, WrapFVMError};
use fvm_shared::{
    address::Address, bigint::Zero, econ::TokenAmount, message::Message, state::StateTreeVersion,
    version::NetworkVersion, ActorID, IPLD_RAW,
};

// TODO: document purpose.
const EAM_ACTOR_ID: ActorID = 10;
const NETWORK_VERSION: NetworkVersion = NetworkVersion::V18;
const DEFAULT_BASE_FEE: u64 = 100;

/// Main interface to test `Actor`s with Kythera.
pub struct Tester {
    // Builtin actors root Cid used in the Machine
    builtin_actors: BuiltInActors,
    // Custom code cid deployed by developer
    code_cids: Vec<Cid>,
    // State tree constructed before instantiating the Machine
    state_tree: StateTree<MemoryBlockstore>,
    // Account used for testing.
    account: Account,
    // Id of the main Actor deployed to be tested.
    main_actor_id: Option<Vec<u8>>,
}

/// Built-in Actors that are deployed to the testing `StateTree`.
struct BuiltInActors {
    root: Cid,
    manifest: Manifest,
}

/// WebAssembly Actor.
pub struct WasmActor {
    name: String,
    code: Vec<u8>,
}

impl WasmActor {
    /// Create a new WebAssembly Actor.
    pub fn new(name: String, code: Vec<u8>) -> Self {
        Self { name, code }
    }
}

impl Tester {
    /// Create a new Kythera Tester.
    pub fn new() -> Self {
        let bs = MemoryBlockstore::default();

        // Initialize state tree
        let mut state_tree = StateTree::new(bs, StateTreeVersion::V5)
            .expect("Should be able to put the Version in the StateTree");

        let builtin_actors = Self::load_builtin_actors(&mut state_tree);

        let account =
            Self::create_account(&mut state_tree, *builtin_actors.manifest.get_account_code());

        Self {
            builtin_actors,
            code_cids: vec![],
            state_tree,
            account,
            main_actor_id: None,
        }
    }

    /// Load the built-in actors into the `Blockstore`.
    /// And activate them on the `StateTree`.
    fn load_builtin_actors<B: Blockstore>(state_tree: &mut StateTree<B>) -> BuiltInActors {
        let blockstore = state_tree.store();
        // Load the built-in Actors
        let builtin_actors = block_on(async { load_car_unchecked(blockstore, BUNDLE_CAR).await })
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

        Self::set_actor(
            state_tree,
            "System Actor",
            sys_state,
            *manifest.get_system_code(),
            system_actor::SYSTEM_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the system Actor");

        Self::set_actor(
            state_tree,
            "Init Actor",
            init_state,
            *manifest.get_init_code(),
            init_actor::INIT_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the Init Actor");

        Self::set_actor(
            state_tree,
            "Eam Actor",
            &[(); 0],
            *manifest.get_eam_code(),
            EAM_ACTOR_ID,
            1,
            Default::default(),
        )
        .expect("Should be able to set the Eam Actor");
        BuiltInActors { root, manifest }
    }

    /// Create a new `Executor` to test the provided test Actor.
    fn new_executor<B: Blockstore + 'static>(
        blockstore: B,
        state_root: Cid,
        builtin_actors: Cid,
    ) -> IntegrationExecutor<B, DummyExterns> {
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

        let machine = DefaultMachine::new(&mc, blockstore, DummyExterns)
            .expect("Should be able to start DefaultMachine");

        DefaultExecutor::new(engine, machine).expect("Should be able to start Executor")
    }

    /// set actor on the `Blockstore`.
    /// And activate them on the `StateTree`.
    fn set_actor<B: Blockstore, S: Serialize>(
        state_tree: &mut StateTree<B>,
        name: &str,
        state: S,
        code_cid: Cid,
        id: ActorID,
        sequence: u64,
        balance: TokenAmount,
    ) -> Result<(), Error> {
        let state_cid = state_tree
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
        state_tree.set_actor(id, actor_state);
        Ok(())
    }

    /// Creates new accounts in the testing context
    /// Inserts the account in the state tree, all with the provided balance, returning it and its public key address.
    fn create_account<B: Blockstore>(
        state_tree: &mut StateTree<B>,
        accounts_code_cid: Cid,
    ) -> Account {
        let rng = &mut rand_chacha::ChaCha8Rng::seed_from_u64(8);

        let priv_key = SecretKey::random(rng);

        let pub_key = PublicKey::from_secret_key(&priv_key);
        let pub_key_addr =
            Address::new_secp256k1(&pub_key.serialize()).expect("PublicKey length should be valid");

        let assigned_addr = state_tree
            .register_new_address(&pub_key_addr)
            .expect("Should be able to register an account public key on the StateTree");

        let state = fvm::account_actor::State {
            address: pub_key_addr,
        };

        let cid = state_tree
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

        state_tree.set_actor(assigned_addr, actor_state);
        (assigned_addr, pub_key_addr)
    }

    /// Deploy a new Actor at a given address, provided with a given token balance
    /// and returns the CodeCID of the installed actor
    fn deploy_actor_from_bin(
        &mut self,
        actor: &WasmActor,
        balance: TokenAmount,
    ) -> Result<Address, Error> {
        let actor_id = rand::random();
        let actor_address = Address::new_id(actor_id);

        // Put the WASM code into the blockstore.
        log::debug!("Deploying Actor {} code ", actor.name);
        let code_cid = self
            .state_tree
            .store()
            .put(
                Code::Blake2b256,
                &Block {
                    codec: IPLD_RAW,
                    data: &actor.code,
                },
            )
            .setting_err(&actor.name)?;

        // Add code cid to list of deployed contract
        self.code_cids.push(code_cid);

        // Set the Actor State on the `BlockStore`.
        Self::set_actor(
            &mut self.state_tree,
            &actor.name,
            &[(); 0],
            code_cid,
            actor_id,
            0,
            balance,
        )?;

        Ok(actor_address)
    }

    /// Deploy the main Actor file into the `StateTree`.
    pub fn deploy_main_actor(&mut self, actor: WasmActor) -> Result<(), Error> {
        let address = self.deploy_actor_from_bin(&actor, TokenAmount::zero())?;
        self.main_actor_id = match address.id() {
            Ok(id) => Some(id.to_ne_bytes().to_vec()),
            Err(_) => panic!("Actor Id should be valid"),
        };

        Ok(())
    }

    /// Test an Actor on a `MemoryBlockstore`.
    pub fn test(&mut self, actor: WasmActor, test: WasmActor) -> Result<(), Error> {
        // TODO: Should we clone the `StateTree` before each test run,
        // and make our `Tester` stateless?

        let main_actor_id = self
            .main_actor_id
            .as_ref()
            .cloned()
            .ok_or(Error::MissingActor)?;

        let test_address = self.deploy_actor_from_bin(&test, TokenAmount::zero())?;

        let root = self
            .state_tree
            .flush()
            .expect("Should be able to acquire the root CID by flushing");

        let blockstore = self.state_tree.store().clone();

        let mut executor = Self::new_executor(blockstore, root, self.builtin_actors.root);

        let message = Message {
            from: self.account.1,
            to: test_address,
            gas_limit: 1000000000,
            method_num: 1,
            params: main_actor_id.into(),
            ..Message::default()
        };

        log::info!("testing test {} to Actor {}", test.name, actor.name);
        executor
            .execute_message(message, ApplyKind::Explicit, 100)
            .tester_err(&format!("Could not test the Actor: {}", actor.name))
            .map(|_| ())
    }
}

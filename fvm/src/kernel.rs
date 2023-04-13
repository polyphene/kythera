use crate::call_manager::KytheraCallManager;
use crate::externs::FakeExterns;
use crate::machine::KytheraMachine;
use cid::Cid;
use fvm::call_manager::CallManager;
use fvm::gas::{Gas, GasTimer, PriceList};
use fvm::kernel::{
    ActorOps, BlockId, BlockRegistry, BlockStat, CircSupplyOps, CryptoOps, DebugOps, EventOps,
    GasOps, IpldBlockOps, LimiterOps, MessageOps, NetworkOps, RandomnessOps, SelfOps, SendOps,
    SendResult,
};
use fvm::machine::Machine;
use fvm::{DefaultKernel, Kernel};
use fvm_ipld_blockstore::MemoryBlockstore;
use fvm_shared::address::Address;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::consensus::ConsensusFault;
use fvm_shared::crypto::signature::{
    SignatureType, SECP_PUB_LEN, SECP_SIG_LEN, SECP_SIG_MESSAGE_HASH_SIZE,
};
use fvm_shared::econ::TokenAmount;
use fvm_shared::piece::PieceInfo;
use fvm_shared::randomness::RANDOMNESS_LENGTH;
use fvm_shared::sector::{
    AggregateSealVerifyProofAndInfos, RegisteredSealProof, ReplicaUpdateInfo, SealVerifyInfo,
    WindowPoStVerifyInfo,
};
use fvm_shared::sys::out::network::NetworkContext;
use fvm_shared::sys::out::vm::MessageContext;
use fvm_shared::sys::SendFlags;
use fvm_shared::{ActorID, MethodNum};

pub struct KytheraKernel {
    inner: DefaultKernel<KytheraCallManager>,
}

impl Kernel for KytheraKernel {
    type CallManager = KytheraCallManager;

    fn into_inner(self) -> (Self::CallManager, BlockRegistry)
    where
        Self: Sized,
    {
        self.inner.into_inner()
    }

    fn new(
        mgr: Self::CallManager,
        blocks: BlockRegistry,
        caller: ActorID,
        actor_id: ActorID,
        method: MethodNum,
        value_received: TokenAmount,
        read_only: bool,
    ) -> Self
    where
        Self: Sized,
    {
        Self {
            inner: DefaultKernel::new(
                mgr,
                blocks,
                caller,
                actor_id,
                method,
                value_received,
                read_only,
            ),
        }
    }

    fn machine(&self) -> &<Self::CallManager as CallManager>::Machine {
        self.inner.machine()
    }
}

impl ActorOps for KytheraKernel {
    fn resolve_address(&self, address: &Address) -> fvm::kernel::Result<ActorID> {
        self.inner.resolve_address(address)
    }

    fn lookup_delegated_address(&self, actor_id: ActorID) -> fvm::kernel::Result<Option<Address>> {
        self.inner.lookup_delegated_address(actor_id)
    }

    fn get_actor_code_cid(&self, id: ActorID) -> fvm::kernel::Result<Cid> {
        self.inner.get_actor_code_cid(id)
    }

    fn next_actor_address(&self) -> fvm::kernel::Result<Address> {
        self.inner.next_actor_address()
    }

    fn create_actor(
        &mut self,
        code_cid: Cid,
        actor_id: ActorID,
        delegated_address: Option<Address>,
    ) -> fvm::kernel::Result<()> {
        self.inner
            .create_actor(code_cid, actor_id, delegated_address)
    }

    fn get_builtin_actor_type(&self, code_cid: &Cid) -> fvm::kernel::Result<u32> {
        self.inner.get_builtin_actor_type(code_cid)
    }

    fn get_code_cid_for_type(&self, typ: u32) -> fvm::kernel::Result<Cid> {
        self.inner.get_code_cid_for_type(typ)
    }

    fn balance_of(&self, actor_id: ActorID) -> fvm::kernel::Result<TokenAmount> {
        self.inner.balance_of(actor_id)
    }
}

impl IpldBlockOps for KytheraKernel {
    fn block_open(&mut self, cid: &Cid) -> fvm::kernel::Result<(BlockId, BlockStat)> {
        self.inner.block_open(cid)
    }

    fn block_create(&mut self, codec: u64, data: &[u8]) -> fvm::kernel::Result<BlockId> {
        self.inner.block_create(codec, data)
    }

    fn block_link(
        &mut self,
        id: BlockId,
        hash_fun: u64,
        hash_len: u32,
    ) -> fvm::kernel::Result<Cid> {
        self.inner.block_link(id, hash_fun, hash_len)
    }

    fn block_read(&self, id: BlockId, offset: u32, buf: &mut [u8]) -> fvm::kernel::Result<i32> {
        self.inner.block_read(id, offset, buf)
    }

    fn block_stat(&self, id: BlockId) -> fvm::kernel::Result<BlockStat> {
        self.inner.block_stat(id)
    }
}

impl CircSupplyOps for KytheraKernel {
    fn total_fil_circ_supply(&self) -> fvm::kernel::Result<TokenAmount> {
        self.inner.total_fil_circ_supply()
    }
}

impl CryptoOps for KytheraKernel {
    fn verify_signature(
        &self,
        sig_type: SignatureType,
        signature: &[u8],
        signer: &Address,
        plaintext: &[u8],
    ) -> fvm::kernel::Result<bool> {
        self.inner
            .verify_signature(sig_type, signature, signer, plaintext)
    }

    fn recover_secp_public_key(
        &self,
        hash: &[u8; SECP_SIG_MESSAGE_HASH_SIZE],
        signature: &[u8; SECP_SIG_LEN],
    ) -> fvm::kernel::Result<[u8; SECP_PUB_LEN]> {
        self.inner.recover_secp_public_key(hash, signature)
    }

    fn hash(&self, code: u64, data: &[u8]) -> fvm::kernel::Result<multihash::Multihash> {
        self.inner.hash(code, data)
    }

    fn compute_unsealed_sector_cid(
        &self,
        proof_type: RegisteredSealProof,
        pieces: &[PieceInfo],
    ) -> fvm::kernel::Result<Cid> {
        self.inner.compute_unsealed_sector_cid(proof_type, pieces)
    }

    fn verify_seal(&self, vi: &SealVerifyInfo) -> fvm::kernel::Result<bool> {
        self.inner.verify_seal(vi)
    }

    fn verify_post(&self, verify_info: &WindowPoStVerifyInfo) -> fvm::kernel::Result<bool> {
        self.inner.verify_post(verify_info)
    }

    fn verify_consensus_fault(
        &self,
        h1: &[u8],
        h2: &[u8],
        extra: &[u8],
    ) -> fvm::kernel::Result<Option<ConsensusFault>> {
        self.inner.verify_consensus_fault(h1, h2, extra)
    }

    fn batch_verify_seals(&self, vis: &[SealVerifyInfo]) -> fvm::kernel::Result<Vec<bool>> {
        self.inner.batch_verify_seals(vis)
    }

    fn verify_aggregate_seals(
        &self,
        aggregate: &AggregateSealVerifyProofAndInfos,
    ) -> fvm::kernel::Result<bool> {
        self.inner.verify_aggregate_seals(aggregate)
    }

    fn verify_replica_update(&self, replica: &ReplicaUpdateInfo) -> fvm::kernel::Result<bool> {
        self.inner.verify_replica_update(replica)
    }
}

impl DebugOps for KytheraKernel {
    fn log(&self, msg: String) {
        self.inner.log(msg)
    }

    fn debug_enabled(&self) -> bool {
        self.inner.debug_enabled()
    }

    fn store_artifact(&self, name: &str, data: &[u8]) -> fvm::kernel::Result<()> {
        self.inner.store_artifact(name, data)
    }
}

impl EventOps for KytheraKernel {
    fn emit_event(&mut self, raw_evt: &[u8]) -> fvm::kernel::Result<()> {
        self.inner.emit_event(raw_evt)
    }
}

impl GasOps for KytheraKernel {
    fn gas_used(&self) -> Gas {
        self.inner.gas_used()
    }

    fn gas_available(&self) -> Gas {
        self.inner.gas_available()
    }

    fn charge_gas(&self, name: &str, compute: Gas) -> fvm::kernel::Result<GasTimer> {
        self.inner.charge_gas(name, compute)
    }

    fn price_list(&self) -> &PriceList {
        self.inner.price_list()
    }
}

impl MessageOps for KytheraKernel {
    fn msg_context(&self) -> fvm::kernel::Result<MessageContext> {
        self.inner.msg_context()
    }
}

impl NetworkOps for KytheraKernel {
    fn network_context(&self) -> fvm::kernel::Result<NetworkContext> {
        self.inner.network_context()
    }

    fn tipset_cid(&self, epoch: ChainEpoch) -> fvm::kernel::Result<Cid> {
        self.inner.tipset_cid(epoch)
    }
}

impl RandomnessOps for KytheraKernel {
    fn get_randomness_from_tickets(
        &self,
        personalization: i64,
        rand_epoch: ChainEpoch,
        entropy: &[u8],
    ) -> fvm::kernel::Result<[u8; RANDOMNESS_LENGTH]> {
        self.inner
            .get_randomness_from_tickets(personalization, rand_epoch, entropy)
    }

    fn get_randomness_from_beacon(
        &self,
        personalization: i64,
        rand_epoch: ChainEpoch,
        entropy: &[u8],
    ) -> fvm::kernel::Result<[u8; RANDOMNESS_LENGTH]> {
        self.inner
            .get_randomness_from_beacon(personalization, rand_epoch, entropy)
    }
}

impl SelfOps for KytheraKernel {
    fn root(&self) -> fvm::kernel::Result<Cid> {
        self.inner.root()
    }

    fn set_root(&mut self, root: Cid) -> fvm::kernel::Result<()> {
        self.inner.set_root(root)
    }

    fn current_balance(&self) -> fvm::kernel::Result<TokenAmount> {
        self.inner.current_balance()
    }

    fn self_destruct(&mut self, beneficiary: &Address) -> fvm::kernel::Result<()> {
        self.inner.self_destruct(beneficiary)
    }
}

impl SendOps for KytheraKernel {
    fn send(
        &mut self,
        recipient: &Address,
        method: u64,
        params: BlockId,
        value: &TokenAmount,
        gas_limit: Option<Gas>,
        flags: SendFlags,
    ) -> fvm::kernel::Result<SendResult> {
        self.inner
            .send(recipient, method, params, value, gas_limit, flags)
    }
}

impl LimiterOps for KytheraKernel {
    type Limiter = <KytheraMachine<MemoryBlockstore, FakeExterns> as Machine>::Limiter;

    fn limiter_mut(&mut self) -> &mut Self::Limiter {
        self.inner.limiter_mut()
    }
}

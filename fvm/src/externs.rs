// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::Result;
use cid::Cid;
use fvm::externs::{Chain, Consensus, Externs, Rand};
use fvm_ipld_encoding::DAG_CBOR;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::consensus;
use fvm_shared::IDENTITY_HASH;
use multihash::MultihashGeneric;
use std::rc::Rc;

/// Provides chain or beacon randomness externally.
pub type RandomnessSource = Rc<dyn Fn(i64, ChainEpoch, &[u8]) -> Result<[u8; 32]>>;

/// Returns a randomness source that returns a constant value.
pub fn const_randomness(v: [u8; 32]) -> RandomnessSource {
    Rc::new(move |_pers, _round, _entropy| Ok(v))
}

/// Provides consensus fault evaluation externally.
pub type ConsensusFaultSource =
    Rc<dyn Fn(&[u8], &[u8], &[u8]) -> Result<(Option<consensus::ConsensusFault>, i64)>>;

/// Returns a constant evaluation of consensus fault evidence.
pub fn const_consensus_fault(
    fault: Option<consensus::ConsensusFault>,
    epoch: ChainEpoch,
) -> ConsensusFaultSource {
    Rc::new(move |_h1, _h2, _extra| Ok((fault.clone(), epoch)))
}

/// Provides tipset CIDs externally.
pub type TipsetSource = Rc<dyn Fn(ChainEpoch) -> Result<Cid>>;

/// Returns a tipset source that returns a constant value.
pub fn const_tipset(cid: Cid) -> TipsetSource {
    Rc::new(move |_epoch| Ok(cid))
}

/// An implementation of VM externs that can be controlled externally for tests.
#[derive(Clone)]
pub struct FakeExterns {
    chain_randomness: RandomnessSource,
    beacon_randomness: RandomnessSource,
    consensus_fault: ConsensusFaultSource,
    tipset: TipsetSource,
}

impl FakeExterns {
    /// Returns a new fake externs that returns constant zero values for all calls.
    pub fn new() -> Self {
        Self {
            chain_randomness: const_randomness([0; 32]),
            beacon_randomness: const_randomness([0; 32]),
            consensus_fault: const_consensus_fault(None, 0),
            tipset: const_tipset(Cid::new_v1(
                DAG_CBOR,
                MultihashGeneric::wrap(IDENTITY_HASH, &0u64.to_be_bytes()).unwrap(),
            )),
        }
    }

    pub fn with_chain_randomness(mut self, randomness: RandomnessSource) -> Self {
        self.chain_randomness = randomness;
        self
    }
    pub fn with_beacon_randomness(mut self, randomness: RandomnessSource) -> Self {
        self.beacon_randomness = randomness;
        self
    }
    pub fn with_consensus_fault(mut self, fault: ConsensusFaultSource) -> Self {
        self.consensus_fault = fault;
        self
    }
    pub fn with_tipset(mut self, tipset: TipsetSource) -> Self {
        self.tipset = tipset;
        self
    }
}

impl Chain for FakeExterns {
    fn get_tipset_cid(&self, epoch: fvm_shared::clock::ChainEpoch) -> Result<Cid> {
        Ok(Cid::new_v1(
            DAG_CBOR,
            MultihashGeneric::wrap(IDENTITY_HASH, &epoch.to_be_bytes()).unwrap(),
        ))
    }
}

impl Externs for FakeExterns {}

impl Rand for FakeExterns {
    fn get_chain_randomness(
        &self,
        dst: i64,
        epoch: ChainEpoch,
        entropy: &[u8],
    ) -> Result<[u8; 32]> {
        (self.chain_randomness)(dst, epoch, entropy)
    }

    fn get_beacon_randomness(
        &self,
        dst: i64,
        epoch: ChainEpoch,
        entropy: &[u8],
    ) -> Result<[u8; 32]> {
        (self.beacon_randomness)(dst, epoch, entropy)
    }
}

impl Consensus for FakeExterns {
    fn verify_consensus_fault(
        &self,
        _h1: &[u8],
        _h2: &[u8],
        _extra: &[u8],
    ) -> Result<(Option<consensus::ConsensusFault>, i64)> {
        (self.consensus_fault)(_h1, _h2, _extra)
    }
}

impl Default for FakeExterns {
    fn default() -> Self {
        Self::new()
    }
}

// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use blake2b_simd::blake2b;
use frc42_dispatch::hash::Hasher;

pub struct Blake2bHasher {}

impl Hasher for Blake2bHasher {
    fn hash(&self, bytes: &[u8]) -> Vec<u8> {
        blake2b(bytes).as_bytes().to_vec()
    }
}

use kythera_common::abi::derive_method_num;
use once_cell::sync::Lazy;

pub const KYTHERA_NETWORK_ID: u64 = 1312;

pub(crate) const WARP_METHOD: &str = "Warp";
pub(crate) const EPOCH_METHOD: &str = "Epoch";
pub(crate) const FEE_METHOD: &str = "Fee";
pub(crate) const CHAIN_ID_METHOD: &str = "ChainId";
pub(crate) const PRANK_METHOD: &str = "Prank";
pub(crate) const TRICK_METHOD: &str = "Trick";

pub(crate) static WARP_NUM: Lazy<u64> = Lazy::new(|| derive_method_num(WARP_METHOD).unwrap());
pub(crate) static EPOCH_NUM: Lazy<u64> = Lazy::new(|| derive_method_num(EPOCH_METHOD).unwrap());
pub(crate) static FEE_NUM: Lazy<u64> = Lazy::new(|| derive_method_num(FEE_METHOD).unwrap());
pub(crate) static CHAIN_ID_NUM: Lazy<u64> =
    Lazy::new(|| derive_method_num(CHAIN_ID_METHOD).unwrap());
pub(crate) static PRANK_NUM: Lazy<u64> = Lazy::new(|| derive_method_num(PRANK_METHOD).unwrap());
pub(crate) static TRICK_NUM: Lazy<u64> = Lazy::new(|| derive_method_num(TRICK_METHOD).unwrap());

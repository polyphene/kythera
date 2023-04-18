use kythera_common::abi::derive_method_num;

pub const KYTHERA_NETWORK_ID: u64 = 1312;

pub(crate) const WARP_METHOD: &str = "Warp";
pub(crate) const EPOCH_METHOD: &str = "Epoch";
pub(crate) const FEE_METHOD: &str = "Fee";
pub(crate) const CHAIN_ID_METHOD: &str = "ChainId";
pub(crate) const PRANK_METHOD: &str = "Prank";
pub(crate) const TRICK_METHOD: &str = "Trick";

lazy_static::lazy_static! {
    pub(crate) static ref WARP_NUM: u64 = derive_method_num(WARP_METHOD).unwrap();
    pub(crate) static ref EPOCH_NUM: u64 = derive_method_num(EPOCH_METHOD).unwrap();
    pub(crate) static ref FEE_NUM: u64 = derive_method_num(FEE_METHOD).unwrap();
    pub(crate) static ref CHAIN_ID_NUM: u64 = derive_method_num(CHAIN_ID_METHOD).unwrap();
    pub(crate) static ref PRANK_NUM: u64 = derive_method_num(PRANK_METHOD).unwrap();
    pub(crate) static ref TRICK_NUM: u64 = derive_method_num(TRICK_METHOD).unwrap();

}

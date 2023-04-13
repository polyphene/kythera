use kythera_common::abi::derive_method_num;

pub(crate) const WARP_METHOD: &'static str = "Warp";

lazy_static::lazy_static! {
    pub(crate) static ref WARP_NUM: u64 = derive_method_num(WARP_METHOD).unwrap();
}

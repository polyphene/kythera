pub const KYTHERA_NETWORK_ID: u64 = 1312;

pub(crate) const WARP_NUM: u64 = 112632689;
pub(crate) const EPOCH_NUM: u64 = 1015545011;
pub(crate) const FEE_NUM: u64 = 1307676284;
pub(crate) const CHAIN_ID_NUM: u64 = 2832802136;
pub(crate) const PRANK_NUM: u64 = 3950310035;
pub(crate) const TRICK_NUM: u64 = 4270775027;
pub(crate) const LOG_NUM: u64 = 340034372;

#[cfg(test)]
mod test {
    use super::*;
    use kythera_common::abi::derive_method_num;

    // Cheatcodes methods names
    pub(crate) const WARP_METHOD: &str = "Warp";
    pub(crate) const EPOCH_METHOD: &str = "Epoch";
    pub(crate) const FEE_METHOD: &str = "Fee";
    pub(crate) const CHAIN_ID_METHOD: &str = "ChainId";
    pub(crate) const PRANK_METHOD: &str = "Prank";
    pub(crate) const TRICK_METHOD: &str = "Trick";
    pub(crate) const LOG_METHOD: &str = "Log";

    #[test]
    fn test_cheatcodes_number() {
        assert_eq!(WARP_NUM, derive_method_num(WARP_METHOD).unwrap());
        assert_eq!(EPOCH_NUM, derive_method_num(EPOCH_METHOD).unwrap());
        assert_eq!(FEE_NUM, derive_method_num(FEE_METHOD).unwrap());
        assert_eq!(CHAIN_ID_NUM, derive_method_num(CHAIN_ID_METHOD).unwrap());
        assert_eq!(PRANK_NUM, derive_method_num(PRANK_METHOD).unwrap());
        assert_eq!(TRICK_NUM, derive_method_num(TRICK_METHOD).unwrap());
        assert_eq!(LOG_NUM, derive_method_num(LOG_METHOD).unwrap());
    }
}

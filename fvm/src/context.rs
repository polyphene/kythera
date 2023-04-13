use fvm::machine::{MachineContext, NetworkConfig};
use fvm_shared::chainid::ChainID;
use fvm_shared::clock::ChainEpoch;
use fvm_shared::econ::TokenAmount;

pub trait Override<S> {
    fn override_with_context(&self, context: &S) -> Self;
}

#[derive(Default, Debug, Clone)]
pub struct OverrideContext {
    /// The Chain ID of the network.
    pub chain_id: Option<ChainID>,

    /// The current epoch.
    pub epoch: Option<ChainEpoch>,

    /// The UNIX timestamp (in seconds) of the current tipset.
    pub timestamp: Option<u64>,

    /// The base fee that's in effect when the Machine runs.
    pub base_fee: Option<TokenAmount>,
}

impl Override<OverrideContext> for MachineContext {
    fn override_with_context(&self, context: &OverrideContext) -> MachineContext {
        MachineContext {
            network: NetworkConfig {
                chain_id: context.chain_id.unwrap_or(self.network.chain_id),
                ..self.network.clone()
            },
            epoch: context.epoch.unwrap_or(self.epoch),
            timestamp: context.timestamp.unwrap_or(self.timestamp),
            base_fee: context.base_fee.clone().unwrap_or(self.base_fee.clone()),
            ..self.clone()
        }
    }
}

use fvm_shared::clock::ChainEpoch;
use fvm_shared::sys::out::network::NetworkContext;
use fvm_shared::sys::out::vm::MessageContext;
use fvm_shared::sys::TokenAmount;
use fvm_shared::ActorID;

pub trait Override<S> {
    fn override_with_context(&self, context: &S) -> Self;
}

#[derive(Default, Debug, Clone)]
pub struct OverrideContext {
    /// The Chain ID of the network.
    pub chain_id: Option<u64>,

    /// The current epoch.
    pub epoch: Option<ChainEpoch>,

    /// The UNIX timestamp (in seconds) of the current tipset.
    pub timestamp: Option<u64>,

    /// The base fee that's in effect when the Machine runs.
    pub base_fee: Option<TokenAmount>,

    /// The current call's origin actor ID.
    pub origin: Option<ActorID>,

    /// The caller's actor ID.
    pub caller: Option<ActorID>,
}

impl Override<OverrideContext> for NetworkContext {
    fn override_with_context(&self, context: &OverrideContext) -> NetworkContext {
        NetworkContext {
            chain_id: context.chain_id.unwrap_or(self.chain_id),
            epoch: context.epoch.unwrap_or(self.epoch),
            timestamp: context.timestamp.unwrap_or(self.timestamp),
            base_fee: context.base_fee.unwrap_or(self.base_fee),
            ..*self
        }
    }
}

impl Override<OverrideContext> for MessageContext {
    fn override_with_context(&self, context: &OverrideContext) -> MessageContext {
        MessageContext {
            origin: context.origin.unwrap_or(self.origin),
            ..*self
        }
    }
}

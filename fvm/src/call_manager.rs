use crate::kernel::KytheraKernel;
use crate::machine::KytheraMachine;
use crate::utils::{CHAIN_ID_NUM, EPOCH_NUM, FEE_NUM, PRANK_NUM, TRICK_NUM, WARP_NUM};
use anyhow::anyhow;
use cid::Cid;
use fvm::call_manager::{CallManager, DefaultCallManager, FinishRet, InvocationResult};
use fvm::engine::Engine;
use fvm::gas::{Gas, GasTracker};
use fvm::kernel::{Block, ExecutionError};
use fvm::machine::Machine;
use fvm::state_tree::ActorState;
use fvm::Kernel;
use fvm_ipld_encoding::from_slice;
use fvm_shared::address::Address;
use fvm_shared::econ::TokenAmount;
use fvm_shared::event::StampedEvent;
use fvm_shared::{ActorID, MethodNum};

#[repr(transparent)]
pub struct KytheraCallManager<C: CallManager = DefaultCallManager<KytheraMachine>>(pub C);

impl<M, C> KytheraCallManager<C>
where
    M: Machine,
    C: CallManager<Machine = KytheraMachine<M>>,
{
    fn handle_cheatcode(
        &mut self,
        method: MethodNum,
        params: Option<Block>,
    ) -> fvm::kernel::Result<()> {
        match method as u64 {
            WARP_NUM => {
                let new_timestamp: u64 = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for Warp cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for Warp cheatcode: {}",
                        err
                    )))
                })?;
                self.machine_mut().override_context.timestamp = Some(new_timestamp);
            }
            EPOCH_NUM => {
                let new_epoch: i64 = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for Epoch cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for Epoch cheatcode: {}",
                        err
                    )))
                })?;
                self.machine_mut().override_context.epoch = Some(new_epoch);
            }
            FEE_NUM => {
                let (lo, hi): (u64, u64) = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for Fee cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for Fee cheatcode: {}",
                        err
                    )))
                })?;

                self.machine_mut().override_context.base_fee =
                    Some(fvm_shared::sys::TokenAmount { lo, hi });
            }
            CHAIN_ID_NUM => {
                let chain_id: u64 = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for ChainId cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for ChainId cheatcode: {}",
                        err
                    )))
                })?;

                self.machine_mut().override_context.chain_id = Some(chain_id);
            }
            PRANK_NUM => {
                let new_caller: Address = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for Prank cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for Prank cheatcode: {}",
                        err
                    )))
                })?;

                let new_caller_id = match new_caller.id() {
                    Ok(id) => id,
                    Err(err) => {
                        return Err(ExecutionError::Fatal(anyhow!(format!(
                            "Address parameter for Prank should have a valid ActorID: {}",
                            err
                        ))))
                    }
                };

                self.machine_mut().override_context.caller = Some(new_caller_id);
            }
            TRICK_NUM => {
                let new_origin: Address = from_slice(
                    params
                        .ok_or(ExecutionError::Fatal(anyhow!(
                            "No parameters provided for Trick cheatcode"
                        )))?
                        .data(),
                )
                .map_err(|err| {
                    ExecutionError::Fatal(anyhow!(format!(
                        "Could not deserialize parameters for Trick cheatcode: {}",
                        err
                    )))
                })?;

                let new_origin_id = match new_origin.id() {
                    Ok(id) => id,
                    Err(err) => {
                        return Err(ExecutionError::Fatal(anyhow!(format!(
                            "Address parameter for Trick should have a valid ActorID: {}",
                            err
                        ))))
                    }
                };

                self.machine_mut().override_context.origin = Some(new_origin_id);
            }
            _ => return Err(ExecutionError::Fatal(anyhow!("Call to unknown cheatcode"))),
        }

        Ok(())
    }
}

impl<M, C> CallManager for KytheraCallManager<C>
where
    M: Machine,
    C: CallManager<Machine = KytheraMachine<M>>,
{
    type Machine = C::Machine;

    fn new(
        machine: Self::Machine,
        engine: Engine,
        gas_limit: u64,
        origin: ActorID,
        origin_address: Address,
        receiver: Option<ActorID>,
        receiver_address: Address,
        nonce: u64,
        gas_premium: TokenAmount,
    ) -> Self {
        Self(C::new(
            machine,
            engine,
            gas_limit,
            origin,
            origin_address,
            receiver,
            receiver_address,
            nonce,
            gas_premium,
        ))
    }

    fn send<K: Kernel<CallManager = Self>>(
        &mut self,
        from: ActorID,
        to: Address,
        method: MethodNum,
        params: Option<Block>,
        value: &TokenAmount,
        gas_limit: Option<Gas>,
        read_only: bool,
    ) -> fvm::kernel::Result<InvocationResult> {
        // If cheatcode actor then we proceed as usual
        if to == Address::new_id(98) {
            self.handle_cheatcode(method, params.clone())?;

            self.0
                .send::<KytheraKernel<K>>(from, to, method, params, value, gas_limit, read_only)
        }
        // If any other actor, check if override caller
        else {
            let caller = self.machine().override_context().caller.unwrap_or(from);
            self.machine_mut().override_context.caller = None;
            self.0
                .send::<KytheraKernel<K>>(caller, to, method, params, value, gas_limit, read_only)
        }
    }

    fn with_transaction(
        &mut self,
        f: impl FnOnce(&mut Self) -> fvm::kernel::Result<InvocationResult>,
    ) -> fvm::kernel::Result<InvocationResult> {
        // This transmute is _safe_ because this type is "repr transparent".
        let inner_ptr = &mut self.0 as *mut C;
        self.0.with_transaction(|inner: &mut C| unsafe {
            // Make sure that we've got the right pointer. Otherwise, this cast definitely isn't
            // safe.
            assert_eq!(inner_ptr, inner as *mut C);

            // Ok, we got the pointer we expected, casting back to the interceptor is safe.
            f(&mut *(inner as *mut C as *mut Self))
        })
    }

    fn finish(self) -> (fvm::kernel::Result<FinishRet>, Self::Machine) {
        self.0.finish()
    }

    fn machine(&self) -> &Self::Machine {
        self.0.machine()
    }

    fn machine_mut(&mut self) -> &mut Self::Machine {
        self.0.machine_mut()
    }

    fn engine(&self) -> &Engine {
        self.0.engine()
    }

    fn gas_tracker(&self) -> &GasTracker {
        self.0.gas_tracker()
    }

    fn gas_premium(&self) -> &TokenAmount {
        self.0.gas_premium()
    }

    fn origin(&self) -> ActorID {
        self.0.origin()
    }

    fn next_actor_address(&self) -> Address {
        self.0.next_actor_address()
    }

    fn create_actor(
        &mut self,
        code_id: Cid,
        actor_id: ActorID,
        delegated_address: Option<Address>,
    ) -> fvm::kernel::Result<()> {
        self.0.create_actor(code_id, actor_id, delegated_address)
    }

    fn resolve_address(&self, address: &Address) -> fvm::kernel::Result<Option<ActorID>> {
        self.0.resolve_address(address)
    }

    fn set_actor(&mut self, id: ActorID, state: ActorState) -> fvm::kernel::Result<()> {
        self.0.set_actor(id, state)
    }

    fn get_actor(&self, id: ActorID) -> fvm::kernel::Result<Option<ActorState>> {
        self.0.get_actor(id)
    }

    fn delete_actor(&mut self, id: ActorID) -> fvm::kernel::Result<()> {
        self.0.delete_actor(id)
    }

    fn transfer(
        &mut self,
        from: ActorID,
        to: ActorID,
        value: &TokenAmount,
    ) -> fvm::kernel::Result<()> {
        self.0.transfer(from, to, value)
    }

    fn nonce(&self) -> u64 {
        self.0.nonce()
    }

    fn invocation_count(&self) -> u64 {
        self.0.invocation_count()
    }

    fn limiter_mut(&mut self) -> &mut <Self::Machine as Machine>::Limiter {
        self.0.limiter_mut()
    }

    fn append_event(&mut self, evt: StampedEvent) {
        self.0.append_event(evt)
    }
}

## Cheatcodes Test Actor

This is the actor embedded in our `kythera-fvm` to expose the cheatcodes interface.

### Cheatcodes

The following cheatcodes are exposed through the actor:
- `Epoch`: Set the `NetworkContext::epoch`
- `Warp`: Set the `NetworkContext::timestamp`
- `Fee`: Set the `NetworkContext::fee`
- `ChaindId`: Set the `NetworkContext::chain_id`
- `Prank`: Sets the **next implicit message**'s `MessageContext::caller` to be the input address
- `Trick`: Sets the **next implicit message and its sub-implicit messages**' `MessageContext::origin` to be the input address

More information available on the [Cheatcodes reference](https://polyphene.github.io/kythera/docs/reference/cheatcodes/) documentation.

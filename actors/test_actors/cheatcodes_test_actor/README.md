## Cheatcodes Test Actor

This is an actor to test Kythera cheatcodes implementation.

### Cheatcodes

The following cheatcodes are tested through the actor:
- `Epoch`: Set the `NetworkContext::epoch`
- `Warp`: Set the `NetworkContext::timestamp`
- `Fee`: Set the `NetworkContext::fee`
- `ChaindId`: Set the `NetworkContext::chain_id`
- `Prank`: Sets the **next call**'s `NetworkContext::caller` to be the input address
- `Trick`: Sets the **next call**'s `NetworkContext::origin` to be the input address
- `Log`: Logs a message from the actor on Stdout

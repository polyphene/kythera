## Basic Target Actor

This is a basic actor that serves as a target actor in some of our tests for Kythera. It is the target actor used against
our `cheatcodes_test_actor` for example. Its entrypoints are:
- `Constructor`: Method that should be called at deployment time. Currently, it initializes the state of this target actor.
- `HelloWorld`: Method that returns the current value stored in the actor's state.
- `Caller`: Method that returns the value of the `MessageContext.caller`.
- `Origin`: Method that returns the value of the `MessageContext.origin`.

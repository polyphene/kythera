## Basic Target Actor

This is a basic actor that serves as a target actor in some of our tests for Kythera. It is the target actor used against
our `cheatcodes_test_actor` for example. Its entrypoints are:
- `Caller`: Method that returns the value of the `MessageContext.caller`
- `Origin`: Method that returns the value of the `MessageContext.origin`

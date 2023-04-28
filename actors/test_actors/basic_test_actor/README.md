## Basic Test Actor

This is a basic actor to test our Kythera testing framework. It's entrypoints are:
- `Constructor`: Method that should be called at deployment time of the test actor. It initializes the state of the actor.
- `Setup`: Method that should be just before a call to a test method. Currently, it updates the state previously set by the 
`Constructor`.
- `TestConstructorSetup`: Method that we use in our tests to check that `Constructor` properly initializes the state and 
that `Setup` updates it. This is the same thing as checking that they are called by Kythera.
- `TestMethodParameter`: Method that checks the parameters passed to our test methods. Currently, it checks that we are 
properly passing the target actor ID.
- `TestFailed`: `Test*` method that fails for testing purposes.
- `TestFailFailed`: `TestFail*` method that fails for testing purposes.
- `TestFailSuccess`: Successful `TestFail*` method for testing purposes. 
# `kythera-fvm`

`kythera-fvm` is the core implementation for the Kythera FVM.

The implementation largely extends the [`ref-fvm`](https://github.com/filecoin-project/ref-fvm) implementation. It
has only been updated to allow for Cheatcodes implementation.

## Cheatcodes

Cheatcodes are implemented by catching messages sent to a specific actor ID (`98`) and manipulating the machine when it happens.

For more information about Cheatcodes please refer to [our documentation](https://polyphene.github.io/kythera/docs/overview/tests/cheatcodes/). 
# `kythera-actors`

`kythera-actors` is a crate that contains all necessary actors for Kythera to run and be tested. 

> 🗒️ **Note**
>
> Not directly because of the project structure but because of Rust itself, `build.rs` will only run if a source file changes.

## Actors

The only actor that is currently used for utilities by Kythera is the [Cheatcodes actor](https://polyphene.github.io/kythera/docs/reference/cheatcodes/).
This actor **will always be deployed at ID `98`**

## Test Actors

Those actors will only be built and made available if `kythera-actors` is used along its `testing` feature. This should only
happen when contributing to Kythera development.

## Acknowledgements

The code to build and use Wasm bytecodes is heavily inspired and copied from the implementation [over the `ref-fvm`](https://github.com/filecoin-project/ref-fvm/tree/37643fc02f0342256afecff5158c43693b5ee4f0/testing/test_actors)
done by @fridrik01.


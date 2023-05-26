# Kythera

[![Github Actions][gha-badge]][gha-url]
[![MIT licensed][mit-badge]][mit-url]
[![APACHE V2 licensed][apache-badge]][apache-url]

[gha-badge]: https://img.shields.io/github/actions/workflow/status/polyphene/kythera/ci.yml
[gha-url]: https://github.com/polyphene/kythera/actions
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: /LICENSE-MIT.txt
[apache-badge]: https://img.shields.io/badge/license-APACHE_V2-blue.svg
[apache-url]: /LICENSE-APACHE.txt

**Kythera is a Toolset for Filecoin Virtual Machine Native Actor development, testing and deployment.**

## Installation 

See the [installation guide](https://polyphene.github.io/kythera/docs/getting-started/installation/) in the documentation.

## Getting started

We recommend starting from [our documentation](https://polyphene.github.io/kythera/docs/getting-started/introduction/) to make you first steps with Kythera. 

## Repository structure

- `actors`
  - This crates is a utility to help us generate Wasm bytecode at build time so that we can leverage them both in our implementation
  and our tests.
  - `actors`
    - Contains actors implementing utilities over the Kythera FVM. Currently, the only actor available in this directory
    is the [Cheatcodes actor](https://polyphene.github.io/kythera/docs/reference/cheatcodes/).
  - `test_actors`
    - Test actors that are useful to test the implementation of Kythera. These actors are only useful while contributing to
    Kythera.
- `cli`
  - Crate implementing the logic for the `kythera` binary.
- `common`
  - This crate was created to make FVM utilities available to any project. For example, ABI serializing and deserializing,
    method name hash...
- `fvm`
  - Implementation of the Kythera FVM. Mostly extends implementation from [`ref-fvm`](https://github.com/filecoin-project/ref-fvm)
  with some custom logic to allow Cheatcodes implementation.
- `lib`
  - Contains the core logic for the testing framework. While it is mostly meant to be leveraged by `kythera- cli`, we hope 
  that having the core testing logic available as a library could prove useful to other projects.

## Contribute

Contributions are welcome! If you'd like to contribute to Kythera, please follow these steps:

1. Fork the repository on GitHub.
2. Create a new branch with a descriptive name.
3. Make your desired changes.
4. Commit your changes and push the branch to your forked repository.
5. Open a pull request on the main repository, describing the changes you made.
6. THANKS!

Please ensure your contributions adhere to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

⚠️ **Warning**

When dealing with updates over the `cheatcodes-actor` crate, it is expected that the contributor also generates the new
artifacts to be embedded within the `kythera-actors` crate. To do so, simply run:
```shell
$ make generate-artifacts
```

## License

This project is licensed under a dual [MIT](LICENSE-MIT.txt) and [APACHE V2](LICENSE-APACHE.txt) licensing model.
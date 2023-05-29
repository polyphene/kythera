---
title: "Gas Snapshots"
description: ""
lead: ""
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
menu:
  docs:
    parent: "gas"
weight: 232
toc: true
---

Kythera can generate gas snapshots for all your test functions. This can be useful to get a general feel for how much gas 
your contract will consume, or to compare gas usage before and after various optimizations.

To generate the gas snapshot, run `kythera snapshot <Path to artifacts>`.

This will generate a file called `.gas-snapshot` by default with all your tests and their respective gas usage.
```shell
$ kythera snapshot ./artifacts
$ cat .gas-snapshot

name,cost,passed
Basic.wasm::TestConstructorSetup,1519299,true
Basic.wasm::TestMethodParameter,2707176,true
Basic.wasm::TestFailed,1306487,false
Basic.wasm::TestFailFailed,1229328,false
Basic.wasm::TestFailSuccess,1306503,true
```

The format of the file is a CSV file containing three columns:
- `name`: identifier of the method for the given CSV line, in the format `<Actor name>.wasm::<Method name>`.
- `cost`: gas cost to run the test method.
- `passed`: boolean representing if the test was successful or not.

## Comparing gas usage

If you would like to compare the current snapshot file with your latest changes, you can use the `--diff` or `--check` options.

`--diff` will compare against the snapshot and display changes from the snapshot.

It can also optionally take a file name (`--diff <FILE_NAME>`), with the default being `.gas-snapshot`.

For example:
```shell
$ kythera snapshot ./artifacts --diff

Basic.t.wasm: testing 5 tests

Generating gas snapshot
Basic.wasm::TestConstructorSetup: gas used is 0% more
Basic.wasm::TestMethodParameter: gas used is the same: 2707176
Basic.wasm::TestFailed: gas used is 6% less
Basic.wasm::TestFailFailed: gas used is the same: 1229328
Basic.wasm::TestFailSuccess: gas used is the same: 1306503
Total gas dif: -77111
```

`--check` will execute the exact same logic but will exit with an exit code 1 if a difference is found.
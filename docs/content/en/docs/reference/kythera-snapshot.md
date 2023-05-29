---
title: "kythera snapshot"
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
menu:
    docs:
        parent: "reference"
weight: 320
toc: true
---

## NAME

`kythera-snapshot` - Run the Kythera snapshot command.

## DESCRIPTION

This Rust command represents the CLI arguments for the Kythera snapshot command.

## USAGE

```bash
kythera snapshot [OPTIONS] <Path to artifacts>
```

## OPTIONS

`--snap <FILE>`

Output file for the gas snapshot. (Default: .gas-snapshot)

`--diff <FILE>`

Output a diff against a pre-existing snapshot. By default, the comparison is done with `.gas-snapshot`.

`--check <FILE>`

Compare against a pre-existing snapshot, exiting with code 1 if they do not match. Outputs a diff if the snapshots do 
not match. By default, the comparison is done with `.gas-snapshot`.

## EXAMPLE

1. Generate a gas snapshot:
```shell
kythera snapshot path/to/artifacts
```
2. Generate a gas snapshot and output a diff against a pre-existing snapshot:
```shell
kythera snapshot --diff existing-snapshot.path path/to/artifacts
```
3. Compare against a pre-existing snapshot and output a diff if they don't match:
```shell
kythera snapshot --check existing-snapshot.path path/to/artifacts
```

---
title: "kythera test"
description: ""
lead: ""
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
menu:
  docs:
    parent: "reference"
weight: 310
toc: true
---

## NAME

`kythera-test-command` - Run the artifact's tests.

## DESCRIPTION

This Rust command represents the CLI arguments for the Kythera test command.

## USAGE

```bash
kythera test [OPTIONS] <Path to artifacts>
```

## OPTIONS

`--verbosity, -v`

Set the verbosity level of the traces. Increase the verbosity by passing multiple times (e.g., -v, -vv, -vvv).

Verbosity levels:
- 2: Print execution traces for failing tests.
- 3: Print execution traces for all tests.

`--gas-report`

Print gas reports.

## EXAMPLE

1. Run the tests:
```shell
kythera test path/to/artifacts
```
2. Run the test and print all traces with gas reports:
```shell
kythera test -vvv --gas-report path/to/artifacts
```
---
title: "Gas Reports"
description: ""
lead: ""
date: 2020-10-06T08:48:57+00:00
lastmod: 2020-10-06T08:48:57+00:00
draft: false
images: []
menu:
  docs:
    parent: "overview"
weight: 231
toc: true
---

Kythera can produce gas reports for your contracts. To show gas reports just pass the `--gas-report` argument while testing
your actor.

Example output:
```shell
╭─────────────────────┬───────────┬───────────┬───────────┬───────────┬─────────╮
│ Basic.wasm contract ┆           ┆           ┆           ┆           ┆         │
╞═════════════════════╪═══════════╪═══════════╪═══════════╪═══════════╪═════════╡
│ Function Name       ┆ min       ┆ max       ┆ avg       ┆ median    ┆ # calls │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ Constructor         ┆ 2385055   ┆ 2385055   ┆ 2385055   ┆ 2385055   ┆ 1       │
├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌╌╌┼╌╌╌╌╌╌╌╌╌┤
│ HelloWorld          ┆ 665472800 ┆ 665472800 ┆ 665472800 ┆ 665472800 ┆ 1       │
╰─────────────────────┴───────────┴───────────┴───────────┴───────────┴─────────╯
```
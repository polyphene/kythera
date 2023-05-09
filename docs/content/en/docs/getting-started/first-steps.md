---
title: "First Steps with Kythera"
date: 2023-04-03T11:01:56+02:00
lastmod: 2023-04-03T11:01:56+02:00
draft: false
images: []
menu:
    docs:
        parent: "getting-started"
weight: 130
toc: true
---

This section provide a quick example of how to use the `kythera` command line tool. We demonstrate how to start up a Rust-based
native actors project, compile and test it.

To start the development clone our dedicated Rust template:
```shell
git clone 
```
## Tmp

The `tmp` command gathers a suite of temporary sub-commands that should be cleaned and removed
before the next release:
- `print-config`: gathers context information from optional configuration file and prints it.
  > Usage:
  > ```shell
  > kythera tmp print-config
  > ```
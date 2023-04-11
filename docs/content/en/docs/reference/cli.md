---
title: "Command-line interface"
date: 2023-04-03T11:01:56+02:00
lastmod: 2023-04-03T11:01:56+02:00
draft: false
images: []
menu:
    docs:
        parent: "reference"
weight: 200
toc: true
---

## Tmp

The `tmp` command gathers a suite of temporary sub-commands that should be cleaned and removed
before the next release:
- `print-config`: gathers context information from optional configuration file and prints it.
  > Usage:
  > ```shell
  > kythera tmp print-config
  > ```
---
title: "Installation"
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
menu:
    docs:
        parent: "getting-started"
weight: 120
toc: true
---

## Building from source

You can build Kythera binary directly from our main repository through `cargo`:
```shell
$ cargo install --git https://github.com/polyphene/kythera --force kythera-cli
```

## Local project

You can build Kythera binary through a local clone of the project.

First, clone the repository:
```shell
$ git clone https://github.com/polyphene/kythera.git
```

Then, run:
```shell
$ cargo build --bin kythera
```

The Kythera binary should now be accessible at the root of the project.
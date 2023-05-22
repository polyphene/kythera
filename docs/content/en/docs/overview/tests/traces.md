---
title: "Understanding traces"
description: ""
lead: ""
date: 2020-10-06T08:48:57+00:00
lastmod: 2020-10-06T08:48:57+00:00
draft: false
images: []
menu:
  docs:
    parent: "test"
weight: 223
toc: true
---

Kythera can produce traces either for failing test (`-vv`) or all tests (`-vvv`).

Traces follow the same general format:
```shell
├─ [<Gas Charge>] OnChainMessage
│   └─ ← <Charging message value>
├─ [<Call>] from 102 to f0104 method: <Method number callled>
├─ [<Gas Charge>] OnMethodInvocation
│   └─ ← 75000.000
├─ [<Gas Charge>] <Gas charge reason>
│   └─ ← <Gas charge value>
...
└─ ← <Execution exit code>
```

Each call can have many more subcalls, each denoting a new message sent to another actor and a returned value.
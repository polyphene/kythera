---
title: "Cheatcodes"
date: 2023-05-22T10:00:00+00:00
lastmod: 2023-05-22T10:00:00+00:00
draft: false
images: []
menu:
    docs:
        parent: "reference"
weight: 330
toc: true
---

## Actor ID

Kythera will always deploy the cheatcode actor at the ID `98`

## List

The following cheatcodes are exposed through the actor:

| Name       | Arguments  | Description                                                                                                    |
|------------|------------|----------------------------------------------------------------------------------------------------------------|
| `Epoch`    | i64        | Set the `NetworkContext::epoch`                                                                                |
| `Warp`     | u64        | Set the `NetworkContext::timestamp`                                                                            |
| `Fee`      | (u64, u64) | Set the `NetworkContext::fee`                                                                                  |
| `ChaindId` | u64        | Set the `NetworkContext::chain_id`                                                                             |
| `Prank`    | Address    | Sets the next implicit message's `MessageContext::caller` to be the input address                              |
| `Trick`    | Address    | Sets the next implicit message and its sub-implicit messages' `MessageContext::origin` to be the input address |
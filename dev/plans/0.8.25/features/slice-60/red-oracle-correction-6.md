# Slice 60 RED-oracle correction 6 — owned rendezvous migration

Independent implementation audit found that the legacy graph-expansion pin hooks
were process-global. That mechanism could not prove request identity: an ordinary
request could consume an armed hook intended for another request. The FIX-2
isolation oracle and the three existing pin-race fixtures therefore migrate to
the owned `GraphExpandRendezvousForTest` API.

The migrated isolation test first leaves a `before_pin` rendezvous unattached,
proves an ordinary request cannot enter it, then attaches a clone only to the
intended request and proves entry, release, and completion. The three legacy
fixtures preserve their frozen/pre-pin and current/post-pin mutation and race
assertions; only their rendezvous plumbing changes from global callbacks to the
request-scoped handle.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs` | `b29aa071102fbcc80164599c34c0a286848e4a013a4bb0bdb1bc4b30587bdf48` | `3ad9ae5ee64043dd4a1aeb6d0f33bd96279e1d239ada590885cdcf0c8d99f8fe` |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix2_global_leak.rs` | `9c16656faecab4d6ffa6ec8120aeb0779854449aa200832efa0b360a0f0f9c6d` | `d6d406b694b97a8b54ea9952fd0ddb46b2a286f28273557277bbd40304c10620` |

This correction changes oracle plumbing only. It does not change graph-expansion
intent, production behavior, fixture inputs, or the original mutation/race
assertions.

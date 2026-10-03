---
title: FathomDB 0.8.27 Slice 100 - native inventory reconciliation
status: IN_PROGRESS
target_release: 0.8.27
---

# Native inventory reconciliation

The source-derived [entry](entry-items.json) and [final](final-items.json)
ledgers parse every Rust item in the PyO3 crate and include item kind,
attributes, signature, source owner and line. The entry has 389 items in the
single root; the final candidate has 476 across 13 source files. Comparing
function, method, carrier, constant, static, type and macro identities as a
multiset finds **zero removed items**. The added items are the bounded logger
adapter and its tests: 33 source identities, including the TLS macro. New
module/import/implementation declarations account for the other structural
growth. The one `_fathomdb` initializer and one `PyEngine` methods block
remain in `lib.rs` and `engine.rs`, respectively. Explicit root registrations
remain visible to the established surface comparator.

The installed [entry](entry-runtime.json) and [final](final-runtime.json)
ledgers record all 118 native names, package exports, class module identities,
method signatures and Python version from fresh isolated installations.
Normalized comparison finds no added or removed native name, package export,
class module identity or signature other than
`Engine.attach_logging_subscriber(self, /, logger, heartbeat_interval_ms=None)`
becoming `Engine.attach_logging_subscriber(self, /, logger)`. That delta is the
reviewed correction in the proposed successor ADR. The final runtime ledger
will be rebound to the clean committed candidate wheel at closeout.

The Slice 50 hook inventory, Slice 70 embedding documentation guard and Windows
WAL attribution guard were retargeted to their actual source owners. Their
mutation checks proved the changed source paths remain load-bearing. The
Windows guard passed 340 checks, including mutations, after its final
`test_support.rs` correction. The immutable Slice 30 baseline was not edited.

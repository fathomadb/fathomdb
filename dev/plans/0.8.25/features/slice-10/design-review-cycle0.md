---
title: 0.8.25 Slice 10 independent design review — cycle 0
status: FAIL_RESOLVED_BY_FIX_1
reviewed_design_version: 2
---

# Slice 10 independent design review — cycle 0

The read-only reviewer evaluated the Slice 10 plan/design against R25/AC25-10,
architecture v2, EARP and experiment persistence code, the immutable GLOBAL-01
receipts, and current `Engine.search` paths. The verdict was **FAIL**.

## Findings and FIX-1 disposition

| Severity | Finding | FIX-1 disposition |
| --- | --- | --- |
| P1 | The sidecar lacked exact path, identity, atomicity, collision, indexing, and blocked-state rules. | Version 3 defines one run-local sidecar, content-bound identity, atomic byte-idempotent writes, conflict rejection, and a non-satisfying blocked artifact. |
| P1 | Components, metrics, and claims were insufficiently typed and metric paths were not source-bound. | Version 3 closes component kinds, binds JSON Pointers to hashed artifacts, exhaustively classifies metric leaves, derives a layer lattice, and derives comparison sets. |
| P1 | Nothing enforced R25-10 for future experiment index rows. | Version 3 freezes the historical index prefix and adds a mandatory validator requiring sidecars for all post-cutover rows without changing the existing index schema. |
| P1 | The initial GLOBAL-01 negative execution claim exceeded its immutable dirty-code receipt. | Version 3 uses `unknown_historical`; only the clean held-out run receives exact positive call witnesses. |
| P2 | The native fixture was not reproducibly pinned to the standard database setup contract. | Version 3 pins literal inputs/config, `prepare_test_database`, CPU/no-model behavior, one instrumented call, outputs, and typed failures. |
| P2 | Verification commands and output paths were not exact. | Version 3 names the module, config, test, sidecar, blocked artifact, status, and exact local/full commands. |
| P2 | Historical migration inclusion/exclusion was ambiguous. | Version 3 names the two complete decision runs and requires a closed ID-by-ID exclusion inventory. |

This is review FIX-1. No implementation begins until an independent reviewer
finds version 3 READY with no unresolved P1/P2.

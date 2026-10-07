# Slice 135 installed Python capability exercise — 2026-10-07 UTC

**Status:** exact-wheel Phase 1 functional exercise, with one confirmed
boundary contract failure. The wheel was built from candidate source
`87d8659f18260a19b9f2932f412a42f50f71a692`; its SHA-256 is
`83b6c184fff27696dce329d5c2ae40e89290c86c3687e762488366c8a22ad34a`.
The wheel filename still carries the workspace's `0.8.26` package metadata;
the [manifest](manifest.json), source trees and native-module hash identify
the actual candidate bytes. The wheel was installed without editable source
imports and checked byte for byte against the installed package.

| Governed operations | Result |
| --- | ---: |
| Supported in the canonical map | 44 |
| Executed with selected real-database assertions | 39 |
| Failed a contract assertion | 1 |
| Explained execution gap | 1 |
| Unavailable in this artifact/environment | 3 |

The [final raw result](final.json) records every operation's positive,
refusal and reopened-state route. Forty-six of 47 selected cases passed; one
shared refusal case had a single isolated field failure. A malformed
`ReadContextV1(schema_version=2)` raised `FrozenReadError` with the expected
message, but both `.reason` and `.field_path` were `None`, contrary to the
documented typed-error contract. The exact [minimal reproduction](frozen-error-repro.json)
and failing assertion remain in the receipt. A valid absence/refusal for
`read_dependency_closure` ran, but no stored committed closure was observed;
that operation is a gap. Real extraction and consolidation providers and a
standalone cross-encoder reranker were unavailable under this wheel's build
features and local qualifications.

The exercise reused inspected existing real-database assertions, and added
separate SQL row, erasure, embedder identity and deterministic-vector checks
after reopen. The [independent audit](independent-audit.json) recomputed the
44-row accounting, checked exact fixture bodies and wheel/native bytes, and
rejected two altered receipts. Earlier [attempt 1](attempt1.json) and
[attempt 2](attempt2.json) are retained as harness qualification failures;
they do not count as product outcomes. [SHA256SUMS](SHA256SUMS) binds the raw
receipt and built wheel.

This is selected functional coverage, not full contract or source-branch
coverage, a paired 0.8.26 timing result, or a Phase 1 checkpoint. The
checkout lacked its own `.venv`, so the full repository gate stopped at
preflight. Seven focused harness tests, Ruff and the receipt audit passed.

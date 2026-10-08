---
title: Slice 135 persistent provider failure and explicit recovery result
status: FOCUSED_EXACT_SOURCE_DIAGNOSTIC
target_release: 0.8.27
---

# Persistent provider failure and explicit recovery — 2026-10-08

The [manifest](manifest.json) binds five fresh-process real-database runs to
committed source `6d7521cc7d606d5ebcebf25d8054178ad710dd3c`, the
feature-gated [test](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_persistent_provider.rs),
`Cargo.toml`, `Cargo.lock`, debug binary SHA-256
`0957d1081a32550e61b0f2f681498d7cdd23ca85095383c89ef1aad9d6f165b1`,
host, kernel, command and raw-file hashes. The test adds no production-source
change relative to the E01–E12 product snapshot. Focused Rust formatting,
Clippy and test execution passed. The full workspace gate remains open.

Each run made four failed embedding calls for the document under one
persistent provider fault. The canonical write remained readable, projection
status became `Failed`, exactly one durable failure audit row existed, and no
vector was reported. After close and reopen with a healthy provider, the
same canonical row and failed terminal state remained; the terminal document
was not silently re-embedded. An explicit `rebuild_projections()` then
produced an `UpToDate` vector that survived a further reopen.

The [independent audit](audit.py) recomputed the five [raw logs](measured-01/stderr.log)
and opened each retained SQLite database. Its [summary](audit.json) accepted
all five runs: exactly one canonical row, one `up_to_date` terminal, one vector
row and the historical `EmbedderError` failure audit were present, with
`PRAGMA integrity_check=ok` and no foreign-key errors. GNU Time recorded zero
child swaps and major faults; peak RSS was 23,712–25,428 KiB. These are
fault-probe resources, not latency comparisons. Two
[negative controls](negative-controls.json) recomputed manifest hashes after
altering a false-ready log event or persisted terminal state; both were
rejected by the semantic/state audit.

The [invalid-attempt register](invalid-attempts.json) distinguishes a missing
operator feature, an overbroad provider-call assertion, a pre-commit pilot
binary and a source-SHA preflight typo from the five accepted runs. The pilot
files remain local as `run-01/`. The retained raw databases and logs are
local pending the end-of-phase archive decision; the small manifest and
summaries are branch evidence. Manifest SHA-256 is
`df20b5b94f4de3998297deb4deea9b7009325a6be64c19ae51647ed2149c2dcc`;
audit SHA-256 is
`106a9bef963559be44186b8d6c18cce9de1f5de986c090332bb12edfb65b16db`.

This closes the bounded persistent-provider projection-failure and explicit
recovery case at the Rust engine boundary. It does not qualify installed
binding panic containment, provider failure in every queue position, or an
in-commit process kill. The broader robustness matrix remains open.

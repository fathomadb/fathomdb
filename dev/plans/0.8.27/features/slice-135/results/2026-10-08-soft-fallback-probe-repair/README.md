---
title: Slice 135 vector soft-fallback probe error repair
status: CONFIRMED_DEFECT_REPAIRED_FOCUSED_GATE
target_release: 0.8.27
---

# Vector soft-fallback probe must propagate SQL errors — 2026-10-08

The [real-database regression](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_soft_fallback_probe_error.rs)
opens a current-schema database with a working eight-dimensional embedder,
writes a text-matchable node but configures no vector kind, and checks the
control search. It then drops `_fathomdb_vector_kinds` through an independent
SQLite connection, verifies the table is absent, checks the embedder remains
usable, and searches again. A missing table in the vector fallback probe must
return `EngineError::Storage`, not a successful text result with no fallback
signal.

The staged test failed against source
`b483ce8295677df5284fafd562ab82e842f3fccb`, whose `search.rs` SHA-256
was `783ab4469ad449a4eb6c5abeabd4fe2dfee50d955c3729c5a8ef33d83fa33020`.
The [RED command](red-command.json) exited 101; [stderr](red.stderr) records
the successful search. The one-line repair changes `query_row(...).ok()` to
`query_row(...).optional()?`, preserving genuine no-row behavior while
propagating SQL errors. Repaired `search.rs` SHA-256 is
`088a559af5c8819eef646ba4d126d2fbf91601ec0bad18c4e1de7978e4b351da`.
The [GREEN command](green-command.json) passed the same staged test source,
SHA-256 `855501d15e1d9c3ae469261ea6507989daa6e219fff70182d69a4b9237548a90`.
The existing AC-031 soft-fallback case, rustfmt and focused engine Clippy
also passed.

The first fixture accidentally returned seven values for a declared
eight-dimensional embedder. Its search calls silently fell back to text
before reaching this probe; an attempted source change did not alter that
invalid observation. An explicit `embed_text` assertion exposed the dimension
error. The fixture was corrected and run against unchanged pre-fix source for
the reported RED result. No invalid attempt is counted as product evidence.
The final full gate and rebuilt installed-artifact comparisons remain due.

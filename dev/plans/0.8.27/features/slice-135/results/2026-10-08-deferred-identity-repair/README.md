---
title: Slice 135 deferred text-hit identity error repair
status: CONFIRMED_DEFECT_REPAIRED_FOCUSED_GATE
target_release: 0.8.27
---

# Deferred text-hit identity must propagate row errors — 2026-10-08

The [real-database regression](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_deferred_text_identity_error.rs)
writes a text-matchable node without a vector projection, then verifies a
default hybrid search returns the text hit with its source ID. After close,
an independent SQLite connection changes only the canonical row's `source_id`
to a blob and verifies the stored type. On reopen, the deferred identity
lookup must return `EngineError::Storage`; successful search with an invented
content ID and missing provenance is false data.

The first fixture attempt did not compile because it applied `SourceId::as_str`
to the hit's `Option<String>` field. That fixture error was corrected before
the product RED run. The staged test then failed on source
`690f249a09b500e30377dc01db41acf1d0e7803f`, whose `search.rs` SHA-256
was `eaa3ebb355b579b81a135ef37b8b605cbd1582595e0876db04e62c42ec4f475e`.
The [RED command](red-command.json) exited 101; [stderr](red.stderr) shows
`Ok(SearchResult { ... branch: Text, source_id: None ... })` instead of an
error. The repair treats only `QueryReturnedNoRows` as a missing identity;
other row errors propagate. Repaired `search.rs` SHA-256 is
`783ab4469ad449a4eb6c5abeabd4fe2dfee50d955c3729c5a8ef33d83fa33020`.
The [GREEN command](green-command.json) passed the same test source, SHA-256
`cd2250d0d9c9e513f4d734d538e43321ecbb931c74fe3024db0e38a57fa9e139`.

The adjacent rank-stream suite passed three cases with `test-hooks`; the
vector-hydration regression passed again, and focused engine Clippy passed.
An initial adjacent-test invocation omitted `test-hooks` and stopped before
tests ran; the corrected invocation is the reported result. A final full
workspace gate and rebuilt installed-artifact measurements remain due.

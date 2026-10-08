---
title: Slice 135 scoped search and binding error-boundary audit
status: INTERIM_STATIC_AUDIT
target_release: 0.8.27
---

# Scoped search and binding error-boundary audit — 2026-10-08

This audit reviews the current Slice 135 branch after the two explanation
repairs, through commit `486097ee5`. Its scope is the engine's `search.rs`,
`search_api.rs` and `fusion.rs` hot/boundary paths, and the Python and N-API
engine/search/FFI dispatchers. It is a targeted source audit, not a proof of
all exception paths. Focused
`cargo clippy -p fathomdb-engine --features test-hooks,default-embedder
--all-targets -- -D warnings` passed after each repair; the final full
repository gate and installed-binding panic probes remain due on the final
candidate.

| Finding | Classification and evidence |
| --- | --- |
| Edge explanation count suppressed prepare/query/row errors | Confirmed defect repaired with a real-database RED/GREEN probe; see [receipt](results/2026-10-08-edge-explanation-repair/README.md). |
| Importance/confidence map suppressed prepare/row errors | Confirmed defect repaired on current-only schema with a real-database importance RED/GREEN probe; see [receipt](results/2026-10-08-importance-lookup-repair/README.md). Edge confidence uses the same repaired fallible query shape but has no separate injected confidence receipt. |
| Modern node, edge and vector candidate row decoding | Earlier confirmed defects repaired test-first; see the [row-site audit](logic-row-error-audit-2026-10-07.md) and its linked receipts. |
| Two remaining `rows.flatten()` sites in legacy node-FTS fallback | Source-reachable only through pre-step-12/pre-step-8 schema branches, while public open admits current schema 34. They remain test-hook/historical-compatibility risks, not evidence of a supported current-runtime defect. A future change to admission requires re-audit. |
| Rank-stream `.ok()` | Deliberate optimization fallback to full stable sort. The focused rank target now passes three cases; a malformed row in the fallback propagates `Storage` rather than returning partial results. A statement-failure route is tested, but every possible SQL failure is not enumerated. |
| `filter.expect` in the edge explanation loop | Guarded by `filter.is_some_and` requiring nonempty attributes in the same branch. No `None` route reaches it. |
| Python `decimal_offset` `parsed.unwrap()` | Guarded by `parsed.is_none()` rejection immediately above. It is an avoidable panic spelling, but no unhandled `None` execution path was found. |
| Engine provider `resume_unwind` | Intentional Rust operation panic boundary for timely provider panics, documented in `dev/interfaces/rust.md` and `dev/design/errors.md`. Python `call_engine`/open and N-API async/sync dispatchers wrap engine calls with `catch_unwind`; standalone embedding helpers also have their own catch. Source review does not establish that every exported method uses the wrapper. Existing Python and TypeScript AC-067 tests cover representative panic and process-survival routes, but they have not been rerun against the final candidate. |
| Other `.ok()`/default conversions on hot search paths | `search.rs` uses `.ok()` for optional SQL soft-fallback detection and query-vector serialization; `search_api.rs` has a poisoned-lock fallback for a dense-disabled reason. These remain scoped leads for failure-specific review. A successful Clippy run does not prove the fallbacks truthful under every failure. |

The source scan used `rg -n` for `unwrap`, `expect`, `panic`, `resume_unwind`,
`catch_unwind`, `.ok()` and `unwrap_or_default` in the scoped files, then read
the surrounding control flow. The scan excludes generic `Option::flatten`
uses because they do not discard `rusqlite::Result` row errors. `semgrep`,
`cargo-deny`, Kani and Flamegraph are not installed on this host; a broad
pattern-pack result is therefore not claimed. The first-party compiler and
Clippy checks, targeted source review and defect-catching probes are the
current static and dynamic evidence. The final checkpoint must state which
binding panic tests actually ran on its exact artifact, and classify the
remaining `.ok()` leads or retain them as explicit risks.

---
title: 0.8.24 Slice 20 — implementation status
status: IMPLEMENTED-AWAITING-INDEPENDENT-REVIEW
target_release: 0.8.24
---

# Slice 20 — implementation status

## Outcome

The bounded Slice 20 implementation is complete on
`release/0.8.24-slice20` and awaits the parent-owned independent code review,
verification, and merge. Eligible direct-text FTS node collection now streams
the complete score group crossing the fixed 100-candidate boundary, restores
the existing stable order, and returns the same candidates as the full-sort
control.

The implementation also restores the accepted writer-only
`WAL + synchronous=NORMAL` durability invariant. Reader-pool/runtime settings,
cache, mmap, temp-store, schema, public SDKs, hybrid/vector behavior, TC-5, and
unrelated experiment infrastructure are unchanged.

## Review and TDD record

| Stage | Commit / result |
| --- | --- |
| Reviewed prep/design | `d62fd46e` — current-code map, local contracts, primary-source research, revised accepted design. |
| RED contract | `70e64e0a` — target exited 101; current code emitted neither streamed/ineligible route evidence nor a writer invariant witness. |
| RED fixture refinement | `9fbd38a6` — made the strict boundary genuinely score-unique; did not change the required route/equivalence assertion. |
| GREEN implementation | `6c4f3ace` — exact rank stream, fallback, property proof, writer invariant, and aligned design/AC text. |

The independent design reviewer returned NEEDS-REVISION before implementation.
The binding corrections are recorded in `design.md`: preserve current-main WAL
attribution, exclude TC-5/experiment infrastructure, restore only writer
NORMAL, pin per-query rank to `bm25()`, compile controls/witnesses only with
`test-hooks`, correct AC-076's mechanism description, and broaden correctness
coverage. All were applied.

## Final implementation allowlist

- `src/rust/crates/fathomdb-engine/src/lib.rs`
- `src/rust/crates/fathomdb-engine/Cargo.toml`
- `src/rust/crates/fathomdb-engine/tests/slice20_fts_rank_stream.rs`
- `src/rust/crates/fathomdb-engine/tests/perf_gates.rs`
- `dev/design/retrieval.md`
- `dev/acceptance.md`
- Slice 20 local records and maintained document indexes

The branch has no changes to `tc5_benchmark.rs`, `tc5_vector_stage.rs`, schema,
bindings, public interface documents, query compiler, workflow, release
publisher, benchmark runner, or registry state.

## Verification

| Check | Result |
| --- | --- |
| Slice 20 stream/control integration target | 2 passed |
| Engine unit/property tests under `test-hooks` | 29 passed, including generated rank-group equivalence |
| Filtered KNN | 6 passed |
| Legacy tokenizer migration/recall | 1 passed |
| Hybrid RRF/fallback | 10 passed |
| Search validity/fixed view | 8 passed |
| Direct-text prefix stability | 2 passed |
| Edge validity on search | 1 passed |
| Targeted engine clippy, all targets, `test-hooks`, warnings denied | PASS |
| Full workspace clippy, all targets, warnings denied | PASS |
| Full workspace check, all targets | PASS |
| Rust format, Markdown/plan/anchor lint, diff check | PASS |
| Normal `agent-test.sh` stage | INCOMPLETE: stopped after sandbox-only fixture failures and the classified release-state checker mismatch; focused engine tests above passed |

`agent-verify.sh` reached the repository lint phase and then stopped on an
unrelated pre-existing release-branch inconsistency: the checker
`scripts/check-public-doc-truth.py` is hard-coded to release state 0.8.21 while
the owner-approved Slice 7 maintained docs correctly state published 0.8.23.
Slice 20 did not widen its exact allowlist to alter that checker. The parent
classified it as pre-existing and outside Slice 20.

The separately requested normal `agent-test.sh` stage was started and stopped
with exit 130 after it continued into long-running tests beyond the parent's
request to return the completion record immediately. Before interruption it
also reported sandbox-environment failures unrelated to Slice 20: fixture HTTP
servers could not bind, temporary tag refs could not be locked in read-only Git
metadata, and npm could not write its cache. These failures affected publisher,
co-tagging, and embedder-drift harnesses; none implicated the changed engine
target. Parent-owned independent verification remains the completion gate.

## No-rerun and external-effects record

No SCALE-02 run, reduced-N timing check, confirming benchmark, hosted workflow,
push, registry query/mutation, runner action, environment approval, tag, or
publication occurred. Correctness tests consume only local synthetic SQLite
fixtures. The retained SCALE-02 receipt and owner `seq-267` selection remain the
sole performance decision basis.

## Handoff

The parent must independently review and verify commit `6c4f3ace` plus its RED
ancestors. If accepted, it may merge the branch into `release/0.8.24`, update
the master-plan Slice 20 row to complete, and remove this temporary worktree and
branch. Any schema, SDK, public-contract, or broader release change remains out
of scope and requires a separate owner decision.

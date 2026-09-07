---
title: 0.8.25 Slice 60 implementation TDD chronology
status: RED
baseline: 0782508c3a072a6bf9ab45cb1a15525f75c33697
design_version: 5
design_review: PASS_CYCLE_4
---

# Slice 60 implementation TDD chronology

## RED

The RED commit is the commit containing this record. It adds only a manually
authored cross-SDK fixture and dedicated test files; production code, public
interface documents, release state, packaging, workflows, and publication
state are unchanged. These test paths become immutable through GREEN:

- `dev/fixtures/slice60-graph-expand-conformance-v1.json`
- `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs`
- `src/rust/crates/fathomdb-engine/tests/slice60_wire.rs`
- `src/rust/crates/fathomdb/tests/slice60_governed_surface.rs`
- `src/python/tests/test_slice60_graph_expand.py`
- `src/ts/tests/slice60-graph-expand.test.ts`

The oracle covers the v5 contract's public construction and re-exports,
closed-request/open-response codecs, canonical integers and exact typed error
paths, explicit and query seeding, vector-arm refusal, all direction and kind
constraints, node-only temporal relaxation, shipped edge recency, bounded
traversal and exact W/W+1 behavior, deterministic origins and edge-only
permutations, explanation/correlation/degradation composition, transaction
rendezvous, endpoint-index plans, schema 33 without migration, legacy method
compatibility, and governed Python/TypeScript surfaces.

Baseline control before RED:

```text
$ cargo test -p fathomdb-engine --test slice20_graph_traversal --no-fail-fast
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Primary RED witness:

```text
$ cargo test -p fathomdb-engine --test slice60_graph_expand --no-run
error[E0432]: unresolved imports `fathomdb_engine::arm_graph_expand_after_pin_hook_for_test`, `fathomdb_engine::arm_graph_expand_before_pin_hook_for_test`, `fathomdb_engine::graph_expansion_degradation_codes_for_test`, `fathomdb_engine::GraphExpandRequestV1`, `fathomdb_engine::GraphExpansionDegradationCodeV1`, `fathomdb_engine::GraphExpansionErrorReasonV1`, `fathomdb_engine::GraphProjectionOriginV1`, `fathomdb_engine::GraphProjectionReadinessV1`, `fathomdb_engine::GraphReadContextV1`, `fathomdb_engine::GraphReadModeV1`, `fathomdb_engine::GraphSeedSourceV1`, `fathomdb_engine::GraphSeedV1`
error[E0599]: no method named `graph_expand` found for struct `fathomdb_engine::Engine` in the current scope
error: could not compile `fathomdb-engine` (test "slice60_graph_expand") due to 35 previous errors
```

Independent focused RED witnesses also fail on the intended absent surface:

- `cargo test -p fathomdb-engine --test slice60_wire --no-run` exits 101 on
  absent Slice 60 request/result codecs and types.
- `cargo test -p fathomdb --test slice60_governed_surface --no-run` exits 101
  on absent facade re-exports and `Engine::graph_expand`.
- `npx tsc --noEmit -p tsconfig.json` exits 2 on absent graph-expansion types,
  error, response validator, and `graph.expand`.

Python syntax, Ruff lint, and Ruff format pass. Source-tree pytest collection is
not a valid RED witness in this worktree: the pre-existing shared native module
is stale and fails earlier while importing the already-shipped
`ProjectionGenerationError`. Per the worktree safety rule it was not rebuilt or
repointed. GREEN and final verification must use the sanctioned build route and
a disposable source-independent artifact.

No test is ignored, skipped, xfailed, or generated from implementation output.
The next phase must not edit the immutable RED paths.

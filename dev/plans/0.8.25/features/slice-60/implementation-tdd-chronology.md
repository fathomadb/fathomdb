---
title: 0.8.25 Slice 60 implementation TDD chronology
status: GREEN
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

## Audited oracle corrections

Independent read-only audits found mechanics defects without changing the
READY v5 intent. Commit `b9c0f67b` added the return-only `fact` target filter to
the global-top-N case and made the empty degradation array mutation reachable.
Commit `5ee4dff4` corrected the Python edge helper to the shipped nested write
shape. Commit `543333e3` replaced two unit-returning `Option::map` expressions
with equivalent `if let` mutations and removed one unused `mut`. The rationale
and before/after hashes are recorded in `red-oracle-correction.md`,
`red-oracle-correction-2.md`, and `red-oracle-lint-correction.md`.

The re-frozen immutable hashes are:

- fixture: `ec43aec001ebfa2b6e3696559e31bfcb19b3c88f109e021ff3aa281a28563baf`
- Rust runtime: `b29aa071102fbcc80164599c34c0a286848e4a013a4bb0bdb1bc4b30587bdf48`
- Rust wire: `9f0626019fbdc5bc451d42cdcf97b98fc17218e2b49886f4bc4ce3ff0b60ee4f`
- Rust facade: `ea168299f7d198435669998300e5f5a838a2b7b71c549f6edd6f7fb435c08f3d`
- Python: `fd07ff1d4032df961fc99b7e8df9af9cb5644877045e8620f6d6f4b5bc1cb934`
- TypeScript: `205fe4a7c0138a10433b3933e31e75fd772d7afa54132cd4aafd72981950a960`

## GREEN

The engine implements canonical request/response codecs, typed errors,
current/frozen transaction semantics, explicit and FTS-only query seeds,
bounded deterministic traversal, exact exhaustive work accounting, compact
origins/explanation/degradation, and test-only cancellation-safe rendezvous.
The default facade, PyO3/Python, N-API/TypeScript, governed allowlist, and all
four interface documents carry the same contract.

Focused GREEN gates, in latency order:

```text
$ cargo test -p fathomdb-engine --test slice60_graph_expand -- --test-threads=1
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p fathomdb-engine --test slice60_wire -- --test-threads=1
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -p fathomdb --test slice60_governed_surface -- --test-threads=1
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

$ PYTHONPATH=src/python .venv/bin/python -m pytest \
    src/python/tests/test_slice60_graph_expand.py -q
21 passed

$ node --test dist/tests/slice60-graph-expand.test.js
pass 12; fail 0
```

The Python test used a temporary higher-priority extension symlink to the local
debug `fathomdb-py` build. The Node test used a local debug N-API module and
fresh TypeScript compilation. The symlink, `.node` file, and generated `dist`
tree were removed immediately after the consumer checks; none is a release
package or staged artifact.

Focused compatibility and static gates:

```text
$ cargo test -p fathomdb-engine --test slice20_graph_traversal \
    --test slice35_frozen_read --test slice35_graph_frontier_pretruncation \
    -- --test-threads=1
test result: 24 passed across three binaries; 0 failed

$ cargo clippy -p fathomdb-engine -p fathomdb -p fathomdb-py \
    -p fathomdb-napi --lib -- -D warnings
Finished `dev` profile; no diagnostics

$ .venv/bin/pyright src/python/fathomdb/{types,errors,engine,graph,__init__}.py
0 errors, 0 warnings, 0 informations

$ npm run typecheck
tsc --noEmit -p tsconfig.json

$ ./scripts/agent-lint.sh
PASS (no output)

$ ./scripts/agent-typecheck.sh
PASS (no output)
```

Disk space was 172 GiB free before GREEN and 167 GiB after the final incremental
debug native consumer rebuild. The disposable consumer copies were removed;
reusable Cargo intermediates remain for the independent verifier. No release
packaging, registry staging, tag, push, or publication operation was performed.

## FIX-1 RED

Implementation-review cycle 1 requires a separate RED commit. The dedicated
`slice60_fix1*` fixture and Rust/Python/TypeScript tests are additive and leave
the six original frozen RED artifacts byte-identical. They require raw
declaration-order request bytes, nested union unknown-field precedence, a
non-shipped owned bounded test rendezvous plus production-SQL EXPLAIN seam, and
binding-local rejection of invalid nested strings and eligibility members before
native transport. Its test-hook ownership contract also requires the dedicated
real-database 10,000/10,001 work/RSS, dependency-closure, erasure, and every
projection-state fixture seam before any GREEN implementation can claim those
observations.

Baseline controls at `4251d801f7dcd315da0b1bd1da943f7bf46ff9e9` remained
green: the original engine runtime oracle passed 18/18 and the original wire
oracle passed 7/7. The narrow Rust FIX-1 target compiles and fails on the
intended candidate defects: map-ordered raw request bytes, `GraphSeedInvalid`
instead of `unknown_field` at `/seed/a~1b~0c`, unconditional global hooks and
substitute EXPLAIN SQL, and absent owned real-database measurement seams. The
Python source-contract target fails because `graph.py` has no recursive
pre-transport validator. TypeScript typecheck passes; its local N-API RED
consumer requires the disposable native build route and is retained for the
next verification window.

## FIX-1 GREEN

The FIX-1 implementation serializes the declaration-ordered request and
result wire structs directly, closes the seed union before its discriminant,
confines graph rendezvous helpers to `test-hooks`, and shares the production
incident-edge SQL/bind builder with EXPLAIN. Python and TypeScript reject NULs,
lone surrogates, and malformed recursive eligibility members before calling a
native binding. `wire.md` now records schema 33's endpoint-index-only,
no-migration sentinel.

I60-01's malformed `serde_json::Value` byte oracle was corrected separately in
`92b1637c` and its Clippy-only hygiene follow-up `ba5450ac`; the re-frozen wire
oracle SHA-256 is
`378f9ecd651d965f661365708d9fdc66353888e9bf93f7a3583ae781cd01a3f9`.
The request fixture remains
`5f6d1ad3b0b5fadd509c51db848cb31f976faf17d3560f9abe86d86cb24f985a` and
the new declaration-order result fixture is
`d1d267548d5e3028327223f22dd28444072c9c611ff36275027a59083baccb28`.

Focused GREEN evidence:

```text
$ cargo test -p fathomdb-engine --features test-hooks \
    --test slice60_graph_expand --test slice60_wire --test slice60_fix1_wire \
    -- --test-threads=1
slice60_graph_expand: 18 passed; slice60_wire: 7 passed; slice60_fix1_wire: 4 passed

$ cargo test -p fathomdb --test slice60_governed_surface
2 passed

$ cargo test -p fathomdb-py
13 passed

$ cd src/ts && npm run --silent build:native:debug && npx tsc -p tsconfig.json \
    && node --test dist/tests/slice60-graph-expand.test.js \
       dist/tests/slice60-fix1-graph-expand.test.js
13 passed

$ ./scripts/agent-typecheck.sh
PASS

$ ./scripts/agent-lint.sh
PASS
```

The original runtime bound oracle includes W=10,000 success and W=10,001
typed rejection, plus exhaustive projection degradation composition. The
fresh local Python consumer route was not rerun: source-tree collection stops
at the pre-existing stale `_fathomdb.abi3.so` import mismatch for
`ProjectionGenerationError`. The FIX-1 source-contract test passes and the
PyO3 crate checks pass; no worktree Python extension was installed or replaced.

## FIX-2 RED

Implementation-review cycle 2 is held by dedicated executable RED artifacts:

- `dev/fixtures/slice60-fix2-unicode-v1.json`
- `src/rust/crates/fathomdb-engine/tests/slice60_fix2_wire.rs`
- `src/rust/crates/fathomdb-engine/tests/slice60_fix2_global_leak.rs`
- `src/rust/crates/fathomdb-engine/tests/slice60_fix2_hooks.rs`
- `src/rust/crates/fathomdb-engine/tests/slice60_fix2_runtime.rs`
- `src/python/tests/test_slice60_fix2_graph_expand.py`
- `src/ts/tests/slice60-fix2-graph-expand.test.ts`

The original runtime and wire controls remain green at the FIX-2 baseline
`31781ee1911d3bdb7016acbe24d58ed582d80606`:

```text
$ cargo test -p fathomdb-engine --features test-hooks \
    --test slice60_graph_expand --test slice60_wire -- --test-threads=1
slice60_graph_expand: 18 passed; slice60_wire: 7 passed
```

The narrow RED witnesses are executable rather than source-text checks. The
context fixture supplies an escaped unknown member with a missing or invalid
discriminant; the current Rust decoder reports `GraphContextInvalid` where the
contract requires `UnknownField` at `/context/a~1b~0c`. A real unrelated graph
request fires the current process-global before-pin hook (`left: 1`,
`right: 0`). The owned handle target cannot compile because
`GraphExpandRendezvousForTest` and
`Engine::graph_expand_with_rendezvous_for_test` do not exist.

The real SQLite high-bound fixture inserts a root, one target node, and one
incident edge per inspected row. Its 10,000-row case completes with
`work_units == 10000`; its 10,001-row case returns the exact typed
`graph_expansion_bound_exceeded` at `/maxWorkUnits`. In the same target, the
observable `both` EXPLAIN result is two separate endpoint plans while the
actual production OR statement reports `MULTI-INDEX OR`, and the current
measurement reports zero RSS delta instead of a bounded observed value.

The Python transport test executes the wrapper with isolated FFI stubs and
fails byte-for-byte at the UTF-8 `é` because `json.dumps` emits `\\u00e9`.
The TypeScript transport route executes against the same fixture and currently
passes; it retains the native-result capture so GREEN must preserve raw result
parity while fixing Python. No existing RED fixture or test changed.

## FIX-2 GREEN

FIX-2 replaces the process-global graph pin callbacks with the owned,
request-scoped `GraphExpandRendezvousForTest` handle behind `test-hooks`.
The handle uses bounded entry and release channels, idempotent release, and
Drop disarm; default builds expose neither the hook nor its storage path.
Production traversal and EXPLAIN share one SQL-and-bind descriptor, including
the single `both` OR statement. Rust and TypeScript close the context union
before validating its discriminant or reading payload, and Python now sends
wire JSON with `ensure_ascii=False`. The wire interface now correctly states
that Slice 60 adds no migration and reuses the endpoint indexes from steps 12
and 23; step 33 is Slice 45 page-index and visibility-state work.

Three independently authorized oracle corrections preserve intent while
repairing test mechanics: `ccdb3a15` removes an impossible stdlib JSON
assertion from the FIX-2 Python Unicode oracle, `1b7695bd` migrates the
original three pin-race fixtures plus FIX-2 isolation coverage from legacy
global callbacks to the owned rendezvous, and `243b29f7` replaces seven
runtime-equivalent dynamic-module assignments with `setattr` so Pyright can
type-check the frozen Python test doubles. Their correction records retain the
before-and-after hashes.

Focused final evidence:

```text
$ cargo test -p fathomdb-engine --features test-hooks \
    --test slice60_fix1_wire --test slice60_fix2_hooks \
    --test slice60_fix2_global_leak --test slice60_fix2_runtime \
    --test slice60_fix2_wire --test slice60_graph_expand --test slice60_wire \
    -- --test-threads=1
slice60_fix1_wire: 4 passed; slice60_fix2_hooks: 2 passed
slice60_fix2_global_leak: 1 passed; slice60_fix2_runtime: 3 passed
slice60_fix2_wire: 3 passed; slice60_graph_expand: 18 passed
slice60_wire: 7 passed

$ PYTHONPATH=src/python .venv/bin/python -m pytest \
    src/python/tests/test_slice60_graph_expand.py \
    src/python/tests/test_slice60_fix1_graph_expand.py \
    src/python/tests/test_slice60_fix2_graph_expand.py -q
23 passed

$ cd src/ts && npx tsc -p tsconfig.json && node --test \
    dist/tests/slice60-graph-expand.test.js \
    dist/tests/slice60-fix1-graph-expand.test.js \
    dist/tests/slice60-fix2-graph-expand.test.js
pass 3; fail 0

$ cargo test -p fathomdb --test slice60_governed_surface -- --test-threads=1
2 passed

$ cargo test -p fathomdb-py
13 passed

$ cargo check -p fathomdb-napi
PASS

$ cargo test -p fathomdb-engine --test slice20_graph_traversal \
    --test slice35_frozen_read --test slice35_graph_frontier_pretruncation \
    -- --test-threads=1
24 passed
```

`./scripts/agent-lint.sh` and `./scripts/agent-typecheck.sh` pass. The pinned
Pyright 1.1.410 check reports 0 errors after the independent mechanical oracle
correction; no broad diagnostic suppression was added.

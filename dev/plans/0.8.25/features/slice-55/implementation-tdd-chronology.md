---
title: 0.8.25 Slice 55 implementation TDD chronology
status: GREEN
---

# Slice 55 implementation TDD chronology

## Test-only RED witness

The first implementation increment added real disposable SQLite coverage for
the governed reciprocal trace, operator integrity, structural explanation,
facade separation, CLI separation, bounded/property seams, and versioned
fault/wire fixtures. No production, binding, interface, ADR, schema, or
governed-surface implementation was present.

RED commit:
`3adedd9e804ce3e26d4ad3c4433615aae5c3044c`.

Command (exit 101):

```text
cargo test -p fathomdb-engine --test slice55_dependency_trace
```

First intended diagnostic, verbatim:

```text
error[E0432]: unresolved imports `fathomdb_engine::DependencyTraceDirectionV1`, `fathomdb_engine::DependencyTraceErrorReasonV1`, `fathomdb_engine::DependencyTraceRequestV1`
 --> src/rust/crates/fathomdb-engine/tests/slice55_dependency_trace.rs:4:40
  |
4 |     ArtifactRevisionId, CanonicalHash, DependencyTraceDirectionV1, DependencyTraceErrorReasonV1,
  |                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `DependencyTraceErrorReasonV1` in the root
  |                                        |
  |                                        no `DependencyTraceDirectionV1` in the root
5 |     DependencyTraceRequestV1, Engine, EngineError, InitialState, PreparedWrite, ProvenancedNodeV1,
  |     ^^^^^^^^^^^^^^^^^^^^^^^^ no `DependencyTraceRequestV1` in the root
```

Command (exit 101):

```text
cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_data_plane_integrity
```

First intended diagnostic, verbatim:

```text
error[E0432]: unresolved imports `fathomdb_engine::DataPlaneIntegrityCheckV1`, `fathomdb_engine::DataPlaneIntegrityErrorReasonV1`, `fathomdb_engine::DataPlaneIntegrityRequestV1`
 --> src/rust/crates/fathomdb-engine/tests/slice55_data_plane_integrity.rs:4:5
  |
4 |     DataPlaneIntegrityCheckV1, DataPlaneIntegrityErrorReasonV1, DataPlaneIntegrityRequestV1,
  |     ^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `DataPlaneIntegrityRequestV1` in the root
  |     |                          |
  |     |                          no `DataPlaneIntegrityErrorReasonV1` in the root
  |     no `DataPlaneIntegrityCheckV1` in the root
```

Command (exit 101):

```text
cargo test -p fathomdb-engine --test slice55_explanation
```

First intended diagnostics, verbatim:

```text
error[E0609]: no field `correlation_id` on type `Explanation`
  --> src/rust/crates/fathomdb-engine/tests/slice55_explanation.rs:32:26
   |
32 |     assert!(!explanation.correlation_id.is_empty());
   |                          ^^^^^^^^^^^^^^ unknown field
   |
   = note: available fields are: `trace`, `per_hit`

error[E0609]: no field `structural` on type `&PerHitExplain`
  --> src/rust/crates/fathomdb-engine/tests/slice55_explanation.rs:34:54
   |
34 |     assert!(explanation.per_hit.iter().all(|hit| hit.structural.schema_version == 1));
   |                                                      ^^^^^^^^^^ unknown field
```

The failures are the intended absent Slice 55 public types, methods, error
variants, and additive explanation members. The existing APIs compiled far
enough to establish that the fixtures use the current public write,
dependency-registration, frozen-context, search, telemetry, facade, and CLI
contracts.

## Binding and codec test-only RED witness

The second test-only increment adds the plan-required canonical trace codec
property, Python old-construction and absent-native compatibility checks,
Python public trace/doctor-absence checks, TypeScript old-object-literal and
absent-native compatibility checks, and direct candidate-native structural
presence fixtures. It precedes every binding, wrapper, and codec production
edit.

Second RED commit:
`cadb9774298ab18f32d3cb917aa877a90d51824e`.

Python source-only command (exit 1):

```text
.venv/bin/pyright src/python/tests/test_slice55_wrapper_compat.py src/python/tests/test_slice55_trace_explanation.py
```

First intended diagnostics, verbatim:

```text
src/python/tests/test_slice55_trace_explanation.py:7:24 - error: "DependencyTraceRequestV1" is not a known attribute of module "fathomdb" (reportAttributeAccessIssue)
src/python/tests/test_slice55_trace_explanation.py:17:21 - error: "DependencyTraceError" is not a known attribute of module "fathomdb" (reportAttributeAccessIssue)
src/python/tests/test_slice55_wrapper_compat.py:30:24 - error: Cannot access attribute "correlation_id" for class "Explanation"
  Attribute "correlation_id" is unknown (reportAttributeAccessIssue)
```

The source-wrapper pytest route was also attempted but was not a valid RED
oracle: the shared worktree extension predates Slice 40 and fails collection
while importing `ProjectionGenerationError`. Per
`agent-worktree-stale-base-trap.md`, it was not rebuilt or repointed from this
worktree. Pyright supplies source-only RED evidence; installed behavior is
reserved for the exact-candidate disposable wheel route.

TypeScript command from `src/ts` (exit 2):

```text
npm run typecheck
```

First intended diagnostics, verbatim:

```text
tests/slice55-absent-native-fields-use-legacy-defaults.test.ts(21,41): error TS2339: Property 'structural' does not exist on type 'PerHitExplain'.
tests/slice55-absent-native-fields-use-legacy-defaults.test.ts(39,41): error TS2339: Property 'structural' does not exist on type 'PerHitExplain'.
tests/slice55-old-object-literals.test.ts(20,28): error TS2339: Property 'correlationId' does not exist on type 'Explanation'.
tests/slice55-old-object-literals.test.ts(21,20): error TS2339: Property 'structural' does not exist on type 'PerHitExplain'.
```

Canonical codec command (exit 101):

```text
cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_wire
```

First intended diagnostic, verbatim:

```text
error[E0432]: unresolved imports `fathomdb_engine::decode_dependency_trace_result_v1`, `fathomdb_engine::encode_dependency_trace_result_v1`, `fathomdb_engine::DependencyTraceDirectionV1`, `fathomdb_engine::DependencyTraceEdgeV1`, `fathomdb_engine::DependencyTraceNodeV1`, `fathomdb_engine::DependencyTraceResultV1`, `fathomdb_engine::TraceArtifactClassV1`, `fathomdb_engine::TraceArtifactRoleV1`, `fathomdb_engine::TraceNodeLifecycleV1`, `fathomdb_engine::TraceReadBoundaryV1`
```

## Verification measurement mechanism (Cycle 4 P3)

The ignored release fixture will use SQLite's per-connection progress handler
to count virtual-machine callback quanta during only the measured trace call;
the configured callback interval multiplied by completed callbacks supplies
the VM-step upper-bound witness and interrupts above 10,000,000. Peak RSS will
come from Linux `getrusage(RUSAGE_SELF).ru_maxrss`, recording a pre-call
baseline and post-call high-water delta in bytes. Fixture construction,
database open, and frozen-context minting are outside the measurement window.
The verification record must retain the exact release command, callback
interval, baseline/reset treatment, kernel/platform, and measured values.

## Mechanical RED witness correction

After production compilation reached the integrity test, Rust could not infer
the target type of its unannotated `Iterator::sum()` because the workspace's
`serde_json` dependency contributes cross-type numeric `PartialEq`
implementations. The orchestrator authorized the exact non-oracle annotation
`.sum::<u32>()`: expected values, fixtures, cases, and assertions are unchanged.
This test-only correction was committed separately before GREEN resumed.
Correction commit:
`65ecd3a1758e1e5ca5df4674661f0e8aed84a1f9`.

## GREEN chronology

Production GREEN adds the governed one-hop trace, canonical codec, operator
integrity request/report and CLI, structural explanation/correlation,
PyO3/Python and N-API/TypeScript mirrors, the successor ADR, five interface
records, and the approved governed-surface delta. No migration, reverse state,
release artifact, tag, registry staging, or publication was created.

Focused GREEN commands:

```text
cargo test -p fathomdb-engine --test slice55_dependency_trace
test result: ok. 7 passed; 0 failed

cargo test -p fathomdb-engine --features test-hooks --test slice55_dependency_trace
test result: ok. 9 passed; 0 failed; 1 ignored

cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_data_plane_integrity
test result: ok. 22 passed; 0 failed

cargo test -p fathomdb-engine --test slice55_explanation
test result: ok. 7 passed; 0 failed

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 1 passed; 0 failed

cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 2 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test check_integrity
test result: ok. 3 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test trace_source_ref
test result: ok. 3 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_wire
test result: ok. 2 passed; 0 failed

cargo test --release -p fathomdb-engine --features test-hooks --test \
  slice55_dependency_trace slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact
test result: ok. 1 passed; 0 failed

.venv/bin/pyright src/python/tests/test_slice55_wrapper_compat.py \
  src/python/tests/test_slice55_trace_explanation.py src/python/fathomdb/engine.py
0 errors, 0 warnings, 0 informations

npm run typecheck
> tsc --noEmit -p tsconfig.json

node --test dist/tests/slice55-old-object-literals.test.js \
  dist/tests/slice55-absent-native-fields-use-legacy-defaults.test.js
tests 2; pass 2; fail 0
```

The first TypeScript native-build attempt inside the restricted sandbox failed
before Cargo with `spawnSync /bin/sh EPERM`. The unchanged command was rerun on
the authorized executor and built the candidate N-API library; the two focused
compiled Slice 55 tests then passed. This is an execution-route deviation, not
a product/test failure.

Repository gates before the product commit:

```text
./scripts/agent-lint-md.sh
exit 0

./scripts/agent-lint.sh
exit 0

./scripts/agent-typecheck.sh
exit 0
```

The first lint run failed on two Clippy `type_complexity` diagnostics in the
new trace module. Factoring the stored node/edge tuples into named private
aliases fixed the diagnostics; the unchanged full lint command then passed.
There was no second occurrence of that failure mode.

The first restricted `./scripts/agent-test.sh` run could not finish as a valid
repository-gate observation. Repository fixture tests were denied writes to
the canonical Git tag lock and npm cache, local HTTP fixture servers could not
bind, and the expected pre-reissue governed-surface pin mismatch was present.
After the process continued without progress beyond normal runtime, it was
interrupted with exit 130. The exact environment diagnostics remain in
`/tmp/fathomdb-agent-test-*.log`; the unchanged gate is rerun outside the
restricted sandbox after the product commit and approved pin reissue.

GREEN implementation commit:
`f97abd0c5b09c7fba1cbb071f15e56974156c5de`.

The governed-surface pin was reissued in a separate, reviewable follow-up from
the exact committed allowlist bytes, matching the established Slice 50
mechanism. Product-commit allowlist identity:

```text
git blob 0cac3a1b9a66e6c5e7b337c08d52f60f332abed4
sha256 fd01c7c44d0eecfc4b8842450bb3506b1fc66f3344ec71b86e459e8d00814b7a
```

The reissued pin records 66 allowlist, 5 core, and 5 recovery-denylist members;
`./scripts/check-governed-surface-pin.sh` passed. The pin/chronology follow-up
commit is `9f70e79b5bc9bfc635e3ec4351d5d3cab0f9121b`.

## Approved governed-pin oracle rollover

The first complete unconfined repository-test observation at chronology commit
`eb4a0f1e` ran all 106 registered suites: 104 passed and 2 failed. The governed
pin fixture retained five expectations for the previously approved 64-member
shape after the Slice 55 pin had correctly advanced to 66. The orchestrator
authorized only those exact shape/count expectations to roll forward: the
explicit oracle now includes `trace_dependency` and `traceDependency`, the
base count is 66, and its added/removed fixtures expect 67/65. Pin hashes,
provenance logic, checker behavior, denylist expectations, and product code are
unchanged.

The other failing suite was the known worktree-native trap: source Python
resolved the stale checked-out `_fathomdb.abi3.so`, which predates the already
landed `ProjectionGenerationError`. That binary was not mutated, deleted, or
replaced. Candidate-native Python verification uses the isolated disposable
wheel workflow below.

Focused rollover verification:

```text
bash scripts/tests/test_check_governed_surface_pin.sh
All check-governed-surface-pin tests passed

./scripts/check-governed-surface-pin.sh
ok governed-surface-pin (66 allowlist / 5 core / 5 recovery_denylist)
```

Approved oracle-update commit:
`396de2bc49e6ec86f93838df25f36a8127bcc630`.

## Exact-candidate Python installation evidence

Candidate commit:
`396de2bc49e6ec86f93838df25f36a8127bcc630`. The worktree was clean before
the build. Version `0.8.24` is the unchanged current workspace/package version
at this Slice 55 CI boundary; no release version bump was performed.

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-wheel.zcwF30/dist \
  --venv-dir /tmp/fathomdb-s55-wheel.zcwF30/venv

wheel smoke: ok
sha256 f1154a70d505d1244d954eb7ae8f547c75683e1107a94d94dfb91436acc8c2f6
module=/tmp/fathomdb-s55-wheel.zcwF30/venv/lib/python3.12/site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-wheel.zcwF30/venv/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so
editable=false
```

The plan-named `src/python/tests/smoke_slice55_installed.py` was absent from
the candidate. A verification-only script under the disposable wheel root
therefore exercised a real SQLite database through the installed module:
canonical and derived provenance writes, dependency registration, frozen
context, reciprocal `trace_dependency`, explained-search structural presence
and correlation, and continued absence of Python doctor methods. It passed:

```text
env -u PYTHONPATH /tmp/fathomdb-s55-wheel.zcwF30/venv/bin/python \
  /tmp/fathomdb-s55-wheel.zcwF30/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

Byte-identical disposable copies of the two checked-in Slice 55 Python test
files were used to prevent repository pytest path injection from selecting the
stale worktree extension. Their source/copy SHA-256 pairs were identical:
`fe64413f...bdc` and `a95e0a84...87dc`; 5 tests passed. The installed-candidate
Slice binding regression suite then passed:

```text
PYTHONPATH=<candidate-site-packages>:<source-support>:<repo-root>:<tool-site-packages> \
  /tmp/fathomdb-s55-wheel.zcwF30/venv/bin/python -P -m pytest \
  -o pythonpath= src/python/tests/test_slice*.py -q
265 passed, 2 skipped in 26.26s
```

For completeness, an attempted entire Python tree against this release-wheel
feature set reported 1435 passed, 16 skipped, and two unrelated failures. The
cwd-sensitive graph receipt test passed when rerun from the repository. The
remaining M1 eval assertion requires the development-only default reranker,
which the repository wheel verifier intentionally omits through its explicit
feature list; it reproduced alone and is not a Slice 55 installed-binding
failure. No worktree native binary was modified, and no artifact was staged,
uploaded, tagged, or published.

## Implementation review FIX-1

The cycle-1 review record is committed at
`7a2faee255afe2852e30ba918818c7442492ec11`. The first FIX-1 RED increment is
`22c65c09`; corrective commit `f0ef98e3` restored the production source to its
pre-RED state and moved the default-build absence oracle into the integration
test surface. Further RED increments are `94953a28` for authenticated trace
authority, `cf227299` for strict recursive wire decoding, and `df88627e` for
Python/native-exception and TypeScript schema-first validation. Integrity and
operator-boundary GREEN is `970aaef27d556c79e557a0a3d2177c2d2227888a`.

The strict decoder correctly rejected the property fixture at `/nodes/0`
because the generated `rootRevisionId` did not update its root node or outgoing
edge. With orchestrator authorization, the fixture was mechanically corrected
to use the same generated revision in those three linked fields. Assertions,
expected results, generated domain, malformed matrices, and production codec
semantics are unchanged. The untracked regression seed emitted by this failed
run was removed and was never committed.

The performance RED increment is `91fe3179`. Its first GREEN observation built
the real 50,000-row fixture and measured 600,000 VM steps, 814 ms, and a zero
peak-RSS delta, but correctly failed byte equality because its independently
created databases had distinct Engine-minted generation identities and times.
With orchestrator authorization, fixture topology was mechanically corrected:
one closed source-only database is byte-copied to the baseline and hidden
cases, both measurements use fixed effective time 1, and the test-only corrupt
fixture seeder restores the pre-seed writer boundary. No response bytes are
normalized or fabricated; the 50,000 count, assertions, ceilings, and trace
production semantics are unchanged.

The default-feature all-target Clippy route then identified that the
`source_only` helper is reachable only with the `test-hooks` feature used by
the ignored performance witness. With orchestrator authorization, the helper
received the matching compile-time gate; no case, assertion, fixture value, or
oracle changed.

## FIX-1 GREEN chronology

Authenticated trace authority and strict recursive codec GREEN landed at
`34f22226156562a1074562e6c027a204f6000060`. Schema-first Python/TypeScript
validation and the actual native Python exception identity landed at
`c77b9e5d140e816fab119e72f4614a26cb865438`. The latter exposed Cargo feature
unification in workspace-wide checking: the operator-enabled facade caused
the PyO3 and N-API matches to see the operator-only engine variant even though
neither SDK exposes the operator API. The final binding arms are explicitly
operator-only/unreachable and retain no SDK doctor surface; the unchanged
workspace typecheck passed after this correction.

The real 50,000-hidden-dependent performance GREEN is
`7d336c2423a4aef11bbea55fbcb5f1e2e3650033`. Its release-mode ignored witness
passed with the following exact observation after fixture construction had
completed outside the measured window:

```text
cargo test --release -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact --nocapture
slice55 trace measurement: hidden_rows=50000 vm_steps=600000 elapsed_ms=780 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out
```

The progress handler aborts above 10,000 callbacks at the 1,000-instruction
cadence, providing the required greater-than-10-million-VM-step fail-closed
ceiling. Peak RSS is sampled from `getrusage(RUSAGE_SELF).ru_maxrss` around the
measured call. The test proves byte-identical responses against the same closed
database image without hidden rows and proves absence of a bound refusal.
Rusqlite hook APIs are enabled only by `test-hooks`; libc is optional in the
library and selected by that feature, with a dev dependency for the integration
test compile surface. No production-default hook API was added.

Structural explanation GREEN landed at
`e4fd954ca4c48dcda97b410f67a45842a4991603`. The engine derives projection
readiness, lifecycle, dependency registration, edge validity, degradation,
and per-search correlation from the authenticated search transaction. The
focused explanation suite passed 11 tests, including the concurrent unique
correlation witness. Installed/package smoke extensions landed at
`bb32310a8ba9ed2a0599ba80996eaeab83e93d10`.

## FIX-1 projection authority follow-up

The first package-installed smoke reached the intended real database
corruption refusal but expected `/dependencyEdges`. The accepted
nondisclosure contract returns `trace_corrupt` with an empty storage-corruption
path. With orchestrator authorization, only that newly added smoke expectation
changed; error class, reason, scenario, and refusal assertion remain intact.
The transparent test-only correction is
`bd026abfe5d8dd7439b1ef3f60b16255ad4b6f7c`, and the unchanged rerun passed:

```text
env -u PYTHONPATH /tmp/fathomdb-s55-fix1.7ziHuf/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

Review-cycle-1's plan-named projection tests were then found to be clean-empty
placeholders rather than real corruption fixtures. The test-only RED increment
`0c9dc9aea4deb1641f31fe4adae3577e30ae3b69` replaced them with public-write,
real-database deletion fixtures. The focused failures were:

```text
test slice55_missing_edge_body_fts ... FAILED
  left: []
 right: [EdgeBodyFtsMissing]
test slice55_missing_canonical_attribute ... FAILED
  left: []
 right: [CanonicalAttributeMissing]
test slice55_missing_property_fts ... FAILED
  left: []
 right: [PropertyFtsMissing]
```

The separate dense-classifier RED is
`a3f27f1382e117be0ab276f8ccf02791b0cc572c`:

```text
test slice55_dense_state_reuses_slice40_classifier ... FAILED
  left: []
 right: [DenseProjectionPartial]
```

The separate receipt-point classifier RED is
`4641937b4301c057aa5aaec8b276e30f7f6300e1`:

```text
test slice55_mutation_readiness_receipt_matrix ... FAILED
  left: []
 right: [MutationReadinessCorrupt]
```

Integrity GREEN is
`c4085cc03177002459255d144daed29fab783886`. It adds cap-plus-one expected and
physical scans for node/edge body FTS, registry-derived canonical attributes
and property FTS, content-free missing/orphan/identity findings, normalized
source-link hash/self-link verification, generation-wide Slice 40 status
validation, and direct reuse of the Slice 40 physical completion classifier
for dense tuples and receipt points. The first aggregate focused run revealed
that a terminal-only row belongs to the synchronous projector even when no
dense projection is declared; dense physical candidates were therefore
correctly restricted to sidecar/vector membership. The unchanged full focused
suite then passed, so no retry-budget stop was activated:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 2 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test check_integrity
test result: ok. 3 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test trace_source_ref
test result: ok. 3 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_wire
test result: ok. 5 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --all-targets -- -D warnings
Finished `dev` profile
```

The first SDK FIX-4 RED uses an exact disposable candidate wheel so the stale
worktree extension remains untouched. Python accepted boolean `true` as trace
response schema 1 and surfaced malformed nested frozen context through the
wrong exception boundary:

```text
test_slice55_python_rejects_boolean_response_schema
Failed: DID NOT RAISE <class '_fathomdb.DependencyTraceError'>

test_slice55_python_malformed_nested_frozen_context_is_frozen_error
expected FrozenReadError
```

The equivalent TypeScript nested-context case already preserved
`FrozenReadError`; it is retained as an explicit cross-SDK nonregression case.

The exact SDK RED commit is
`872d8216cd8ad9b122c6d85551d00fe0531b9a1a`. GREEN rejects bool response
schemas and validates both frozen-context schema layers with the established
`FrozenReadError` reason and nested field paths. Ruff and Pyright pass.

The trace predecode RED makes an endpoint lifecycle-visible while its derived
source-link hash is invalid and its selected dependency ID is a BLOB. The
bounded forward trace returned `Storage` instead of ignoring the unauthorized
candidate and returning the one valid relation; reverse trace requires the
same nondisclosing unavailable result as an absent root.

The SDK strictness RED covers Python's bool-as-schema, invalid root,
direction, and context construction in declaration order; TypeScript's null
context; and unknown explanation arms/nonfinite or invalid scalar fields in
both wrappers. The old paths either accepted the value, leaked `AttributeError`
or native `TypeError`, defaulted an unknown arm to text, or propagated NaN.

The first installed-candidate full module exposed that its pre-existing native
refusal test intentionally constructs an invalid direction before entering the
exception assertion. HITL therefore authorized preserving constructor
compatibility: bool/schema remains a constructor RED, while the three new
root/direction/context cases mutate a valid frozen request immediately before
`Engine.trace_dependency`. Reasons, paths, and declaration-order precedence are
unchanged; validation remains before the native argument conversion that had
leaked Python/PyO3 type errors.

The exact cross-SDK RED is
`39a766b07999f1c9dcd120710ab359fc116be327`; the authorized compatibility
correction is `fe4d3f9d48a98aa24974d1a9caa7d783e7ddc21c`. GREEN applies the same
schema/root/direction/context/bounds precedence before Python native conversion,
rejects TypeScript null context at `/context`, and validates every per-hit
integer, rank, branch, and finite score before mapping. Focused results:

```text
node --test dist/tests/slice55-request-validation.test.js \
  dist/tests/slice55-absent-native-fields-use-legacy-defaults.test.js
tests 11; pass 11; fail 0

.venv/bin/ruff check src/python/fathomdb/types.py \
  src/python/fathomdb/engine.py src/python/tests/test_slice55_trace_explanation.py
All checks passed!

.venv/bin/pyright src/python/fathomdb/types.py \
  src/python/fathomdb/engine.py src/python/tests/test_slice55_trace_explanation.py
0 errors, 0 warnings, 0 informations
```

The verification-only exact candidate wheel is
`fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl`, SHA-256
`df3371bf825e131e4674541f9c38ad63ba449917dbee144baba01b77af560186`.
It was installed under `/tmp/fathomdb-s55-fix3-sdk-green.3UJKZS/venv`; the
wheel smoke passed and the copied focused module, with repository source
excluded from import resolution, passed `18 passed in 3.58s`. This artifact is
local verification evidence only and was not staged, tagged, uploaded, or
published.

The closure audit confirmed the searchable-orphan, missing-member,
generation-authority, guarded-receipt, boundary property, hidden-corruption,
and multi-edge codec cases now mutate real database/wire authority. The one
remaining happy-only trace oracle is replaced with a zero-generation
corruption asserted reciprocally after both endpoints remain authorized. A new
integrity RED leaves real EAV and property-FTS residue behind a resolvable
deleted owner and requires both class-ordered findings to carry its revision
ID. The old physical direction emitted empty `artifact_revision_ids`.

The trace-authorization RED places a malformed BLOB dependency identity on an
inactive derived endpoint that sorts before the one eligible relation, then
runs with exactly one relation and two work units. Because the old query
decoded the unfiltered row before checking endpoint visibility, it leaked a
storage failure instead of returning the eligible relation:

```text
slice55_trace_hidden_corruption_is_filtered_before_decode_and_limit
called `Result::unwrap()` on an `Err` value: Storage
```

## Final exact-candidate artifact evidence

The disposable wheel was rebuilt from exact product commit
`c4085cc03177002459255d144daed29fab783886`; version `0.8.24` remains the
expected pre-release package version and was not mechanically bumped.

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix1-final.fCFxfd/dist \
  --venv-dir /tmp/fathomdb-s55-fix1-final.fCFxfd/venv
wheel smoke: ok
ba13892fecb445e8f2083b146fe0a864b7543afd36ae7e8aa82862b7938ba27d  /tmp/fathomdb-s55-fix1-final.fCFxfd/dist/fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix1-final.fCFxfd/venv/lib/python3.12/site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix1-final.fCFxfd/venv/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix1-final.fCFxfd/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

The isolated installed candidate, with the repository Python source excluded
from import resolution, passed both Slice 55 binding files: `8 passed in
0.23s`. The exact N-API debug build plus the three focused Slice 55 runtime
files passed `5` tests. The durable smoke scripts and installed Python smoke
also passed their syntax/Ruff checks:

```text
bash -n scripts/release/smoke/smoke-pypi-wheel.sh \
  scripts/release/smoke/smoke-npm-package.sh \
  scripts/release/smoke/smoke-crates-cli.sh
.venv/bin/ruff check src/python/tests/smoke_slice55_installed.py
All checks passed!
```

Final writer gates from the clean product commit plus this evidence-only
chronology update:

```text
./scripts/agent-lint.sh
exit 0
./scripts/agent-typecheck.sh
exit 0
```

Per the orchestrator's explicit direction, the 13-minute aggregate
`agent-test.sh` was not repeated in this writer turn; independent verification
owns the final aggregate gate. No source test required a skip or weakening, no
worktree Python native artifact was changed, and no package was staged,
uploaded, tagged, or published.

## Implementation review FIX-2 chronology

Cycle-2 verdict `FAIL` is recorded at `fd4b94f3`. Execution-boundary RED
`43178a92` proved public struct literals bypassed constructor validation for
integrity schema/check/cap invariants and trace schema/root/cap invariants.
GREEN `23b6ed00` revalidates before opening the reader transaction, reports the
duplicate at `/checks/1`, and restores canonical check order.

Bounded-registry RED `ee6fc53a` placed malformed JSON in an unselected row and
proved the former full registry load returned storage failure before the work
cap. GREEN `b2298a54` selects cap-plus-one names in primary-key order, reserves
each unit, and only then point-loads the selected declaration.

Trace-authority RED `0e23cbf1` captured malformed lifecycle leaking as storage
failure and shape-valid but byte-invalid hashes remaining visible. The false
source-version happy-path oracle was strengthened separately at `fb9cafc1`.
GREEN `5e5c3bc2` collapses malformed lifecycle to absence and authenticates the
source-version, canonical owner/body, UTF-8 locator boundaries, and SHA-256
bytes. Focused trace result: 16 passed, 1 ignored.

Receipt RED `52e3d16c` proved malformed pending cursors were parsed and reported
despite no aggregate capacity for the two pending units. GREEN `366a917e`
reads only guarded array length first and returns the bound before fetching or
decoding the pending list.

The prior `trace-v1.json` was an incomplete agent-created seed that contradicted
the mandatory nonempty nodes and read-boundary schema. With explicit HITL
authorization, test-only RED `dea48b0a` replaced it with the supplied
design-derived one-line canonical bytes, without a trailing newline, and made
the test consume the file by exact byte equality. GREEN `6364a9cd` replaces
nested `serde_json::Value` maps with explicit declaration-order serializers.
Semantic RED `cdb4bca2` then covered root role, edge endpoint, and revision
uniqueness; GREEN `f8e19639` enforces those invariants and hard response maxima.
Focused wire result: 6 passed.

The FIX-2 live-explanation RED `e4c80aa9` initially reached 12 of 14 GREEN
cases. Two remaining failures were fixture mechanics, and the HITL authorized
only these test-only corrections with every assertion and threshold preserved:
serialize the two process-global one-shot finalization hooks through a test
mutex, and change the 64-node/63-edge graph fixture's non-seed topology from a
depth-truncated chain to a star rooted at the first resolved seed. The latter
makes the real max-depth-three traversal exceed its existing cap and can
therefore exercise `graph_bound_reached`; no traversal limit was reduced.
Production edits remained unstaged while this correction was committed.

The serialized hook tests still allowed unrelated parallel explained searches
to consume the process-global one-shot. At the retry-cap rethink, the HITL
authorized a thread-targeted rendezvous and moving arming into the intended
search thread. The new RED records the failure directly: the
unrelated search ran on `ThreadId(2)`, consumed a hook armed by `ThreadId(14)`,
and failed with `an unrelated thread consumed the armed hook`. Assertions and
the telemetry/explanation race outcomes remain unchanged.

The prior `slice55_concurrent_explain_telemetry_ids_unique` was a single-call
nonempty-string assertion. Its test-only replacement starts explained and
ordinary searches at one barrier against an enabled real JSONL sink, then
requires exactly two events, the exact identity set `q0-0`/`q0-1`, and the
explained result's identity in that set. It exercises the mixed finalization
path without accepting alternate event counts or identifier shapes.

Recursive structural-explanation SDK RED adds Python and TypeScript cases for
unsupported nested schema, included/nonempty-degradation incoherence, and
duplicate degradation codes at their exact nested paths. The direct TypeScript
run failed both behavioral cases with `Missing expected exception`; Python
source lint passed, while runtime remains reserved for the exact disposable
candidate because the preserved worktree native module is stale.

### FIX-2 completion from the clean checkpoint

Physical projection-generation-member RED
`66b9d5d7191ce73ffa62af4e8150169d40aada50` requires every physical dense
member to be counted, attributed to its revision and serving generation, and
classified as `ProjectionMemberCorrupt` when its terminal is missing. It also
requires cap-plus-one to fail atomically. GREEN
`682ba1f1a3740fab9007cfed2446f0f891797f35` implements the bounded physical
enumeration. Query-plan RED
`905e02b003b5c20fbad5944a781b97c2614f22ff` replaced an empty happy-path
oracle with exact index and no-materialization assertions; test-hook GREEN
`9f1f4931f3367b3e87f88b440432b3fc0c80a21c` exposes the production plans.

Endpoint-closure RED `d1b900afe2b94cf33fc4d73f9df369de8802db46`
proved an unrelated active closure incorrectly hid a valid relation and
required either relation endpoint's closure to fence both directions. GREEN
`93ddd5368f4a98114fc22882fa9b9fa609fb6761` applies those endpoint-specific
fences. Bounded-paging RED
`a645e2d5b09c3b8f1820d68d65e947d5668fe81a` uses genuine hidden rows before
and after the visible row, requires exact-byte absence parity, pins the
after-key SQL, and rejects a temp B-tree or materialization. GREEN
`a295e150875e1cb129f4de98d7fbc603c8bddcf0` pages authorized candidates by
after-key with remaining-plus-one accounting.

Recursive trace SDK RED
`1b168bbdd86336ac31cf4beb4ac386cc97be70cb` covers nested lifecycle schemas,
duplicate revision IDs, canonical integers, escaped RFC 6901 paths, malformed
JSON, and noninteger public bounds without native `TypeError` leakage. GREEN
`da19eb1c8685295e456da239850a064d38bc4e20` supplies schema-first Python and
TypeScript validation and the CLI `/checks/<index>` path.

Live-explanation RED
`e4c80aa9a250a28a333f0741e951b19f6c4032d8`, authorized fixture-mechanics
commit `dc0f3bbbb1a215bfb51a7b01c42a1e804959b635`, thread-isolation RED
`ff4a632ee7be9c9d82d21b50508a5c45f1f8c38e`, genuine mixed-telemetry RED
`c4515d8862af26be8a7fe4e0a0a7f2082f4e7381`, and recursive structural SDK RED
`3a83eae889030643cb7d5d0a072a775e66a463cb` all precede product GREEN
`ee196c6c0ef3c4a4891d02d17cd0189375837032`. The GREEN validates the full
registered dependency/source/hash/closure chain from the search snapshot,
loads lifecycle state from live records, emits `GraphBoundReached` from real
traversal state, targets one-shot test hooks to the arming thread, and validates
nested Python and TypeScript structural responses.

Focused product results at that exact GREEN commit were:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 30 passed; 0 failed

cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace
test result: ok. 18 passed; 0 failed; 1 ignored

cargo test -p fathomdb-engine --test slice55_explanation
test result: ok. 15 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --all-targets -- -D warnings
Finished `dev` profile

.venv/bin/ruff check src/python/fathomdb/engine.py \
  src/python/tests/test_slice55_trace_explanation.py
All checks passed!

cd src/ts && npm run typecheck
tsc --noEmit -p tsconfig.json
```

The first operator clippy run found the restored point-registry helper absent
and two tuple-key inference errors. The second run reduced this to one tuple-key
error. At the two-attempt boundary, implementation stopped, reread the physical
member test, the design's exact member authority, and the Slice 40 and Slice 50
memory lessons, then used explicit owned key clones. The next run exposed a
different type-complexity lint in the new hook and passed after a named alias;
the retry budget was not exceeded.

The release-mode performance witness used the real 50,000-row hidden fixture
outside the measured window. Its progress-handler interrupt remained armed at
10,000,000 VM steps, and the measured response was byte-identical to the
source-only baseline with no bound refusal:

```text
cargo test --release -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact --nocapture
slice55 trace measurement: hidden_rows=50000 vm_steps=1000000 \
  elapsed_ms=1156 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed
```

A serial physical-dense residue RED changes a previously enrolled node to an
unregistered kind only after its terminal and sidecar members exist. The
integrity classifier correctly reports `DenseProjectionOutsideMembership`,
but the finding loses the still-resolvable source revision:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_dense_residue_reports_resolvable_revision \
  -- --exact --nocapture
assertion `left == right` failed
  left: []
 right: ["dense-residue-r1"]
```

The exact dense-residue RED commit is
`8a98c198af89c30ae8a2ec3728e5ba375af87b4c`. GREEN resolves the revision only
after proving that a canonical owner exists. Focused and aggregate results:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_dense_residue_reports_resolvable_revision \
  -- --exact --nocapture
test result: ok. 1 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 36 passed; 0 failed

cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace
test result: ok. 19 passed; 0 failed; 1 ignored
```

The final cycle-3 oracle audit replaced the remaining trace endpoint
happy-path-only case with reciprocal corruption refusals and added a physical
EAV/property residue case that requires a resolvable owner revision ID and
canonical attribute-before-property ordering. The exact test-only RED commit
is `71a7357d97e7dcd365a6b1086e6b7fc5ca123d86`. Its intended failure was:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_projection_residue_reports_resolvable_revision_in_class_order \
  -- --exact --nocapture
assertion `left == right` failed
  left: [[], []]
 right: [["node-r1"], ["node-r1"]]
```

GREEN resolves the physical member's canonical node revision only after the
owner-existence classifier succeeds, preserving nondisclosure for owner-missing
residue. The unchanged focused oracle passes:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_projection_residue_reports_resolvable_revision_in_class_order \
  -- --exact --nocapture
test result: ok. 1 passed; 0 failed
```

The next RED creates 200 genuine active nodes whose configured attribute path
does not resolve, followed by one node whose scalar attribute is required. It
deletes only that final node's generated EAV and property-FTS members. With a
bounded allowance smaller than the raw owner prefix but larger than the fully
classified member set, the old pre-classification `LIMIT` hid the required
owner and returned an empty finding set:

```text
slice55_sparse_attribute_owners_cannot_hide_later_required_members
left: []
right: [CanonicalAttributeMissing, PropertyFtsMissing]
```

The exact sparse-owner RED commit is
`e29df0b496b7e3476dfbcd348c09cdebbbac9fb7`. GREEN walks each declaration's
active owners with an indexed after-key query until all fully classified scalar
members are found, invokes the shared closure-eligibility classifier, and keeps
canonical-attribute work ahead of property-FTS work. Results:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 34 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- -D warnings
Finished `dev` profile
```

The exact hidden-corruption RED commit is
`4e966062b59285f471ab78a3eace5c6ba94bb153`. GREEN moves both endpoint
lifecycle, validity, metadata-filter, attribute-filter, role, and completeness
authorization into the indexed candidate SQL ahead of projection, decoding,
and `LIMIT`. The existing full normalized-chain verifier remains the second
stage for already-authorized candidates. The full focused result is:

```text
cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace
test result: ok. 19 passed; 0 failed; 1 ignored
```

The graph-bound RED builds a live 50-candidate traversal, exactly equal to the
closed graph-arm cap, and requires no degradation. The old traversal marked
the cap-th accepted candidate as omitted work:

```text
slice55_graph_bound_is_absent_at_exact_eligible_cap
assertion failed: ... !...contains(&GraphBoundReached)
```

The initial GREEN rerun reproduced the assertion because the fixture itself
had 51 eligible candidates: the matched edge emits both endpoints as seeds,
then 49 additional nodes. Per the retry rule, work stopped for HITL review.
The authorized mechanical correction uses 50 nodes and 49 edges, preserving
the assertion and production cap, so two seeds plus 48 reached nodes is exactly
50 eligible candidates.

The exact graph-bound RED is
`82cf64038f8c4a7e117e281d48ac1dcec68c934f`; the authorized fixture-only
correction is `9a8bdc0c1bd5e4fbd3b8f4c620729ea6c8bedaf7`. GREEN continues traversal after
accepting the cap-th candidate and reports `graph_bound_reached` only upon
classifying one additional eligible candidate that must be omitted. Results:

```text
cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_explanation slice55_graph_bound -- --nocapture
test result: ok. 2 passed; 0 failed

cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_explanation
test result: ok. 17 passed; 0 failed
```

The strict Rust wire RED adds a real two-relation response shape and requires
duplicate dependency identities and noncanonical relation order to fail at
stable nested paths. It also requires both public Slice 55 error records to
carry their version discriminator. Before production changes, the targets fail
to compile on the absent `schema_version` fields and the decoder accepts both
duplicate and reordered relation arrays.

The exact strict-wire RED is
`ec83ce9666c39cc748be63cf9747409792f65872`. GREEN adds `schema_version=1` to
both typed error records and validates dependency-ID uniqueness plus canonical
relation/node order before accepting a decoded response. Results:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_wire slice55_wire -- --nocapture
test result: ok. 7 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_integrity_execution_boundary_revalidates_public_struct_literals \
  -- --exact --nocapture
test result: ok. 1 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_wire --test slice55_data_plane_integrity -- -D warnings
Finished `dev` profile
```

Full writer gates passed at the same product commit:

```text
./scripts/agent-lint.sh
exit 0
./scripts/agent-typecheck.sh
exit 0
```

The exact-source N-API build initially failed in the sandbox with
`Type Error: Could not parse the Cargo.toml: Error: spawnSync /bin/sh EPERM`.
The unchanged package-local command was rerun unconfined under the standing
authorization and passed. The plan's root `--workspace fathomdb` spelling is
not executable because the repository root declares no npm workspaces, so the
equivalent package-local command was used:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 64; pass 64; fail 0
```

### FIX-2 exact installed-candidate evidence

The disposable wheel was built from exact product commit
`ee196c6c0ef3c4a4891d02d17cd0189375837032`. Version `0.8.24` remains the
expected pre-release package version.

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix2-final.dfnM1W/dist \
  --venv-dir /tmp/fathomdb-s55-fix2-final.dfnM1W/venv
wheel smoke: ok
4adb9ee31162c08ec997b51c6c55fdd8bc36818749eb6db4a3e25b523b361819  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix2-final.dfnM1W/venv/lib/python3.12/site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix2-final.dfnM1W/venv/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix2-final.dfnM1W/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

Direct pytest invocation by repository path twice demonstrated pytest's known
root insertion shadowing the installed candidate with the preserved stale
worktree extension. That extension was not changed. At the retry boundary, the
unchanged two test files were copied to the disposable directory and run there
with no repository package path; the installed candidate then passed all 16
cases in 0.23 seconds. No local artifact was staged, uploaded, tagged, or
published.

## Implementation review FIX-3 chronology

Cycle-3 verdict `FAIL` is recorded in docs-only commit
`65ac12b168d7097da7f3f4c8cfbc6c6e19816a3a`.

The first test-only increment proves both sides of the explanation-hook safety
boundary. A disposable default-feature consumer compiled successfully when it
imported both test rendezvous functions, producing the intended RED:

```text
cargo test -p fathomdb-engine --test slice55_explanation_hook_surface \
  -- --nocapture
default consumer compiled test-only hooks
test result: FAILED. 0 passed; 1 failed
```

The runtime RED makes the after-finalization callback re-enter
`enable_telemetry` from another thread and requires completion within one
second. It timed out because the callback ran while the search thread retained
the telemetry mutex:

```text
cargo test -p fathomdb-engine --test slice55_explanation \
  slice55_after_finalization_hook_runs_outside_telemetry_lock \
  -- --exact --nocapture
after-finalization callback ran while telemetry lock was held: Timeout
test result: FAILED. 0 passed; 1 failed
```

The compile-surface command required an unconfined rerun after the sandbox
could not extract a cached Cargo dependency into the read-only registry. The
unchanged unconfined run produced the intended product failure above.

The exact test-only RED commit is
`7a5ea4b99535d2d83d87b2eb60a7d39946bfa841`. GREEN gates the module, public
arm functions, and finalization callsites with `test-hooks`; the after-hook now
runs only after the telemetry mutex guard has left scope. The explanation test
target explicitly requires that private feature. Focused results:

```text
cargo test -p fathomdb-engine --test slice55_explanation_hook_surface
test result: ok. 1 passed; 0 failed

cargo test -p fathomdb-engine --features test-hooks \
  --test slice55_explanation
test result: ok. 16 passed; 0 failed
```

The next test-only integrity increment replaces the remaining empty and
misdirected oracles with physical terminal residue, guarded receipt identity,
committed-boundary, generation-authority, unrelated-field, oversized-field,
and real corrupt-generation fixtures. Its bounded-subset fixture places a
non-text operation identity after the selected row, so an implementation that
decodes the raw cap-plus-one prefix returns `Storage` rather than the required
bound. The nontrivial property generates one to four real pending writes and a
committed boundary below the greatest pending cursor. Intended RED diagnostics
included:

```text
slice55_terminal_only_residue_is_enumerated_by_both_integrity_checks
left: 0
right: 1

slice55_receipt_boundary_covers_every_pending_cursor
left: []
right: [MutationReceiptCorrupt]

slice55_receipt_generation_must_be_current_authority
left: []
right: [MutationReceiptCorrupt]

slice55_receipt_variable_field_guards_precede_fetch
called `Result::unwrap()` on an `Err` value: Storage

slice55_receipt_boundary_property
minimal failing input: count = 1, boundary = 0
left: 0
right: 1
```

The exact integrity RED commit is
`c26410f23eb06e666f0418f17ecc4d734a62fa60`. The first GREEN increment adds
terminal-only residue enumeration without misclassifying legitimate
synchronous terminals, guards operation identity entirely in the first SQL
pass, reserves work before guarded row fetch, enforces committed boundary and
generation authority, and handles the valid legacy-null-generation path as
unavailable. Focused results:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 33 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- -D warnings
Finished `dev` profile
```

The next integrity RED binds the plan oracle to execution-owned candidate SQL,
rather than allowing the test hook to explain unrelated hand-written
statements. It requires all seven synchronous owner/physical scans to expose
their indexed after-key and aggregate remaining-cap-plus-one query. Before the
production query set existed, the focused target failed to compile with:

```text
error[E0599]: no method named `data_plane_integrity_candidate_queries_for_test`
found for struct `Engine` in the current scope
```

The exact query-identity RED commit is
`ca694c030670a7cf0aab4fcbba85e5e2b8849f1b`. GREEN moves those statements
into the integrity execution module, uses them for the owner and physical
scans, and makes both the candidate hook and `EXPLAIN QUERY PLAN` hook consume
that same set. The focused plan oracle passes:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity \
  slice55_projection_scan_plans_use_indexed_order -- --exact --nocapture
test result: ok. 1 passed; 0 failed
```

The remaining cycle-3 serial RED/GREEN commits are:

- sparse projection owner paging: RED
  `e29df0b496b7e3476dfbcd348c09cdebbbac9fb7`, GREEN
  `4878329d9d2b9a245cac629227fe721bc72b03eb`;
- hidden trace corruption filtered before decode and eligible limit: RED
  `4e966062b59285f471ab78a3eace5c6ba94bb153`, GREEN
  `f713e184fbce261334789e1ff9b57c683cfda424`;
- exact-cap graph traversal: RED
  `82cf64038f8c4a7e117e281d48ac1dcec68c934f`, approved topology-only fixture
  correction `9a8bdc0c1bd5e4fbd3b8f4c620729ea6c8bedaf7`, GREEN
  `e39fc20b3438318b5b9a32260a8d8e86c52f980c`;
- Rust error schema, duplicate dependency identity, and canonical ordering:
  RED `ec83ce9666c39cc748be63cf9747409792f65872`, GREEN
  `ff91c61411184d59ece4a2c559c9f13d4ebdcde0`;
- recursive Python/TypeScript request, trace, and explanation validation: RED
  `39a766b07999f1c9dcd120710ab359fc116be327`, compatibility-preserving
  test correction `fe4d3f9d48a98aa24974d1a9caa7d783e7ddc21c`, GREEN
  `f425aa90e04e6f857564767fb9793c0448bf2cc6`;
- physical projection residue source identities: RED
  `71a7357d97e7dcd365a6b1086e6b7fc5ca123d86`, GREEN
  `8246a0ae74d041a7cf64e7916320452a8bffaaa7`, followed by the dense-member
  RED/GREEN pair recorded above.

The closing focused matrix at product commit
`14a997d3fb58a17d2ec20a2f352cc8aa8f6e9724` passed:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_explanation --test slice55_wire \
  --test slice55_dependency_trace --test slice55_data_plane_integrity
integrity: 36 passed; trace: 19 passed, 1 ignored;
explanation: 17 passed; wire: 8 passed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_explanation --test slice55_wire \
  --test slice55_dependency_trace --test slice55_data_plane_integrity \
  -- -D warnings
Finished `dev` profile

./scripts/agent-lint.sh
exit 0
./scripts/agent-typecheck.sh
exit 0
```

The first lint tool session closed without returning its final status. One
unchanged unconfined rerun completed with exit 0; this was an evidence-channel
retry, not a product diagnostic or a test retry.

### FIX-3 exact installed-candidate evidence

The disposable wheel was built from exact clean product commit
`14a997d3fb58a17d2ec20a2f352cc8aa8f6e9724`; version `0.8.24` remains the
expected pre-release package version. No artifact was staged or published.

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix3-final.4nJJDA/dist \
  --venv-dir /tmp/fathomdb-s55-fix3-final.4nJJDA/venv
wheel smoke: ok
12af4d88c537ebb22d45fcf155b12904cf93ee7a41e8d82a9f46a71ca80c2e5f  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix3-final.4nJJDA/venv/lib/python3.12/site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix3-final.4nJJDA/venv/lib/python3.12/site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix3-final.4nJJDA/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok

pytest -o pythonpath= test_slice55_trace_explanation.py \
  test_slice55_wrapper_compat.py -q
24 passed in 0.33s
```

The exact-source N-API candidate also passed:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 73; pass 73; fail 0
```

## Implementation review FIX-4 chronology

Cycle-4 verdict `FAIL` is recorded in docs-only commit
`d2d8f4d497334cdedbc7e7c8e2f9cbb0e4c45050`.

The first test-only RED increment covers exact request precedence and indexed
duplicate paths, finding-cap attribution, both source and derived revision
identities, and declaration-name-before-cursor sparse finding order. Intended
diagnostics were:

```text
slice55_integrity_constructor_uses_declared_precedence_and_duplicate_path
  left: "/checks"
 right: "/checks/1"

slice55_integrity_finding_overflow_names_max_findings
assertion failed: value.field_path == "/maxFindings"

slice55_dependency_source_finding_names_both_revisions
  left: ["integrity-derived-r1"]
 right: ["integrity-derived-r1", "integrity-source-r1"]

slice55_attribute_findings_follow_declaration_then_cursor_order
  left: [["zeta-r1"], ["alpha-r1"]]
 right: [["alpha-r1"], ["zeta-r1"]]
```

The exact RED commit is
`c263c64cb0131a88c09dcc5799c6a4a646684d18`. GREEN preserves the declared
validation precedence, reports the later duplicate index, separates work and
finding bound paths, carries both dependency endpoint revisions, and keys
sparse findings by declaration name then cursor. All four focused cases pass.

The second integrity RED binds the production-plan hook to dense, generation,
and receipt authorities; completes a real source supersession before pruning
the derived body members; and transitions the receipt's genuine generation to
retired. Intended failures were:

```text
slice55_integrity_plan_hook_covers_dense_generation_and_receipts
missing production candidate query for _fathomdb_projection_terminal

slice55_completed_closure_excludes_pruned_body_members
completed closure was treated as retained authority
findings: [NodeBodyFtsMissing, NodeBodyFtsV2Missing]

slice55_receipt_rejects_retired_generation_even_with_historical_boundary
left: 0
right: 1
```

The prior plan oracle's candidate-query count was mechanically updated from 7
to the approved 12-query shape: the existing seven synchronous statements plus
three dense member scans, joined current-generation authority, and receipt
guard scan. No query-plan, index, after-key, or no-temporary-sort assertion was
changed. This HITL-authorized test-only correction was committed separately
while production remained unstaged.

The retired-generation test initially used a test-only kind-enrolment helper,
which does not transition generation authority. The HITL authorized replacing
only that setup call with public `configure_projections`, which performs a real
generation transition. The receipt and expected corruption assertions were
unchanged; the test-and-chronology correction was committed separately while
production remained unstaged.

The exact second RED commit is
`ca032c098d0121369a36d2f46bdfb60faba90005`; authorized test-only corrections
are `4720f8ab9e471de61920032a94ae2b803158243d` and
`7381279b2c542725364f9aea07aa351e15d223b2`. GREEN uses the shared closure
classifier for synchronous owners, charges dense authority candidates before
classification, scans the three physical tuple components with bounded
after-key production SQL, joins current generation authority, rejects retired
receipt generations, and exposes those exact statements to the plan hook.

The first aggregate run exposed three residuals: generation candidates omitted
all-missing enrolled owners, the vec0 virtual table requested an unsupported
ordered rowid scan that produced `USE TEMP B-TREE`, and non-dense sparse owners
were unnecessarily charged. After the required retry-boundary rethink, GREEN
adds bounded enrolled-owner candidates, skips owner enumeration when no dense
authority exists, and uses vec0's constrained rowid traversal without a
temporary sort. The unchanged aggregate and lint target pass:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 43 passed; 0 failed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- -D warnings
Finished `dev` profile
```

### FIX-4 normalized trace-chain authorization

The trace test-only RED constructs a lifecycle- and filter-visible derived
endpoint whose normalized source-link hash is invalid and whose selected
relation carries a BLOB `dependency_id`. Forward tracing originally disclosed
the corrupt relation as `Storage`; reverse tracing had to remain
nondisclosing. The exact RED commit is
`002b40a595378dda857145170ef092ab0e94792d`.

GREEN `a0828528e416121c45bf2ed4e671ac9fc8582e4a` selects only guarded
`(rowid, endpoint)` keys during candidate enumeration. Before decoding any
selected relation field, counting an eligible edge, or applying `LIMIT`, it
proves the canonical self link, source version, exact SHA-256 source link,
endpoint visibility and role, and closure chain. Root preflight applies the
same proof before lifecycle and filter decoding. After-key paging and the
indexed candidate query plans are unchanged.

The first implementation attempt still reached a negative cursor through the
visibility helper before rejecting the invalid hash. Reordering the complete
chain proof fixed forward tracing. The next focused run exposed the distinct
reverse root-preflight ordering defect; moving root chain proof before
visibility made the reverse result nondisclosing. The retry cap was not
exceeded. The focused and aggregate checks then passed:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_invalid_chain_precedes_relation_blob_decode_and_cap \
  -- --exact --nocapture
test result: ok. 1 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace
test result: ok. 20 passed; 0 failed; 1 ignored

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace -- -D warnings
Finished `dev` profile
```

### FIX-4 strict candidate-native SDK explanations

Test-only RED `4273669eb212eae05cc5669b5794c211f94b8cf8` requires both SDKs to
reject candidate-native explanations unless query-level integer and finite
ranged fields are valid, correlation is nonempty and canonical, structural
explanation is present on every hit, `perHit` and results have coherent counts,
positions, and identities, result arms are known, and arm, blended score, and
candidate-exact scores agree. Existing simulated old-native tests continue to
permit absent new fields and apply their legacy defaults.

The exact RED wheel produced the intended ten new Python failures while six
legacy compatibility cases passed. TypeScript failed at the new exported
candidate mapping seam. No production file was part of this commit.

```text
wheel=/tmp/fathomdb-s55-fix4-red.CRPnU9/dist/\
fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
sha256=0ebd6619b1cc8c53700c40a70dad922065b0d2a79f6d2c0e06422844885a5c4c
Python: 6 legacy passed; 10 intended strict candidate cases failed
TypeScript: missing exported mapNativeSearchResult
```

The fixture-only typing commit
`0a85d75383b75f6c0fd35d04c3f2ca49eb6469de` initially cast the deliberately
malformed frozen request to its intended structural type. Exact N-API runtime
testing later showed that the fixture still failed at an earlier top-level
schema check and had never exercised its asserted nested frozen boundary. At
the retry boundary the `ReadContext` contract was reread. Test-only RED
`a798ed646e24f3097ee98286f32e488e2d74e7d6` supplies `view: {}` so the unchanged
assertion reaches the malformed nested context and genuinely fails with
`DependencyTraceError` instead of the required `FrozenReadError`.

Python and TypeScript explanation GREEN
`6f3e451d2ea2924f5b91626dcc9b75bcca96813b` centralizes strict mapping for
candidate-native search results while leaving direct legacy per-hit mapping
permissive for absent additions. Frozen-boundary GREEN
`ff1d43f12f0be720eb8709f804365d8374e3efa2` maps nested frozen request schema
failures to `FrozenReadError`. The corrected focused TypeScript fixture passes
all eight cases.

### FIX-4 closing verification

The closing focused matrix at exact product commit
`ff1d43f12f0be720eb8709f804365d8374e3efa2` passed:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_explanation --test slice55_wire \
  --test slice55_dependency_trace --test slice55_data_plane_integrity
integrity: 43 passed; trace: 20 passed, 1 ignored;
explanation: 17 passed; wire: 8 passed

cargo clippy -p fathomdb-engine --features operator,test-hooks \
  --test slice55_explanation --test slice55_wire \
  --test slice55_dependency_trace --test slice55_data_plane_integrity \
  -- -D warnings
Finished `dev` profile

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 2 passed; 0 failed
cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --test check_integrity
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --test trace_source_ref
test result: ok. 3 passed; 0 failed

cargo test --release -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact --nocapture
hidden_rows=50000 vm_steps=3200000 elapsed_ms=51 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed

./scripts/agent-lint.sh
exit 0
./scripts/agent-typecheck.sh
exit 0
```

The disposable wheel was built and verified from that exact product commit;
version `0.8.24` remains the expected pre-release package version. No artifact
was staged or published, and the stale worktree `_fathomdb.abi3.so` was not
mutated.

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix4-final.ES9qCH/dist \
  --venv-dir /tmp/fathomdb-s55-fix4-final.ES9qCH/venv
wheel smoke: ok
23b216e4a4127b3ea1a9052c921c83e47626a708b1aafc5daebb614ce08c11ab  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix4-final.ES9qCH/venv/lib/python3.12/\
  site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix4-final.ES9qCH/venv/lib/python3.12/\
  site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix4-final.ES9qCH/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok

PYTHONPATH=/tmp/fathomdb-s55-fix4-final.ES9qCH/venv/lib/python3.12/\
site-packages:/home/coreyt/projects/fathomdb/.venv/lib/python3.12/site-packages \
  /usr/bin/python3 -P -m pytest -o pythonpath= \
  /tmp/fathomdb-s55-fix4-final.ES9qCH/test_slice55_trace_explanation.py \
  /tmp/fathomdb-s55-fix4-final.ES9qCH/test_slice55_wrapper_compat.py -q
36 passed
```

The exact-source N-API build initially hit the known restricted-sandbox
`spawnSync /bin/sh EPERM`; one unchanged unconfined retry passed:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 65; pass 65; fail 0
```

That test creates `src/ts/slice55-malformed-frozen-context` and its lock file.
Both were confirmed generated by the exact test invocation and removed after
the run; they were never staged.

## Implementation review FIX-5 chronology

Review cycle 5 was recorded verbatim at
`917dddc2` before any FIX-5 test or production change. It identified remaining
integrity work-accounting, authority-classification, ordering, identifier,
request-precedence, receipt-generation, SDK-compatibility, and frozen-context
gaps. It also identified two tests whose expected behavior contradicted the
READY design.

### FIX-5 review-authorized oracle corrections

Test-only commit `2173e324` made the two narrow, design-authorized oracle
corrections while production remained unchanged:

- duplicate operation IDs precede invalid integrity limits;
- a valid receipt for the expected but retired generation maps to
  `MutationReadinessUnavailable`, not `MutationReceiptCorrupt`.

Both corrected tests were genuine REDs against the cycle-4 product:

```text
slice55_duplicate_operation_id_precedes_invalid_limits
  left: IntegrityLimitInvalid
 right: DuplicateCheck

slice55_receipt_rejects_retired_generation_even_with_historical_boundary
  left: MutationReceiptCorrupt
 right: MutationReadinessUnavailable
```

Test-only commit `2cbca0d5` corrected the cycle-4 wrapper-compatibility oracle.
The real Python and TypeScript full-result wrappers must accept absence of the
additive explanation fields, just as the direct per-hit legacy seams do. When
the fields are present, recursive validation remains strict. Candidate-native
presence is asserted at the raw Rust/Python and Rust/N-API boundaries; it is
not inferred by making the compatibility wrapper reject old-native shapes.

Test-only commit `1e20cba7` updated one aggregate work-count expectation from
three to four and enlarged one unrelated sparse fixture's budget. The physical
row itself and each authority input are distinct bounded work. This correction
did not remove the adversarial small-bound expectations.

These corrections supersede the cycle-4 chronology's overbroad statement that
the candidate full-result mappers require every additive explanation field.
The accurate contract is legacy absence compatibility plus strict validation
when present, with raw-native candidate presence proved separately.

### FIX-5 adversarial REDs

Test-only commit `54b4014f` added public-boundary and full-mapper coverage for:

- aggregate work across physical and authority inputs;
- closure residue whose owner remains present outside membership;
- stable physical rowid ordering rather than lexical owner ordering;
- omission of a malformed optional revision identifier;
- absent additive fields through the Python and TypeScript full-result
  mappers; and
- complete malformed frozen context, token, view, and eligibility shapes with
  exact nested paths in both SDKs.

The four focused Rust integrity tests failed against the cycle-4 product as
intended:

```text
slice55_integrity_work_limit_is_aggregate_across_physical_sources
  returned Ok with checked_count=3 instead of IntegrityBoundExceeded

slice55_completed_closure_residue_owner_is_outside_membership
  classified the present owner as missing

slice55_physical_body_findings_follow_rowid_not_owner_name
  left: [50, 100]
 right: [100, 50]

slice55_malformed_optional_revision_is_omitted
  returned revision_id: Some("!")
```

Test-only commit `5c3d1496` added a separate public integrity RED with two
excluded non-scalar attribute owners. At `max_work_items=11`, the cycle-4
product returned `Ok` with `checked_count=11`; the required result was
`IntegrityBoundExceeded`. The RED was reproduced at the exact commit in a
disposable detached worktree, then that worktree was removed.

The SDK compatibility REDs were also reproduced from exact commit `5c3d1496`
through the actual built artifacts. The installed wheel ran 16 wrapper tests:
15 passed and the new full-result compatibility case failed at
`/correlationId`. The real N-API build ran the corresponding five-test file:
four passed and the same case failed with
`FathomDbError: invalid explanation response at /correlationId`. These are
full public mapper boundaries, not source-text proxies.

### FIX-5 GREEN

Product commit `f8eeb619` implements the review requirements:

- duplicate detection now precedes integrity-limit validation;
- every physical row and authority input consumes the same aggregate
  `remaining + 1` work budget before reconciliation or deduplication;
- excluded and non-scalar attribute owners are bounded before classification;
- shared lifecycle, supersession, validity, closure, and governed-membership
  classifiers are used for physical residue and authority-missing directions;
- owner-present outside membership is distinct from owner-missing;
- physical body and attribute candidate rowids survive reconciliation and
  drive the normative final order;
- every optional identifier passes through the same validity-and-privacy
  filter before emission;
- receipt verification reuses the Slice-40 receipt-aware point classifier with
  the operation ID and expected generation; a coherent receipt for a retired
  generation reaches readiness-unavailable instead of corruption;
- Python and TypeScript full-result wrappers accept absent additive explanation
  fields while validating every present field; and
- both SDKs translate malformed frozen containers, token, view, and eligibility
  data to `FrozenReadError` with exact nested paths.

An exact installed-wheel run then exposed one preserved-precedence regression:
a top-level null frozen context was reaching nested frozen validation before
the established trace refusal. Product commit `e3366d01` restores the
top-level `DependencyTraceError(trace_corrupt, /context)` boundary without
relaxing nested frozen validation.

Refactor commit `055c2a12` removes the new dynamic dense-owner statement and
reuses the exact production candidate-query constants already covered by the
plan hook. This keeps the dynamic execution plans identical to the plans
asserted for bounded after-key scans.

The focused GREEN matrix at `055c2a12` passed:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 48 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace
test result: ok. 20 passed; 0 failed; 1 ignored

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_explanation --test slice55_wire
explanation: 17 passed; wire: 8 passed

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 2 passed; 0 failed
cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --test check_integrity
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --test trace_source_ref
test result: ok. 3 passed; 0 failed
```

The initial FIX-5 exact wheel at `e3366d01` passed the verifier, the 43 copied
Slice-55 Python tests, and the installed native smoke. Because the later
integrity plan refactor changes Rust source, closing artifact evidence is
recorded only after rebuilding from the final product commit.

## Implementation review FIX-6 chronology

Review cycle 6 was persisted before test or production changes in docs-only
commit `5b6e8787b557a21433304a8de6903ec31b681fac`. It records the independent
`FAIL` against `b754e8fe3703645edb242e9baee5b5792089a58e`: dormant expired-edge
artifacts were misclassified, retained revisions stood in for canonical-owner
existence, malformed SQLite storage classes escaped as `Storage`, null receipt
keys were invisible, and the corresponding real-database coverage was absent.

### FIX-6 RED and authorized oracle correction

Test-only commit `2807baa55ac30b57227e31629ce26a6a79973f10` added ten real-database
fixtures. Against the cycle-5 product, nine failed and the existing dense
state-classifier regression remained green. The failures reproduced:

- retained expired-edge body FTS and dense state reported as outside
  membership or corrupt;
- a deleted canonical owner with a retained revision reported as outside
  membership instead of owner-missing;
- malformed dependency row, normalized source-link, dependency singleton,
  generation authority, body/attribute FTS, and dense storage classes escaping
  as `EngineError::Storage`;
- a malformed dense cursor escaping in both integrity directions; and
- a null receipt operation ID being skipped, so the cap did not fail.

The exact RED command was:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity slice55_fix6_ -- --nocapture
test result: FAILED. 1 passed; 9 failed; 48 filtered out
```

The first production run made nine of ten new cases green. The remaining test
expected every malformed dependency row to omit `dependency_id`, but the READY
matrix in `design.md` lines 470-472 requires a grammar-valid dependency ID to
remain present when another dependency-row field is malformed. With explicit
review authorization, isolated test-only commit
`0c01cefef03c924cf707c992fc635f94bfdf7c3b` parameterized only that presence:
valid IDs remain for malformed schema, derived revision, or generation, while
malformed/null IDs remain omitted. Finding kind, revision privacy, and product
code were unchanged in that commit.

### FIX-6 GREEN

Product commit `06aa3b89b7b555716066cf798b99abcbf2e3abd7` implements the review
requirements:

- one owner-retention classifier distinguishes required membership,
  historical/dormant retention, governed-pruning residue, and absent canonical
  owners across expected and physical body, dense, and generation directions;
- dormant expired-edge FTS and complete or accepted terminal-only dense state
  remain legal, while erased, superseded, completed-closure, and otherwise
  pruning-governed residue retain the outside-membership findings;
- bounded candidate scans preserve aggregate remaining-plus-one accounting and
  indexed ordering while carrying rowid separately from guarded dynamic values;
- dependency, source-link, singleton, generation, FTS, attribute, and dense
  values are inspected through SQLite `ValueRef` metadata before decoding;
- malformed rows map to the READY typed finding and omit malformed optional
  identities; and
- receipt enumeration uses stable rowid ordering, so a null operation ID costs
  work, trips the same aggregate bound, and emits private
  `mutation_receipt_corrupt` rather than disappearing.

The first complete 58-test run exposed two production-only regressions without
changing tests: a live node whose kind had become noncommittable reached source
type resolution instead of governed-pruning residue, and explicit vec0 rowid
ordering introduced a forbidden temporary B-tree. The classifier now treats
that node as governed residue, and the vec0 rowid range scan relies on its
indexed iteration without the redundant sort. The complete target then passed.

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity
test result: ok. 58 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace
test result: ok. 20 passed; 0 failed; 1 ignored

cargo test -p fathomdb-engine --features test-hooks --test slice55_explanation
test result: ok. 17 passed; 0 failed
cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_wire
test result: ok. 8 passed; 0 failed

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 2 passed; 0 failed
cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --features operator,test-hooks --test check_integrity
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --features operator,test-hooks --test trace_source_ref
test result: ok. 3 passed; 0 failed

cargo clippy -p fathomdb-engine --all-targets \
  --features operator,test-hooks -- -D warnings
exit 0
```

The preregistered release trace ceiling remained green:

```text
hidden_rows=50000 vm_steps=3200000 elapsed_ms=51 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed
```

### FIX-6 exact artifact evidence

The source-wrapper pytest command was attempted first and rejected the stale
worktree `_fathomdb.abi3.so` during collection because it predates the current
native API. That is not candidate evidence and the stale module was not
modified. A fresh disposable wheel was therefore built from exact clean
product commit `06aa3b89b7b555716066cf798b99abcbf2e3abd7`:

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix6-final.b2TRJ4/dist \
  --venv-dir /tmp/fathomdb-s55-fix6-final.b2TRJ4/venv
wheel smoke: ok
f940b60b357c926cb9ec5add82408d5631c86a2a43ff4e15c0d777e339f6392b  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix6-final.b2TRJ4/venv/lib/python3.12/\
  site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix6-final.b2TRJ4/venv/lib/python3.12/\
  site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix6-final.b2TRJ4/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

Byte-identical copies of both checked-in Slice 55 Python modules were run with
the candidate site-packages first and repository source excluded. Their
SHA-256 pairs matched (`1341189e...40a3` and `1463f7f5...56dc`), and all 43
parameterized cases passed in 0.62 seconds.

The exact-source N-API build first encountered the expected restricted-sandbox
`spawnSync /bin/sh EPERM`; the unchanged unconfined retry passed. TypeScript
checking and the Slice 55-filtered Node run also passed:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 65; pass 65; fail 0
```

The Node run's two named disposable database files were removed afterward and
were never staged. These are focused writer checks only; the independent
implementation reviewer owns the broader acceptance verdict.

## 2026-09-06 — FIX-7: full signed rowids and normalized-chain matrix

Independent implementation review cycle 7 examined clean candidate
`e590fd4a93e30ef89350a4c65499d57d0c6d05c5` and product commit
`06aa3b89b7b555716066cf798b99abcbf2e3abd7`. Its complete FAIL was persisted
unchanged in `implementation-review-cycle7.md` and committed alone as
`edf39567a1b958cc60a51449b4a11815d2076689` before any test or production
change.

### FIX-7 RED and fixture reconciliation

Test-only commit `103204612b32fa55309a400b210aeedf9b4b7283` added real-database
coverage for the cycle-7 findings:

- the full signed SQLite rowid domain, including `i64::MIN` and `-1`, across
  physical body FTS, attribute/property, dense terminal/sidecar, generation,
  and receipt checks, with exact aggregate bounds and stable finding order;
- all ten READY dependency-chain classifications, including BLOB owner roles,
  generation zero, exact severities, and each finding's minimum revision-ID
  set;
- the nontrivial proptest `slice55_normalized_chain_round_trip`, which writes
  randomized valid source/derived identities to a real database, registers and
  reads the normalized dependency through both public lookup directions, then
  checks the clean, BLOB-derived-role, and zero-generation integrity outcomes;
  and
- semantic consumption of the checked-in dependency, projection, and
  explanation fixtures.

The fixtures were reconciled without edits. The dependency fixture's schema
and declared-code subset are checked against the live ten-case matrix; the
projection fixture's schema and exact required-code list are checked; and the
explanation fixture's JSON-pointer values are checked before the real explained
search result is compared with the corresponding structural enums. None was
stale against READY.

The first complete real-database RED was genuine:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- --test-threads=1
test result: FAILED. 54 passed; 8 failed
```

The eight failures were the signed dense/generation scan, dependency fault
matrix, source-only minimum-ID contract, dense/generation/receipt plan hook,
signed receipt scan, normalized-chain proptest, signed physical-member scan,
and indexed-plan test. The physical scan observed zero work for negative rows;
the BLOB-derived-role matrix case and the proptest minimal case
`suffix = "0", fault = 1` escaped as generic `Storage`; generation zero and
source-side IDs had the old classifications; and the plan hooks still exposed
the lossy `rowid>?1` start. Proptest failure persistence was disabled for this
test, and its generated local regression file was removed rather than promoted
to an oracle.

The dense projection-generation RED initially encoded five work units. READY
states that projection generation costs one current-generation singleton, one
current generation record, and one unit per physical member. This fixture has
four physical rows, so its exact total is six. The release owner authorized
only the `5` to `6` correction; test-only commit
`6ebc6bbbc6f3c836cb9a8705cef2b82dbc3aeb84` contains that one-line change and
had no production change staged. No other assertion or fixture changed.

### FIX-7 GREEN

Product commit `f70cc5c31c3294a53d5dfa0f6b39a7e94280feb8` implements the
cycle-7 requirements:

- every physical rowid range starts inclusively at `i64::MIN`; ordinary and
  FTS tables retain indexed rowid traversal, and the bounded vec0 scan avoids
  an `ORDER BY` that SQLite would implement with a temporary B-tree while the
  successful at-cap result is canonicalized by the existing ordered set;
- dense candidates retain rowid and cursor separately, so negative or
  dynamically malformed identities are counted and classified without lossy
  conversion;
- artifact owners, canonical nodes, and canonical self-links inspect stored
  values through SQLite `ValueRef` guards before decoding; all remaining typed
  `row.get` calls in the dependency path read SQL-generated `EXISTS` booleans,
  not variable stored fields;
- registered generation zero maps to
  `dependency_generation_mismatch`/critical; and
- findings attach exactly the READY minimum IDs: derived-side findings use the
  derived revision, source-side findings use the valid source revision,
  link-mismatch uses both, and generation findings use neither.

The exact focused and nonregression results were:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- --test-threads=1
test result: ok. 62 passed; 0 failed

cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_dependency_trace
test result: ok. 20 passed; 0 failed; 1 ignored
cargo test -p fathomdb-engine --features test-hooks --test slice55_explanation
test result: ok. 17 passed; 0 failed
cargo test -p fathomdb-engine --features operator,test-hooks --test slice55_wire
test result: ok. 8 passed; 0 failed

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 2 passed; 0 failed
cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 3 passed; 0 failed
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test check_integrity --test trace_source_ref
test result: ok. 3 passed; 0 failed
test result: ok. 3 passed; 0 failed

cargo clippy -p fathomdb-engine --all-targets \
  --features operator,test-hooks -- -D warnings
exit 0
```

The preregistered release trace ceiling remained green:

```text
hidden_rows=50000 vm_steps=3200000 elapsed_ms=51 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed
```

### FIX-7 exact artifact evidence

As in FIX-6, the documented source-wrapper pytest first rejected the stale
worktree `_fathomdb.abi3.so`, which predates the native API and is not candidate
evidence. Ruff passed, and Pyright passed from `src/python`, where its declared
venv and extra paths resolve:

```text
.venv/bin/ruff check src/python/fathomdb \
  src/python/tests/test_slice55_trace_explanation.py
All checks passed!

cd src/python
../../.venv/bin/pyright fathomdb tests/test_slice55_trace_explanation.py
0 errors, 0 warnings, 0 informations
```

A fresh disposable wheel was built from exact clean product commit
`f70cc5c31c3294a53d5dfa0f6b39a7e94280feb8`:

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix7-wheel.Jr9VCI/dist \
  --venv-dir /tmp/fathomdb-s55-fix7-wheel.Jr9VCI/venv
wheel smoke: ok
f0d99b68c8df6bfeb72a11f187c425eb4e5de3b544d550e52426ef67b6024763  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix7-wheel.Jr9VCI/venv/lib/python3.12/\
  site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix7-wheel.Jr9VCI/venv/lib/python3.12/\
  site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix7-wheel.Jr9VCI/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

Byte-identical copies of the two Slice 55 Python test modules were run from the
named wheel directory with candidate site-packages first and repository source
excluded. The SHA-256 pairs matched
`1463f7f5591eb6280ed4014c7595e8c2d7acd09934159815943e4bf4b97256dc`
and `1341189e548bd027c929d45c9d3a3af26c757318ab3c1db04cb6fa94b64640a3`;
all 43 parameterized cases passed in 0.64 seconds.

The exact-source N-API build first encountered restricted-sandbox
`spawnSync /bin/sh EPERM`; the unchanged unconfined retry passed. TypeScript
checking and the Slice 55-filtered Node run also passed:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 65; pass 65; fail 0
```

The two named disposable Node database files were removed afterward and were
never staged. These are focused FIX-7 writer checks; independent implementation
review and the broader verifier remain separate gates.

## 2026-09-06 — FIX-8: exact generation work and dependency corruption

Independent implementation review cycle 8 examined clean candidate
`31bef1768df6af62080ef108c32d605371d0f6b3` and product commit
`f70cc5c31c3294a53d5dfa0f6b39a7e94280feb8`. Its complete FAIL was persisted
in `implementation-review-cycle8.md` and committed alone as
`22c6c4bcd77db7954be9c63eb5ad30b1965f2e2b` before any test or production
change.

### FIX-8 authorized oracle correction and RED

READY design lines 397-399 charge exactly the current-generation singleton,
the current generation record, and each physical member. The release owner
authorized only two existing projection-generation expectations to stop
charging canonical owners: the corrupt-member fixture changed from four to
three work units, and the canonical-owners-only fixture changed from a bound
error to success with `checked_count=2`. Test-only commit
`68b3a2eec107c1c7351a834804409eace533d10b` contains only those mechanical
corrections.

Test-only commit `e4e62917d3cd008fb1887b69f7d552eb7c0f10e4` then added real-database
coverage for the remaining cycle-8 requirements:

- canonical owners cannot consume projection-generation work or invent a
  missing physical-member candidate;
- schema-version-2 derived and canonical source owners map to the exact READY
  role findings, severities, and minimum IDs;
- a derived source self-reference maps to
  `dependency_derived_role_invalid`/error before source-link mismatch and emits
  exactly one derived revision ID; and
- canonical singleton text at `i64::MAX + 1` and `u64::MAX` maps to the global
  `dependency_generation_mismatch`/critical finding and never becomes the read
  boundary.

Against the unchanged cycle-7 product, the complete integrity target produced
exactly the expected six failures: the two corrected work-count tests and the
four new regressions.

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity -- --test-threads=1
test result: FAILED. 60 passed; 6 failed
```

The failures were `slice55_projection_generation_enumerates_and_attributes_corrupt_members`,
`slice55_projection_generation_member_cap_is_all_or_error`,
`slice55_canonical_owners_do_not_become_projection_generation_candidates`,
`slice55_owner_schema_versions_follow_the_dependency_role_matrix`,
`slice55_derived_self_reference_precedes_source_link_mismatch`, and
`slice55_dependency_generation_rejects_canonical_u64_above_i64_max`.

### FIX-8 GREEN

Product commit `87f6c707c761e0693c739241e1b97fd173d15df8` implements only the reviewed
corrections:

- projection-generation enumeration now uses only its two generation
  authorities and bounded physical dense-member rows; it no longer enumerates,
  merges, or charges canonical owners;
- `StoredArtifactOwner` loads `schema_version` through the existing
  corruption-safe `ValueRef` path and validates it in both the derived and
  canonical-source role branches;
- derived source self-reference classification precedes the general
  source-link mismatch and therefore uses the READY error severity and minimum
  derived-side ID set; and
- dependency-generation singleton text is constrained to
  `0..=i64::MAX` before it can populate the read boundary.

Focused integrity, query-plan, property, fixture, trace, explanation, wire,
legacy, facade, and CLI verification passed:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity --test slice55_dependency_trace \
  --test slice55_explanation --test slice55_wire --test check_integrity \
  --test trace_source_ref -- --test-threads=1
integrity: 66 passed
dependency trace: 20 passed; 1 ignored
explanation: 17 passed
wire: 8 passed
legacy check_integrity: 3 passed
legacy trace_source_ref: 3 passed

cargo test -p fathomdb --test slice55_governed_surface
test result: ok. 2 passed; 0 failed

cargo test -p fathomdb-cli --test slice55_data_plane_integrity_cli
test result: ok. 3 passed; 0 failed

cargo clippy -p fathomdb-engine --all-targets \
  --features operator,test-hooks -- -D warnings
exit 0
```

The preregistered release trace ceiling also remained green:

```text
cargo test --release -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact --nocapture
hidden_rows=50000 vm_steps=3200000 elapsed_ms=51 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed
```

### FIX-8 exact artifact evidence

A fresh disposable wheel was built from exact clean product commit
`87f6c707c761e0693c739241e1b97fd173d15df8` with no `PYTHONPATH`:

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix8-wheel.G2wwHT/dist \
  --venv-dir /tmp/fathomdb-s55-fix8-wheel.G2wwHT/venv
wheel smoke: ok
e542899d35383ac936f4da242076013a4df785afc6799d298a85d58ad0eca1f9  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix8-wheel.G2wwHT/venv/lib/python3.12/\
  site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix8-wheel.G2wwHT/venv/lib/python3.12/\
  site-packages/fathomdb/_fathomdb.abi3.so

env -u PYTHONPATH /tmp/fathomdb-s55-fix8-wheel.G2wwHT/venv/bin/python \
  src/python/tests/smoke_slice55_installed.py
slice55 installed native smoke: ok
```

The plan's root-level `npm run build:debug --workspace fathomdb` invocation
first exited with `No workspaces found`; the package is not declared as a root
npm workspace. The established exact-source package-directory route then
passed without changing source:

```text
cd src/ts
npm run build:debug
./node_modules/.bin/tsc -p tsconfig.json
node --test --test-name-pattern slice55 dist/tests/*.test.js
tests 65; pass 65; fail 0
f6a2214706aee4b3763c0d9cfd9b6d34076b014389dfcf63f9918509caad1241  \
  fathomdb.linux-x64-gnu.node
```

The N-API run generated only
`src/ts/slice55-malformed-frozen-context{,.lock}`; both disposable files were
removed and were never staged. Available disk remained 171 GiB and `target/`
remained at the 8.0-GiB cleanup target. These are focused FIX-8 writer checks,
not a broad repository-gate or independent-review verdict.

## 2026-09-06 — FIX-9: trace generation codec boundary

Independent implementation review cycle 9 and its owner reproduction showed
that every strict trace codec accepted edge generation zero or an edge
generation above the decoded dependency-generation boundary. The reproduction
also showed that the canonical trace fixture itself encoded edge generation 1
with boundary 0. The owner authorized the first of two additional review-driven
FIX cycles and bounded it to codec validation, the fixture, exact tests, the
invalid TypeScript plan command, and evidence.

### Authorized fixture correction and RED

Test-only commit
`6a4867e38a499632988b38427ef74e007a8d41bc` changes only the canonical
fixture boundary and its mechanically dependent Rust value from 0 to 1. The
fixture remains one-line canonical JSON without a trailing newline; its
SHA-256 is
`7553ca038fae62b397db385ef818044c535d394645d7b4caa33be50653700ad3`.
The unchanged canonical-byte test passed.

Test-only RED commit
`fb638e3c471a2e686c4e43e3528f26812191f368` adds Rust encode and decode
refusals for zero and above-boundary generations, an invalid second-edge check,
a valid two-edge decode, and property-generated valid
`1 <= registered generation <= boundary` round trips. It adds the same zero
and above-boundary decoder refusals to Python and TypeScript at the exact path
`/dependencyEdges/0/registeredDependencyGeneration`.

Against unchanged production, the Rust target returned successful encode or
decode values instead of errors in all three new refusal paths:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_wire -- --nocapture
test result: FAILED. 7 passed; 3 failed
```

The installed FIX-8 Python package accepted both invalid values:

```text
pytest -o pythonpath= test_slice55_trace_explanation.py -q \
  -k edge_generation_outside_read_boundary
FAILED ...[0] - Failed: DID NOT RAISE DependencyTraceError
FAILED ...[2] - Failed: DID NOT RAISE DependencyTraceError
2 failed; 28 deselected
```

The directly compiled TypeScript target retained 15 passing cases and failed
the new strict mapper case with `Missing expected exception`.

### Production GREEN and exact public-type corrections

Product commit `6d69eb2ec386d07794b3ce59824adf2281147242` validates every Rust edge
before emitting canonical bytes and again after the decoder reads the complete
boundary. Python and TypeScript likewise validate every edge only after their
boundary decoders complete. All three use `trace_corrupt` and the indexed edge
generation path. No integrity scanner, trace query, schema, or public shape
changed.

The first fresh wheel run then exposed that Python's already-established public
u64 mapping is a canonical decimal string. The initial comparison therefore
rejected `"2"` but not `"0"`, and the new valid-case assertions incorrectly
expected integer values. With explicit owner authorization, isolated test-only
commits `8d9b38515e07b29e93db56c391cfa5cccb899db4` and
`a434a9f935e47b6100949af0aa87c829917db9ee` change only the edge and boundary
valid-case expectations from integer `1` to canonical string `"1"`. Every
zero/ahead refusal assertion and exact path remains unchanged. Product commit
`80ac13636c92aadb7b6bbf09914d2bbf42894f6e` compares numeric values derived
from those already-canonical strings without changing their public types.

Documentation-only commit
`9794467be4b7e177c7951b410bc1684db104ae11` replaces the invalid root npm
workspace command in `plan.md` with the established package-local route:

```text
cd src/ts
npm run build:debug
node --test --test-name-pattern slice55 dist/tests/*.test.js
```

### Focused and nonregression evidence

The complete focused Rust matrix passed serially:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice55_data_plane_integrity --test slice55_dependency_trace \
  --test slice55_explanation --test slice55_wire --test check_integrity \
  --test trace_source_ref -- --test-threads=1
integrity: 66 passed
dependency trace: 20 passed; 1 ignored
explanation: 17 passed
wire: 10 passed
legacy check_integrity: 3 passed
legacy trace_source_ref: 3 passed
```

The facade and CLI targets passed 2 and 3 tests respectively. Touched-engine
Clippy passed with `operator,test-hooks` and all targets. The unchanged
preregistered release ceiling also passed:

```text
cargo test --release -p fathomdb-engine --features test-hooks \
  --test slice55_dependency_trace \
  slice55_trace_hidden_dependents_performance_ceiling \
  -- --ignored --exact --nocapture
hidden_rows=50000 vm_steps=3200000 elapsed_ms=53 peak_rss_delta_bytes=0
test result: ok. 1 passed; 0 failed
```

The exact fresh wheel was built from clean final product commit
`80ac13636c92aadb7b6bbf09914d2bbf42894f6e` without `PYTHONPATH`:

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix9-wheel-final.lekElg/dist \
  --venv-dir /tmp/fathomdb-s55-fix9-wheel-final.lekElg/venv
wheel smoke: ok
349ade9d33ea5e9e29f3405d758f004d201ee07a9a97c17f78bd1ee9a1503acf  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
module=/tmp/fathomdb-s55-fix9-wheel-final.lekElg/venv/lib/python3.12/\
  site-packages/fathomdb/__init__.py
native=/tmp/fathomdb-s55-fix9-wheel-final.lekElg/venv/lib/python3.12/\
  site-packages/fathomdb/_fathomdb.abi3.so
```

The installed smoke passed. Byte-identical copies of the two checked-in Slice
55 Python modules ran with the candidate site-packages first and repository
source excluded. Their final source/copy SHA-256 values matched
`51104ee26a9bd4a04e6f5b0d6cffed7274b230b874d421f59d0173370a386fae`
and `1463f7f5591eb6280ed4014c7595e8c2d7acd09934159815943e4bf4b97256dc`;
all 46 tests passed in 0.66 seconds.

The exact-source N-API build first hit restricted-sandbox
`spawnSync /bin/sh EPERM`; the unchanged authorized unconfined retry passed.
Package-local TypeScript compilation and the plan-corrected Slice 55 test route
passed 65 tests. The native artifact SHA-256 is
`9345716ec6def0d933c1d22db5bd3a51d715b32c2ed2d6c6965cb71776069267`.
The run generated only `src/ts/slice55-malformed-frozen-context{,.lock}`; both
files were removed and never staged.

Available disk remained 170 GiB and `target/` remained 7.9 GiB. These are
focused FIX-9 writer checks, not a broad repository-gate or independent-review
verdict. No package was staged, tagged, uploaded, or published.

## 2026-09-06 — FIX-10: bounded Python trace u64 decoding

Independent implementation review cycle 10 found that Python's strict
dependency-trace decoder passed an arbitrary-length canonical decimal string
to `int()`. Python 3.12's 4,300-digit conversion limit therefore leaked a raw
`ValueError` instead of the stable native `DependencyTraceError` contract. The
owner authorized one final remediation and re-review cycle, bounded to this
decoder defect.

### RED

Test-only commit `a35b0efc116b39dc7260f79d5717f263b0d75e9a` adds a
strict-decoder boundary test for all three response fields that share the u64
helper:

- `/dependencyEdges/0/registeredDependencyGeneration`;
- `/readBoundary/observedWriteBoundary`; and
- `/readBoundary/dependencyGeneration`.

Each case supplies a syntactically canonical 5,000-digit decimal string and
requires `DependencyTraceError(reason="trace_corrupt", field_path=<path>)`.
The first worktree-source invocation encountered the known stale native-module
binding before collection and was not used as RED evidence. A byte copy of the
committed test was instead run with the exact FIX-9 wheel's site-packages first
and repository source excluded. The wheel SHA-256 was
`349ade9d33ea5e9e29f3405d758f004d201ee07a9a97c17f78bd1ee9a1503acf`.

```text
env PYTHONPATH=<fix9-site-packages> .venv/bin/python -m pytest \
  -o pythonpath= <copied-test> -q \
  -k overlong_u64_never_leaks_decoder_errors
FFF
E ValueError: Exceeds the limit (4300 digits) for integer string conversion: value has 5000 digits; use sys.set_int_max_str_digits() to increase the limit
3 failed, 30 deselected
```

All three paths leaked the same raw conversion exception.

### GREEN

Product commit `5dfd267859a39112e56206503c0a4b0b84a36db6`
replaces unbounded integer conversion in `_trace_u64` with canonical decimal
width and lexical comparison against `18446744073709551615`. It does not
change the public string representation. The maximum valid u64 remains
accepted; max-plus-one, malformed spelling, and arbitrary-length strings are
rejected before any conversion. The later edge-versus-boundary integer
comparison remains safe because both inputs have already been limited to at
most 20 digits.

The exact source-overlay focused checks passed against the last installed
native artifact:

```text
pytest test_slice55_trace_explanation.py \
  -k 'overlong_u64_never_leaks_decoder_errors or \
      rejects_edge_generation_outside_read_boundary or \
      accepts_valid_trace_generation_boundary or \
      recursively_validates_trace_responses'
9 passed, 24 deselected

pytest test_slice55_wrapper_compat.py test_slice55_trace_explanation.py
49 passed

ruff check src/python/fathomdb/engine.py \
  src/python/tests/test_slice55_trace_explanation.py
All checks passed!

pyright src/python/fathomdb/engine.py
0 errors, 0 warnings, 0 informations
```

An explicit strict-decoder matrix accepted canonical u64 max simultaneously
at all three paths and rejected both `18446744073709551616` and `01` with the
typed error and exact path at every field. The complete committed Python test
module also retains its zero/ahead edge-generation refusal cases.

The bounded Rust wire nonregression remained green:

```text
timeout 600s cargo test -p fathomdb-engine \
  --features operator,test-hooks --test slice55_wire -- --test-threads=1
test result: ok. 10 passed; 0 failed
```

### Exact disposable installed-wheel evidence

A fresh wheel was built from exact clean product commit
`5dfd267859a39112e56206503c0a4b0b84a36db6` without `PYTHONPATH`:

```text
env -u PYTHONPATH ./scripts/verify-release-python-wheel.sh \
  --python /usr/bin/python3 \
  --wheel-dir /tmp/fathomdb-s55-fix10-wheel.OR7YWM/dist \
  --venv-dir /tmp/fathomdb-s55-fix10-wheel.OR7YWM/venv
wheel smoke: ok
c4d78651b4a2445d1f2b0d1b30a8b6a6d792a3b115cb9768b88497199aab6690  \
  fathomdb-0.8.24-cp310-abi3-manylinux_2_39_x86_64.whl
```

The installed native smoke passed. Byte copies of the two Slice 55 Python
test modules then ran with candidate site-packages first and repository source
excluded: 49 passed in 0.62 seconds. The installed and repository `engine.py`
files had the identical SHA-256
`d3e7353ccb8958492925af8ebb14de2e5555cfaf37929eb458e1ba920273019e`.
The explicit installed-decoder u64 matrix also passed, including valid max,
max-plus-one, malformed, and exact typed-path checks for all three shared
fields.

The host used Python 3.12.3, cargo 1.95.0, rustc 1.95.0, and Linux x86-64
7.0.0-30-generic. Available disk remained 169 GiB and `target/` was 8.7 GiB.
Disposable copies and wheels remained under `/tmp`; no release package was
staged, tagged, uploaded, or published. These are focused FIX-10 writer checks,
not a broad repository-gate or independent-review verdict.

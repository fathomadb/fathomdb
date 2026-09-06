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

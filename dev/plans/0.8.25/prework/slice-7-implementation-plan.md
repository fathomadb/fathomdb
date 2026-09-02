---
title: 0.8.25 Slice 7 — approved prework implementation plan
status: APPROVED
target_release: 0.8.25
observed_on: 2026-09-02
plan_version: 2
---

# Slice 7 — approved prework implementation plan

## Purpose

Prepare the 0.8.25 release branch for feature work by implementing only the
repository repairs approved in Slice 6. This plan consolidates related proposal
IDs into seven dependency-ordered packages. It does not implement any product
behavior assigned to Slices 10–75.

Decision authority is `seq-272` and `seq-273`. The complete owner mapping is in
[`slice-6-hitl-decisions.md`](slice-6-hitl-decisions.md). Implementation may
start only after the required independent review passes and the owner approves
this reviewed plan.

Both gates are closed: independent review passed after FIX-1/FIX-2, and the
owner approved this plan at `seq-274`. Slice 7 implementation may proceed only
within this file's scope and stop conditions.

## Post-scope alignment

The 2026-09-02 scope/design reconciliation did not change the approved Slice 7
proposal set, but it changed three implementation assumptions:

- architecture v2.1 now defines a narrow 0.8.25 profile and allocates
  multi-source, persisted-lease, rich-continuation, and experimental retrieval
  work after 0.8.25;
- Slices 65/70 are historical reviewed evidence, not active release slices, so
  maintained traceability points to the active owners and durable future-work
  records; and
- the owner's current execution instruction limits implementation review to two
  FIX-n cycles.

The package designs below incorporate those changes. No Slice 10+ product
behavior enters Slice 7. This alignment and the instruction to proceed are the
owner authority for plan version 2.

## Accepted scope

Slice 7 contains exactly P25-01, P25-04 through P25-07, P25-11 through P25-16,
P25-18 through P25-20, P25-23, and P25-24. P25-17 is a retain-all constraint,
not an implementation package. P25-25 supplies architecture constraints only;
its product implementation remains in Slices 15–60.

The following remain outside Slice 7:

- Windows CUDA, the coupled Candle/`paste` migration, Pyright, Ruff, and an
  optional evidence manifest; these are preserved in
  `dev/plans/0.8.26-draft-scope.md`;
- all product requirements, APIs, schema changes, SDK changes, migrations, and
  feature verification assigned to Slices 10–75;
- deletion, pruning, relocation, or bulk rewriting of runs, outputs,
  performance data, experiment receipts, historical plans, or ledgers;
- Dependabot re-enablement, hosted workflow dispatch, publication, tags,
  registry mutation, runner administration, or branch protection; and
- changes to the Candle revision, SQLite/sqlite-vec, ORT, Pyo3, or N-API pins.

## Requirements and acceptance criteria

| Requirement | Acceptance criteria |
| --- | --- |
| S7-R1 — Build and test evidence must originate from the durable release worktree. | S7-AC1: a fresh release-bound Rust target contains no `/tmp/fathomdb-release-0.8.25` reference in executable or dependency metadata, and the unchanged serial workspace test route passes or reports a new product failure with the stale-path cause excluded. |
| S7-R2 — Release-state tooling must represent release-branch completion generically and truthfully. | S7-AC2: any release may declare only `origin/release/<release>` with `PENDING` or `COMPLETE` main integration; wrong refs/states/drift fail and historical states without the object retain existing behavior. Until a separately authorized push makes local commits reachable from that ref, 0.8.25 keeps `COMPLETE_ON_RELEASE_BRANCH` entries and an empty `landed` set. |
| S7-R3 — Python verification must prove the current worktree's built wheel and native module. | S7-AC3: a fresh external venv installs a wheel built from this worktree, imports Python/native code only from that venv, and passes open/write/search/close without editable-install or primary-checkout leakage. |
| S7-R4 — Required Rust advisory corrections must not move protected product pins. | S7-AC4: accepted patch floors resolve, `httpmock` is 0.8.3, `async-std` is absent, existing loader tests pass unchanged in intent, and any product-pin movement or mock API incompatibility stops the package. |
| S7-R5 — Property testing must exercise real existing contracts. | S7-AC5: the trivial identity scaffolds are rejected, and bounded generated cases prove Engine/Python write-close-reopen identity plus schema migration ordering/contiguity without mocking the database. |
| S7-R6 — Maintained traceability must resolve to current authority. | S7-AC6: a repeatable checker rejects unresolved need references; NEED-026 has a concise matching need; REQ-067/AC-077 remain historical placeholders with current successor-slice pointers and no invented threshold. |
| S7-R7 — Active architecture and navigation must be singular and verifiable. | S7-AC7: architecture v2.1 incorporates the 0.8.25 form of A25-01–A25-07, identifies later-profile work explicitly, and becomes the active versioned authority; v1 remains deprecated history; stale active paths/status/comments are corrected narrowly; all runs/data remain untouched. |

## Implementation discipline

- Use the durable `release/0.8.25` worktree with one writer. Do not create a
  shared-checkout writer or edit `main`.
- For behavior or tooling, add the failing test first and preserve the RED
  diff/commit before implementation. Do not change that oracle to obtain GREEN.
- For a documentation-only correction, retain the exact stale/broken baseline
  and use the applicable contract/link/lint check; do not manufacture a code
  test.
- Keep test files read-only during each GREEN correction, except S7-05 where
  the product is the test suite itself: first make the scaffold-quality guard
  RED, then replace the trivial properties without weakening that guard.
- Run packages serially in the order below. If a stop condition fires, record
  it in the completion record and return to HITL; do not improvise a wider fix.
- After implementation, obtain an independent high-effort code review with at
  most two documented FIX-n cycles and a separate read-only verification
  subagent. Record the actual reviewer model; do not misstate an unavailable
  requested model.

## Ordered packages

### S7-01 — clean Rust build provenance

**Maps:** P25-05, P25-INFRA-04, V25-06; satisfies S7-R1/S7-AC1.

1. Reconfirm the release worktree and record the old-path scan against the
   retained target as the failing baseline.
2. Allocate a new empty target directory whose name identifies 0.8.25; do not
   reuse or mutate the contaminated target.
3. Build and test from current source with `CARGO_TARGET_DIR` bound to that
   directory.
4. Search executable and `.d`/fingerprint metadata for the removed
   `/tmp/fathomdb-release-0.8.25` path, then run the unchanged serial workspace
   suite. Do not patch tests around compile-time paths.

**Proof:** with `CARGO_TARGET_DIR=<fresh-0.8.25-target>`, run
`cargo test --workspace --all-targets --no-fail-fast -- --test-threads=1` and
`rg -a -l --fixed-strings '/tmp/fathomdb-release-0.8.25'
<fresh-0.8.25-target>`. The first must exit 0; the scan must exit 1 with no
matches. Record the target path and both results in `slice-7-completion.md`.

**Stop:** a failure reproduced after a clean compile is treated as a product or
test defect and must be diagnosed before continuing; target deletion is not a
substitute for evidence.

### S7-02 — generic release-branch completion state

**Maps:** P25-04/P25-INFRA-03; satisfies S7-R2/S7-AC2.

**Design:** extend the existing optional `completion` object rather than add a
parallel release-state mechanism. Validate `completion.ref` as
`origin/release/<state.release>`, retain `main_integration` values `PENDING` and
`COMPLETE`, verify landed commits against that ref, and preserve the existing
origin-main semantics for states with no `completion` object.

**Files:** `scripts/check-release-state-views.sh`,
`scripts/tests/test_check_release_state_views.sh`, the 0.8.25 release-state JSON,
and generated 0.8.25 plan/board regions through the writer script only.

**Symbols:** generalize `completion_facts` and its existing remote-landing/render
callers; keep the current state schema rather than adding another claim path.

**RED:** replace the current “non-0.8.23 must fail” fixture with generic-release
fixtures that initially fail: valid 0.8.25 pending completion, wrong-release
ref, invalid integration state, PENDING after main reachability, COMPLETE before
main reachability, and a legacy state without completion.

**GREEN:** remove only the 0.8.23 special case, keep strict validation, then run
`bash scripts/tests/test_check_release_state_views.sh`,
`bash scripts/check-release-state-views.sh --write`, and
`bash scripts/check-release-state-views.sh`. Because no push is authorized,
do not add a 0.8.25 `completion` object or populate `landed`: local completed
entries remain `COMPLETE_ON_RELEASE_BRANCH`. After a separately authorized
`git push origin release/0.8.25`, a state-only follow-up may add
`completion.ref = origin/release/0.8.25` and regenerate the views.

**Stop:** historical generated text changes unexpectedly, a completion ref
cannot be verified locally, or the design would weaken origin-main claims for
legacy states.

### S7-03 — release-built Python wheel verifier

**Maps:** P25-01/P25-ENV-01/V25-02; satisfies S7-R3/S7-AC3.

**Design:** add one local verifier that builds the Python wheel from the current
checkout, creates a fresh temporary external venv, installs the wheel without
an editable source path, asserts both `fathomdb.__file__` and the native module
resolve inside that venv, and runs a minimal real-database lifecycle smoke.

**Files/symbols:** `scripts/verify-release-python-wheel.sh` exposes
`--python`, `--wheel-dir`, and `--venv-dir`; its embedded smoke performs the
real `Engine.open/write/search/close` path.
`scripts/tests/test_verify_release_python_wheel.sh` fixtures provenance and
argument failures. Wire only that fixture test into `scripts/agent-test.sh`; it
must not perform an actual build in the fast gate.

**RED:** fixture a release checkout beside a conflicting primary checkout and
prove the verifier rejects a primary-tree Python module, native module, missing
wheel, and editable install.

**GREEN:** implement the verifier and run its fixture test, followed by one
actual wheel build/install/smoke from the release worktree. The actual route
uses `maturin build --release --out <fresh-wheel-dir> --features
pyo3/extension-module,default-embedder -i python3.12` from `src/python`, then
installs that single wheel with `pip install --no-index --no-deps` in the fresh
venv. Run exactly `bash scripts/tests/test_verify_release_python_wheel.sh`, then
`bash scripts/verify-release-python-wheel.sh --python python3.12 --wheel-dir
<fresh-wheel-dir> --venv-dir <fresh-venv-dir>`. Record wheel hash, venv
module/native paths, and smoke result.

**Stop:** the build would install into the primary checkout/shared `.venv`, an
import can escape the fresh venv, or packaging changes beyond the verifier are
needed.

### S7-04 — bounded Rust dependency security correction

**Maps:** P25-06/P25-07; satisfies S7-R4/S7-AC4.

**Design:** update the test-only `httpmock` declaration to exactly 0.8.3 and
apply only lock-compatible floors for `crossbeam-epoch >= 0.9.20`,
`anyhow >= 1.0.103`, `event-listener >= 5.4.2` if still resolved, and
`memmap2 >= 0.9.11`. The existing loader test behavior remains the oracle.

**Files/symbols:** `src/rust/crates/fathomdb-embedder/Cargo.toml`, `Cargo.lock`,
`scripts/check-dependency-policy.py` (`validate_dependency_policy`),
`scripts/tests/test_check_dependency_policy.sh`, and its fast-suite entry in
`scripts/agent-test.sh`. The checker preserves the accepted exact/minimum/
absence assertions. No production manifest source or feature definition may
change. `src/rust/crates/fathomdb-embedder/tests/loader.rs` is a read-only
oracle for this package and must remain byte-unchanged; needing to edit it is
the P25-07 API-incompatibility stop condition.

**RED:** record `cargo tree --all-features -i async-std` and the affected
package trees/advisories on the old lock; add assertions for `httpmock 0.8.3`,
the patch floors, and absence of `async-std` before editing the manifest/lock.

**GREEN:** change the manifest, then use `cargo update -p httpmock --precise
0.8.3`, `cargo update -p crossbeam-epoch --precise 0.9.20`,
`cargo update -p anyhow --precise 1.0.103`, and `cargo update -p memmap2
--precise 0.9.11`. If `event-listener` remains resolved after the `httpmock`
update, run `cargo update -p event-listener@5.4.1 --precise 5.4.2`; otherwise
the checker records that the 5.4.x instance disappeared. Then run:

- `cargo test -p fathomdb-embedder --features default-embedder,loader-test-hooks --test loader`;
- `git diff --exit-code -- src/rust/crates/fathomdb-embedder/tests/loader.rs`;
- `cargo tree --all-features -i async-std`, expecting exit 101 and the
  no-matching-package diagnostic; any printed dependency tree fails;
- `python3 scripts/check-dependency-policy.py --root .` and
  `bash scripts/tests/test_check_dependency_policy.sh`;
- unfiltered `cargo audit --json` as the complete residual receipt, followed by
  gating `cargo audit --ignore RUSTSEC-2024-0436`; only the deliberately
  postponed transitive `paste` advisory may be ignored;
- `bash scripts/check-pinned-override-rot.sh`;
- `cargo test -p fathomdb-embedder --features default-embedder,loader-test-hooks`;
- after `nvidia-smi`, `cargo test -p fathomdb-embedder --features
  embed-cuda,rerank-cuda,loader-test-hooks`, using the authorized GPU-capable
  build; and
- `bash scripts/agent-verify.sh` after focused checks pass.

**Stop:** any existing loader API is incompatible, a loader-test assertion must
be weakened, or the resolver moves the Candle Git revision, sqlite-vec/rusqlite,
ORT, Pyo3, N-API, or another product pin. Report the exact resolver diff rather
than continuing.

### S7-05 — meaningful bounded property tests

**Maps:** P25-11/V25-01; satisfies S7-R5/S7-AC5.

**Design:** preserve small generated domains and deterministic settings. Engine
and Python use real temporary databases to generate valid bodies/source IDs,
write, close, reopen, and assert stable ID/body/source retrieval. Schema
generates supported starting steps and asserts the selected migration suffix is
unique, contiguous, increasing, and ends at `SCHEMA_VERSION`.

**Files/symbols:** replace the three existing `property_template` tests with
`written_record_identity_survives_reopen` in Engine,
`migration_suffix_is_contiguous_and_ends_at_head` in schema, and
`test_written_record_identity_survives_reopen` in Python. Add
`scripts/check-property-test-scaffolds.py` (`find_trivial_properties`),
`scripts/tests/test_check_property_test_scaffolds.sh`, and the checker entry in
`scripts/agent-lint.sh`.

**RED:** the guard must reject the current `x == x`/unused-generated-input
scaffolds. Preserve that failing result before editing the property tests.

**GREEN:** replace the scaffolds with the three real invariants, then run the
focused tests using
`cargo test -p fathomdb-engine --test property_template` and
`cargo test -p fathomdb-schema --test property_template`. Run
`python3 scripts/check-property-test-scaffolds.py --root .` and
`bash scripts/tests/test_check_property_test_scaffolds.sh`. Run the Python
property test with
`FATHOMDB_TESTS_NO_REBUILD=1 <S7-03-external-venv>/bin/python -m pytest
<release-worktree>/src/python/tests/test_property_template.py -q`; do not set
`PYTHONPATH`, so `fathomdb` and its native module remain the wheel installed by
S7-03. No database mock and no generated golden oracle are permitted.

**Stop:** an invariant would require a new product API/behavior or duplicates a
Slice 10+ acceptance criterion instead of testing an existing contract.

### S7-06 — bounded traceability consistency

**Maps:** P25-12/P25-23/P25-24/V25-03; satisfies S7-R6/S7-AC6.

**Design:** extend the maintained documentation checks with a small structural
checker that every NEED referenced by traceability resolves in `dev/needs.md`.
Restore only a concise NEED-026 security-hardening statement matching its
existing trace row. Preserve REQ-067/AC-077 as historical placeholders, adding
successor pointers to active 0.8.25 Slices 10/75 and to the durable
post-0.8.25 design/experimental-review records for former Slices 65/70,
without inventing metrics or thresholds.

**Files/symbols:** `dev/needs.md`, `dev/requirements.md`, `dev/acceptance.md`,
`dev/traceability.md`, and `dev/test-plan.md` only if its existing mapping
requires the same pointer; add `scripts/check-traceability-contracts.py`
(`validate_need_references`), `scripts/tests/test_check_traceability_contracts.sh`,
and its entry in `scripts/agent-lint-md.sh`.

**RED:** fixtures for a missing need, malformed reference, and the current
NEED-026 omission fail before canonical prose is changed.

**GREEN:** run `bash scripts/tests/test_check_traceability_contracts.sh`,
`python3 scripts/check-traceability-contracts.py --root .`,
`bash scripts/agent-lint-md.sh`, and `git diff --check`.

**Stop:** the correction requires changing historical thresholds, renumbering
accepted requirements/criteria, or broad acceptance-document reconciliation.

### S7-07 — architecture authority and narrow documentation hygiene

**Maps:** P25-13 through P25-16, P25-18 through P25-20, with P25-17/P25-25 as
constraints; satisfies S7-R7/S7-AC7.

**Design:** make architecture v2.1 the active versioned successor after
incorporating the 0.8.25 profile of A25-01–A25-07: optional reproducible read
contexts, revision-bound UTF-8 locators, governed operational-state naming,
the zero/one-source core with multi-source liveness explicitly allocated to
0.8.26, precise wire evolution, pre-truncation constraints, and opt-in
visibility-bound evidence resolution. For A25-05, each
public/persisted request and response carries a version. Unknown request fields
or variants fail typed before execution. Older clients may ignore additive
unknown response fields, but an unknown response variant that affects identity,
lifecycle, visibility, mutation, ranking, or continuation returns a typed
unsupported-version outcome and is never mapped to a default. An older writer
must not mutate or reindex a persisted newer-version artifact it cannot fully
interpret. Deprecate v1 and foldback v1 in place with successor pointers; do
not delete them. Preserve the zero/one-source 0.8.25 decision and link the
post-0.8.25 design notes rather than embedding deferred contracts in active
architecture.

Also make only these proven active-surface corrections:

- replace the obsolete 0.8.25 `/tmp` worktree path;
- banner root `0.8.23-release-todo.md` as historical/superseded;
- mark maintained 0.8.24 navigation rows shipped/history;
- replace broken `/tmp` links in active chunking guidance with committed
  performance-program evidence without changing the guidance conclusion;
- correct stale Dependabot comments from current authenticated evidence while
  retaining the paused policy; and
- update only maintained current-authority index rows required by these edits.

**Baseline/GREEN:** retain exact pre-edit scans, then run
`bash scripts/agent-lint-md.sh`, `bash scripts/agent-lint-docs.sh`,
`mkdocs build --strict`, and `git diff --check`. These aggregate commands own
the maintained design, findings, plans, anchor, navigation, Markdown, and link
checks; do not substitute an ad hoc subset.

**Stop:** any edit would rewrite historical evidence, broaden indexes beyond
changed active authority, change product/public API behavior, or require a new
architecture decision not already present in A25-01–A25-07.

## Combined verification and review

After package-level GREEN evidence:

1. Run `bash scripts/agent-verify.sh` from the release worktree; rerun the
   unchanged strict ptrace gate unconfined if the sandbox denies ptrace.
2. Run the fresh-target serial Rust suite, actual wheel verifier, RustSec/pin
   checks, focused CPU/CUDA loader verification, and documentation/link checks
   named above. Record exact command, exit, environment, and any legitimate
   non-executed platform route.
3. Ask an independent reviewer to inspect the implementation diff, decisions,
   tests, protected pins, generated views, architecture authority, and scope.
   Resolve at most two FIX-n cycles; unresolved material findings return to
   HITL.
4. Ask a separate read-only subagent to rerun or inspect the planned evidence.
   The implementation author does not self-certify completion.
5. Write `dev/plans/0.8.25/prework/slice-7-completion.md` with per-package
   baseline/RED, changed files, GREEN commands/results, review verdict, commit
   SHA, and final disposition. Update the release-state JSON and regenerate its
   declared views; never hand-edit generated blocks.

## Completion and handoff

Slice 7 is complete when every accepted package is implemented and verified or
explicitly returned to HITL as blocked, the worktree is clean after commit, the
completion record and release-state agree with Git, no excluded work appears in
the diff, and the immediate-next pointer names Slice 10. The durable release
worktree remains in place for 0.8.25 feature slices; it is not cleaned up as a
temporary child worktree.

No push, main merge, hosted workflow, tag, publication, or registry action is
authorized by approving this plan.

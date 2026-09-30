---
title: Slice 85 recovery receipt
status: IN_PROGRESS
target_release: 0.8.27
date: 2026-09-29
branch: slice-85-fix
source_candidate: 294af94b5b0e9076ccb35956a3863ef12c5599af
implementation_status: COMPLETE
qualification_status: BLOCKED
---

# Slice 85 recovery receipt

## Candidate and authority

The owner authorized the root `plan-slice-85-recovery.md`. Recovery proceeds
forward from `df1ffd000`; the committed plan is `c64fad7b6` and reduced design
contract is `f4a0b6000`. Source work is serialized in an isolated implementer
checkout and integrated into `slice-85-fix`. Implementation and the bounded
independent review are complete at source candidate
`294af94b5b0e9076ccb35956a3863ef12c5599af`. Final closeout changes documentation
only; checks and native provenance below bind this source candidate.
Its `src`, `scripts`, `dev/tools` and `dev/interfaces` trees match the isolated
qualified implementer head `3f6ecb7e1` exactly. The clean temporary implementer
worktree was removed after integration; its branch and receipts are retained.

Release authority remains unchanged: Slice 85's bound candidate is
`7a2f9bf90783f545603516502bac0016d4b93a14`. Recovery has not landed, rebound
release state, pushed, tagged, or published. Historical evidence qualifies its
recorded candidates, not recovery HEAD.

## Removal result

Phase A is `49f03a1db` on the recovery branch, equivalent to implementer
`f711aa46b`. It removes 8,431 net lines. The engine ownership/error deliverable
remains; traversal is private and its test-only encoder import is localized.

| Maintained surface | Before | After |
| --- | ---: | ---: |
| Gate Rust source | 6,548 lines | 4,142 lines |
| Policy | 4,691 lines | 163 lines |
| Routine shell suite | 1,804 lines | 132 lines |
| Explicit qualification helper | No separate helper | 182 Python + 6 shell lines |
| Configurations | 248 | 104 |
| Qualification assertions | Historical 240 | 42 negative cases + benign control |
| Active compiling architectural witnesses | Historical duplicated fixtures | 28 |

Retired: edge, receiver, inherent and SCC freezes; general receiver typing and
namespace resolution; consumer-profile enumeration/minimization; redundant
mutants; qualification from routine tests. Classification, Engine field/method
ownership, exact named admission, forbid floor, descendant prohibitions, two
graphs, root-helper reachability, macro extraction and cache guards remain.

Grammar: direct qualified/relative paths, grouped/named imports, simple aliases,
current re-exports and lexical block imports, conservative existing outside
globs, Engine-specific receiver aliases, explicit callable/type references and
parseable macro bodies. Arbitrary dot-call typing, recursive type projections
and serde string-path expansion are outside the contract; unsupported source
indirection and unknown relevant cfg fail closed.

Configuration table: 64 points for test-hooks/tc5/operator combined with
test/non-test, Linux/non-Linux and debug/release; 40 explicit helper points for
actual CPU ML, CUDA ML, migration, slice72 and combined helper predicates, with
manifest feature closure. Other manifest features do not automatically
multiply the table. No claim covers arbitrary absent combinations.

Focused Phase A checks passed: 29 library and 5 binary gate tests, standalone
clippy/formatting, production check (71 modules/18 governed), cheap wrapper and
ownership suite, tier/collection regressions, and explicit qualification.
The final 28 actual witness bodies compile together with `--tests` and
`test-hooks,operator,tc5-benchmark`; non-Linux/release and rejected syntax remain
explicitly parser-only. Disposable fixtures restore source/policy after cases.

Warm check: 1.32 s and 31,356 KiB peak RSS, versus 2.56 s and 45,388 KiB for
the old report on this host. Final qualification after the review fix: 65.7 s,
versus the historical approximately 9.2-minute suite. Source graphs intentionally
lose inferred-only edges; retained architectural witnesses pass independently,
without recreating a golden census.

## Remaining engine work

B1 fresh source review at `f711aa46b` found ownership, narrow errors, public
conversions/refusal precedence, transaction/attribution handoff and required
graph visibility sound. Defensive error mappings remain justified. Exact error
text checks preserve refactor parity; they do not establish a new message ABI.

Slice 90 still owns validated runtime configuration, bounded orchestration,
private provider dispatch, cancellation and quiescence before provider drain.
It consumes the settled attribution owner and typed reader capabilities;
worker SQLite/commit ownership stays intact. These are future obligations,
not defects to implement during recovery.

B2/B3/B4 are `6b8533344`, equivalent to implementer `e21d9a622`:

- One private test fixture witnesses actual callback uninstallation, empty
  managed-connection registry and zero live readers before logical profile-box
  release, for explicit close and implicit drop. Original boxed userdata remains
  in test custody until actual teardown; production storage/order is unchanged.
  A controlled early clear failed with `profile callback still installed at
  context release`; restored ordering passed both cases.
- The existing tc5 envelope test reproduced 144 bytes. Boxing only the
  `VectorStage` request payload passes its unchanged 128-byte assertion and
  all three existing vector-stage tests. No public benchmark route changes.
- Reader-search pauses now belong to the intended Engine's collector. The
  guard bounds waits to 15 seconds and cancels on timeout, disconnection,
  destruction and unwind. Exact-token identity protects replacement pauses.
  The regression first failed when another Engine consumed the global hook;
  it now proves isolation, cancellation with a retained guard, absence of
  subsequent consumption and replacement/unwind behavior.

The three existing hook consumers preserve their typed-refusal, frozen-read
and evidence assertions. Their serial `test-hooks` runs passed 13, 5 and 23
tests. The final library run passed 81 tests, including existing WAL
finish/handoff/worker-zero controls; final witness assertions were also run
directly. Default and tc5 envelope checks, engine clippy with
`test-hooks,tc5-benchmark`, formatting and the reduced production gate passed.

The hidden Engine-only hook intentionally changes from an arbitrary global
closure plus global clear to `arm_reader_search_hook_for_test(&Engine)` returning
`ReaderSearchPauseForTest`, with bounded `wait_ready` and `release`. This is
documented in `dev/interfaces/rust.md`; it is not exported by SDK facades.

The staged pre-fix patch is `/tmp/fathomdb-s85-recovery-engine-staged-red.patch`,
SHA-256 `71e98397a33d489ae6db905cbef13f566bd20a2b0dd2a72757e9a6b6e1607f68`.
The deliberate early-release mutation is absent from committed source.

## Independent review and combined qualification

One independent review found R1: an Engine method target used `method` while
its body used `Engine::method`, disconnecting a supported root/helper item
cycle. Commit `294af94b5` aligns those identities and adds one compiling
search-to-Engine-to-reader-pool-to-search witness. The old gate missed the
cycle (RED); the fixed gate rejects it with the required item-cycle diagnostic
(GREEN). The final qualification passed 42 cases and a benign control, compiling
28 actual architectural witnesses in 65.7 seconds. Gate units, clippy and helper
lint passed. The independent reviewer attested R1 closure as PASS without
starting another general review. Original FAIL provenance and closure are in
`/tmp/fathomdb-s85-recovery-final-review.json`; RED/GREEN logs are
`/tmp/s85-engine-method-red.log` and `/tmp/s85-engine-method-green.log`.

One combined `scripts/agent-verify.sh` run on `294af94b5` completed in 1,173 s.
Lint and typecheck passed, including workspace clippy, formatting, Markdown,
the production gate and Python typecheck. All Rust workspace tests passed.
Of 128 registered suites, 125 passed, one failed and two skipped. The original
full run is **FAIL**, not retrospectively changed to PASS:

- Python reported 1,522 passed, 30 skipped and eight failed. All eight failures
  were child-process imports of `fathomdb` or `eval`: pytest's source path did
  not propagate to children in the fresh noneditable worktree environment.
  Only those eight tests were rerun with this checkout's `src/python` supplied
  through `PYTHONPATH`; all eight passed in 1.20 s. No source or test changes
  were made, and the whole regression was not repeated.
- TypeScript and hermetic N-API executable suites skipped because the worktree's
  TypeScript dependencies are absent. No TypeScript source changed.
- The fresh candidate Python native build and native receipt verifier passed.

Raw combined diagnostics are `/tmp/fathomdb-s85-recovery-verify.log` and
`/tmp/fathomdb-agent-test-python-4032359.log`; the focused environment rerun is
`/tmp/fathomdb-s85-recovery-python-import-rerun.log`.

The exact source candidate also passed 33 focused existing engine tests with
`test-hooks,operator,migration-test-hooks,tc5-benchmark`: reader-transaction
release, filter error routes, both slice60 wire/graph-expand targets, operator
embedding verification and vector-stage routes. Evidence is
`/tmp/fathomdb-s85-recovery-focused-features.log`.

Actual cached CPU models were exercised with both device selectors set to
`cpu` and `FATHOMDB_REQUIRE_LIVE=1`: 15 Rust reranking/open-report tests and
13 Python FFI/embedding/graph tests passed without skips. These exercise real
inference and moved facade routes; they do not establish GPU coverage. Logs are
`/tmp/fathomdb-s85-recovery-live-cpu-rust.log` and
`/tmp/fathomdb-s85-recovery-live-ffi.log`.

The sanctioned checkout-owned test harness built a noneditable native wheel;
no editable worktree installation or manual native-module copy was used.
Candidate-bound native receipt:

- Schema: `fathomdb.python-test-hooks-receipt/v1`.
- Candidate: `294af94b5b0e9076ccb35956a3863ef12c5599af`.
- Nonce: `agent-test-4032359`.
- Artifact: `src/python/fathomdb/_fathomdb.abi3.so` in this checkout.
- SHA-256: `5a6852c544b2b65af7240d8b13c482cd9ba9e0f66475dcafefccf4faf0905a9b`.
- Receipt file: `.cache/0.8.27-python-test-hooks-receipt.json`.

## Evidence limits and handoff

Exact-source official public capture exited 2 before producing an inventory:
`surface-comparator: cache/scratch filesystem requires at least 100000000000 free bytes`.
This host cannot satisfy its 100 GB floor. Evidence is
`/tmp/fathomdb-s85-recovery-public-capture.log`.

Official hidden capture also exited 2 without an inventory. Its unchanged CUDA
preflight was repeated unconfined to distinguish sandbox restrictions from host
failure: the GPU inventory command exited 18. NVML reports a driver/library
version mismatch (library 580.178). Evidence is
`/tmp/fathomdb-s85-recovery-hidden-capture.log`. The hidden tool separately
requires 20 GB; its observed blocker is CUDA preflight.

Consequently official public/hidden comparisons and the hidden release probe
remain unverified. The intended internal privacy/test-hook delta is documented,
but official inventories have not compared it. Immutable baselines, guards and
hardware settings were not changed to obtain a pass. No GPU-runtime pass is
claimed from feature compilation or successful CPU routes.

Implementation is complete and reviewed; acceptance remains blocked on these
named captures/comparisons and applicable moved GPU-route evidence on a capable
executor. The original combined-run failure and two skipped suites remain
explicit above. Release authority stays unchanged; recovery is not declared
settled for Slice 90, and no landing, rebinding or publication occurred.

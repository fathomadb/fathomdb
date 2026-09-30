---
title: Slice 85 recovery receipt
status: IN_PROGRESS
target_release: 0.8.27
date: 2026-09-29
branch: slice-85-fix
---

# Slice 85 recovery receipt

## Candidate and authority

The owner authorized the root `plan-slice-85-recovery.md`. Recovery proceeds
forward from `df1ffd000`; the committed plan is `c64fad7b6` and reduced design
contract is `f4a0b6000`. Source work is serialized in an isolated implementer
checkout and integrated into `slice-85-fix`.

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
| Gate Rust source | 6,548 lines | 4,139 lines |
| Policy | 4,691 lines | 163 lines |
| Routine shell suite | 1,804 lines | 132 lines |
| Explicit qualification helper | No separate helper | 177 Python + 6 shell lines |
| Configurations | 248 | 104 |
| Qualification assertions | Historical 240 | 41 negative cases + benign control |
| Active compiling architectural witnesses | Historical duplicated fixtures | 27 |

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
The 27 actual witness bodies compile together with `--tests` and
`test-hooks,operator,tc5-benchmark`; non-Linux/release and rejected syntax remain
explicitly parser-only. Disposable fixtures restore source/policy after cases.

Warm check: 1.32 s and 31,356 KiB peak RSS, versus 2.56 s and 45,388 KiB for
the old report on this host. Final substantive Phase A qualification: 61.8 s,
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

## Combined qualification

Final integrated checks, the additional named feature targets and independent
reduced-contract review are pending. Phase-specific checks above qualify their
recorded substantive sources; they are not substituted for exact-candidate
evidence. No full regression was run for individual small corrections.

## Evidence limits

This host currently has approximately 56 GB free; official public capture
requires 100 GB (hidden capture separately requires 20 GB and CUDA inventory
preflight). Its GPU query fails with NVML driver/library mismatch
(library 580.178). These observations are environmental blockers, not passing
surface or GPU-runtime evidence. No baselines, guards or hardware settings are
altered to obtain a pass. Exact-candidate attempts and native evidence remain
pending until the combined source candidate is committed.

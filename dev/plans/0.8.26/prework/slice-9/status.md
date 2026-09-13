---
title: FathomDB 0.8.26 Slice 9 status
status: COMPLETE
completed_on: 2026-09-13
---

# Slice 9 status

## Outcome

Implementation is complete for the narrow preparation bundle ruled at
`seq-286` and authorized at `seq-289`. No product feature, dependency upgrade,
action runtime change, tag, publication, registry mutation, or main integration
occurred.

## Delivered scope

- P26-01: `release-current.py` and the public-document truth checker now fail
  closed on a publication-complete lifecycle without a canonical receipt and
  reject malformed present receipts. The 0.8.25 state records the exact
  `v0.8.25` peeled commit, publication date, and npm `latest` dist-tag.
- P26-02: the tracked 0.8.26 state and board are active, own four generated
  regions, and select the release branch as the preflight baseline.
- P26-03: preflight performs no fetch, prefers `origin/main`, warns loudly on
  local-`main` fallback, fails when neither resolves, and reports `main_ref`
  with `main_sha`.
- P26-05: only the four approved maintained documents were corrected for
  current platform, TypeScript package, planning, workspace, and state-selector
  truth.
- P26-08: exactly seven `download-artifact` comments now identify v4.3.0; all
  seven action SHAs and runtime behavior are unchanged.

The active-plan anchor names were also aligned with the existing commission
manifest contract after the commit hook proved the original draft headings
unresolvable. This was the smallest P26-02 validity correction; it did not
change release scope.

## TDD chronology and commits

| Phase | Commit | Evidence |
| --- | --- | --- |
| P26-01 RED | `af5ac67e` | New lifecycle fixtures failed because publication-complete/null and malformed receipts were accepted. |
| P26-01 GREEN | `faffc9dd` | Both readers reject incomplete/malformed lifecycle state; repaired 0.8.25 truth passes. |
| P26-03 RED | `bc993acb` | Remote-only lacked `main_ref`; stale local won; local fallback was silent; neither-ref exited without a structured hard failure. |
| P26-03 GREEN | `01209b7f` | Stale-local, remote-only, local-only, and neither-ref arms pass. |
| Mechanical truth | `787d63c8` | Four maintained documents and seven action comments corrected without runtime or product change. |
| State activation | `e93e369d` | 0.8.26 state, board, generated views, release selection, and Slice 8 dependency preflight pass. |
| Calendar-date RED | `ddcb92e1` | Both readers accepted the syntactically ISO but impossible date `2026-02-30`. |
| Calendar-date GREEN | `71a795f5` | Both readers now require a real ISO calendar date. |
| State activation reset | `3e302d51` | A normal revert removed only the original activation so its acceptance test could be witnessed RED without rewriting history. |
| State activation RED | `bc48cf6e` | The 0.8.26/Slice 8 expectation failed with `no live release` before activation. |
| State activation GREEN | `ded6c72f` | State, board, and views restored the active release and made the isolated expectation pass. |

Before P26-02, `release-current.py` returned successful empty output and
worktree preflight failed with `release-state discovery failed: no live
release`. After activation, it selects the exact 0.8.26 state/board pair and
preflight verifies Slice 8 at `7a5682796bc446ac38e5053cff5a18e9d33e9c00`.

## Environment witness

- Worktree: `/home/coreyt/projects/fathomdb-worktrees/slice9-implementation`.
- Baseline: `310a1b63eb83501963c2d2bc0319f636da722d91`.
- Linux amd64; Node 25.9.0; npm 11.19.0; Python 3.12.3; Cargo 1.95.0.
- Disk before broad verification: 183 GiB free, above the 10 GiB preflight
  minimum.
- Root `npm ci` produced a checkout-owned dependency tree. It reported two
  high-severity audit findings already inventoried by Slice 1; Slice 9 made no
  dependency change.
- A checkout-owned `.venv` contains the exact Ruff 0.15.17 and Pyright 1.1.410
  pins plus declared test dependencies. The native package was built as a
  disposable wheel under `/tmp` with test hooks and installed non-editably.
  No `pip install -e` or `maturin develop` ran, and no primary-checkout
  environment was used as evidence.

## Verification

Green evidence:

- public-document truth and release-current focused suites;
- stale-local, remote-only, local-only, and neither-ref preflight suite;
- release-state preflight unit suite and exact 0.8.26 worktree/dependency
  preflight;
- generated-view, platform-capability, actionlint fixture, and CUDA
  release-contract suites;
- installed-wheel Python suite: 1,498 passed and 27 skipped;
- `./scripts/agent-verify.sh`: 109/110 suites passed, one intentional skip,
  zero failures or exclusions;
- `cargo clippy --workspace --all-targets -- -D warnings -A missing-docs`;
- `cargo check --workspace --all-targets`.

The broader `./scripts/check.sh` was also attempted. Its `AGENT_LONG=1` Rust
matrix emitted `ac_013_vector_retrieval_latency --- FAILED` at 3/22. The
collecting runner had not emitted the assertion details when it was stopped
after the failure. This is not represented as green or as release evidence.
Slice 9 changes no product, query, storage, performance, or acceptance-test
code; the lean-slice rule did not justify spending the remaining long matrix
after a performance-only failure outside the change blast radius.

The first `agent-verify` attempt was an environment witness rather than a
product verdict: sandbox policy initially denied `target/` writes, and the
next attempt proved missing checkout-local tools/native installation. The
unchanged gate passed after the non-editable local environment was completed.

After FIX-1, both publication-reader suites, all 11 release-state preflight
unit tests, the generated-view suite, and the Slice 8 dependency preflight
passed. The regenerated views reproduce exactly, and `release-current.py`
selects the 0.8.26 state/board pair. A Slice 9 dependency preflight correctly
remains non-green in this isolated branch because `ded6c72f` is not yet an
ancestor of `refs/heads/release/0.8.26`; the landing owner must rerun that
preflight after integrating these commits onto the release branch.

## Reviews

The pre-implementation design package passed the independent Slice 8 review
recorded in `../slice-8/slice-9-independent-review.md` and was reconciled at
`310a1b63`. Independent implementation review found two P2 issues at
`e93e369d`: calendar syntax was not calendar validity, and P26-02 lacked a
genuine RED activation witness. FIX-1 added both RED/GREEN histories without
rewriting history. Independent re-review passed `ded6c72f` with no remaining
findings.

## Deferred and next

D26-01 remains the sole open decision and is intentionally deferred until
Slice 15 evidence; it blocks Slice 20 only. P26-09 remains in Slice 50, and all
other postponed/rejected maintenance remains outside this slice. Slice 10 is
next. Publication remains separately unauthorized.

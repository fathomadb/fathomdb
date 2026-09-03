---
title: 0.8.25 Slice 7 — prework completion record
status: REVIEW_PENDING
target_release: 0.8.25
---

# Slice 7 — prework completion record

Slice 7 implements the owner-approved repository preparation only. Product
behavior assigned to Slices 10 and later is unchanged. This record remains
`REVIEW_PENDING` until the independent implementation review, read-only
verification audit, and closure gates finish.

## S7-01 — durable Rust build provenance

- **Baseline:** the retained target contained binaries built under the removed
  `/tmp/fathomdb-release-0.8.25` checkout.
- **Change:** none to product code; verification used fresh release-bound
  targets under this durable worktree.
- **Evidence:** the fresh serial workspace log at
  `/tmp/s7-final-rust.log` contains no failed test result, and its target tree
  contains no obsolete `/tmp/fathomdb-release-0.8.25` reference.

## S7-02 — generic release completion and current-state readers

- **RED:** `f2eb4530`, `b6ff5528`, and `da4245f2` prove generic 0.8.25
  completion, deleted completed release branches, and release-branch board
  currency were not represented correctly.
- **GREEN:** `b86c0fa9`, `cafea71b`, `c0e7c189`, and `60d4579f` generalize the
  release-state gate and reconcile completed release-branch rows across the
  retained board/orientation readers.
- **Evidence:** `scripts/tests/test_check_release_state_views.sh`,
  `scripts/tests/test_check_board_currency.sh`, and
  `scripts/tests/test_steward_orient.sh` pass.

## S7-03 — release-built Python wheel

- **RED:** `a6438fae` rejects missing, checkout-leaking, native-module-leaking,
  and editable-install evidence.
- **GREEN:** `78a3323e` adds the isolated wheel verifier; `5c329be6` closes its
  shell-quality findings.
- **Evidence:** the actual current-worktree wheel passed
  open/write/search/close from a fresh external venv. Wheel SHA-256 is
  `412169e76c56de6b7f3ea673cc8577dfe380e58cc89a04e00e9813810ef0d94b`;
  both Python and native module resolved inside
  `/tmp/fathomdb-s7-final-wheel.pgL8YG/venv`. The installed-wheel property test
  also passed (`1 passed`).

## S7-04 — bounded dependency correction

- **RED:** `00676237` establishes the exact `httpmock` pin, accepted advisory
  floors, protected product pins, and `async-std` exclusion.
- **GREEN:** `60a252a6` pins `httpmock = 0.8.3`, removes `async-std` from the
  resolved graph, and updates only the approved patch-level Rust packages.
- **Evidence:** the dependency-policy fixture passes; `cargo audit --json`
  reports zero vulnerabilities; `cargo tree -i async-std` finds no package;
  the unchanged-intent loader suite passes 12 tests; protected Candle,
  rusqlite/sqlite-vec, ORT, pyo3, and napi pins remain fixed.

## S7-05 — meaningful property tests

- **RED:** `96fbe230` rejects trivial identity-only property scaffolds.
- **GREEN:** `0c774897` adds bounded generated write-close-reopen identity and
  source tests for the Rust Engine and installed Python wheel, plus schema
  migration ordering/contiguity.
- **Evidence:** focused Rust properties and the installed-wheel Python property
  pass; the scaffold checker passes.

## S7-06 — maintained traceability

- **RED:** `a4fada2a` rejects unresolved maintained need references.
- **GREEN:** `b20fb059` adds NEED-026 and points historical REQ-067/AC-077 to
  active Slices 10/75 and durable future-design notes without inventing a
  threshold.
- **Evidence:** the traceability checker and fixture pass.

## S7-07 — active architecture and documentation authority

- **GREEN:** `a0c9f927` activates data-plane architecture v2.1, preserves v1
  and foldback v1 as superseded history, repairs active navigation and broken
  temporary paths, and retains all experiment runs/data.
- **Evidence:** Markdown, documentation, strict MkDocs, link, and diff checks
  passed after the authority transition.

## Aggregate verification state

- The fresh Rust workspace suite, actual wheel smoke/property, RustSec and pin
  checks, focused CPU loader suite, release-state readers, and documentation
  gates have passed.
- `agent-verify.sh --tier=fast` is being rerun after its two release-reader
  regressions were corrected.
- CUDA and the standalone strict ptrace probe are authorized but not yet
  executable in this task's current managed `workspace-write` sandbox:
  `/dev/nvidia*` is absent and escalation is disabled by the active permission
  profile. This is an environment evidence gap, not a passing product result.
- Independent implementation review and read-only verification are in
  progress. No review verdict is recorded yet.

## Scope and handoff

No feature-slice implementation, publication, tag, registry action, main merge,
or push is included. After closure, the release-state writer will mark Slice 7
complete on the release branch, remove it from the remaining ladder, and point
the immediate next action to Slice 10.

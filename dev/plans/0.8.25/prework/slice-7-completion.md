---
title: 0.8.25 Slice 7 — prework completion record
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.25
completed_on: 2026-09-03
implementation_commit: fdbae48a
---

# Slice 7 — prework completion record

Slice 7 implements the owner-approved repository preparation only. Product
behavior assigned to Slices 10 and later is unchanged. The independent
implementation review, read-only verification audit, and closure gates pass.

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

- Fresh Rust workspace: with
  `CARGO_TARGET_DIR=target/release-0.8.25-final`,
  `cargo test --workspace --all-targets --no-fail-fast --
  --test-threads=1` exited 0. `/tmp/s7-final-rust.log` contains 193 passing
  `test result:` records and no failure. The fixed-string scan of the target
  for `/tmp/fathomdb-release-0.8.25` exited 1 with no match.
- Agent gates: `npm_config_cache=/tmp/fathomdb-npm-cache bash
  scripts/agent-verify.sh --tier=fast` exited 0 with 103/103 suites passing.
  `bash scripts/agent-verify.sh --tier=heavy` exited 0 with 2/3 applicable
  suites passing and one explicitly excluded suite.
- Python package: `bash scripts/verify-release-python-wheel.sh --python
  python3.12 --wheel-dir <fresh-wheel-dir> --venv-dir <fresh-venv-dir>` exited
  0 from the release worktree. The external-venv lifecycle and installed-wheel
  property tests pass.
- Dependency/security: the policy checker, loader tests, pin checks, absence of
  `async-std`, and unfiltered RustSec audit pass. The final resolved package
  versions are recorded in S7-04 above.
- Strict process gate: `strace -f -qq -e trace=%file,%network -o
  /tmp/codex-ptrace-probe.trace true` exited 0 with a non-empty trace.
  `cargo test -p fathomdb-cli --lib
  doctor_gpu_process_matrix_has_exact_outputs_and_no_side_effects --
  --nocapture` exited 0 with 1/1 passing.
- CUDA: unconfined `nvidia-smi` found two idle RTX 3090 24 GiB devices. With
  GPU 0 selected, K620 excluded, CUDA 12.6 bound through `CUDA_HOME`, `PATH`,
  `LIBRARY_PATH`, and `LD_LIBRARY_PATH`, `cargo test -p fathomdb-embedder
  --features embed-cuda,rerank-cuda,loader-test-hooks` exited 0. All 79 tests
  passed, including CPU/GPU logits and real GPU load/score.
- Documentation: maintained Markdown, public-doc, strict MkDocs, link, and
  `git diff --check` gates pass after the authority transition.
- Review: the independent high-effort implementation review passed after the
  allowed FIX-1/FIX-2 cycles with no P1/P2/P3 finding remaining. The separate
  read-only verifier independently confirmed the RED/GREEN and final evidence.
  See
  [`slice-7-implementation-review.md`](slice-7-implementation-review.md).

## Scope and handoff

No feature-slice implementation, publication, tag, registry action, main merge,
or push is included. The release-state writer marks Slice 7 complete on the
release branch, removes it from the remaining ladder, and points the immediate
next action to Slice 10.

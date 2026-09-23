---
title: FathomDB 0.8.27 hidden-surface oracle - requirements and acceptance
status: PROPOSED
target_release: 0.8.27
baseline_entry_sha: f45c5d60
ruling: slice-50-hidden-surface-oracle (TC-3e54ed95); hidden-surface-extra-rows; hidden-surface-release-probe; hidden-surface-effective-and-inventory; feature-complete-test-coverage
---

# Hidden-surface oracle plan

## Why

The Slice 30 comparator's Rust rows come from `cargo public-api`, which omits
`#[doc(hidden)]` items. Every engine test seam is doc-hidden, so the rows cannot
show a dropped or widened feature gate on a hook. Slices 50 through 90 move
more hidden items (`fuse_rrf`, `rerank_fused`, `vector_phase1_sql_for_test`,
`take_slice71_search_statement_trace_for_test`, and others). The owner ruled
(2026-09-23, `slice-50-hidden-surface-oracle`) that hidden-inclusive rows with
a supplementary baseline at `e3358800` must exist before Slice 50 starts, and
after design review extended the rows to cover every hidden-item gate found
(`migration-test-hooks`, `tc5-benchmark`, the `debug_assertions` gate on
`Engine::execute_for_test`, and the facade's hidden proof modules). Because
rustdoc always enables `debug_assertions`, a release-profile rustdoc row
cannot exist; the owner replaced it with cfg predicates recorded in every
signature plus a release compile probe (`hidden-surface-release-probe`).

## Scope

In scope: a separate hidden-inclusive Rust surface capture and comparison tool
with test inventories, its committed baselines, retroactive Slice 40 evidence,
the cadence text that makes later slices use it, and its retirement. Also in
scope, and permanent: a feature-complete test gate and a test-target coverage
check (`feature-complete-test-coverage`), so that after retirement every test
target still runs somewhere.

Out of scope: the Slice 30 comparator, its schema, row identities, and
immutable `slice-30/baseline.json` (unchanged); Python, NAPI, and TypeScript
rows (already covered by Slice 30); gating the test seams themselves (Slice
140, `slice-140-gate-test-seams`).

**Deviation from the ruling's wording:** the ruling says to add rows "to the
surface comparator". They are added as a sibling tool with its own schema,
because changing the Slice 30 tool's row identities would make its immutable
baseline incomparable.

## Rows

| Row id | Crate | Features (`--no-default-features` plus) | Profile |
| --- | --- | --- | --- |
| `engine-default` | `fathomdb-engine` | none | dev |
| `engine-test-hooks` | `fathomdb-engine` | `test-hooks` | dev |
| `engine-slice72-test-hooks` | `fathomdb-engine` | `slice72-test-hooks` | dev |
| `engine-operator-test-hooks` | `fathomdb-engine` | `operator,test-hooks` | dev |
| `engine-migration-test-hooks` | `fathomdb-engine` | `migration-test-hooks` | dev |
| `engine-tc5-benchmark` | `fathomdb-engine` | `tc5-benchmark` | dev |
| `facade-default` | `fathomdb` | none | dev |
| `facade-operator` | `fathomdb` | `operator` | dev |
| `release-probe` | engine and facade | none | release (`cargo check --release` of a generated probe example) |
| `tests-<row>` (one per rustdoc row) | same as the row | same as the row | dev (per-binary `--list`) |
| `tests-req-<n>` (one per uncovered required-feature set) | the crate declaring it | that set | dev (per-binary `--list`) |
| `test-targets` | every workspace crate | none (static) | read from `Cargo.toml` and `tests/` |

## Requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| RH-1 | Capture every item reachable by public path from the crate root, doc-hidden items included, for each rustdoc row above, recording the item's effective cfg predicate and hidden flag across every site on its public path (definition, re-export, impl, enclosing module). | ACH-1 (heavy): a capture has exactly the eight rustdoc rows, the release probe, the eight `tests-<row>` rows, the derived `tests-req-<n>` rows, and `test-targets`; `arm_reader_search_hook_for_test` is in every engine row; `slice72_test_hooks` only in `engine-slice72-test-hooks`; `Engine::open_with_migrations_for_test` only in `engine-migration-test-hooks`; `tc5_benchmark` only in `engine-tc5-benchmark`; `Engine::execute_for_test` is in every engine row with the effective predicate `any(debug_assertions, feature = "test-hooks")` at `e3358800` and `any(debug_assertions, feature = "test-hooks", test)` at closeout `HEAD` (the `test` term came from the Slice 40 review fixes); `governed_surface_method_absence_proof` is in `facade-default` only. |
| RH-10 | Prove in a real release build that debug-only items are absent and release-only items are present. | ACH-10 (heavy): the `release-probe` row lists every item whose recorded predicate requires `debug_assertions` as `unresolved` and every release-only item (for example the facade's `release_surface_raw_sql_absence_proof`) as `resolved`, from `cargo check --release --locked` of a probe example generated inside the exported crates; the probe fails the capture if any item's actual status differs from what its predicate requires. |
| RH-2 | Capture any full 40-character commit SHA from an exported copy of that commit, without touching the working tree, index, refs, stash, or worktree list, and without reusing another commit's workspace crate builds. | ACH-2: self-tests reject a short, unknown, or non-commit SHA before any build. Heavy: capturing a non-`HEAD` SHA from a dirty checkout leaves `git status --porcelain`, `git worktree list`, `git stash list`, and `git for-each-ref` unchanged; capturing X, then Y, then X with the same cache gives byte-identical manifests for X. |
| RH-3 | Entries are deterministic and independent of source location, id numbering, and which site carries a gate: moving an item between files or private modules while keeping its public path, renumbering rustdoc ids, or moving a `#[cfg]` or `#[doc(hidden)]` between an item's definition, re-export, impl, or enclosing module without changing its effect gives no difference; changing its public path, kind, signature, visibility, effective cfg, or effective hidden flag does. | ACH-3 (fast tier): over committed real rustdoc fixtures, a cross-module move whose public paths are unchanged (including a signature that names the moved type) compares equal; a consistent renumbering of every id compares equal; moving a gate or hidden attribute from a definition to its re-export compares equal; a path, kind, signature, visibility, effective-cfg (dropped, widened, narrowed), and effective-hidden change each give a difference; an engine-side move of an item the facade re-exports leaves the facade row equal. Heavy: two captures of one SHA are byte-identical. |
| RH-11 | Test pathways that exist before a move still exist after it, per feature set. | ACH-11: each capture includes a test-inventory row per rustdoc row's crate and features, listing every test (with its ignored status) that `cargo test -- --list` reports; a self-test shows a removed or compiled-out test as a removal. Heavy: the inventory at `5f5c1798` equals the baseline's inventory apart from tests Slice 40 added, each listed in `status.md`. |
| RH-13 | The inventory covers every test target, including those that no rustdoc row's features compile. | ACH-13: each capture has a static `test-targets` row, built from each crate's `Cargo.toml` `[[test]]` entries, their `required-features`, the files in `tests/`, and any file-level `#![cfg(...)]`, one entry per target; and one inventory-only row `tests-req-<n>` for every distinct required-feature set (including file-level cfg requirements) not already compiled by a rustdoc row, derived from that list rather than hand-maintained. A self-test shows a dropped or renamed target and a changed requirement as differences. Heavy: every target in `test-targets` appears in at least one inventory row. |
| RH-14 | A permanent feature-complete test gate runs every test target, and all pass. | ACH-14 (heavy): `scripts/test-feature-complete.sh` runs, for every workspace crate, every distinct feature set its test targets require (from the same derivation as RH-13), downloading missing model weights and using the RTX 3090s (`cuda:0`/`cuda:1`, never the K620) for CUDA targets. A run on this host passes every target with none skipped, including the nine targets no gate ran before this unit; the run is recorded in `status.md`. It is not in `agent-verify`; `scripts/check.sh` runs it when `FATHOMDB_FEATURE_COMPLETE=1`, and Slice 150 runs it. |
| RH-15 | A permanent fast-tier check fails when a test target is run by no gate. | ACH-15: `scripts/check-test-target-coverage.py`, run in the fast tier, reads every crate's test targets and requirements (as RH-13) and the feature sets declared by the gates (the workspace gate, whose features are unified across the workspace, and the RH-14 gate); it fails naming any target whose requirements no gate satisfies. Self-tests show a new target with an uncovered `required-features` and one with an uncovered file-level `#![cfg(feature = ...)]` each fail. |
| RH-12 | The oracle is temporary, scoped to the 0.8.27 decomposition. | ACH-12: `plan-0.8.27.md` Slice 150 includes the unwind step: after the final hidden and test-inventory comparisons pass and are recorded in this unit's `status.md` (with manifest digests), delete `dev/tools/hidden_surface.py`, its self-tests, fixtures, fast-tier wiring, and committed baselines, and prune its cache and scratch roots. Git history keeps them. The static `test-targets` row and all inventory rows go with the tool. The feature-complete test gate (RH-14), the coverage check (RH-15), the warning-free test-build typecheck gates, dead-code lints, and the removal-changelog gate stay: they are what lets the code carry itself afterwards. Retirement is the last step of Slice 150, after qualification fully passes, and it rewrites the cadence and Slice 150 text to say the oracle was retired (citing the `status.md` digests) instead of leaving references to a deleted tool. If the decomposition is abandoned or deferred, the same unwind runs at that decision. A later release that needs the oracle restores it from history. |
| RH-4 | Compare reports per-row added, removed, and changed entries, reports toolchain, rustdoc format version, target, and row-identity mismatches as metadata differences, rejects a manifest of another schema, and never writes either input. | ACH-4: self-tests show each diff class, a metadata mismatch is unequal, a Slice 30 manifest is rejected, and input bytes are unchanged after compare. |
| RH-5 | Committed, immutable baselines. The first is captured at `e3358800`. A later intentional hidden-surface change is recorded as a new `baseline-<sha>.json` beside a reviewed diff record, never by rewriting a file. | ACH-5: `baseline-e3358800.json` exists with capture SHA `e3358800…`; a fresh capture of the implementation closeout `HEAD` compares equal to it, or every difference is listed and justified in `baseline-<HEAD>-diff.md` and that capture is committed as the successor baseline `baseline-<HEAD>.json`; a guard refuses to write over an existing file. |
| RH-6 | Provide the Slice 40 hook-gate evidence the Slice 30 rows could not, and prove the oracle is not vacuous on real code. | ACH-6 (heavy): a capture at `5f5c1798` compares equal to the baseline on every rustdoc and release-probe row (test-inventory differences are handled by ACH-11) and is recorded in `status.md` and cited from `slice-40/adversarial-review.md`; an injected defect (the `test-hooks` gate removed from the definition of the self-contained `decode_dependency_trace_root_for_test`, and that name split out of its gated root `pub use` group into an ungated one, in an exported tree of the baseline commit) captures with exit 0 and compares with exit 1, showing exactly that one added `engine-default` entry; the injected manifest is labelled `source_modified` and cannot be written under `hidden-surface/`. |
| RH-7 | Later slices use the oracle. | ACH-7: `plan-0.8.27.md` cadence step 4 (Slices 40 to 130) runs the hidden comparison with the Slice 30 comparison; Slices 140 and 150 run it too; an unexpected hidden difference blocks the batch; an intended one (for example Slice 140 gating) is recorded as a successor baseline with a reviewed diff record. |
| RH-8 | Self-tests run in the fast tier without a nightly toolchain or network. | ACH-8: `scripts/agent-test.sh` runs the self-tests in the fast tier using committed, canonicalized, host-independent rustdoc JSON fixtures generated by the pinned toolchain from a fixture crate; `regenerate.sh` reproduces them byte-for-byte. |
| RH-9 | The capture refuses to run with the wrong toolchain or rustdoc JSON format. | ACH-9: a format version other than the pinned one, or a missing pinned nightly, fails with a diagnostic before any row is written. |

The requirement IDs are local to this unit; they do not extend the locked
release acceptance list. Fast-tier ACs run in `agent-test.sh`; heavy ACs are
run once at closeout and recorded in `status.md`.

## Test plan

- RED first: the self-test file fails for the missing tool and entry points
  before implementation.
- Fixture crate cases: doc-hidden fn, hidden module, hidden `pub use` of a
  visible item, `#[doc(hidden)] impl` block, named, aliased, and glob
  re-exports (including shadowing and an external-crate glob), a cyclic module
  re-export, enum variants, trait items, an inherent impl, a trait impl with
  generic arguments, a local blanket impl, a feature-gated hook, a
  `debug_assertions`-gated item, a constant whose `expr` names another
  constant, and a moved variant with identical public paths.
- Cfg cases: an item gated by `feature`, `debug_assertions`, `not(...)`,
  `any(...)`, and `all(...)`, gated at the re-export only, and gated on an
  `impl` block; an unparseable attribute fails the capture.
- Gate-site cases: the same effective gate carried on the definition in one
  variant and on the re-export in the other (equal); a gate dropped from
  both sites (different).
- Test-inventory cases: a recorded `--list` output with one test removed, one
  newly ignored, and one added.
- Heavy verification at closeout: ACH-1, ACH-2 (heavy part), ACH-3 (heavy
  part), ACH-5, ACH-6, ACH-10, ACH-11.

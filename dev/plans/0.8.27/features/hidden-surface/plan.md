---
title: FathomDB 0.8.27 hidden-surface oracle - requirements and acceptance
status: PROPOSED
target_release: 0.8.27
baseline_entry_sha: f45c5d60
ruling: slice-50-hidden-surface-oracle (TC-3e54ed95); hidden-surface-extra-rows; hidden-surface-release-probe
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

In scope: a separate hidden-inclusive Rust surface capture and comparison tool,
its committed baselines, retroactive Slice 40 evidence, and the cadence text
that makes later slices use it.

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
| `release-probe` | engine and facade | none | release (`cargo check --release` of a generated consumer) |

## Requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| RH-1 | Capture every item reachable by public path from the crate root, doc-hidden items included, for each rustdoc row above, recording at each site (definition, re-export, impl) the item's cfg predicate. | ACH-1 (heavy): a capture has exactly the nine rows; `arm_reader_search_hook_for_test` is in every engine row; `slice72_test_hooks` only in `engine-slice72-test-hooks`; `Engine::open_with_migrations_for_test` only in `engine-migration-test-hooks`; `tc5_benchmark` only in `engine-tc5-benchmark`; `Engine::execute_for_test` is in every engine row with the predicate `any(test, debug_assertions, feature = "test-hooks")`; `governed_surface_method_absence_proof` is in `facade-default` only. |
| RH-10 | Prove in a real release build that debug-only items are absent and release-only items are present. | ACH-10 (heavy): the `release-probe` row lists every item whose recorded predicate requires `debug_assertions` as `unresolved` and every release-only item (for example the facade's `release_surface_raw_sql_absence_proof`) as `resolved`, from `cargo check --release` of a generated consumer crate; the probe fails the capture if any item's actual status differs from what its predicate requires. |
| RH-2 | Capture any full 40-character commit SHA from an exported copy of that commit, without touching the working tree, index, refs, stash, or worktree list, and without reusing another commit's workspace crate builds. | ACH-2: self-tests reject a short, unknown, or non-commit SHA before any build. Heavy: capturing a non-`HEAD` SHA from a dirty checkout leaves `git status --porcelain`, `git worktree list`, `git stash list`, and `git for-each-ref` unchanged; capturing X, then Y, then X with the same cache gives byte-identical manifests for X. |
| RH-3 | Entries are deterministic and independent of source location and id numbering: moving an item between files or private modules while keeping its public path, or renumbering rustdoc ids, gives no difference; changing its public path, kind, signature, visibility, or any doc-hidden flag does. | ACH-3 (fast tier): over committed real rustdoc fixtures, a cross-module move whose public paths are unchanged (including a signature that names the moved type) compares equal; a consistent renumbering of every id compares equal; a path, kind, signature, visibility, own-hidden, use-hidden, impl-hidden, and cfg-predicate change each give a difference; an engine-side move of an item the facade re-exports leaves the facade row equal. Heavy: two captures of one SHA are byte-identical. |
| RH-4 | Compare reports per-row added, removed, and changed entries, reports toolchain, rustdoc format version, target, and row-identity mismatches as metadata differences, rejects a manifest of another schema, and never writes either input. | ACH-4: self-tests show each diff class, a metadata mismatch is unequal, a Slice 30 manifest is rejected, and input bytes are unchanged after compare. |
| RH-5 | Committed, immutable baselines. The first is captured at `e3358800`. A later intentional hidden-surface change is recorded as a new `baseline-<sha>.json` beside a reviewed diff record, never by rewriting a file. | ACH-5: `baseline-e3358800.json` exists with capture SHA `e3358800…`; a fresh capture of the implementation closeout `HEAD` compares equal to it, or every difference is listed and justified in `status.md`; a guard refuses to write over an existing file. |
| RH-6 | Provide the Slice 40 hook-gate evidence the Slice 30 rows could not, and prove the oracle is not vacuous on real code. | ACH-6 (heavy): a capture at `5f5c1798` compares equal to the baseline and is recorded in `status.md` and cited from `slice-40/adversarial-review.md`; an injected defect (the `test-hooks` gate removed at both the definition and the re-export of the self-contained `decode_dependency_trace_root_for_test` in an exported tree of the baseline commit) captures with exit 0 and compares with exit 1, showing exactly that one added `engine-default` entry; the injected manifest is labelled `source_modified` and cannot be written under `hidden-surface/`. |
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
- Heavy verification at closeout: ACH-1, ACH-2 (heavy part), ACH-3 (heavy
  part), ACH-5, ACH-6, ACH-10.

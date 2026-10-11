---
title: FathomDB 0.8.27 to 0.8.28 merge-forward plan
status: ADVERSARIALLY REVIEWED (gpt-6.1-sol high; FIX-1 and FIX-2 closed); merge held for owner approval
target_release: 0.8.28
observed_on: 2026-10-11
---

# Merge `release/0.8.27` forward into `release/0.8.28`

## Boundary and evidence

This is the execution plan for Slice 31 W0 step 2, separate from the cudarc
fork implementation. Stop after planning and review; **do not start the merge
without the owner's next approval**. During the merge, do not write to
`release/0.8.27`, publish, tag, install drivers/toolkits, or run the full
workspace test/verify gates. Focused checks below establish a reviewed merge
candidate, **not** a full-workspace or release green claim. Slice 31's later
qualification and full gate remain separate.

Inspection at `release/0.8.28` `98abb67d7` and `release/0.8.27`
`c82faa95d` finds merge base `b65283317` (2026-10-07). The source branch is
live; these SHAs describe the plan input, not an instruction to merge a stale
tip. Re-read both exact heads and recompute the inventory before execution.
The current `release/0.8.28` worktree is
`/home/coreyt/projects/fathomdb-worktrees/release-0.8.28`; keep its sole
writer here. The source worktree is read-only.

The main 0.8.27 engine decomposition, Windows WAL path/VFS fix
(`9d3e418f4`, `6b271b5ae`), WAL ownership moves (`30e3e0993`,
`965d00d13`), and embedder-close fixes (`c816b8653`, `8247d91a4`) are
ancestors of `b65283317`. The vendored SQLite amalgamation is byte-identical
at both heads. Post-fork 0.8.27 WAL source changes only narrow test-hook
configuration; the Windows production path is not being reimplemented. The
0.8.28 private pool driver compiles only with `tegra-pool` + CUDA on aarch64
Linux; Windows retains `Device::new_cuda`. Its cross-platform changes are
error types, open/search mappings, reports, and dependency resolution.

Read-only `merge-tree` predicts 13 textual conflict paths. There are 51
paths modified on both sides, 19 of them product/manifest paths. Only
`Cargo.toml` is a predicted textual conflict in product/manifest files;
cleanly combined code and build scripts still require semantic review. The
0.8.27 branch adds a Tegra Node package and hosted Pages delivery **after**
the fork. Slice 30 already gives the Tegra Python wheel `tegra-pool`, but
0.8.27's new `CUDA_TEGRA_NAPI_FEATURES` omits it. A plain merge would build
the Node addon with the old allocator. The requirement is already recorded
in `dev/plans/0.8.27/features/slice-117/plan.md` on 0.8.28; make the
governed build and its contract agree with it.

## Known reconciliation map

| Area | Merge decision and files to inspect together | Proof before close |
| --- | --- | --- |
| Version/dependency graph | In `Cargo.toml`, keep 0.8.27 axis-W workspace crate versions and the bundled `libsqlite3-sys` patch; keep 0.8.28 embedder-api axis E `0.7.0`, Candle fork rev `859b8ea1`, and vendored cudarc 0.19.7 until Slice 31 replaces it. Check affected crate manifests, `Cargo.lock`, `scripts/pinned-override-rot.json`, and `scripts/check-pinned-override-rot.py` as one unit. Do not make the 0.8.28 publication version bump here. | One intended cudarc 0.19 graph; locked resolution, pin checker and version-alignment checks pass. No registry/ABI assumption silently changes. |
| Windows WAL | Preserve `third_party/libsqlite3-sys-0.38.1`, `connection_runtime.rs::sqlite_runtime_path`, `open.rs::canonical_database_path`/SQLite URI use, and `wal_runtime.rs`/`wal_attribution.rs` ownership. Reconcile post-fork `#[cfg(any(test, feature = "test-hooks"))]` changes without restoring debug-only hooks. | Blob/ancestry check for the SQLite patch; focused Windows WAL source guard and, when a Windows executor is available, the existing local-canonical-path WAL unit test. No Linux result is described as Windows runtime evidence. |
| Search/error contract and build configuration | Combine 0.8.27 Slice 135's fail-closed SQLite row/FTS/vector error handling with 0.8.28's `search_query_vector` rule: ordinary provider failure may use sparse fallback, a CUDA pool/context error remains typed, and malformed SQLite rows remain errors. Inspect `embedding.rs`, `search.rs`, `search_api.rs`, `projection_worker.rs`, `errors.rs`, and Rust/Python/TS error adapters as a single path. **Known clean-merge hazards:** 0.8.27 gates `map_runtime_embedder_error`, while 0.8.28 uses it from production `search_query_vector` and `embed_text`; leave the mapper in production. It also gates `set_projection_retry_delays_for_test` and `projection_failure_count_for_test`, called by 0.8.28's `slice30_cuda_error_kinds` integration target; add that target to engine `Cargo.toml` with `required-features = ["test-hooks"]`. Review every 0.8.27 cfg narrowing against 0.8.28 production callers and integration targets. | Run `cargo check -p fathomdb-engine --locked --no-default-features` *without* `test-hooks` before tests, then the relevant production feature configuration. Run `scripts/check-test-target-coverage.py`; compile/run `slice30_cuda_error_kinds` explicitly with `test-hooks`, alongside Slice 135 malformed-row tests. Add a narrow combined test if an uncovered boundary remains. A `test-hooks` test pass cannot substitute for the production build. |
| Tegra Node/Python artifact | In `scripts/release/cuda-artifact-contract.sh`, set the separate Tegra Node tuple to `embed-cuda,rerank-cuda,tegra-pool` while leaving x86, generic arm64, and CPU tuples unchanged. Check `build-napi-cuda-tegra.sh`, `build-python-cuda-tegra.sh`, `check-cuda-release-contract.py`, the Jetson workflow, `src/ts/src/binding.ts`, and exact-version package metadata together. The target checker currently recognizes only `CUDA_*FEATURES_TEGRA` assignments and the incoming `require_tegra_napi_route` pins the Node tuple to the old rerank alias: teach both checks the two explicit approved Tegra feature variables, while retaining the blanket rejection of pool features in every non-Tegra tuple. Early `cuInit` remains shared; `tegra-pool` selection is first GPU use. | RED then GREEN targeted contract mutation for a Tegra Node tuple without `tegra-pool`; static check for both Tegra tuples and rejection outside Tegra; focused Node package-selection and allocator-report unit tests. A later Orin artifact run must observe `private`, not `not_built`, before claiming runtime qualification. |
| Release metadata and ledgers | Preserve 0.8.27's live `release-state-0.8.27.json` and state-owned generated board regions; retain 0.8.28's archive pointer in `STATUS-0.8.27.md`. Seq 287/288 collide, but the source records are **DONE updates** to IDs already OPEN at target seq 271/272, while target seq 287/288 are two separate OPEN IDs. Keep target lineage, then replay the source DONE transitions for IDs `TC-52910b98-a33c-46d4-a6f7-7fa043cb2820` and `TC-4a6418d1-a9d9-4e32-92dd-e9c470cb656e` with `ledgerwrite --profile todos --expected-prior-seq 271` and `272` respectively. After deleting the superseded plan and D28-13 row, mark `TC-d71a9e36-4509-470e-a677-525ea91193e4` DONE with its then-current prior seq. Recheck OCC priors if either branch advances; resolve `.seq` consistently. Do not concatenate, hand-edit JSONL, or open duplicate IDs. | `ledgerwatch` confirms item histories; `scripts/check-ledgers.sh` proves seq continuity and sidecar agreement. Original target records remain, while latest status is DONE for the two source updates and cleanup ID, and OPEN for the CUDA 13 follow-up. `check-release-state-views.sh` passes. |
| Plans and historical receipts | Keep one D28-13 row pointing to Slice 31 and remove the source branch's superseded `dev/plans/0.8.28/features/cudarc-fork/plan.md` per todo `TC-d71a9e36-4509-470e-a677-525ea91193e4`. Reconcile `0.8.28-draft-scope.md`, pool study, program sequencing, Slice 117 plan, and Slice 90 root reconciliation against the merged source. The Slice 90 `lib_rs_blob` must match the merged `lib.rs` rather than either parent. Preserve 0.8.27 historical results and 0.8.28 archive references. | Plan, anchor, generated-view, and source-inventory validators pass; review the combined prose against the exact merge tree. |
| Public contract and guidance | Reconcile 0.8.27's delivered Tegra Node package with 0.8.28's pool contract in `ADR-0.8.28-tegra-private-cuda-pool.md`, `dev/interfaces/typescript.md`, `dev/interfaces/python.md`, `dev/interfaces/rust.md`, `dev/design/release.md`, `docs/embedder.md`, `docs/positions/sdk-parity.md`, and the relevant install/API docs. The ADR currently says only a manual Node recipe carries the pool; after this merge the governed Pages addon should carry it. Amend the ADR/interface docs in the same change as the build contract. The incoming `docs/generated/contributor-map.md` hashes `scripts/agent-test.sh`, which differs on both branches: regenerate it from the merged source with `scripts/generate-public-docs.py --write`, then run `--check`. Inspect the generator's hard-coded 0.8.27 source links for the 0.8.28 branch. Preserve 0.8.27 publication status and Windows path contract. | No statement claims that published 0.8.27 artifacts contain the pool; 0.8.28 Node and Python claims match the feature tuples and available evidence. `generate-public-docs.py --check`, `check-public-doc-truth.py`, and Markdown/docs scoped validators pass. |
| CI and packaging | Carry 0.8.27's Tegra Pages packaging/retention and the later CUDA rehearsal fixes, plus 0.8.28's pool feature exclusions. Inspect `release.yml`, `jetson-tegra-cuda-evidence.yml`, CUDA artifact/release contract scripts and tests, native-artifact receipt tests, and Pages/tegra package scripts. Do not promote or publish 0.8.28 artifacts during the merge. | `actionlint` and the focused contract/receipt tests prove route, tuple, version, and retention constraints. |

## Execution order after owner approves the merge

1. **Freeze evidence once.** Confirm both worktrees are clean and their
   branches match; fetch the two release refs if needed; record local and
   remote SHAs, merge base, and any new commits since this plan. If the
   source changed, refresh `merge-tree`, overlapping paths and the table
   above **before** editing. A new semantic area means revise this plan first.
   Keep 0.8.27 read-only and use only the existing 0.8.28 worktree.
2. **Merge without committing.** Start a no-commit, non-fast-forward merge of
   the pinned 0.8.27 SHA into `release/0.8.28`. Resolve the 13 predicted
   text conflicts by subject area, then inspect every auto-merged product
   overlap and the newly added Tegra Node route. Preserve the 0.8.27
   publication/version state and 0.8.28 Slice 30 behavior. Record any
   unexpected conflict before changing it; do not use blanket `ours` or
   `theirs` across files.
3. **Reconcile the append-only ledger.** Restore the 0.8.28 ledger lineage
   as the merge target. Use `ledgerwrite --profile todos` guarded by
   `--expected-prior-seq 271` and `272` to carry the source DONE updates for
   their existing IDs forward at new seq values; preserve both target
   seq 287/288 OPEN records. Complete the target cleanup todo only after its
   plan and D28-13 changes are applied. If fresh branch commits changed the
   latest prior seq or item status, recompute the OCC operation; do not
   assume the old values remain valid. Use `ledgerwatch` to inspect histories
   and `scripts/check-ledgers.sh` for seq/sidecar integrity. Never hand-edit
   ledger records or generated regions.
4. **Establish the RED proof.** Before altering the Tegra Node tuple, extend
   its focused contract test to require `tegra-pool` and reject its leakage
   into x86/CPU/generic arm64 tuples. Stage the test and record the expected
   local failure. Reuse existing engine error and Windows WAL tests; write a
   new combined unit test only for a demonstrated uncovered merge boundary.
   Do not modify a test during the fix-to-spec step.
5. **Resolve implementation and contract together.** Fix the Node tuple,
   its parser/validator expectations and only code actually needed for the
   merged behavior. Reconcile `Cargo.toml`/lockfile and pin metadata once.
   Then review/amend the ADR, interface docs, public guidance, historical
   plan prose and changelog against the exact combined tree. Review all
   changed requirements, architecture, design and tests as one merge delta;
   a source-only or pool-only reading is insufficient.
6. **Run one ordered, scoped verification pass.** Before selecting commands,
   inventory the target's own `.venv`, root `node_modules`, TS
   `node_modules`, Rust toolchain, Python >=3.11, and Windows/Orin access.
   Establish only the checkout-owned tooling needed for the targeted cells;
   record unavailable cells and do not borrow a native extension from the
   primary checkout. First whitespace/conflict markers, manifest/lock/pin
   checks, then the ordinary production
   `fathomdb-engine` build **without `test-hooks`** (`cargo check -p
   fathomdb-engine --locked --no-default-features`) and its relevant default
   embedder configuration. This catches gated-out production helpers before
   test-only fixtures can hide them. Next run shell/Python/TS syntax and scoped
   lint/typecheck for changed code. Then the Tegra feature-contract RED/GREEN
   test, Windows WAL source guard, relevant Slice 135/30 Rust unit tests,
   focused Python/TS mapping and package-selection unit tests, native
   receipt/Pages contract tests, and Markdown/docs/release-state validators.
   Run `scripts/generate-public-docs.py --write` after source resolution,
   then `--check` and `scripts/check-public-doc-truth.py` before docs lint.
   Python tests requiring `test-hooks` cannot rebuild from a dirty/staged
   merge: `src/python/tests/_test_hooks_gate.py::clean_candidate_sha`
   rejects it. Before commit, verify binding adapters with focused Rust
   checks or a clean isolated candidate snapshot at the exact tree; run
   Python native tests only with a provenance-matched candidate extension.
   Do not treat a stale installed extension as a pass. Use the checkout's
   own dependencies; do not borrow the primary `.venv`.
   On Jetson Python 3.10, select Python >=3.11 as bootstrap specifies. The
   Windows runtime test and Orin installed-artifact/pool checks are targeted
   follow-ups on their actual hosts, not grounds for a Linux-only PASS.
   **Do not run `agent-test.sh`, `agent-verify.sh`, `check.sh`, or the full
   workspace test suite in this merge task.** Record skipped/unavailable
   cells explicitly. A failed focused check gets its cause fixed, then that
   check rerun; after two repeats, re-read its spec/ADR rather than trying
   speculative fixes.
7. **Review the exact candidate before commit.** Compare the staged tree
   with both parents: all 0.8.27 product fixes and delivery files remain,
   0.8.28 pool behavior and docs remain, the duplicate plan is gone, ledger
   IDs survive, and the feature tuples match the accepted ADR amendment.
   Review the combined requirements/design/test diff and focused receipts.
   Only then create the merge commit and push its explicit branch ref, after
   the owner's merge approval. Record both parent SHAs and check results.
   Re-merge forward at Slice 31's later review gate if 0.8.27 advances.

## Merge acceptance and deferred evidence

The merge is ready when the reviewed tree meets every row of the
reconciliation map; the local scoped checks pass; Windows/Orin checks are
either actual targeted receipts or plainly marked pending; and no source
claim calls a scoped pass a full release gate. The prior Slice 30 Orin
qualification was on the pre-forward branch, so it cannot certify the merged
Node addon. Slice 31's later T-ORIN, qualification, independent code review
and full gate remain required by its own plan. Stop this task before step 2
until the owner approves beginning the merge.

---
title: FathomDB 0.8.27 Slice 140 — large code-file review
status: BASELINE_RECORDED
target_release: 0.8.27
source_baseline: 01f4063f7ba19e4d9c38377c3d1956f35903ff9c
---

# Large code-file review

This is the Slice 140 review register on the integrated Slice 117 and Slice 140 code candidate
`01f4063f7ba19e4d9c38377c3d1956f35903ff9c`. The scan counts
physical lines in every Git-tracked `.rs`, `.py`, `.ts`, `.tsx`, `.js`, `.mjs`,
`.c`, `.h`, and `.sh` file, including in-file tests. It found **39** files at or
above 2,000 lines: 19 first-party owned, one generated fixture, and 19
vendored files. The bands are advisory attention signals, not CI limits.
The final source scan remains complete after Slice 117 integration and
test-seam gating; it found no new triggering path. The [machine-readable scan](large-file-scan.json)
records every path and current physical count. This register requires no human review or approval for Slice
140 or release closure.

## First-party owned files

| Lines | File | Finding and recorded disposition |
| ---: | --- | --- |
| 3,905 | `src/rust/crates/fathomdb-engine/src/lib.rs` | Root stores `Engine`, re-exports, cross-domain fixture seams, and a large in-file `#[cfg(test)] mod tests` at `mod tests`. Production root is much smaller than the physical count. **Keep:** moving test bodies just to meet a count risks qualified-test identity and adds no product boundary. Revisit only if a concrete maintenance fault appears. |
| 2,999 | `src/rust/crates/fathomdb-engine/src/evidence.rs` | Evidence resolution and search stay under one owner; test tail remains under its owning module. **Keep**, with symbol navigation and owning tests recorded; split only for demonstrated co-change pressure. |
| 2,792 | `src/rust/crates/fathomdb-cli/src/lib.rs` | Doctor/recovery command dispatch and a in-file test module share one CLI contract. **Keep**; Slice 140 validates docs and examples without a CLI redesign. |
| 2,719 | `src/rust/crates/fathomdb-engine/src/actuation.rs` | Bounded actuation transaction logic with in-file tests. **Keep**; no separate mutable transaction owner is justified by size alone. |
| 2,487 | `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs` | Integrity checks stay together under one read-only inspection owner. **Keep**; a split needs an actual independent invariant boundary. |
| 2,479 | `src/rust/crates/fathomdb-engine/src/search.rs` | Search execution and a substantial in-file test module. **Keep** during test-seam gating; revisit only with a source-grounded owner map. |
| 2,359 | `src/rust/crates/fathomdb-engine/src/projection_registry.rs` | Projection configuration and registry authority remain colocated. **Keep**; no count-driven schema/control split. |
| 2,089 | `src/rust/crates/fathomdb-engine/src/open.rs` | Open/runtime initialization plus interleaved test-only helpers. **Keep**; test-seam gating reviews each helper without moving the open owner. |
| 2,016 | `src/rust/crates/fathomdb-engine/src/projection_generation.rs` | Generation/readiness classification with in-file tests. **Keep** as one invariant owner. |
| 2,403 | `src/rust/crates/fathomdb-engine/tests/slice50_evidence.rs` | End-to-end evidence fixture and assertions. **Keep** as a test target; do not change its oracle to reduce size. |
| 2,254 | `src/rust/crates/fathomdb-engine/tests/slice55_data_plane_integrity.rs` | Integrity end-to-end fixture and assertions. **Keep**; change only if a verified fixture-sharing need arises. |
| 2,603 | `scripts/check-c1-conformance.sh` | Cross-language conformance scanner/check. **Keep** for Slice 140; any extraction needs its own RED mutation controls and is outside document convergence. |
| 2,370 | `scripts/tests/test_check_c1_conformance.sh` | Scanner adversarial tests. **Keep** paired with the scanner. |
| 2,179 | `scripts/tests/test_check_release_state_views.sh` | Generated-view validation tests. **Keep**; neither release-state authority nor fixtures move in this slice. |
| 2,332 | `scripts/tests/test_commission_manifest.sh` | Manifest-policy mutation tests. **Keep**; no production facade size pressure. |
| 2,271 | `scripts/tests/test_worktree_consolidator.py` | Worktree consolidation test matrix. **Keep**; no related Slice 140 change. |
| 2,114 | `scripts/worktree-consolidator.py` | Single-purpose worktree consolidation tool. **Keep**; revisit with a tested ownership split only if its behavior changes. |
| 2,105 | `dev/tools/hidden_surface.py` | Temporary 0.8.27 hidden-surface oracle. **Follow-up:** Slice 150 records the final diff and retires this tool under RH-12. |
| 2,446 | `experiments/measurement_classification.py` | Historical experiment analysis code. **Keep** as an experiment artifact; it is not a current runtime subsystem. |

## Generated fixture and third-party exceptions

The 39,117-line `scripts/tests/fixtures/dorny-paths-filter-v4/dist/index.js`
is checked-in generated distribution output for a policy fixture. **Recorded
exception:** retain the reproducible fixture rather than hand-splitting its
generated bundle. Its test and provenance must remain linked when the fixture
changes.

The following **19** files under `third_party/` are vendored CUDA/SQLite
sources or generated bindings. **Recorded blanket exception:** preserve the
upstream/versioned source and generated shape; local hand-splitting would make
vendor updates and license/provenance review less reliable. Count/path pairs
are retained so the register covers every trigger:

| Lines | Tracked vendor path |
| ---: | --- |
| 8,804 | `third_party/cudarc-0.19.7/src/cublas/sys/mod.rs` |
| 3,560 | `third_party/cudarc-0.19.7/src/cublaslt/sys/mod.rs` |
| 6,846 | `third_party/cudarc-0.19.7/src/cudnn/sys/mod.rs` |
| 2,671 | `third_party/cudarc-0.19.7/src/cufile/sys/mod.rs` |
| 34,817 | `third_party/cudarc-0.19.7/src/cupti/sys/mod.rs` |
| 8,511 | `third_party/cudarc-0.19.7/src/cusolver/sys/mod.rs` |
| 9,805 | `third_party/cudarc-0.19.7/src/cusparse/sys/mod.rs` |
| 3,371 | `third_party/cudarc-0.19.7/src/driver/safe/core.rs` |
| 15,464 | `third_party/cudarc-0.19.7/src/driver/sys/mod.rs` |
| 12,768 | `third_party/cudarc-0.19.7/src/runtime/sys/mod.rs` |
| 3,218 | `third_party/libsqlite3-sys-0.38.1/bindgen-bindings/bindgen_3.34.1.rs` |
| 7,107 | `third_party/libsqlite3-sys-0.38.1/bindgen-bindings/bindgen_3.34.1_ext.rs` |
| 3,622 | `third_party/libsqlite3-sys-0.38.1/sqlcipher/bindgen_bundled_version.rs` |
| 271,671 | `third_party/libsqlite3-sys-0.38.1/sqlcipher/sqlite3.c` |
| 14,028 | `third_party/libsqlite3-sys-0.38.1/sqlcipher/sqlite3.h` |
| 3,677 | `third_party/libsqlite3-sys-0.38.1/sqlite3/bindgen_bundled_version.rs` |
| 7,950 | `third_party/libsqlite3-sys-0.38.1/sqlite3/bindgen_bundled_version_ext.rs` |
| 269,385 | `third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.c` |
| 14,347 | `third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.h` |

## Original five-outlier disposition

The prework baseline counted engine 34,184, PyO3 5,943, NAPI 5,406,
TypeScript 4,439, and Python 2,846 lines. The current engine root is 3,905
and is explained above; the other four facades were split by Slices 100–130
and now have no production file above 2,000 lines (their largest files are
1,049, 1,111, 1,035, and 1,551 lines respectively). Those improvements are
already landed structural work, not a Slice 140 code-change claim.

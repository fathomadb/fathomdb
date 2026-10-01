---
title: Slice 90 current-source entry inventory
status: IN_PROGRESS
target_release: 0.8.27
source_tree: e3ea0b1a647e577c9a4978d8dc13640f82c24101
---

# Slice 90 current-source entry inventory

## Identity and limits

This is an operational inventory for the first Slice 90 production edit, not a
qualification receipt or a change in design authority. The engine `src` tree
at the implementation checkout `df1858640` and current `release/0.8.27`
(`4db5363a`) is the same Git tree,
`e3ea0b1a647e577c9a4978d8dc13640f82c24101`. The release-only change
after the implementation branch is Slice 90 documentation. The D27 historical
performance source is separately fixed at `7a2f9bf9`; it is not the source
for this owner inventory.

The adjacent [symbol census](current-source-symbol-census.csv) enumerates
540 lexical declarations through the production portion of engine `lib.rs`:
132 `Engine` methods, 37 fields, 90 free functions, 51 structs, 50 module
declarations, 43 constants, 23 impls, 18 enums, three statics, one alias and
92 import/re-export statements. Duplicate rows are intentional cfg twins or
distinct imports. The census gives exact source lines. It is an entry checklist,
not a claim that lexical extraction proves all cfg branches or that a method
may be moved without the reviewed owner map in [design](design.md).

Root composition consists of the 50 module declarations and 92 import and
re-export statements. Public paths remain rooted by re-export. The `Engine`
struct and its 37 fields remain rooted as state identity; the method bodies
move to their reviewed semantic owners. `Engine::path` and `ensure_open` remain
root controls. `Engine`'s `Debug` impl is a facade concern; `Drop` belongs to
`runtime_lifecycle`. Root test modules and explicitly identified Slice 140
test seams require separate qualified-test identity checks before a move.

The design's fixed owners apply to the named declarations in the census:
`open`, `runtime_configuration`, `embed_dispatch`, `connection_runtime`,
`runtime_lifecycle`, `wal_runtime`, the retained `reader_pool` request arms,
`operator`, `index_projector`, and existing domain owners. The root has four
cfg twin groups requiring both arms in a move: `process_current_rss_bytes`,
`process_peak_rss_bytes`, `run_requested_gpu_allocation_witness`, and
`Engine::open_default_embedder`. `RuntimeSqliteMode` and global
`SQLITE_RUNTIME_STATE` are process-wide configuration, distinct from the new
per-engine settings. The 37 storage fields include `ProjectionRuntimeShared`
controls only by reference; the four shared search controls remain physically
in `projection_runtime` as the design directs.

Two live root constants are not individually allocated by the current design:
`EDGE_FACT_KIND` (read by `embedding` and `projection_worker`) and
`MEAN_VEC_PIN_THRESHOLD` (read by open/mean/vector write and
`projection_commit`). They must receive item-specific reviewed dispositions
before their movement. The census also exposes test-only method bodies such as
`execute_for_test` and `run_one_thread_poison_for_test`; these need a precise
Slice 140 retention or semantic-owner decision before extraction. This record
does not silently assign those items.

## Inherited Slice 85 recovery delta

The current release source includes recovery through `294af94b5` after the
historical D27 commit. `49f03a1db` reduced the module-boundary gate and its
policy, removed the frozen edge census, and split explicit qualification from
routine checks; its supported grammar and 104 configuration points are in the
[recovery receipt](../slice-85/recovery-receipt.md). `6b8533344` added a
callback-lifetime fixture, boxed the tc5 vector-stage payload, and moved
reader-search pauses from a global hook to an Engine-owned bounded guard.
`af7f1e7d1` removed unused root filter snapshot re-exports. `294af94b5`
repaired Engine method item-path identities in the gate and added a compiling
cycle witness. No Slice 90 production owner move has yet occurred.

Source ancestry does not qualify Slice 85 recovery: its official public and
hidden surface comparisons and GPU-route evidence remain open. The 0.8.27
release-state binding has not advanced with those source commits. The old
`7a2f9bf9` public/hidden receipts cannot be reused as current-source proofs.

## Provider and capacity call sites

Every current engine provider invocation found in production source is listed
here; only `write_vector_for_test` is the reviewed direct-provider exception.

| Path | Exact current call | Planned obligation |
| --- | --- | --- |
| Projection batch | `projection_worker.rs:566` -> `embedding.rs:81` | Batch dispatch, one deadline; fallback releases its serialization guard. |
| Projection per-job | `projection_worker.rs:776` -> `embedding.rs:41` | Bounded dispatch and existing retry/terminal mapping. |
| Direct text embedding | `embedding.rs:123` | Engine dispatcher and typed direct error. |
| Ordinary search | `search_api.rs:543` | Dispatcher and existing sparse fallback. |
| Frozen search | `search.rs:1415` | Dispatcher and frozen-snapshot semantics. |
| Vector-equivalence probe | `vector_equivalence.rs:38` | Dispatch before runtime startup, preserve degraded open. |
| Test vector write | `lib.rs:4931` | Temporary Slice 140 exception; excluded from runtime evidence. |

The current fixed projection constants are `PROJECTION_WORKERS` at `lib.rs:562`,
`PROJECTION_INFLIGHT_LIMIT` at `lib.rs:581`, and `PROJECTION_SCAN_FETCH` at
`lib.rs:584`. The worker count is consumed by `projection_runtime.rs:418-419,
621-622,653-654` and engine WAL/inventory checks at `lib.rs:3611,3625,3742,
3838`; test assertions also refer to it under the root test module. Inflight
and scan limits are consumed by `projection_worker.rs:156,174,217,219`.
`DEFAULT_EMBED_TIMEOUT_MS` and `DEFAULT_EMBED_CIRCUIT_THRESHOLD` initialize
the current projection shared state at `projection_runtime.rs:367,371`;
watchdog calls spawn detached provider threads in `embedding.rs:26-85`.
The exact candidate inventory must use the resolved worker count rather than
these literals.

## Scanner and surface inputs

The scanner retarget checklist before moving a named body is:

| Consumer | Current input or assumption | Move gate |
| --- | --- | --- |
| `scripts/tests/test_windows_wal_attribution_ci_job.sh` | `ENGINE_SOURCE` defaults to engine `lib.rs`; it extracts named function bodies and mutates them. | Retarget each moved body and show old location fails, new location passes, weakened behavior still fails. |
| `dev/tools/module-boundary-gate` and `scripts/tests/test_module_boundary_gate.sh` | Source/module paths and `dev/tools/module-boundary-policy.txt`; reduced Slice 85 explicit-path grammar. | Update exact ownership and maintain compiling architectural witnesses; do not restore retired frozen census. |
| `scripts/release/verify-slice85-manifest.py` | Hashes engine `src` tree and other candidate inputs. | Bind fresh candidate, do not reuse Slice 85 source manifest. |
| `scripts/check-c1-conformance.sh` | Named engine and binding declarations. | Mutant check for every moved declaration it scans. |
| `dev/tools/surface_comparator.py`, `dev/tools/hidden_surface.py`, and `scripts/security/check_removal_changelog.py` | Public, hidden and removal surfaces. | Capture current and candidate feature/test-qualified inventories. |
| Slice 35/source mutation suites and `slice60_fix1_wire` tests | Named source/test seams in engine and bindings. | Search and retarget each exact pattern at its move; require negative fixture. |
| `scripts/lint-plan-anchors.sh` | Source symbol/path anchors in plans. | Run after documentation and source moves; preserve valid source anchors. |

Current feature and qualified-test surface capture is still pending. The
official public comparator requires its 100 GB scratch floor; the Slice 85
hidden capture is blocked by the host NVML driver/library mismatch. Neither
has a current-source PASS receipt. This inventory and the missing D27 valid
historical entry are explicit preconditions, not an excuse to start runtime
semantic edits.

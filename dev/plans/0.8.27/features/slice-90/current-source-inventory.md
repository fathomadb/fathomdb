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

The [design](design.md) now proposes item-specific dispositions for the four
previously unallocated census items. `EDGE_FACT_KIND` belongs to `embedding`:
its consumers are the readiness classifier at `lib.rs:2304` and the
absent-provider projection branch at `projection_worker.rs:698`.
`MEAN_VEC_PIN_THRESHOLD` belongs to `mean`, with the same public root re-export;
its consumers are open recovery at `lib.rs:3314`, the test vector write at
`lib.rs:4966`, and projection commit at `projection_commit.rs:206`.
`Engine::execute_for_test` remains a Slice 140 root test seam because it is a
general writer-SQL and slow-event fixture used across lifecycle, write,
integrity and dependency tests. `Engine::run_one_thread_poison_for_test`
remains a Slice 140 root test seam because it composes write, search,
projection-state and lifecycle event behavior in one debug-only fixture.
Their source cfgs and hidden public paths remain exact. These four decisions
require the independent prospective design review before movement.

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

The current public source capture passed 13 rows at release commit
`77b019c39` (`/tmp/fathomdb-s90-entry-public.json`, SHA-256
`0af7b505ab18a52984dc2cf45151d6f413822df0ffce06d9357466d7032bbaa1`).
The hidden capture is blocked by a host NVML driver/library mismatch:
kernel module 580.173.02 versus library 580.178.04. The missing hidden and
qualified-test comparisons, plus a valid D27 historical entry, remain
explicit preconditions for the corresponding source moves and runtime edit.

The D27 runner must execute in the host PID namespace. In the default sandbox,
`ps` sees PID 1 as `codex` while `/proc/vmstat` reports host-wide swap; that
process view cannot establish the protocol's no-competing-workload condition.
The initial host namespace on `windchill3` is `pid:[4026531836]` with PID 1
`systemd`; the runner and verifier now fail closed on a different namespace.
Use a qualified host-execution context for the historical and candidate runs,
and retain its process observations with the raw output. This does not relax
the swap invalidator; the historical entry remains blocked while host swap
activity continues.

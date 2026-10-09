# Roadmap

This file parks future work; entries do not commission implementation or change
the release plan of record.

## Deferred additions to admin.configure_runtime

Recorded 2026-09-10. The agreed initial API scope is SQLite memory-statistics
configuration: performance (`MEMSTATUS=0`) and diagnostics (`MEMSTATUS=1`),
configured before SQLite initialization, with an application restart to change
modes. No shared-runtime compatibility mode, connection-opening guard, ownership
marker, or database-file access restriction is requested. The implementation
must still demonstrate AC-020 recovery and preserve protected read/write results.
This roadmap entry does not implement that API or mark its verification complete.

Only settings genuinely shared across the loaded runtime belong here. Defer the
following candidates; none is required for the MEMSTATUS change:

1. **SQLite diagnostic-log routing.** Consider routing SQLite's runtime-wide
   error/warning callback into FathomDB's existing logging infrastructure. First
   check for existing coverage; avoid another logging subsystem or arbitrary SDK
   callbacks. Define redaction, callback lifetime, thread safety, and non-reentrant
   handling. Test routing and disabled-path overhead before exposing an option.
2. **Diagnostics-mode SQLite heap limits.** Consider an optional hard limit and/or
   soft pressure threshold for the SQLite runtime, not a per-Engine quota or a
   whole-process memory limit. These interfaces require memory statistics;
   reject their use in performance mode. Any future design needs allocation-
   failure tests, useful errors, and proof that unrelated process allocations
   are not represented as covered by the limit.
3. **Runtime page-cache provisioning, only if profiling justifies it.** SQLite
   supports runtime-level page-cache buffer configuration. Investigate only for
   a measured allocation bottleneck; do not expose a custom allocator or PCACHE2
   plugin interface as part of this task. Require bounded memory ownership,
   fallback behavior, multi-Engine coverage, and an end-to-end performance win
   before proposing a public setting.

SQLite documents these facilities under `SQLITE_CONFIG_LOG`,
`SQLITE_CONFIG_MEMSTATUS`, and `SQLITE_CONFIG_PAGECACHE` in its
[configuration reference](https://www.sqlite.org/c3ref/c_config_covering_index_scan.html).
Its [heap-limit reference](https://www.sqlite.org/c3ref/hard_heap_limit64.html)
describes the soft/hard limit semantics. These are placement candidates, not
approved API names or promised features.

### Keep out of this surface

- Per-Engine or per-connection controls: reader-pool sizing, busy timeouts,
  prepared-statement cache capacity, lookaside tuning, and database WAL/checkpoint
  policy. Review existing surfaces before designing any new public options;
  do not assume these controls are already exposed.
- Connection authentication, opening permits, file ownership checks, encryption,
  and private-runtime packaging. Do not reintroduce these as prerequisites for
  the accepted lightweight MEMSTATUS direction.
- Arbitrary SQLite configuration passthrough, mutex/threading-mode switches,
  allocator replacement, and live MEMSTATUS toggling. No demonstrated need
  justifies that additional public contract here.

Any future addition needs a concrete user need, existing-surface review, focused
tests, and a public interface/design update before implementation. Leave these
items unscheduled; do not expand the current performance work to deliver them.

## Deferred Engine and connection configuration

These options do not belong in `admin.configure_runtime()`. The existing
`admin.configure(engine, name=..., body=...)` submits schema configuration and
does not currently expose either option below. Decide later whether to extend
the admin surface or use Engine-opening options; no API placement is approved
by this roadmap entry.

1. **Connection busy timeouts.** Consider an Engine-scoped option applied to its
   relevant SQLite connections. Distinguish lock-wait timeouts from query
   execution deadlines. Define interaction with existing operation deadlines,
   cancellation, and deliberately non-blocking checkpoint behavior. Test real
   lock contention and timeout errors without weakening existing guarantees.
2. **Prepared-statement cache sizing.** Consider an Engine-scoped capacity option
   applied per relevant connection. Define reader/writer coverage, defaults,
   validation, and whether changes require reopening the Engine. Measure cache
   reuse, eviction, memory growth across the connection pool, and end-to-end
   latency before selecting defaults; do not assume a larger cache is faster.

Keep both items unscheduled and outside the MEMSTATUS implementation. Preserve
current behavior unless a separately approved design and focused tests justify
a change; neither item is implemented by this entry.

## Deferred read-performance experiments

Do not add these to Slice 79's statement-reuse plus MEMSTATUS on/off comparison.
They are candidates for separately approved bounded experiments, not shipping
features or reasons to delay that comparison.

1. **Lookaside sizing.** First measure per-reader `LOOKASIDE_HIT`,
   `LOOKASIDE_MISS_SIZE`, `LOOKASIDE_MISS_FULL` and usage high-water over a defined
   workload. Only then choose at most one counter-directed setting for an
   initial comparison. Lookaside serves eligible small connection allocations;
   it is not the database page cache, and extension allocations may bypass it.
2. **Page-cache allocation.** Investigate separately from lookaside. Attribute
   page-cache allocations and overflow before considering runtime PAGECACHE
   provisioning versus connection cache policy. Page-cache stack samples do not
   prove lookaside misses. Preserve bounded memory and use direct workload
   comparisons; no custom PCACHE2 or allocator replacement is commissioned.
3. **Query-vector BLOB transport.** Not tested as a treatment in Slice 77 and
   not ruled out by its under-sampled profile. The research identifies JSON
   serialization/parsing and allocation work that binary float32 binding may
   remove, but provides no measured FathomDB recovery result. First compare
   JSON versus f32 BLOB transport with SQLite quantization unchanged. Use RED
   differential/property tests for supported dimensions, f32 values including
   signed zero, result ordering and error semantics. Keep corpus, candidates,
   ranking and reranking identical. A bounded experiment should include the
   unchanged 8-dimensional AC-020 fixture and a representative 384-dimensional
   read workload; never substitute the latter for the registered gate. Measure
   absolute latency, scaling and allocation activity where feasible.
4. **Rust-side binary quantization.** A separate follow-on to BLOB transport,
   not bundled into its first comparison. Require byte/semantic parity with
   the bundled sqlite-vec implementation and measured incremental benefit.
5. **Dispatch or remaining allocation work.** Require direct residual evidence
   on the selected candidate. Dispatch needs queue-wait and reader-idle overlap
   measurements; an on-CPU profile cannot establish blocked time.

For any later experiment, hold statement reuse and MEMSTATUS mode fixed between
control and treatment. Effects measured with statistics on may change after
statistics are disabled. Use a corrected, sufficiently sampled search-only
profile if profile-based selection is required; do not repeat Slice 77's full
timing series solely to repair its diagnostic shortfall.

Grounding: [retained research](dev/plans/runs/0.8.25-ac020-research/report.md),
[SQLite allocation architecture](https://www.sqlite.org/malloc.html), and
[sqlite-vec vector inputs](https://alexgarcia.xyz/sqlite-vec/api-reference.html#vec-f32-vector).
Validate implementation details against the bundled version before coding.

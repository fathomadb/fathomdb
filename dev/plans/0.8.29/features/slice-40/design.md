# Slice 40 conditional design and source inventory

Status: .29 retains reference paths with no treatment selected; the design below
is conditional guidance for any future adoption, not a qualified optimization.
The source inventory is frozen
`2f6913c1d59ac8ff3a30de835b57290a8635998c`. Symbols below are checked against
that tree. [Measurement](measurement-plan.md) must precede selection.

## Authoritative contracts

The accepted [eligibility ADR](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md)
requires eligibility before every bounded stage and whole-arm typed fallback when
unsupported. [Frozen pagination](../../../../adr/ADR-0.8.25-frozen-pagination-and-operational-state.md)
uses branch-sensitive O(1) serving authority, not a new O(N) terminal digest.
[Dependency closure](../../../../adr/ADR-0.8.25-dependency-lifecycle-closure.md)
keeps nonterminal closure barriers unconditional; its accepted cleanup scheduling
successors remain in force.

[Format37 coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
already owns IMMEDIATE admission, fresh clean marker checks, 19 table/57 trigger
manifest, one committed relevant advance/nonce rotation and rollback behavior.
No cache may skip its fresh refusal or cache an armed/uncommitted result.
[Format35 layout](../../../../adr/ADR-0.8.28-store-format-35.md) fixes 64-row vec0
chunks and native direct eligibility before the binary cap. Current schema is
37 in [schema source](../../../../../src/rust/crates/fathomdb-schema/src/lib.rs).
[Projection model](../../../../adr/ADR-0.6.0-projection-model.md) and its accepted
[runtime successor](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md)
retain post-commit push publication and terminal cursor authority.

The [per-connection SQLite page-cache ADR](../../../../adr/ADR-0.8.28-sqlite-per-connection-page-cache.md)
is **accepted and exited** through registry libsqlite3-sys 0.39.1. The .28 Unit I
condition passed historically; do not ask for approval or price another run to
reconfirm that settled decision. This is distinct from selecting new .29 decoded
eligibility/metadata reuse. No cache-size/build-flag change is proposed.

## Existing caching and residual work

| Existing source and symbol | Actual behavior | Slice 40 consequence |
| --- | --- | --- |
| [Reader pool](../../../../../src/rust/crates/fathomdb-engine/src/reader_pool.rs), `reader_worker_loop` | Each reader connection sets prepared-statement cache capacity to 10. SQLite page caches are connection owned. | Do not treat statement/page cache hits as reused eligibility answers; preserve capacities. |
| [Search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs), `prepare_search_statement` | Reuses compiled statements. Dependency presence is read once per search transaction and captured for arm predicates. | No duplicate per-arm presence scan is assumed. |
| [Eligibility](../../../../../src/rust/crates/fathomdb-engine/src/dependency_closure.rs), `all_nodes_directly_eligible`, `has_source_dependencies`, `vector_arm_requires_fallback` | Separate SQL answers. Direct whole-node check survives only behind deferred-text fast-path guards; native KNN handles direct lifecycle/validity. Source fallback remains time/view/barrier sensitive. | Measure each call/route separately. Do not restore the obsolete global vector direct-eligibility scan. Transaction-local reuse currently has no demonstrated duplicate probe. |
| [Projection generation](../../../../../src/rust/crates/fathomdb-engine/src/projection_generation.rs), `cached_status_in_snapshot`, `read_mutation_projection_status` | Engine caches use primary connection snapshots, visibility generation/data_version/boundary/runtime and validity horizon or exact time/request. | Existing caches are not a reader-pool authority proof. `PRAGMA data_version` is not a cross-connection/global snapshot identity. Preserve their current scope. |
| [Projection registry](../../../../../src/rust/crates/fathomdb-engine/src/projection_registry.rs), `rederive_projections_on_boot` | Durable registry is authority for derived attribute/FTS materialization; boot can reconcile it. | Do not invent another persisted authority or use derived metadata as canonical proof. |
| [Vector equivalence](../../../../../src/rust/crates/fathomdb-engine/src/vector_equivalence.rs), `probe_verification_is_cached` | Persisted identity fingerprint avoids repeating the existing verification probe. | Separate boot/provider contract; no TC68 fingerprint/settings/cadence redesign belongs here. |
| [Projection commit](../../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs), `commit_projection_outcomes` | IMMEDIATE plus commit gate; generation/effective time and initial mean already once per batch. Mean may change at threshold. | No credit for reimplementing existing hoists. Current mean is not immutable batch metadata. |
| [Vector shape](../../../../../src/rust/crates/fathomdb-engine/src/vector_storage.rs), `actual_vector_attr_columns`; registry `vector_attr_insert_fragments` | Every successful row reads shape; attribute path reads shape again, loads registry and builds SQL fragments; body/scalars remain per row. | These are residual candidates only if 30 proves material batch cost; no-attribute path must retain no body lookup. |

This is the relevant search/projection inventory, not a claim that every Engine
cache was exhaustively audited. Slice 80's candidate at
`/tmp/fathomdb-0829-direct/slice-80/phase-a` proposes body-free **stored committed
head-span aggregates**, not a live eligible denominator. Slice 40 neither builds
that cache nor reuses its scope as a live eligibility proof. Reconcile shared
projection-status seams if both later select work.

## Reader reuse proof required before selection

Default remains existing uncached SQL answers. Prefer eliminating demonstrated
same-snapshot redundancy before considering cross-read storage, but do not build
a transaction-local memo with no duplicate consumer. For any selected value:

1. Define the exact function inputs and result. Dependency presence, direct-node proof and source fallback require separate proofs and entries; a false fallback answer is not proof of direct eligibility.
2. Establish a SQLite reader transaction snapshot through actual SQL, then read authority and consume rows in that same transaction. Never authorize it from the writer connection, Engine atomics, a previous transaction or a shared latest generation. Old/new WAL readers may legitimately need different entries simultaneously; replacement can cost hits, never change correctness.
3. For cross-read reuse, fresh authority must bind database identity and branch-sensitive nonce plus relevant visibility/dependency/projection/schema state. Database ID or integer generation alone is insufficient for copied/diverged stores. Audit every table/DDL/trigger affecting the selected result against actual format37 invalidation; include main/TEMP custom triggers and external commits. An incomplete dependency inventory means no cross-read reuse. Reusing full frozen-token machinery is not assumed cheap or necessary; prove the minimal sufficient projection of authority.
4. Bind exact resolved time and complete view semantics. Initially prefer exact effective-time equality if a candidate is later selected; an interval requires proof over every lifecycle/source validity boundary, inclusive start/exclusive end, backward time and all view flags. Time advances without writes. A status cache's next-boundary function is not automatically the eligibility boundary set.
5. Keep filter/route state fresh unless the cached function explicitly includes it. Retain every deferred-text guard: vector available, no filter/perf limit/dependencies/history relaxation/recency/importance/rerank/graph/explain/full-sort override. Preserve separate edge-expiry semantics and pre-cap behavior.
6. Fresh malformed/committed-armed refusal and frozen authentication/drift checks precede consumption as required by their existing contracts. Cache miss never suppresses a typed error. Read-only memo publication requires a successfully derived clean snapshot; transaction-local results cannot escape their owner. Batch metadata is discarded on rollback and never used as cross-read authority.
7. Specify owner, key/value sizes, numeric entry/byte bound, eviction and close/reopen behavior before code. No global path-only cache, snapshot lease, body retention or unbounded per-time/per-flag map. Measure authority/key validation cost as part of the treatment, not an off-clock setup freebie.

These are proof obligations, not a claim that any proposed key already proves
safety. Exact minimal fields, invalidation inventory and limits remain held until
30 identifies the candidate. If cheap safe authority cannot be established, keep
the original query path and record not-selected/deferred with evidence.

## Batch metadata proof required before selection

After existing IMMEDIATE and clean admission, potential immutable descriptors
are live attribute columns, registry extraction descriptors and SQL/placeholder
shape. External writers are serialized by IMMEDIATE, but user triggers run
inside this writer and may mutate relevant metadata between outcomes. The
existing `projection_batch_has_no_custom_triggers` guard governs visibility
coalescing on its own table set; it is not automatically an adequate metadata
hoist guard. Audit every trigger-able write that can precede descriptor use.

Prove invariance for the entire reachable success/failure/mean-pin sequence, or
bypass hoisting on custom-trigger paths using a reviewed fresh admission proof.
Do not change custom-trigger effects, statement reprepare behavior or SQL error
precedence to create the proof. Configuration source changes can preserve the
same column names while changing extraction, so shape alone is insufficient.

Keep each row's body/scalars, source-type resolution, ownership/lifecycle/time
check, stale generation/terminal/physical-tuple checks and created-at timestamp
semantics. Bind every live vec0 column; preserve absent `''` versus present-empty
marker and the no-attribute route. Mean accumulator candidate, threshold crossing,
retroactive requantization and shared events retain existing ordered semantics.
Physical vector/sidecar/span/terminal/cursor changes commit together; publish
process accumulator/events only after `tx.commit` succeeds. Failure or no-op
cannot seed reusable metadata or invent a visibility advance.

## Review boundary

Primary source research for any later selected optimization must resolve pinned
SQLite WAL/snapshot/trigger behavior, rusqlite statement lifecycle and vec0 binding
constraints where the code proof depends on them. No network research or runtime
experiment occurred in this bounded source-only preparation. Accepted repository
ADRs and actual source support the constraints above; unresolved external/runtime
claims remain obligations before concrete treatment approval.

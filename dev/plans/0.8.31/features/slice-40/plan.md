---
title: Slice 40 Plan — Historical Graph Reconstruction (HISTORY-GRAPH)
status: DRAFT
target_release: 0.8.31
slice: 40
date: 2026-11-22
---

# Slice 40 Plan — Historical Graph Reconstruction (`HISTORY-GRAPH`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Depends on:** Slice 10 (`PREWORK-PREP`, landed `3c4b84439`), Slice 20 (`PROPERTY-SCHEMA`, landed `975637f3c`), Slice 30 (`STRICT-GRAPH`, landed `8e44a47ad`)
- **Authority:** [`dev/plans/0.8.31/plan.md`](../../plan.md), [`dev/plans/0.8.31/scope.md`](../../scope.md), [`dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md`](../../../../adr/ADR-0.8.31-historical-graph-and-fused-seeding.md), [`dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md`](../../../../adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md), [`dev/adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`](../../../../adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md), [`dev/plans/0.8.31/prework/slice-9-triage.md`](../../prework/slice-9-triage.md)
- **Allocated Prework Items:** `P3-10`, `P3-11`, `P4-09` (read-side `as_of_epoch_ms` consumer), `P4-10` (part 2), `P4-11` (part 2), `P4-13` (workspace compile hygiene), `P5-06`, `P5-07` (part 3), `P6-06` (part 2)

---

## 1. Goals & Non-Goals

### Goals

1. **Opt-in bitemporal `graph.expand` over immutable write revisions (`R31-03`):**
   - Extend `GraphExpandRequestV1` (`#[non_exhaustive]` from Slice 30) with optional, mutually exclusive knowledge-time bounds: `as_of_write_cursor: Option<u64>` and `as_of_epoch_ms: Option<i64>`, composing orthogonally with world/validity time (`ReadView.valid_as_of` / `FrozenReadContextV1.effective_valid_at` and `[t_valid, t_invalid)` / `[valid_from, valid_until)`).
   - Keep `ReadView` (`src/rust/crates/fathomdb-engine/src/temporal.rs`) strictly **world-time-only** and keep `context.view.include_superseded` / `include_inactive` refused on `graph.expand` (`graph_api.rs:532-534`).
   - Saturate `u64 -> i64` SQLite bind safely (`i64::try_from(as_of).unwrap_or(i64::MAX)`) and admit future cursors (`as_of_write_cursor > current_write_cursor` on `Current` and `as_of_write_cursor > canonical_write_boundary` on `Frozen`, including `u64::MAX`) as a monotonic upper bound bounded by the pinned snapshot (never exposing post-pin writes; matching future `as_of_epoch_ms`).
2. **O(log N) epoch-to-cursor resolution & Schema 37→38 pre-upgrade semantics (`P4-09`):**
   - Resolve `as_of_epoch_ms` once per reader transaction via a single reverse seek on Schema 38's `_fathomdb_commit_timestamps_ms_idx`:
     `SELECT write_cursor FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1 ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1` (`.optional()?.unwrap_or(0)`).
   - Cursor `0` (including `as_of_epoch_ms` before the first Schema-38 commit on fresh or migrated Schema 37→38 stores) yields an empty historical revision snapshot (`Query` seeds return empty `seeds: []`, `targets: []`; `Explicit` seeds refuse with `GraphSeedUnavailable`), while pre-upgrade and post-upgrade revisions remain directly queryable via `as_of_write_cursor`.
3. **Zero penalty on default current-time traversal & unified historical index `UNION ALL` (`P4-10` part 2, `P5-07` part 3):**
   - Preserve the default unpredicated current-time `graph.expand` path (`EdgeViewBoundary::Current`, `superseded_at IS NULL` via `canonical_edges_{from,to}_active_idx`) with zero extra joins, branches, or historical scans.
   - Extend Slice 30's `EdgeViewBoundary` with `AsOf { edge_now: i64, as_of_cursor: i64 }` emitting direct compound `SELECT ... UNION ALL SELECT ... LIMIT ?limit` queries (2 arms for `Outgoing`/`Incoming`, 4 arms for `Both` with `AND from_id != ?1` on Incoming arms to prevent self-loop duplication), pushing `write_cursor <= ?as_of`, `superseded_at IS NULL` (active arm, `canonical_edges_{from,to}_active_idx`) vs. `superseded_at > ?as_of` (historical arm, `canonical_edges_{from,to}_sup_kind_idx`), `edge_kinds`, `[t_valid, t_invalid)`, `(temporal_fallback IS NULL OR temporal_fallback = 0)` (explicit `AsOf` policy per ADR `#75`), and historical derived-edge dependency/closure-barrier eligibility directly into SQL.
4. **Synchronized node revision coherence, disabled active-only `cpv` pushdown, historical explanation & honest current-state `Engine::transition` semantics:**
   - Bound both seed selection (`Explicit` + `Query` FTS) and visited/target node hydration (`load_node`) to the revision alive at `as_of` (`write_cursor <= ?as_of AND (superseded_at IS NULL OR superseded_at > ?as_of)`), returning historical `kind`, `body`, `properties`, and `write_cursor`.
   - Explicitly disable active-only `canonical_property_values` pushdown (`(pushed_edge_predicates, pushed_target_predicates) = (Vec::new(), Vec::new())`) when `ResolvedHistoryCursor::AsOf(_)` is active, and route **both** `eligibility.attributes` and `eligibility.property_predicates` through `filter.rs`'s inline `(properties > body)` SQL fallback across `load_node`, `resolve_seeds`, `query_graph_evidence_batch`, and `resolve_graph_evidence` so superseded historical revisions with declared `Filterable` keys are never silently dropped.
   - Add history-aware `structural_dependency_state_as_of` / `registered_dependency_for_cursor_as_of` so `include_explanation: true` accurately reports `StructuralDependencyStateV1::Registered` for historical derived targets whose canonical source was later superseded.
   - Truthfully reflect amended ADR `#75`: because `Engine::transition` (`record_lifecycle.rs:230-299`) mutates `canonical_nodes.state` in place without allocating a `write_cursor`, current-state lifecycle (`state = 'active'` on both un-superseded and logical-head rows) applies consistently to historical reads.
5. **Strict erasure subordination & historical frozen evidence (`R31-06`, `P4-11` part 2):**
   - Purged (`purge`), source-erased (`erase_source`), and active-closure-barrier (`_fathomdb_dependency_closures.phase != 'complete'`) records — including derived edges whose canonical source was soft-deleted or barrier-blocked — never reconstruct at any `as_of` cursor or timestamp (including cursors prior to erasure).
   - Enable `include_evidence: true` with `Frozen` context in historical `as_of` mode (`preflight_graph_evidence`, prefix-free `encode_graph_evidence_request` tags `2u8`/`3u8`, `load_stored`, and `resolve_graph_evidence`) while preserving the 704-char `fdbgev1.` wire format and keeping erased, soft-deleted, or barrier-blocked artifacts `EvidenceUnavailable`.
6. **Cross-surface parity, workspace compile hygiene & doc truth (`P3-10`, `P3-11`, `P4-13`, `P6-06` part 2):**
   - Deliver Rust (`fathomdb-engine`, `fathomdb`, `fathomdb-sdk`), Python (`fathomdb`), TypeScript (`fathomdb`), wire (`codec.rs`), interface doc, public guide, and API reference parity, keeping all 13 workspace crates (including `src/inspector`) compiling cleanly under `cargo clippy --workspace --all-targets`.

### Non-Goals

- **No changes to `ReadView` (`temporal.rs`) or non-graph verbs:** `search`, `search_text_only`, `search_frozen`, `read.get`/`list`, `graph.neighbors`, and `graph.search_expand` do not gain `as_of_write_cursor` / `as_of_epoch_ms`.
- **No transition-history event log:** Do not alter `Engine::transition` to emit synthetic node revisions or backdated transition cursors; immutable write revisions remain the history boundary.
- **No backfill of pre-upgrade commit timestamps:** `fathomdb migrate` (Slice 50) does not synthesize wall-clock timestamps for Schema 37 commits.
- **No Inspector UI work in this slice:** Interactive Inspector graph `as_of` controls and admission to Schema 38 belong to Slice 60 (`INSPECTOR`); note Inspector queries SQLite directly (`src/inspector/src/graph.rs:1-200`) and Slice 40 simply keeps the full workspace compiling cleanly.
- **No edits to `dev/acceptance.md`:** `dev/acceptance.md` remains LOCKED; Slice 40 uses slice-local `AC31-40-*` IDs mapped to `R31-03` and `R31-06`.

---

## 2. Draft Needs, Requirements & Acceptance Criteria

### Traceability to Release Scope

- **`R31-03` (Historical / Bitemporal Graph Reconstruction):** `GraphExpandRequestV1` `as_of_write_cursor` / `as_of_epoch_ms` + `valid_as_of`, synchronized node/edge revisions, `_fathomdb_commit_timestamps` O(log N) resolution, dual-index `UNION ALL`, zero penalty on default traversal, and interface/guide updates.
- **`R31-06` (Security, Boundary Hardening & Conformance Invariants):** Closed wire validation, mutual-exclusion & JSON `null` refusals, strict subordination to physical erasure and dependency-closure barriers (for both nodes and derived edges), nondisclosing `EvidenceUnavailable` for erased/barrier-blocked historical evidence, and Rust/Python/TypeScript conformance parity.
- **Cross-Cutting DoDs:** `X1` (Rust/Python/TS parity), `X3` (`EXPLAIN QUERY PLAN` `SEARCH`-only & unpredicated fast-path invariance), `X4` (interfaces, public guides, and API reference truth verified by `mkdocs build --strict` and `check-public-doc-truth.py`).

### Slice-Local Acceptance Criteria

| ID | Criterion | Verification Anchor |
| --- | --- | --- |
| `AC31-40-01` | **Bitemporal 4-quadrant matrix & declared-key inline parity:** Divergent `as_of` (knowledge time) and `valid_as_of` (world time) reconstruct exact pre- and post-supersession chains under `[write_cursor <= as_of < superseded_at]` and `[t_valid <= valid_as_of < t_invalid)`, excluding `temporal_fallback != 0` edges in `AsOf` mode (explicitly reconciled vs. `Current` in oracle tests), honoring `edge_kinds`, `target_kinds`, `edge_predicates`, `target_predicates`, `eligibility.attributes`, and `eligibility.property_predicates` on superseded historical revisions even when those keys are declared `Filterable` (active-only `cpv` pushdown disabled in `AsOf` mode). | `slice38_historical_graph.rs` (Rust), Py/TS parity |
| `AC31-40-02` | **`as_of_epoch_ms`, O(log N) lookup, cursor-`0` & future-cursor semantics:** `as_of_epoch_ms` resolves via `SELECT write_cursor FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1 ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`; cursor `0` (including timestamps prior to the first Schema-38 commit on migrated Schema 37→38 DBs) yields empty `Query` seeds (`targets: []`) and `GraphSeedUnavailable` for `Explicit` seeds while `as_of_write_cursor` addresses pre- and post-upgrade revisions; future `as_of_write_cursor` (including `> current_write_cursor`, `> canonical_write_boundary` on `Frozen`, and `u64::MAX`) binds safely via saturating `i64::MAX` without signed overflow or post-pin leakage. | `slice38_historical_graph.rs` (`P4-09`, `P5-06`) |
| `AC31-40-03` | **Closed request schema, mutual exclusion & JSON `null` refusal:** Specifying both `asOfWriteCursor` and `asOfEpochMs` fails deterministically with `GraphContextInvalid` at `/asOfEpochMs`; `asOfWriteCursor` serializes as canonical unsigned base-10 string (`"0"`..`u64::MAX`), `asOfEpochMs` as JSON integer (`i64`, `Number.isSafeInteger` in TS), explicit JSON `null` refuses with `GraphContextInvalid` at `/asOfWriteCursor` / `/asOfEpochMs` across Rust/Py/TS, unknown fields refuse with `UnknownField`, and `ReadView.include_superseded` / `include_inactive` remain refused at `/context`. | `slice60_wire.rs`, `slice38_historical_graph.rs`, Py/TS tests |
| `AC31-40-04` | **Node revision coherence, `include_explanation` & current-state `Engine::transition`:** Explicit/query seeds and traversed targets return `kind`, `body`, `properties`, `write_cursor`, and `origin.terminal_edge_properties` as of `as_of` (ignoring later `write()` edits); `include_explanation: true` reports `StructuralDependencyStateV1::Registered` accurately for historical derived targets whose canonical source was later superseded; `Engine::transition` to `Pending` or `Deleted` (on either an un-superseded node or a logical identity's active head) removes that entity from historical `as_of` eligibility. | `slice38_historical_graph.rs`, `proptest` oracle |
| `AC31-40-05` | **Erasure & closure-barrier subordination (nodes AND derived edges):** `purge`, `erase_source`, active `_fathomdb_dependency_closures` (`phase != 'complete'`), and soft-deleted canonical sources prevent affected nodes, derived nodes, and **derived edges** from replaying at any `as_of_write_cursor` or `as_of_epoch_ms` (including pre-erasure cursors). | `slice38_historical_graph.rs` (`P5-06`) |
| `AC31-40-06` | **Historical frozen evidence:** `graph.expand(..., Frozen, include_evidence: true, as_of_*)` binds `as_of` via prefix-free `request_commitment` tags (`2u8`/`3u8`), mints valid 704-char `fdbgev1.` tokens when artifacts and their canonical sources existed at `as_of` (`source_cursor <= c`) and remain non-erased/active/barrier-free (including with `eligibility.attributes` and `eligibility.property_predicates` on superseded rows), reports `artifact_lifecycle.superseded == true` when later superseded in the frozen snapshot, and returns `EvidenceUnavailable` once erased, barrier-blocked, or transitioned `Deleted`. | `slice38_historical_graph.rs`, `slice20_graph_evidence.rs` (`P4-11` part 2) |
| `AC31-40-07` | **Pushdown, self-loop non-duplication & work-unit accuracy:** Large quantities of pre-`as_of`-superseded, post-`as_of`-written, wrong-`kind`, or out-of-window edges never enter SQL `EdgeRow`s or inflate `work_units` / trip `GraphExpansionBoundExceeded`; `Both`-direction self-loops are emitted once (`AND from_id != ?1` on Incoming arms) without double-charging `work_units`. | `slice38_historical_graph.rs` (`P4-10`) |
| `AC31-40-08` | **Planner & fast-path invariance (production SQL helpers):** `tests/slice38_query_plans.rs` invokes `#[cfg(any(test, feature = "test-hooks"))]` production explain helpers to verify `EXPLAIN QUERY PLAN` `SEARCH` plans (zero unexpected `SCAN`) on `EdgeViewBoundary::AsOf` (`canonical_edges_{from,to}_active_idx` + `canonical_edges_{from,to}_sup_kind_idx`, empty & non-empty `edge_kinds`), `_fathomdb_commit_timestamps_ms_idx` single reverse seek (`ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`), and historical `load_node` (`canonical_nodes_logical_active_idx` + `canonical_nodes_logical_sup_idx`), plus unchanged default current-time SQL plans. | `slice38_query_plans.rs` (`P5-07` part 3) |
| `AC31-40-09` | **Cross-SDK & workspace compile parity:** Rust SDK (`fathomdb-sdk/tests/surface.rs` builder checks), Python (`test_slice130_package_boundary.py` + all 4 `python_surface_additions.json` entries), and TypeScript (`fathomdb`) match Rust behavior identically; `cargo clippy --workspace --all-targets -- -D warnings` passes across all 13 crates. | Py/TS test suites, `surface.rs`, `cargo clippy --workspace` (`P4-13`) |
| `AC31-40-10` | **Interface, guide & reference alignment:** `dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md`, `docs/guides/{graph-retrieval,frozen-evidence}.md`, and `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md` reflect shipped `as_of` semantics with stale exclusions deleted. | `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, `check-public-doc-truth.py` (`P3-10`, `P3-11`, `P6-06` part 2) |

---

## 3. Architecture & Draft Design

### 3.1 API Carriers, Wire Codec & Request Validation

1. **Rust types (`src/rust/crates/fathomdb-engine/src/graph_expand/types.rs`):**
   - Extend `GraphExpandRequestV1` (`#[non_exhaustive]` from Slice 30) with:
     - `pub as_of_write_cursor: Option<u64>` (default `None` in `GraphExpandRequestV1::new`)
     - `pub as_of_epoch_ms: Option<i64>` (default `None` in `GraphExpandRequestV1::new`)
   - Add fluent builders `.with_as_of_write_cursor(mut self, cursor: u64) -> Self` and `.with_as_of_epoch_ms(mut self, epoch_ms: i64) -> Self` on `GraphExpandRequestV1` (re-exported via `fathomdb` and `fathomdb-sdk`), pinned in `src/rust/crates/fathomdb-sdk/tests/surface.rs`.
   - Do **not** modify `ReadView` (`temporal.rs`) or `GraphReadContextV1` variants.
2. **Closed wire codec (`src/rust/crates/fathomdb-engine/src/graph_expand/codec.rs` & `execution.rs`):**
   - Update `RequestWire` and `check_closed` root allowlist with `"asOfWriteCursor"` and `"asOfEpochMs"`, both `#[serde(skip_serializing_if = "Option::is_none")]` so default current-time requests serialize identically to Slice 30.
   - `"asOfWriteCursor"` is serialized as canonical base-10 string (`u64::to_string`) and decoded via `parse_canonical_u64` (refusing non-string, JSON `null`, leading-zero, negative, or `> u64::MAX` with `GraphContextInvalid` at `/asOfWriteCursor`; `"0"` is valid).
   - `"asOfEpochMs"` is serialized as JSON integer and decoded via `as_i64` (refusing non-integer, JSON `null`, float, or string with `GraphContextInvalid` at `/asOfEpochMs`).
   - **Mutual exclusion:** If both `as_of_write_cursor.is_some()` and `as_of_epoch_ms.is_some()`, both `decode_graph_expand_request_v1` (`codec.rs`) and `validate_semantics` (`execution.rs`) return `GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphContextInvalid, "/asOfEpochMs")`.
   - **Prefix-free `encode_graph_evidence_request(request)` (`execution.rs:1508-1559`):** Slice 30 appends tag `1u8` + sorted predicates only when `!edge_predicates.is_empty() || !target_predicates.is_empty()`. To stay 100% prefix-free and byte-identical for `(None, None)`, omit any history suffix when both `as_of_*` are `None`, and append disjoint tag `2u8` + `encode_u64(cursor)` for `as_of_write_cursor` or `3u8` + `encode_i64(epoch_ms)` for `as_of_epoch_ms` *after* the optional `1u8` predicate block.
3. **Python & TypeScript carriers:**
   - **Python (`src/python/fathomdb/{types.py,graph.py}`):** Add `as_of_write_cursor: str | None = None` and `as_of_epoch_ms: int | None = None` to `GraphExpandRequestV1` (`types.py:712`). Enforce canonical decimal `u64` string validation (rejecting non-string/leading-zero/`> u64::MAX`), `type(as_of_epoch_ms) is int` signed 64-bit bounds, and mutual exclusion (`/asOfEpochMs`) in Python normalization before invoking `_native.graph_expand`. Register all **4** entries in `src/python/tests/python_surface_additions.json`: `fathomdb.GraphExpandRequestV1.{as_of_write_cursor,as_of_epoch_ms}` and `fathomdb.types.GraphExpandRequestV1.{as_of_write_cursor,as_of_epoch_ms}` (matching Slice 30 lines `2645-2658`, `2744-2757`) while leaving `_fathomdb.pyi` untouched (`_native.graph_expand` crosses FFI via JSON string).
   - **TypeScript (`src/ts/src/{graph.ts,index.ts}`):** Add `asOfWriteCursor?: string` and `asOfEpochMs?: number` to `GraphExpandRequestV1` (`graph.ts:101`). Extend `validateGraphExpandRequest`: reject explicit `null` (`GraphContextInvalid` at `/asOfWriteCursor` / `/asOfEpochMs`), validate `asOfWriteCursor` with `/^(?:0|[1-9][0-9]*)$/` `<= 0xffff_ffff_ffff_ffffn`, validate `asOfEpochMs` with `typeof root.asOfEpochMs !== "number" || !Number.isSafeInteger(root.asOfEpochMs)` (matching `effectiveValidAt` at `graph.ts:703`), enforce mutual exclusion (`/asOfEpochMs`), and emit in `canonicalGraphRequest` only when defined.

### 3.2 Snapshot History Cursor Resolution & `u64 -> i64` Safe Binding (`execution.rs`)

Inside `read_graph_expand_with_stop_words_in_tx` (`execution.rs:960`), immediately after pinning the reader transaction and running `validate_snapshot`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolvedHistoryCursor {
    Current,
    AsOf(u64),
}

fn as_of_sql_i64(cursor: u64) -> i64 {
    i64::try_from(cursor).unwrap_or(i64::MAX)
}
```

- `(None, None)` -> `ResolvedHistoryCursor::Current` (zero SQL overhead).
- `(Some(cursor), None)` -> `ResolvedHistoryCursor::AsOf(cursor)` (zero lookup SQL).
- `(None, Some(epoch_ms))` -> execute once in the pinned reader tx (O(log N) single reverse seek on `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)` because `record_batch_commit_timestamp` guarantees monotonically non-decreasing `committed_at_ms` with `write_cursor`):

```sql
SELECT write_cursor
FROM _fathomdb_commit_timestamps
WHERE committed_at_ms <= ?1
ORDER BY committed_at_ms DESC, write_cursor DESC
LIMIT 1
```

(with `.optional()?.unwrap_or(0)` in Rust, returning `ResolvedHistoryCursor::AsOf(cursor)`).

- **Saturating SQLite binding & future-cursor semantics:** All persisted SQLite `write_cursor` values lie in `1..=i64::MAX`, so `as_of_sql_i64(cursor)` (`i64::try_from(cursor).unwrap_or(i64::MAX)`) is mathematically exact for all `u64` values including `u64::MAX` (never wrapping negative to `-1`). On both `Current` and `Frozen` contexts (where `FrozenReadBinding` already guarantees no post-pin writes exist in the pinned snapshot via `validate_snapshot`), an `as_of_write_cursor` greater than the current/pinned write boundary acts as a monotonic upper bound over that pinned snapshot.
- **Cursor-`0` behavior:** When `resolved_cursor == 0` (either `as_of_write_cursor == Some(0)` or `as_of_epoch_ms` precedes the first recorded Schema-38 commit, including pre-upgrade rows on migrated 37→38 DBs), no revision has `write_cursor <= 0`. `Query` seeds yield `Ok(GraphExpandResultV1 { seeds: [], targets: [], ... })`; `Explicit` seeds refuse via `load_node` with `GraphSeedUnavailable` at `/seed/logicalIds/0`.

### 3.3 Historical Incident-Edge `UNION ALL`, Self-Loop Guard & Derived-Edge Barriers (`execution.rs`, `dependency_closure.rs`)

- **Extend Slice 30's `EdgeViewBoundary` (`execution.rs:621-628`):**

```rust
pub(crate) enum EdgeViewBoundary {
    Current { edge_now: i64 },
    AsOf { edge_now: i64, as_of_cursor: i64, include_out_of_window: bool, check_derived_eligibility: bool },
}
```

- **Default path (`EdgeViewBoundary::Current`):** Remains byte-identical to Slice 30 (`superseded_at IS NULL` on `canonical_edges_{from,to}_active_idx`, `UNION ALL` for `Both` with `AND from_id != ?1`, `[t_valid, t_invalid)` pushdown against `view.edge_now()`, optional `edge_kinds` and sound `cpv` pre-filter).
- **Historical path (`EdgeViewBoundary::AsOf`):**
  - **Disable active-only `cpv` pushdown (`execution.rs:1049-1079`):** When `ResolvedHistoryCursor::AsOf(_)`, force `(pushed_edge_predicates, pushed_target_predicates) = (Vec::new(), Vec::new())` so declared `Filterable` keys never query purged active-only `canonical_property_values`.
  - Build direct compound `SELECT ... UNION ALL SELECT ... LIMIT ?limit` SQL (no outer `SELECT * FROM (...)` wrapper, matching Slice 30 `execution.rs:649-654`): 2 arms for `Outgoing` (`from_id = ?1`) or `Incoming` (`to_id = ?1`), or 4 arms for `Both` (2 Outgoing arms with `from_id = ?1`, 2 Incoming arms with `to_id = ?1 AND from_id != ?1` to prevent self-loop duplication and double `work_units` charging):
  - **Active arm (`canonical_edges_{from,to}_active_idx`):**

```sql
SELECT write_cursor, kind, from_id, to_id, logical_id, superseded_at, t_invalid, properties
FROM canonical_edges
WHERE {from_id = ?1 | to_id = ?1 [AND from_id != ?1]}
  AND superseded_at IS NULL
  AND write_cursor <= ?as_of
  [AND kind = ?k | AND kind IN (...)]
  AND (t_valid IS NULL OR t_valid <= ?edge_now)
  AND (t_invalid IS NULL OR t_invalid > ?edge_now)
  AND (temporal_fallback IS NULL OR temporal_fallback = 0)
  [AND <historical derived-edge barrier & source eligibility>]
```

- **Historical arm (`canonical_edges_{from,to}_sup_kind_idx`):**

```sql
SELECT write_cursor, kind, from_id, to_id, logical_id, superseded_at, t_invalid, properties
FROM canonical_edges
WHERE {from_id = ?1 | to_id = ?1 [AND from_id != ?1]}
  AND superseded_at > ?as_of
  AND write_cursor <= ?as_of
  [AND kind = ?k | AND kind IN (...)]
  AND (t_valid IS NULL OR t_valid <= ?edge_now)
  AND (t_invalid IS NULL OR t_invalid > ?edge_now)
  AND (temporal_fallback IS NULL OR temporal_fallback = 0)
  [AND <historical derived-edge barrier & source eligibility>]
```

- **Derived-edge dependency-closure barriers & source eligibility (`P0-2`):** Because `dependency_closure::apply_soft_effects` (`dependency_closure.rs:1452-1462`) soft-invalidates derived edges via `superseded_at = ?boundary` (and `canonical_edges` has no `state` column), a derived edge whose canonical source was soft-deleted or has an active/incomplete closure barrier would otherwise satisfy `superseded_at > ?as_of AND write_cursor <= ?as_of`. Emit `dependency_closure::read_eligibility_sql_historical_for_cursor("canonical_edges.write_cursor", as_of_idx, edge_now_idx, include_out_of_window)` on `EdgeViewBoundary::AsOf` arms (fast-path skippable via a cheap once-per-tx check when `_fathomdb_source_dependencies` and `_fathomdb_dependency_closures` are both empty, keeping zero-dependency corpora fast while enforcing 100% barrier/source subordination whenever dependencies or closures exist).
- **`temporal_fallback` policy reconciliation:** Explicitly preserve `(temporal_fallback IS NULL OR temporal_fallback = 0)` in `EdgeViewBoundary::AsOf` per ADR `#75`, and account for it explicitly when comparing `AsOf(latest)` against `Current` in tests.
- **Restore `superseded_at: Option<i64>` on `EdgeRow` (`execution.rs:476-485, 767`):** Replace Slice 30's `superseded: bool` with `superseded_at: Option<i64>` (preserving `Box<str>` and `Option<Box<PropertyMap>>` so `slice60_fix4_rss` / `slice60_fix5_rss` remain well under 4 MiB) and update the runtime guard at `execution.rs:1171-1175` to check `superseded_at.is_none()` for `Current` and `write_cursor <= c && superseded_at.is_none_or(|s| s > as_of_sql_i64(c))` for `AsOf(c)`.
- Evaluate Slice 30 `edge_predicates` inline via `properties::eval_property_predicate` against fetched `EdgeRow.properties` immediately after `work_units += 1` (before `visited_by_seed` and terminal skip), keeping deterministic sorting (`execution.rs:1136-1153`) and `origin_cmp` unchanged.

### 3.4 Synchronized Node Hydration, Inline Eligibility, Historical Explanation & `Engine::transition` (`execution.rs`, `filter.rs`, `dependency_closure.rs`, `structural_state.rs`, `dependency_trace.rs`)

Both `load_node` (`execution.rs:575`) and `resolve_seeds` (`execution.rs:853`, Explicit & Query FTS) apply synchronized node rules when `ResolvedHistoryCursor::AsOf(as_of)`:

1. **Revision interval (direct two-arm compound `UNION ALL` + unqualified `ORDER BY write_cursor DESC LIMIT 1` in `load_node` backed by Schema 38's `canonical_nodes_logical_active_idx` and `canonical_nodes_logical_sup_idx`):**
   - Arm 1 (un-superseded): `n.logical_id = ?1 AND n.superseded_at IS NULL AND n.write_cursor <= ?as_of ...` -> uses `canonical_nodes_logical_active_idx`.
   - Arm 2 (superseded after `as_of`): `n.logical_id = ?1 AND n.superseded_at > ?as_of AND n.write_cursor <= ?as_of ...` -> uses `canonical_nodes_logical_sup_idx`.
   - Trailing compound `ORDER BY write_cursor DESC LIMIT 1` (unqualified column name required by SQLite compound `SELECT`).
   - In `resolve_seeds` `GraphSeedV1::Query`, `search_index` joins `canonical_nodes cn ON cn.write_cursor = search_index.write_cursor` (retaining superseded node revisions until purge per `write_commit.rs:746-750` and `slice80_history.rs:32-66`), so bounding `cn.write_cursor <= ?as_of AND (cn.superseded_at IS NULL OR cn.superseded_at > ?as_of)` selects each logical node's revision alive at `as_of` in deterministic BM25 + cursor order before `seen.insert(logical_id)` deduplication.
2. **Honest current-state `Engine::transition` lifecycle:**
   - Keep `n.state = 'active'` (catches un-superseded nodes transitioned to `Pending`/`Deleted` and derived nodes tombstoned by `dependency_closure::apply_soft_effects`).
   - Also reject revisions whose logical head was transitioned out of `active` in place (`record_lifecycle.rs:230` updates `WHERE superseded_at IS NULL`):

   ```sql
   AND NOT EXISTS (
     SELECT 1 FROM canonical_nodes cur
     WHERE cur.logical_id = n.logical_id
       AND cur.superseded_at IS NULL
       AND cur.state != 'active'
   )
   ```

3. **Node validity & dependency-closure barriers (`dependency_closure.rs`):**
   - Keep `view.validity_sql("n", now_idx)` unchanged (`[valid_from, valid_until)` at `valid_as_of` / `effective_valid_at`, unless `include_out_of_window`).
   - Always emit `dependency_closure::read_barrier_sql_for_cursor("n.write_cursor")` so incomplete `_fathomdb_dependency_closures` barriers block historical reads.
   - Add `dependency_closure::read_eligibility_sql_historical_for_cursor(cursor_expr, as_of_idx, now_idx, include_out_of_window)` for derived nodes and derived edges: enforces active closure barriers and requires `source_node` to physically exist at `source_node.write_cursor <= ?as_of AND (source_node.superseded_at IS NULL OR source_node.superseded_at > ?as_of)`, satisfy `source_node.state = 'active'` (plus current-head active check), and satisfy source node validity unless `include_out_of_window`.
4. **Row-JSON inline fallback for BOTH `eligibility.attributes` AND `eligibility.property_predicates` (`filter.rs:737-854, 1203-1233`):**
   - Extend `filter::append_node_eligibility_sql_view` (`filter.rs:1203-1233`) with `historical: bool` (or add `append_node_eligibility_sql_historical`) so when `historical == true`, **both** `eligibility.attributes` (`append_inline_attr_predicate`, preserving `properties > body` precedence at `filter.rs:817-854`) and `eligibility.property_predicates` (`append_inline_property_predicate`, preserving `properties > body` precedence at `filter.rs:737-815`) evaluate against historical row JSON (`{alias}.properties` / `{alias}.body`) instead of purged active-only `canonical_attributes` / `canonical_property_values` (`write_commit.rs:751-762, 842-849, 864-871`).
   - Evaluate Slice 30 `target_predicates` strictly inline against the historical revision's `canonical_nodes.properties` JSON (since `pushed_target_predicates` is empty in `AsOf` mode).
5. **Historical `include_explanation: true` (`structural_state.rs:6-33`, `dependency_trace.rs:587-643`, `execution.rs:1419-1435`):**
   - Add history-aware `structural_dependency_state_as_of(&tx, target.write_cursor, view.edge_now(), as_of_cursor)` backed by `dependency_trace::registered_dependency_for_cursor_as_of` checking `source_node.write_cursor <= ?as_of AND (source_node.superseded_at IS NULL OR source_node.superseded_at > ?as_of)`, `source_node.state = 'active'`, current-head active check, and closure barriers so derived historical targets report `StructuralDependencyStateV1::Registered` accurately even after their canonical source is later superseded.
6. **Hydrated output fields:** `load_node` returns `kind`, `body`, historical `properties`, and `write_cursor` from the `as_of` revision row so `GraphTargetV1` carries exact point-in-time node data alongside `origin.terminal_edge_properties`.

### 3.5 Historical Provenance & `evidence.rs` (`P4-11` part 2)

1. **Preflight authorization (`evidence.rs:1513-1577, 1713-1920`):**
   - Thread `history_as_of: Option<u64>` from `read_graph_expand_with_stop_words_in_tx` into `preflight_graph_evidence(&tx, frozen, history_as_of, &node_cursors, &edge_cursors)`.
   - When `history_as_of == None`: behavior strictly unchanged.
   - When `history_as_of == Some(c)`:
     - Extend `INTRINSIC_NODE_PREFLIGHT_SQL` (`evidence.rs:1513-1544`) and `INTRINSIC_EDGE_PREFLIGHT_SQL` (`evidence.rs:1546-1577`) to project `n_head_inactive` (exists current `superseded_at IS NULL` head for `n.logical_id` with `state != 'active'`, nodes only) and `sn_head_inactive` (exists current `superseded_at IS NULL` head for `sn.logical_id` with `state != 'active'`), plus `source_cursor`.
     - `query_graph_evidence_batch` (`evidence.rs:1713-1779`) routes **both** `TARGET_ELIGIBILITY` (`n`) and `SOURCE_ELIGIBILITY` (`sn`, replacing `evidence.rs:1743`'s active-only `append_node_eligibility_sql`) through `filter::append_node_eligibility_sql_view(..., historical = true)` so superseded historical revisions with `eligibility.attributes` or `eligibility.property_predicates` evaluate inline `(properties > body)` JSON rather than purged `canonical_attributes` / `canonical_property_values`.
     - `authorize_graph_evidence` (`evidence.rs:1781`) and `materialize_graph_evidence_batch` (`evidence.rs:1846-1920`) enforce `write_cursor <= as_of_sql_i64(c)` AND `source_cursor <= as_of_sql_i64(c)` (fixing `materialize_graph_evidence_batch`'s ignored `_source_cursor` at `L1913`), admit `artifact_superseded_at` and `source_superseded_at` iff `None || > as_of_sql_i64(c)`, and strictly require `n.state == "active"`, `sn.state == "active"`, `!n_head_inactive`, `!sn_head_inactive`, `temporal_fallback == 0`, `[valid_from, valid_until)` / `[t_valid, t_invalid)` at `frozen.effective_valid_at`, and `!closure_active`.
2. **Opaque token framing & `resolve_graph_evidence` (`evidence.rs:1073-1084, 1169-1190, 2246-2614`):**
   - Preserve `GRAPH_SELECTOR_BYTES = 300` and `GRAPH_TOKEN_BYTES = 704` (`fdbgev1.` + 696 hex chars) so `dev/interfaces/wire.md` and `GraphEvidenceResolveRequestV1` remain fully compatible.
   - Use reserved selector byte `payload[7]` (`evidence.rs:2246, 2381`): `0` = current-time minted reference, `1` = historical `as_of` minted reference (bound via `request_commitment` in `payload[108..140]`).
   - Extend `load_stored` (`evidence.rs:1169-1190`) / `StoredEvidence` to fetch `sn.logical_id` and compute `artifact_head_inactive` / `source_head_inactive`.
   - In `resolve_graph_evidence` (`evidence.rs:2409-2614`) and `authorize_graph_intrinsic_lifecycle` (`evidence.rs:1073-1084`):
     - When `payload[7] == 0`: strictly unchanged.
     - When `payload[7] == 1`:
       - Allow `artifact_superseded` / `source_superseded` ONLY for retained active revisions while reporting `ResolvedGraphEvidenceV1.artifact_lifecycle.superseded == true` accurately when the revision was later superseded in the frozen snapshot.
       - Route **both** target `eligible` (`evidence.rs:2438-2446`) and `source_eligible` (`evidence.rs:2451-2457`) through historical inline `(properties > body)` evaluation (`text_hit_passes_filter_historical` / relaxed-view flag) so `eligibility.attributes` and `eligibility.property_predicates` work identically on superseded historical frozen evidence.
       - Unconditionally return `EvidenceUnavailable` if the artifact or source row was physically deleted (`purge` / `erase_source`), has an active closure barrier (`active_barrier_for_source`), has `artifact_state != Active` or `source_state != Active`, or has `artifact_head_inactive || source_head_inactive`.

### 3.6 Interfaces, Guides, Reference & Workspace Hygiene (`P3-10`, `P3-11`, `P4-13`, `P6-06` part 2)

- **`dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md` (`P3-10`):** Document `asOfWriteCursor` / `as_of_write_cursor`, `asOfEpochMs` / `as_of_epoch_ms`, mutual exclusion -> `GraphContextInvalid` at `/asOfEpochMs`, JSON `null` refusal, `ReadView` world-time-only invariant, `_fathomdb_commit_timestamps` O(log N) lookup + cursor-`0` pre-upgrade semantics, future-cursor snapshot-bounded behavior, `Engine::transition` current-state semantics, erasure & derived-edge closure-barrier subordination, and `fdbgev1.` historical evidence behavior (`artifact_lifecycle.superseded`).
- **`docs/guides/graph-retrieval.md` (`P3-11`, `P6-06` part 2):** Update `L36-39` to distinguish current-time (`superseded_at IS NULL`) vs. opt-in bitemporal `as_of_write_cursor` / `as_of_epoch_ms` + `valid_as_of` reconstruction, pre-upgrade `as_of_epoch_ms` cursor-`0` behavior, current-state `Engine::transition`, and physical erasure subordination.
- **`docs/guides/frozen-evidence.md` & `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md` (`P3-11`):** Document historical frozen evidence and `GraphExpandRequestV1` `as_of` fields across all three SDKs.
- **Workspace hygiene (`P4-13`):** Keep all 13 workspace crates (including `src/inspector`, `fathomdb-mcp`, `fathomdb-cli`) compiling warning-free under `cargo clippy --workspace --all-targets`.

---

## 4. Verified Post-Slice-30 Entry State & Key Code Seams

Verified against live `release/0.8.31` HEAD (`8e44a47ad`):

1. **Already landed in Slices 20 & 30:**
   - Schema 38 (`fathomdb-schema/src/lib.rs:158-213`) already carries `_fathomdb_commit_timestamps` + `_fathomdb_commit_timestamps_ms_idx`, `canonical_{nodes,edges}.properties`, `canonical_nodes_logical_sup_idx`, `canonical_edges_{from,to}_active_idx`, and `canonical_edges_{from,to}_sup_kind_idx` (all verified in `tests/slice38_query_plans.rs`).
   - Slice 30 already marked `GraphExpandRequestV1` `#[non_exhaustive]` with `GraphExpandRequestV1::new(...)`, landed `EdgeViewBoundary::Current`, parameterized `GraphExpandIncidentQuery`, valid SQLite `UNION ALL` with `AND from_id != ?1`, `[t_valid, t_invalid)` pushdown, and inline `eval_property_predicate`.
2. **Active-only `cpv` / `canonical_attributes` seams to gate in Slice 40:**
   - `execution.rs:1049-1079` (`pushed_edge_predicates`, `pushed_target_predicates`) must become empty under `AsOf(_)`.
   - `filter.rs:1203-1233` (`append_node_eligibility_sql_view`), `evidence.rs:1743` (`query_graph_evidence_batch`), and `evidence.rs:2438-2457` (`resolve_graph_evidence`) must route both `attributes` and `property_predicates` through inline `(properties > body)` evaluation when `historical == true`.
3. **Derived-edge closure barrier & `EdgeRow` seams:**
   - `dependency_closure.rs:1452-1462` tombstones derived edges via `superseded_at`; `EdgeViewBoundary::AsOf` must enforce historical barrier & source eligibility on derived edges.
   - `execution.rs:476-485, 767, 1171-1175` stores `superseded: bool` on `EdgeRow`; restore `superseded_at: Option<i64>` for history-aware checks.
4. **Historical explanation & evidence seams:**
   - `structural_state.rs:6-33` / `dependency_trace.rs:587-643` require `source_revision_is_strictly_eligible` (`superseded_at IS NULL`); add history-aware `structural_dependency_state_as_of`.
   - `evidence.rs:1073-1084, 1169-1190, 1513-1577, 1781, 1913, 2246-2614` need `source_cursor <= c`, `sn.logical_id`, current-head active checks, `payload[7] = 1`, and historical inline filter checks.
   - `execution.rs:1553-1557` (`encode_graph_evidence_request`) uses conditional tag `1u8` for predicates; use disjoint trailing tags `2u8`/`3u8` for `as_of_write_cursor` / `as_of_epoch_ms`.
5. **Prework vs. Amended ADR Reconciliation:**
   - Slice 40 enforces amended ADR `#75`: current-state `Engine::transition` applies to historical reads while immutable `write()` content/property revisions replay cleanly at `as_of`.

---

## 5. TDD Implementation & Verification Plan

Execute strictly RED → GREEN → Refactor against real SQLite stores (no mocks).

### Phase 5.1 — RED Tests First

1. **Wire & Closed-Codec Tests (`slice60_wire.rs`, `slice38_historical_graph.rs`):** canonical `"asOfWriteCursor"` / `"asOfEpochMs"` round-trip, mutual exclusion `GraphContextInvalid` at `/asOfEpochMs`, explicit JSON `null` refusal at `/asOfWriteCursor` & `/asOfEpochMs`, `ReadView.include_superseded` / `include_inactive` still refused at `/context`, and unpredicated current-time byte-identity.
2. **Bitemporal 4-Quadrant Matrix & Declared-Key Inline Parity (`slice38_historical_graph.rs`, `P5-06`):** divergent `as_of` vs `valid_as_of`, `[write_cursor <= c < superseded_at]`, `[t_valid <= valid_as_of < t_invalid)`, `temporal_fallback = 0` in `AsOf`, `edge_predicates`/`target_predicates`/`eligibility.attributes`/`eligibility.property_predicates` on superseded historical revisions **when declared `Filterable`**, hydrated historical `body`/`properties`, `include_explanation: true` (`StructuralDependencyStateV1::Registered` after source supersession), and current-state `Engine::transition`.
3. **`as_of_epoch_ms`, Cursor-`0`, Upgraded-Store & Future-Cursor/`u64::MAX` Semantics (`slice38_historical_graph.rs`, `P4-09`):** exact `_fathomdb_commit_timestamps` resolution, migrated Schema 37→38 pre-upgrade cursor-`0` empty behavior vs. `as_of_write_cursor` access, future `as_of_write_cursor` (`> current_write_cursor`, `> canonical_write_boundary` on `Frozen`, and `u64::MAX`) without overflow or post-pin leakage.
4. **Property-Based Snapshot Oracle (`proptest` in `slice38_historical_graph.rs`, `P5-06`):** random writes/supersessions/transitions/self-loops/declared `Filterable` keys compared against an exact in-memory snapshot BFS oracle.
5. **Erasure Subordination (Nodes & Derived Edges) & Work-Unit Bounds (`slice38_historical_graph.rs`, `P5-06`, `P4-10`):** `purge`/`erase_source` non-resurrection at pre-erasure cursors, active closure barriers & soft-deleted canonical sources blocking both derived nodes AND derived edges, `Both`-direction self-loop non-duplication, and 500 pre-`as_of` superseded edges + `max_work_units = 2` pushdown proof.
6. **Historical Frozen Evidence (`slice38_historical_graph.rs`, `slice20_graph_evidence.rs`, `P4-11` part 2):** `include_evidence: true` + `resolve_graph_evidence` across superseded self-sourced and derived nodes/edges (including with `eligibility.attributes` and `eligibility.property_predicates`, verifying `artifact_lifecycle.superseded == true` and `source_cursor <= c`), prefix-free `request_commitment` tamper rejection, and `EvidenceUnavailable` after erasure/barrier/transition.
7. **`EXPLAIN QUERY PLAN` Pins via Production Helpers (`tests/slice38_query_plans.rs`, `P5-07` part 3):** invoke `#[cfg(any(test, feature = "test-hooks"))]` production explain helpers proving zero-`SCAN` `SEARCH` plans on `EdgeViewBoundary::AsOf` (`canonical_edges_{from,to}_active_idx` + `canonical_edges_{from,to}_sup_kind_idx`, empty & non-empty `edge_kinds`), `_fathomdb_commit_timestamps_ms_idx` single reverse seek (`ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`), and historical `load_node` (`canonical_nodes_logical_active_idx` + `canonical_nodes_logical_sup_idx`), plus unchanged default current-time plans.
8. **Python, TypeScript & SDK Parity:** `src/python/tests/test_slice38_historical_graph.py`, `src/ts/tests/slice38-historical-graph.test.ts` (including `!Number.isSafeInteger` and JSON `null` refusals), `test_slice130_package_boundary.py`, `fathomdb-sdk/tests/surface.rs`.

### Phase 5.2 — GREEN Implementation Sequence

1. **Layer 1 — Request Carriers, Closed Codec, `EdgeViewBoundary::AsOf`, Historical Traversal, Inline Eligibility & Historical Explanation (`types.rs`, `codec.rs`, `execution.rs`, `filter.rs`, `dependency_closure.rs`, `structural_state.rs`, `dependency_trace.rs`, `graph_api.rs`):**
   - Add `as_of_write_cursor` / `as_of_epoch_ms` + builders, closed codec (incl. JSON `null` refusal & mutual exclusion), prefix-free `encode_graph_evidence_request` tags `2u8`/`3u8`, O(log N) `_fathomdb_commit_timestamps` seek + `as_of_sql_i64` saturation, `EdgeViewBoundary::AsOf` (`UNION ALL` + `from_id != ?1` + derived-edge barriers), disabled `cpv` pushdown in `AsOf` mode, restored `EdgeRow.superseded_at: Option<i64>`, historical two-arm `load_node` (`ORDER BY write_cursor DESC LIMIT 1`), historical `resolve_seeds`, `filter.rs` inline `(properties > body)` fallback for both `attributes` and `property_predicates`, `structural_dependency_state_as_of`, and production explain helpers.
2. **Layer 2 — Historical Frozen Evidence (`evidence.rs`):**
   - Extend `INTRINSIC_{NODE,EDGE}_PREFLIGHT_SQL`, `query_graph_evidence_batch` (`historical = true` on both `n` and `sn`), `authorize_graph_evidence`, `materialize_graph_evidence_batch` (`source_cursor <= c`), `load_stored` (`sn.logical_id` + head-inactive flags), `payload[7] = 1`, and `resolve_graph_evidence` (`eligible` + `source_eligible` historical inline checks + `artifact_lifecycle.superseded`).
3. **Layer 3 — Cross-SDK Parity & Documentation (`fathomdb-sdk`, `fathomdb`, `src/python/`, `src/ts/`, `dev/interfaces/*`, `docs/*`):**
   - Wire `fathomdb-sdk` (`surface.rs`), Python (`types.py`, `graph.py`, 4 entries in `python_surface_additions.json`, `test_slice38_historical_graph.py`), TypeScript (`graph.ts` with `Number.isSafeInteger` & `null` refusal, `slice38-historical-graph.test.ts`), and update `dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md`, `docs/guides/{graph-retrieval,frozen-evidence}.md`, and `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md`.

### Phase 5.3 — Proportional Verification

- `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p fathomdb-engine --features "operator test-hooks" --locked --test slice38_historical_graph --test slice38_query_plans --test slice60_graph_expand --test slice60_wire --test slice60_fix4_rss --test slice60_fix5_rss --test slice20_graph_evidence --test slice80_history`
- `cargo test -p fathomdb-sdk -p fathomdb -p fathomdb-inspector -p fathomdb-mcp --locked`
- `bash scripts/setup-python-owned.sh --build && .venv/bin/pyright -p src/python && .venv/bin/ruff check src/python && PYTHONPATH=/mnt/data/worktrees/release-0.8.31/.venv/lib/python3.12/site-packages:/mnt/data/worktrees/release-0.8.31/src/python .venv/bin/pytest -o pythonpath= src/python/tests/test_slice38_historical_graph.py src/python/tests/test_slice60_graph_expand.py src/python/tests/test_slice20_graph_evidence.py src/python/tests/test_slice130_package_boundary.py`
- `cd src/ts && npm run build && npm run typecheck && npx tsc -p tsconfig.json && node --test dist/tests/slice38-historical-graph.test.js dist/tests/*graph*.test.js dist/tests/*evidence*.test.js`
- `./scripts/agent-lint-md.sh && ./scripts/agent-lint-docs.sh && python3 scripts/check-public-doc-truth.py && bash scripts/check-design-refs.sh`

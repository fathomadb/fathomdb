---
title: Slice 40 Plan — Historical Graph Reconstruction (HISTORY-GRAPH)
status: DRAFT
target_release: 0.8.31
slice: 40
date: 2026-11-22
---

# Slice 40 Plan — Historical Graph Reconstruction (`HISTORY-GRAPH`)

- **Branch / Worktree:** `release/0.8.31` in `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Depends on:** Slice 10 (`PREWORK-PREP`, `WP-10A`), Slice 20 (`PROPERTY-SCHEMA`), Slice 30 (`STRICT-GRAPH`)
- **Authority:** [`dev/plans/0.8.31/plan.md`](../../plan.md), [`dev/plans/0.8.31/scope.md`](../../scope.md), [`dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md`](../../../../adr/ADR-0.8.31-historical-graph-and-fused-seeding.md), [`dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md`](../../../../adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md), [`dev/adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`](../../../../adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md), [`dev/plans/0.8.31/prework/slice-9-triage.md`](../../prework/slice-9-triage.md)
- **Allocated Prework Items:** `P3-10`, `P3-11`, `P4-09` (read-side `as_of_epoch_ms` consumer), `P4-10` (part 2), `P4-11` (part 2), `P4-13` (Inspector workspace compile hygiene), `P5-06`, `P5-07` (part 3), `P6-06` (part 2)

---

## 1. Goals & Non-Goals

### Goals

1. **Opt-in bitemporal `graph.expand` over immutable write revisions (`R31-03`):**
   - Extend `GraphExpandRequestV1` (`#[non_exhaustive]` from Slice 30) with optional, mutually exclusive knowledge-time bounds: `as_of_write_cursor: Option<u64>` and `as_of_epoch_ms: Option<i64>`, composing orthogonally with world/validity time (`ReadView.valid_as_of` / `FrozenReadContextV1.effective_valid_at` and `[t_valid, t_invalid)` / `[valid_from, valid_until)`).
   - Keep `ReadView` (`src/rust/crates/fathomdb-engine/src/temporal.rs`) strictly **world-time-only** and keep `context.view.include_superseded` / `include_inactive` refused on `graph.expand` (`graph_api.rs:529-531`).
2. **Epoch-to-cursor resolution & Schema 37→38 pre-upgrade semantics (`P4-09`):**
   - Resolve `as_of_epoch_ms` once per reader transaction via Schema 38's `_fathomdb_commit_timestamps_ms_idx`: `SELECT COALESCE(MAX(write_cursor), 0) FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1`.
   - Cursor `0` (including `as_of_epoch_ms` before the first Schema-38 commit on fresh or migrated Schema 37→38 stores) yields an empty historical revision snapshot (`Query` seeds return empty `seeds: []`, `targets: []`; `Explicit` seeds refuse with `GraphSeedUnavailable`), while pre-upgrade and post-upgrade revisions remain directly queryable via `as_of_write_cursor`.
3. **Zero penalty on default current-time traversal & unified historical index `UNION ALL` (`P4-10` part 2, `P5-07` part 3):**
   - Preserve the default unpredicated current-time `graph.expand` path (`superseded_at IS NULL` via `canonical_edges_{from,to}_active_idx`) with zero extra joins, branches, or historical scans.
   - Implement historical incident-edge traversal as mutually exclusive two-arm `UNION ALL` queries per direction (`Both` uses four arms), pushing `write_cursor <= ?as_of`, `superseded_at IS NULL` (active arm, `canonical_edges_{from,to}_active_idx`) vs. `superseded_at > ?as_of` (historical arm, `canonical_edges_{from,to}_sup_kind_idx`), `edge_kinds`, and `[t_valid, t_invalid)` directly into SQL so filtered/pre-superseded/post-`as_of` rows never inflate `LIMIT` or `work_units`.
4. **Synchronized node revision coherence & honest current-state `Engine::transition` semantics:**
   - Bound both seed selection (`Explicit` + `Query` FTS) and visited/target node hydration (`load_node`) to the revision alive at `as_of` (`write_cursor <= ?as_of AND (superseded_at IS NULL OR superseded_at > ?as_of)`), returning historical `kind`, `body`, `properties`, and `write_cursor`, and evaluating `eligibility.attributes` and Slice 30 `target_predicates` / `edge_predicates` against historical row JSON (`body` / `properties`) rather than active-only EAV/property tables.
   - Truthfully reflect amended ADR `#75`: because `Engine::transition` (`record_lifecycle.rs:230-299`) mutates `canonical_nodes.state` in place without allocating a `write_cursor`, current-state lifecycle (`state = 'active'` on both un-superseded and logical-head rows) applies consistently to historical reads.
5. **Strict erasure subordination & historical frozen evidence (`R31-06`, `P4-11` part 2):**
   - Purged (`purge`), source-erased (`erase_source`), and active-closure-barrier (`_fathomdb_dependency_closures.phase != 'complete'`) records never reconstruct at any `as_of` cursor or timestamp (including cursors prior to erasure).
   - Enable `include_evidence: true` with `Frozen` context in historical `as_of` mode (`preflight_graph_evidence`, `encode_graph_evidence_request`, and `resolve_graph_evidence`) while preserving the 704-char `fdbgev1.` wire format and keeping erased, soft-deleted, or barrier-blocked artifacts `EvidenceUnavailable`.
6. **Cross-surface parity, Inspector compile hygiene & doc truth (`P3-10`, `P3-11`, `P4-13`, `P6-06` part 2):**
   - Deliver Rust (`fathomdb-engine`, `fathomdb`, `fathomdb-sdk`), Python (`fathomdb`), TypeScript (`fathomdb`), wire (`codec.rs`), interface doc, public guide, and API reference parity, keeping `src/inspector` compiling cleanly under `cargo clippy --workspace --all-targets`.

### Non-Goals

- **No changes to `ReadView` (`temporal.rs`) or non-graph verbs:** `search`, `search_text_only`, `search_frozen`, `read.get`/`list`, `graph.neighbors`, and `graph.search_expand` do not gain `as_of_write_cursor` / `as_of_epoch_ms`.
- **No transition-history event log:** Do not alter `Engine::transition` to emit synthetic node revisions or backdated transition cursors; immutable write revisions remain the history boundary.
- **No backfill of pre-upgrade commit timestamps:** `fathomdb migrate` (Slice 50) does not synthesize wall-clock timestamps for Schema 37 commits.
- **No Inspector UI work in this slice:** Interactive Inspector graph `as_of` controls and admission to Schema 38 belong to Slice 60 (`INSPECTOR`); Slice 40 keeps `src/inspector` compiling cleanly.
- **No edits to `dev/acceptance.md`:** `dev/acceptance.md` remains LOCKED; Slice 40 uses slice-local `AC31-40-*` IDs mapped to `R31-03` and `R31-06`.

---

## 2. Draft Needs, Requirements & Acceptance Criteria

### Traceability to Release Scope

- **`R31-03` (Historical / Bitemporal Graph Reconstruction):** `GraphExpandRequestV1` `as_of_write_cursor` / `as_of_epoch_ms` + `valid_as_of`, synchronized node/edge revisions, `_fathomdb_commit_timestamps` resolution, dual-index `UNION ALL`, zero penalty on default traversal, and interface/guide updates.
- **`R31-06` (Security, Boundary Hardening & Conformance Invariants):** Closed wire validation, mutual-exclusion refusals, strict subordination to physical erasure and dependency-closure barriers, nondisclosing `EvidenceUnavailable` for erased/barrier-blocked historical evidence, and Rust/Python/TypeScript conformance parity.
- **Cross-Cutting DoDs:** `X1` (Rust/Python/TS parity), `X3` (`EXPLAIN QUERY PLAN` `SEARCH`-only & unpredicated fast-path invariance), `X4` (interfaces, public guides, and API reference truth verified by `mkdocs build --strict` and `check-public-doc-truth.py`).

### Slice-Local Acceptance Criteria

| ID | Criterion | Verification Anchor |
| --- | --- | --- |
| `AC31-40-01` | **Bitemporal 4-quadrant matrix:** Divergent `as_of` (knowledge time) and `valid_as_of` (world time) reconstruct exact pre- and post-supersession chains under `[write_cursor <= as_of < superseded_at]` and `[t_valid <= valid_as_of < t_invalid)`, always excluding `temporal_fallback != 0` edges and honoring `edge_kinds`, `target_kinds`, `edge_predicates`, and `target_predicates` on the historical revision. | `slice38_historical_graph.rs` (Rust), Py/TS parity |
| `AC31-40-02` | **`as_of_epoch_ms` & cursor-`0` pre-upgrade semantics:** `as_of_epoch_ms` resolves via `SELECT COALESCE(MAX(write_cursor), 0) FROM _fathomdb_commit_timestamps WHERE committed_at_ms <= ?1`; cursor `0` (including timestamps prior to the first Schema-38 commit on migrated Schema 37→38 DBs) yields empty `Query` seeds (`targets: []`) and `GraphSeedUnavailable` for `Explicit` seeds, while `as_of_write_cursor` addresses both pre- and post-upgrade revisions. | `slice38_historical_graph.rs` (`P4-09`, `P5-06`) |
| `AC31-40-03` | **Closed request schema & mutual exclusion:** Specifying both `asOfWriteCursor` and `asOfEpochMs` fails deterministically with `GraphContextInvalid` at `/asOfEpochMs`; `asOfWriteCursor` serializes as canonical unsigned base-10 string (`"0"`..`u64::MAX`), `asOfEpochMs` as JSON integer (`i64`), unknown fields refuse with `UnknownField`, and `ReadView.include_superseded` / `include_inactive` remain refused at `/context`. | `slice60_wire.rs`, `slice38_historical_graph.rs`, Py/TS tests |
| `AC31-40-04` | **Node revision coherence & current-state `Engine::transition`:** Explicit/query seeds and traversed targets return `kind`, `body`, `properties`, `write_cursor`, and `origin.terminal_edge_properties` as of `as_of` (ignoring later `write()` edits), while `Engine::transition` to `Pending` or `Deleted` (on either an un-superseded node or a logical identity's active head) removes that entity from historical `as_of` eligibility. | `slice38_historical_graph.rs`, `proptest` oracle |
| `AC31-40-05` | **Erasure & closure-barrier subordination:** `purge`, `erase_source`, and active `_fathomdb_dependency_closures` (`phase != 'complete'`) prevent affected nodes, edges, and derived artifacts from replaying at any `as_of_write_cursor` or `as_of_epoch_ms` (including pre-erasure cursors). | `slice38_historical_graph.rs` (`P5-06`) |
| `AC31-40-06` | **Historical frozen evidence:** `graph.expand(..., Frozen, include_evidence: true, as_of_*)` binds `as_of` in `request_commitment`, mints valid 704-char `fdbgev1.` tokens when artifacts and their canonical sources existed at `as_of` and remain non-erased/active/barrier-free, and both preflight and `resolve_graph_evidence` return `EvidenceUnavailable` once erased, barrier-blocked, or transitioned `Deleted`. | `slice38_historical_graph.rs`, `slice20_graph_evidence.rs` (`P4-11` part 2) |
| `AC31-40-07` | **Pushdown & work-unit accuracy:** Large quantities of pre-`as_of`-superseded, post-`as_of`-written, wrong-`kind`, or out-of-window edges never enter SQL `EdgeRow`s or inflate `work_units` / trip `GraphExpansionBoundExceeded`. | `slice38_historical_graph.rs` (`P4-10`) |
| `AC31-40-08` | **Planner & fast-path invariance:** `tests/slice38_query_plans.rs` verifies `EXPLAIN QUERY PLAN` `SEARCH` plans (zero unexpected `SCAN`) on `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx` (empty and non-empty `edge_kinds`), `_fathomdb_commit_timestamps_ms_idx`, and historical `load_node` (`canonical_nodes_logical_sup_idx`), plus unchanged default current-time SQL plans. | `slice38_query_plans.rs` (`P5-07` part 3) |
| `AC31-40-09` | **Cross-SDK & Inspector compile parity:** Python (`test_slice130_package_boundary.py` + `python_surface_additions.json`) and TypeScript (`fathomdb`) match Rust behavior identically; `cargo clippy --workspace --all-targets -- -D warnings` passes including `src/inspector`. | Py/TS test suites, `cargo clippy --workspace` (`P4-13`) |
| `AC31-40-10` | **Interface, guide & reference alignment:** `dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md`, `docs/guides/{graph-retrieval,frozen-evidence}.md`, and `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md` reflect shipped `as_of` semantics with stale exclusions deleted. | `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, `check-public-doc-truth.py` (`P3-10`, `P3-11`, `P6-06` part 2) |

---

## 3. Architecture & Draft Design

### 3.1 API Carriers, Wire Codec & Request Validation

1. **Rust types (`src/rust/crates/fathomdb-engine/src/graph_expand/types.rs`):**
   - Extend `GraphExpandRequestV1` (marked `#[non_exhaustive]` in Slice 30) with:
     - `pub as_of_write_cursor: Option<u64>` (default `None` in `GraphExpandRequestV1::new`)
     - `pub as_of_epoch_ms: Option<i64>` (default `None` in `GraphExpandRequestV1::new`)
   - Add ergonomic builders `.with_as_of_write_cursor(mut self, cursor: u64) -> Self` and `.with_as_of_epoch_ms(mut self, epoch_ms: i64) -> Self` on `GraphExpandRequestV1` (re-exported via `fathomdb` and `fathomdb-sdk`), keeping existing Slice 30 callers and `src/inspector/src/graph.rs` unaffected (`P4-13`).
   - Do **not** modify `ReadView` (`temporal.rs`) or `GraphReadContextV1` variants.
2. **Closed wire codec (`src/rust/crates/fathomdb-engine/src/graph_expand/codec.rs` & `graph_api.rs`):**
   - Update `RequestWire` and `check_closed` root allowlist with `"asOfWriteCursor"` and `"asOfEpochMs"`, both `#[serde(skip_serializing_if = "Option::is_none")]` so default current-time requests serialize identically to Slice 30.
   - `"asOfWriteCursor"` is serialized as canonical base-10 string (`u64::to_string`) and decoded via `parse_canonical_u64` (matching `maxWorkUnits` / `writeCursor`, refusing non-string, leading-zero, negative, or `> u64::MAX` with `GraphContextInvalid` at `/asOfWriteCursor`; `"0"` is valid).
   - `"asOfEpochMs"` is serialized as JSON integer and decoded via `as_i64` (matching `validAsOf`, refusing non-integer/float/string with `GraphContextInvalid` at `/asOfEpochMs`).
   - **Mutual exclusion:** If both `as_of_write_cursor.is_some()` and `as_of_epoch_ms.is_some()`, both `decode_graph_expand_request_v1` (`codec.rs`) and `validate_semantics` (`graph_api.rs`) return `GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphContextInvalid, "/asOfEpochMs")`.
   - `encode_graph_evidence_request(request)` (`execution.rs:1114`): Append canonical history framing bytes (`0u8` for current-time; `1u8` + `encode_u64(cursor)` for `as_of_write_cursor`; `2u8` + `encode_i64(epoch_ms)` for `as_of_epoch_ms`) so `request_commitment` distinctly binds every historical query mode and cursor/epoch value.
3. **Python & TypeScript carriers:**
   - **Python (`src/python/fathomdb/{types.py,graph.py,_sdk_graph.py}`):** Add `as_of_write_cursor: str | None = None` and `as_of_epoch_ms: int | None = None` to `GraphExpandRequestV1` (`types.py:676`). Enforce canonical decimal `u64` string validation, `isinstance(as_of_epoch_ms, int) and not isinstance(as_of_epoch_ms, bool)` signed 64-bit bounds, and mutual exclusion (`/asOfEpochMs`) in Python normalization before invoking `_native.graph_expand`. Update `src/python/tests/python_surface_additions.json` cleanly while leaving `_fathomdb.pyi` untouched (`_native.graph_expand` crosses FFI via JSON string).
   - **TypeScript (`src/ts/src/{graph.ts,index.ts}`):** Add `asOfWriteCursor?: string` and `asOfEpochMs?: number` to `GraphExpandRequestV1` (`graph.ts:93`). Extend `validateGraphExpandRequest` (matching `maxWorkUnits` regex `/^(?:0|[1-9][0-9]*)$/` <= `0xffff_ffff_ffff_ffffn`, `Number.isInteger(asOfEpochMs)`, mutual exclusion `/asOfEpochMs`) and `canonicalGraphRequest` (emit only when defined).

### 3.2 Snapshot History Cursor Resolution (`execution.rs`)

Inside `read_graph_expand_with_stop_words_in_tx` (`execution.rs:709`), immediately after pinning the reader transaction and running `validate_snapshot`:

```rust
enum ResolvedHistoryCursor {
    Current,
    AsOf(u64),
}
```

- `(None, None)` -> `ResolvedHistoryCursor::Current` (zero SQL overhead).
- `(Some(cursor), None)` -> `ResolvedHistoryCursor::AsOf(cursor)` (zero lookup SQL).
- `(None, Some(epoch_ms))` -> execute once in the pinned reader tx:

```sql
SELECT COALESCE(MAX(write_cursor), 0)
FROM _fathomdb_commit_timestamps
WHERE committed_at_ms <= ?1
```

This executes a single reverse seek on `_fathomdb_commit_timestamps_ms_idx(committed_at_ms, write_cursor)`.

- **Batch cursor invariant:** Because Slice 20 populates `_fathomdb_commit_timestamps.write_cursor` with each committed batch's high-water `WriteReceipt.cursor` (`max(row_cursors)`) and monotonic `committed_at_ms`, every row inserted in that batch satisfies `row.write_cursor <= resolved_cursor`, every later write satisfies `row.write_cursor > resolved_cursor`, and every later supersession stamped `superseded_at` satisfies `superseded_at > resolved_cursor`.
- **Cursor-`0` behavior:** When `resolved_cursor == 0` (either `as_of_write_cursor == Some(0)` or `as_of_epoch_ms` precedes the first recorded Schema-38 commit, including pre-upgrade rows on migrated 37→38 DBs), no revision has `write_cursor <= 0`. `Query` seeds yield `Ok(GraphExpandResultV1 { seeds: [], targets: [], ... })`; `Explicit` seeds refuse via `load_node` with `GraphSeedUnavailable` at `/seed/logicalIds/0`.

### 3.3 Historical Incident-Edge `UNION ALL` & SQL Pushdown (`execution.rs`)

- **Default path (`ResolvedHistoryCursor::Current`):** Remains byte-identical to Slice 30 (`superseded_at IS NULL` on `canonical_edges_{from,to}_active_idx`, `UNION ALL` for `Both`, `[t_valid, t_invalid)` pushdown against `view.edge_now()`, optional `edge_kinds` and `edge_predicates` evaluation).
- **Historical path (`ResolvedHistoryCursor::AsOf(as_of)`):**
  Build mutually exclusive two-arm `UNION ALL` SQL per direction (`Outgoing`, `Incoming`, or 4 arms for `Both`):
  - **Active arm (`canonical_edges_{from,to}_active_idx`):**

```sql
SELECT write_cursor, kind, from_id, to_id, logical_id, superseded_at, t_valid, t_invalid, properties
FROM canonical_edges
WHERE {from_id | to_id} = ?1
  AND superseded_at IS NULL
  AND write_cursor <= ?as_of
  [AND kind IN (...)]
  AND (t_valid IS NULL OR t_valid <= ?edge_now)
  AND (t_invalid IS NULL OR t_invalid > ?edge_now)
  AND (temporal_fallback IS NULL OR temporal_fallback = 0)
```

- **Historical arm (`canonical_edges_{from,to}_sup_kind_idx`):**

```sql
SELECT write_cursor, kind, from_id, to_id, logical_id, superseded_at, t_valid, t_invalid, properties
FROM canonical_edges
WHERE {from_id | to_id} = ?1
  AND superseded_at > ?as_of
  AND write_cursor <= ?as_of
  [AND kind IN (...)]
  AND (t_valid IS NULL OR t_valid <= ?edge_now)
  AND (t_invalid IS NULL OR t_invalid > ?edge_now)
  AND (temporal_fallback IS NULL OR temporal_fallback = 0)
```

- Wrap the `UNION ALL` arms in `SELECT * FROM (...) LIMIT ?limit` (`remaining + 1`), keeping existing deterministic sorting (`origin_cmp` / edge sort at `execution.rs:844-866`) and work-unit metering unchanged.
- Evaluate Slice 30 `edge_predicates` inline against the fetched `EdgeRow.properties` JSON (prior to incrementing frontier/candidate eligibility or consuming displacement slots, mirroring Slice 30).
- Replace the unconditional `if edge.superseded_at.is_some()` skip at `execution.rs:882` with history-aware enforcement (`ResolvedHistoryCursor::Current` enforces `superseded_at.is_none()`, `AsOf(c)` enforces `write_cursor <= c && superseded_at.is_none_or(|s| s > c)`).

### 3.4 Synchronized Node Hydration, Eligibility & `Engine::transition` (`execution.rs`, `filter.rs`, `dependency_closure.rs`)

Both `load_node` (`execution.rs:454`) and `resolve_seeds` (`execution.rs:602`, Explicit & Query FTS) apply synchronized node rules when `ResolvedHistoryCursor::AsOf(as_of)`:

1. **Revision interval (as a two-arm `UNION ALL` + `ORDER BY write_cursor DESC LIMIT 1` in `load_node` backed by `canonical_nodes_logical_sup_idx` in Schema 38):**
   - Arm 1 (un-superseded): `n.superseded_at IS NULL AND n.write_cursor <= ?as_of` -> uses `canonical_nodes_logical_active_idx`.
   - Arm 2 (superseded after `as_of`): `n.superseded_at > ?as_of AND n.write_cursor <= ?as_of` -> uses `canonical_nodes_logical_sup_idx`.
   - In `resolve_seeds` `GraphSeedV1::Query`, `search_index` already joins `canonical_nodes cn ON cn.write_cursor = search_index.write_cursor` (and retains superseded node revisions until purge, as established in `slice80_history.rs:208`), so bounding `cn.write_cursor <= ?as_of AND (cn.superseded_at IS NULL OR cn.superseded_at > ?as_of)` selects each logical node's revision alive at `as_of` in deterministic BM25 + cursor order before `seen.insert(logical_id)` deduplication.
2. **Honest current-state `Engine::transition` lifecycle:**
   - Keep `n.state = 'active'` (catches un-superseded nodes transitioned to `Pending`/`Deleted` and derived nodes tombstoned by `dependency_closure::apply_soft_effects`).
   - Also reject revisions whose logical head was transitioned out of `active` in place (`record_lifecycle.rs:230` only updates `WHERE superseded_at IS NULL`):

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
   - Add `dependency_closure::read_eligibility_sql_historical_for_cursor(cursor_expr, as_of_idx, now_idx, include_out_of_window)` for derived nodes: requires `source_node` to physically exist at `source_node.write_cursor <= ?as_of AND (source_node.superseded_at IS NULL OR source_node.superseded_at > ?as_of)`, satisfy `source_node.state = 'active'` (plus current-head active check), and satisfy source node validity unless `include_out_of_window`.
4. **Row-JSON attribute & property predicate fallback (`filter.rs`):**
   - Extend `filter::append_node_eligibility_sql_view` (`filter.rs:654`) with explicit historical awareness (`historical: bool` parameter or helper) so `eligibility.attributes` evaluates `scalar_attribute_sql` against historical row JSON (`{alias}.body`) whenever `historical == true`, bypassing active-only `canonical_attributes` (whose superseded rows are purged in `write_commit.rs:686-692`).
   - Evaluate Slice 30 `target_predicates` inline against the historical revision's `canonical_nodes.properties` JSON, never active-only `canonical_property_values`.
5. **Hydrated output fields:** Ensure `load_node` returns `kind`, `body`, historical `properties`, and `write_cursor` from the `as_of` revision row so `GraphTargetV1` carries exact point-in-time node data alongside `origin.terminal_edge_properties`.

### 3.5 Historical Provenance & `evidence.rs` (`P4-11` part 2)

1. **Preflight authorization (`evidence.rs`):**
   - Thread `history_as_of: Option<u64>` from `read_graph_expand_with_stop_words_in_tx` into `preflight_graph_evidence(&tx, frozen, history_as_of, &node_cursors, &edge_cursors)`.
   - When `history_as_of == None`: behavior unchanged.
   - When `history_as_of == Some(c)`:
     - `authorize_graph_evidence` (`L1603`) and `materialize_graph_evidence_batch` (`L1668`) admit `artifact_superseded_at` and `source_superseded_at` iff `write_cursor <= c` and `(superseded_at IS NULL || superseded_at > c)`, while still strictly requiring `n.state == "active"`, `sn.state == "active"`, current logical head active for both target and source (`!EXISTS(cur.state != 'active')`), `temporal_fallback == 0`, `[valid_from, valid_until)` / `[t_valid, t_invalid)` at `frozen.effective_valid_at`, and `!closure_active`.
     - `query_graph_evidence_batch` (`L1559-1576`) passes `historical = true` to `append_node_eligibility_sql_view` for both `TARGET_ELIGIBILITY` (`n`) and `SOURCE_ELIGIBILITY` (`sn`) so superseded historical revisions with attribute filters match row JSON rather than purged `canonical_attributes`.
2. **Opaque token framing & `resolve_graph_evidence` (`evidence.rs:2049-2429`):**
   - Preserve `GRAPH_SELECTOR_BYTES = 300` and `GRAPH_TOKEN_BYTES = 704` (`fdbgev1.` + 696 hex chars) so `dev/interfaces/wire.md:196` and `GraphEvidenceResolveRequestV1` remain fully compatible.
   - Use reserved selector byte `payload[7]` (`evidence.rs:2096, 2201`): `0` = current-time minted reference, `1` = historical `as_of` minted reference (bound via `request_commitment` in `payload[108..140]`).
   - In `resolve_graph_evidence` (`L2229`) and `authorize_graph_intrinsic_lifecycle` (`L924`):
     - When `payload[7] == 0`: strictly unchanged.
     - When `payload[7] == 1`: allow `artifact_superseded` / `source_superseded` ONLY for retained active revisions, while unconditionally returning `EvidenceUnavailable` if the artifact or source row was physically deleted (`purge` / `erase_source`), has an active closure barrier (`active_barrier_for_source`), has `artifact_state != Active` or `source_state != Active`, or has a current `superseded_at IS NULL` logical head with `state != 'active'`.

### 3.6 Interfaces, Guides, Reference & Inspector Hygiene (`P3-10`, `P3-11`, `P4-13`, `P6-06` part 2)

- **`dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md` (`P3-10`):** Document `asOfWriteCursor` / `as_of_write_cursor`, `asOfEpochMs` / `as_of_epoch_ms`, mutual exclusion -> `GraphContextInvalid` at `/asOfEpochMs`, `ReadView` world-time-only invariant, `_fathomdb_commit_timestamps` + cursor-`0` pre-upgrade semantics, `Engine::transition` current-state semantics, erasure subordination, and `fdbgev1.` historical evidence behavior.
- **`docs/guides/graph-retrieval.md` (`P3-11`, `P6-06` part 2):** Delete the unconditional claim (`L31-32`) that superseded edges are always excluded from `graph.expand`; document bitemporal `as_of_write_cursor` / `as_of_epoch_ms` + `valid_as_of` reconstruction, pre-upgrade `as_of_epoch_ms` cursor-`0` behavior, current-state `Engine::transition`, and physical erasure subordination.
- **`docs/guides/frozen-evidence.md` & `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md` (`P3-11`):** Document historical frozen evidence and `GraphExpandRequestV1` `as_of` fields across all three SDKs.
- **Inspector (`P4-13`):** Keep `src/inspector/src/{graph,lexical}.rs` building warning-free via `GraphExpandRequestV1::new` defaults (`as_of_* = None`).

---

## 4. Prerequisites Discovered

Flagged explicitly for Slice 20/30 handoff checks and Slice 40 entry verification:

1. **Upstream Slice 20 & Slice 30 deliverables required at entry:**
   - Slice 20 must have landed `_fathomdb_commit_timestamps` + `_fathomdb_commit_timestamps_ms_idx`, `canonical_nodes.properties` / `canonical_edges.properties`, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx`, and **`canonical_nodes_logical_sup_idx(logical_id, superseded_at, write_cursor)`** (folded into Schema 38 per Prerequisite #2 below!).
   - Slice 30 must have marked `GraphExpandRequestV1` `#[non_exhaustive]` with `GraphExpandRequestV1::new(...)`, landed inline `properties` on `GraphTargetV1` / `terminal_edge_properties` on `GraphOriginV1`, refactored `graph_expand/execution.rs` SQL pushdown (`UNION ALL` for `Both`, `[t_valid, t_invalid)`), created `tests/slice38_query_plans.rs`, and migrated `src/inspector/src/graph.rs` away from literal construction (`P4-13`).
2. **Concrete Code Seam — Missing Non-Partial `canonical_nodes(logical_id, ...)` Index (`fathomdb-schema/src/lib.rs:375`):**
   - `canonical_nodes` currently has ONLY a partial index keyed by `logical_id`: `canonical_nodes_logical_active_idx ON canonical_nodes(logical_id) WHERE superseded_at IS NULL`.
   - Adding `CREATE INDEX IF NOT EXISTS canonical_nodes_logical_sup_idx ON canonical_nodes(logical_id, superseded_at, write_cursor)` directly in Schema 38 (Slice 20 + ADRs `#73`/`#75`/`#76`) lets historical `load_node` run as a zero-`SCAN` two-arm `UNION ALL` pinned by `EXPLAIN QUERY PLAN` in `tests/slice38_query_plans.rs`.
3. **Concrete Code Seam — Active-Only EAV/Property Projection Pruning (`filter.rs:661`, `write_commit.rs:686-692`):**
   - Because `ReadView.include_superseded` stays `false` on `graph.expand`, `load_node`, `resolve_seeds`, and `evidence.rs` must pass an explicit `historical` override so attribute filters and `target_predicates` evaluate against historical row JSON (`body` / `properties`) instead of purged `canonical_attributes`/`canonical_property_values`.
4. **Concrete Code Seam — `dependency_closure` & `evidence.rs` Self-Sourced Supersession Rejection (`dependency_closure.rs:1046-1105`, `evidence.rs:883-922, 1603-1891, 2229-2429`):**
   - For any self-sourced canonical node updated *after* `as_of`, BOTH `n.superseded_at` and `sn.superseded_at` are non-NULL (`> as_of`), so historical authorization must check `write_cursor <= c && (superseded_at IS NULL || superseded_at > c)` rather than `superseded_at IS NULL`.
5. **Prework vs. Amended ADR Reconciliation (`prework/slice-5.md:88` vs. `ADR-0.8.31 #75` `WP-10A`):**
   - Slice 40 tests enforce the amended ADR `#75` contract (current-state `Engine::transition` applies to historical reads, while immutable `write()` content/property revisions replay cleanly at `as_of`).
6. **Execution & Packaging Seams (`execution.rs:882`, `test_slice130_package_boundary.py`):**
   - Replace the unconditional `if edge.superseded_at.is_some()` guard at `execution.rs:882`.
   - Register Python `types.py` additions in `src/python/tests/python_surface_additions.json`.

---

## 5. TDD Implementation & Verification Plan

Execute strictly RED → GREEN → Refactor against real SQLite stores (no mocks).

### Phase 5.1 — RED Tests First

1. **Wire & Closed-Codec Tests (`slice60_wire.rs`, `slice38_historical_graph.rs`):** canonical `"asOfWriteCursor"` / `"asOfEpochMs"` round-trip, mutual exclusion `GraphContextInvalid` at `/asOfEpochMs`, `ReadView.include_superseded` still refused at `/context`.
2. **Bitemporal 4-Quadrant Matrix (`slice38_historical_graph.rs`, `P5-06`):** divergent `as_of` vs `valid_as_of`, `[write_cursor <= c < superseded_at]`, `[t_valid <= valid_as_of < t_invalid)`, `temporal_fallback = 0`, `edge_predicates`/`target_predicates`, hydrated historical `body`/`properties`, and current-state `Engine::transition`.
3. **`as_of_epoch_ms` & Cursor-`0` / Upgraded-Store Semantics (`slice38_historical_graph.rs`, `P4-09`):** exact `_fathomdb_commit_timestamps` resolution + migrated Schema 37→38 pre-upgrade cursor-`0` empty behavior vs. `as_of_write_cursor` access.
4. **Property-Based Snapshot Oracle (`proptest` in `slice38_historical_graph.rs`, `P5-06`):** random writes/supersessions/transitions compared against exact snapshot BFS oracle.
5. **Erasure Subordination & Work-Unit Bounds (`slice38_historical_graph.rs`, `P5-06`, `P4-10`):** `purge`/`erase_source` non-resurrection at pre-erasure cursors, active closure barriers, 500 pre-`as_of` superseded edges + `max_work_units = 2` pushdown proof.
6. **Historical Frozen Evidence (`slice38_historical_graph.rs`, `slice20_graph_evidence.rs`, `P4-11` part 2):** `include_evidence: true` + `resolve_graph_evidence` across superseded self-sourced nodes/edges and `EvidenceUnavailable` after erasure/barrier/transition.
7. **`EXPLAIN QUERY PLAN` Pins (`tests/slice38_query_plans.rs`, `P5-07` part 3):** zero-`SCAN` `SEARCH` plans on `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_sup_kind_idx` (empty & non-empty `edge_kinds`), `_fathomdb_commit_timestamps_ms_idx`, and `canonical_nodes_logical_sup_idx`, plus unchanged default current-time plans.
8. **Python, TypeScript & SDK Parity:** `src/python/tests/test_slice38_historical_graph.py`, `src/ts/tests/slice38-historical-graph.test.ts`, `test_slice130_package_boundary.py`, `fathomdb-sdk` tests.

### Phase 5.2 — GREEN Implementation Sequence

1. Verify `canonical_nodes_logical_sup_idx` in Schema 38 (`fathomdb-schema`) and `tests/slice38_query_plans.rs`.
2. Add `as_of_write_cursor` / `as_of_epoch_ms` + builders to `GraphExpandRequestV1`, extend `codec.rs`, `graph_api.rs`, and `encode_graph_evidence_request`; verify `fathomdb-inspector` compiles cleanly (`P4-13`).
3. Implement `_fathomdb_commit_timestamps` resolution, historical `UNION ALL` incident SQL, historical `load_node` / `resolve_seeds`, and `filter.rs` historical row-JSON evaluation in `execution.rs` + `dependency_closure.rs`.
4. Implement historical `preflight_graph_evidence`, `payload[7] = 1` marker, and historical `resolve_graph_evidence` in `evidence.rs`.
5. Implement `fathomdb-sdk`, `fathomdb`, Python (`types.py`, `graph.py`, `_sdk_graph.py`, `python_surface_additions.json`), and TypeScript (`graph.ts`, `index.ts`) parity.
6. Update `dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md`, `docs/guides/{graph-retrieval,frozen-evidence}.md`, and `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md`.

### Phase 5.3 — Proportional Verification

- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p fathomdb-engine --features test-hooks --test slice38_historical_graph --test slice38_query_plans --test slice60_graph_expand --test slice60_wire --test slice20_graph_evidence --test slice80_history`
- `cargo test -p fathomdb-sdk -p fathomdb`
- `bash scripts/setup-python-owned.sh --build && .venv/bin/pyright -p src/python && .venv/bin/ruff check src/python && .venv/bin/pytest src/python/tests/test_slice38_historical_graph.py src/python/tests/test_slice60_graph_expand.py src/python/tests/test_slice20_graph_evidence.py src/python/tests/test_slice130_package_boundary.py`
- `cd src/ts && npm run build && npm test -- tests/slice38-historical-graph.test.ts tests/slice60-graph-expand.test.ts tests/slice20-graph-evidence.test.ts`
- `./scripts/agent-lint-md.sh && ./scripts/agent-lint-docs.sh && python3 scripts/check-public-doc-truth.py && bash scripts/check-design-refs.sh`

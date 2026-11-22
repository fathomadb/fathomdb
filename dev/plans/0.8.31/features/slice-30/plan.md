---
title: Slice 30 Plan — Strict Endpoint Integrity, Typed Traversal Predicates & Opt-In Fused-Hit Seeding
status: DRAFT
target_release: 0.8.31
slice: 30
date: 2026-11-22
---

# Slice 30 Plan — Strict Endpoint Integrity, Typed Traversal Predicates & Opt-In Fused-Hit Seeding (`STRICT-GRAPH`)

- **Branch / Worktree:** `release/0.8.31` (isolated implementer worktree off `release/0.8.31` after Slice 20 closes)
- **Depends on:** Slice 20 (`PROPERTY-SCHEMA` — Schema 38 inline `properties`, `canonical_property_values`, `canonical_edges_{from,to}_active_idx`, `PropertyValue`/`PropertyMap`, `SearchFilter` property predicates, Inspector `fixtures/mod.rs` helpers)
- **Authority:** [`dev/plans/0.8.31/plan.md`](../../plan.md) (`R31-02`, `R31-04`, `R31-06`, X1–X4), [`dev/plans/0.8.31/scope.md`](../../scope.md), [`dev/adr/ADR-0.8.31-strict-endpoint-integrity.md`](../../../../adr/ADR-0.8.31-strict-endpoint-integrity.md), [`dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md`](../../../../adr/ADR-0.8.31-historical-graph-and-fused-seeding.md), [`dev/plans/0.8.31/prework/slice-9-triage.md`](../../prework/slice-9-triage.md)
- **Tracked Prework IDs:** `P2-05`, `P3-02`, `P3-08`, `P3-09`, `P4-07`, `P4-08`, `P4-10` (part 1: SQL pushdown, `UNION ALL`, `t_valid`, `L895` early-skip fix, `encode_graph_evidence_request`), `P4-11` (part 1: `search.rs` Seed Source A slot fix + Phase 2 `UNION ALL` + `edge_kinds`/`edge_predicates`), `P4-13` (part 2: `#[non_exhaustive]` `GraphExpandRequestV1::new` + 3 Inspector dangling-edge fixtures), `P5-04`, `P5-05`, `P5-07` (part 2), `P6-02`, `P6-04`, `P6-06`

---

## 1. Goals & Non-Goals

### Goals

1. **Strict-only write endpoint integrity (`R31-04`, `P4-07`, `P4-08`, `P2-05`):**
   - Enforce atomic endpoint existence and active lifecycle integrity (`superseded_at IS NULL AND state = 'active'`) across **every** edge written via `Engine::write` and `Engine::actuate`, checking **all** edges in a batch (including edges superseded later in the same batch under either G0 `logical_id` or G11 `(from_id, to_id, kind)` rules).
   - Return typed `EngineError::WriteValidation` (`WriteValidationError` in Python/TypeScript, `ErrorKind::InvalidInput` / write validation in `fathomdb-sdk`) on violation with zero partial state committed across canonical tables, revisions, property indexes, FTS, or projection queues after close and reopen, while keeping same-batch forward references (`[Edge(A -> B), Node(A), Node(B)]`), active self-loops, `actuate` same-batch edge-before-node / `ActuationOutcomeV1::Refused` (`ReferenceUnavailable`), and the node-only zero-SQL-read `Authorizer` fast path intact.
   - Keep `WriteReceipt::dangling_edge_endpoints` on the receipt struct returning `0` on every committed `0.8.31` batch.

2. **`GraphExpandRequestV1` `#[non_exhaustive]` + `::new(...)` & Closed Typed Traversal Predicates (`R31-02`, `P4-10`, `P4-13`):**
   - Mark `GraphExpandRequestV1` `#[non_exhaustive]` with constructor `GraphExpandRequestV1::new(...)` and additive builder helpers so Slice 40 (`as_of`) and `0.8.32` additions stay source-compatible; update all workspace callers (`src/inspector/src/graph.rs`, `fathomdb-mcp`, `evidence.rs`, `codec.rs`, tests).
   - Add closed typed `edge_predicates` and `target_predicates` (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte` over `PropertyValue` `String | Int | Float | Bool`; implicit `AND`; zero SQL, zero `OR`/nesting) on `GraphExpandRequestV1`, evaluated before an edge can enter `visited_by_seed`/frontier or displace a candidate target.
   - Hydrate inline `properties` on `GraphTargetV1` and revision-bound `terminal_edge_properties` on `GraphOriginV1` (including exact revision binding on `resolve_graph_evidence` and closed wire/commitment authentication in `codec.rs` + `encode_graph_evidence_request`).

3. **Current-Time `graph_expand` SQL Pushdown, `UNION ALL`, `t_valid` Gating & Terminal-Skip Fix (`R31-02`, `P4-10`, `X3`):**
   - Push `superseded_at IS NULL`, `kind IN (...)`, `(t_valid IS NULL OR t_valid <= ?now)`, and `(t_invalid IS NULL OR t_invalid > ?now)` directly into `graph_expand_incident_sql`, rewriting `TraversalDirection::Both` as `UNION ALL` backed by Schema 38 partial active incident indexes (`canonical_edges_from_active_idx`, `canonical_edges_to_active_idx`), preventing superseded/out-of-window/wrong-kind edges from consuming incident-edge `LIMIT` or `work_units`.
   - Fix `t_valid` omission on edges and eliminate unsound ordering/early-skip hazards around `visited_by_seed`, `L895`, and `next_frontier.push` (`L905`) while retaining bounded RSS (`slice60_fix4_rss`, `slice60_fix5_rss`).

4. **Opt-In `use_graph_arm` Controls, Seed Provenance & Mixed-Edge `SEED_FTS_N` Fix (`R31-02`, `P4-11`):**
   - Fix Seed Source A in `bfs_graph_arm_candidates` (`search.rs`) so edges bridging an eligible endpoint and an ineligible endpoint never emit or reserve a `SEED_FTS_N = 10` slot on behalf of the ineligible endpoint, and rewrite Phase 2 edge lookups as `UNION ALL` with `graph_edge_kinds` and `graph_edge_predicates` support.
   - Add opt-in `graph_seed` (`"graph_fts"` [default] | `"fused_top_k"` | `"both"`), `graph_seed_k` (default `10`, bounded), `graph_edge_kinds`, and `graph_edge_predicates` on search requests (`Engine::search`, `search_frozen`, `search_with_evidence`) across Rust, Python, and TypeScript, exposing per-hit `graph_seed_source` (`"fused_top_k" | "edge_fts" | "entity_fts"`) and `graph_hop_depth` on explanation (`PerHitExplain`) and evidence sidecars (`EvidenceGraphOriginV1` / `EvidenceProjectionOriginV1`), while keeping `use_graph_arm = false` byte-identical with zero added SQL/joins.

5. **Legacy, Convenience-Edge & Inspector Fixture Updates + SDK/Doc Parity (`P2-05`, `P3-02`, `P3-08`, `P3-09`, `P4-08`, `P4-13`, `P5-04`, `P5-05`, `P5-07`, `P6-02`, `P6-04`, `P6-06`, `X1`–`X4`):**
   - Rewrite legacy dangling-edge tests, inactive-intermediate tests, and pre-`0.8.31` `slice60_graph_expand.rs` expectations (`work_units` on superseded edges, `t_valid` gating, Schema 38 active index plans); seed missing endpoint nodes in all ~25 convenience-edge tests across Rust, Python, and TypeScript; update the 3 Inspector dangling-edge fixtures (`add_graph`, `add_graph_missing_prefix`, `add_graph_highdegree`) cleanly.
   - Keep `python_surface_additions.json` + `test_slice130_package_boundary.py` (`native_stub_sha256`), `fathomdb-sdk/tests/surface.rs`, `governed_surface.rs`, `sdk-surface-parity.test.ts`, `dev/interfaces/*`, and `docs/*` in lockstep, strictly in product-generic FathomDB terminology, and prove physical erasure leaves zero `GraphTargetV1.properties`/`terminal_edge_properties` or evidence leaks.

### Non-Goals (Explicitly Out of Scope for Slice 30)

- **Slice 40 (`HISTORY-GRAPH`):** `as_of_write_cursor`, `as_of_epoch_ms`, `known_as_of` node/edge historical replay, `_fathomdb_commit_timestamps` resolution, `canonical_edges_{from,to}_sup_kind_idx` historical query plans, and historical `evidence.rs` revision authorization are excluded (Slice 30 keeps `graph_expand` SQL/request carriers cleanly extensible for Slice 40).
- **Slice 50 (`SDK-CLI-UPGRADE`):** `resolve_evidence_many`, `search(hints=)`, `OpenReport` capability booleans, `fathomdb migrate` 37->38, and `fathomdb-cli` PyPI wheel packaging.
- **Slice 60 (`INSPECTOR`):** Inspector Schema 38+ property/history UI panels and `local-ci.sh` decoupling (Slice 30 only updates Inspector struct literals for `GraphExpandRequestV1::new(...)` and the 3 dangling-edge fixtures so workspace builds and Inspector tests remain green).
- **Deferred to `0.8.32` / canonical docs:** Opt-in loose/bulk import (`dangling_edge_endpoints > 0` mode), caller `scope` filtering, `OR`/nested boolean predicate trees, and any edits to locked `dev/acceptance.md`.

---

## 2. Draft Needs, Requirements & Acceptance Criteria

`dev/acceptance.md` stays **LOCKED** (no new `AC-NNN` identifiers). Slice 30 binds release requirements **`R31-02`**, **`R31-04`**, **`R31-06`**, and cross-cutting DoDs **`X1`–`X4`** to concrete slice-local acceptance criteria **`AC31-30-01`..`AC31-30-10`**:

### Needs Derived from Release Requirements

- **Strict write integrity (`R31-04`):** Embedded applications cannot risk silently storing edges pointing at non-existent, superseded, `Pending`, or `Deleted` entities in `Engine::write` while `Engine::actuate` rejects them; all `0.8.31` write paths must fail closed atomically.
- **Expressive bounded local graph filtering & hybrid graph seeding (`R31-02`):** Applications using outside-`body` properties on nodes and edges need closed typed predicates on `graph.expand` and `use_graph_arm` without writing SQL, need vector/reranked hits to seed body-less property-graph edges opt-in with auditable provenance, and need mixed-edge FTS matches not to waste `SEED_FTS_N` slots on ineligible counterparties.
- **Zero hidden performance or erasure regressions (`R31-06`, `X2`, `X3`):** Unpredicated `use_graph_arm=false` search and node-only writes must incur zero additional SQL reads or joins; `graph.expand` must use Schema 38 partial active incident indexes without `SCAN` or wasted `work_units`; `purge`/`erase_source` must never reconstruct or leak inline properties through graph outputs or evidence.

### Acceptance Criteria (`AC31-30-*`)

| Slice AC | Maps to | Concrete Acceptance Oracle |
| --- | --- | --- |
| `AC31-30-01` | `R31-04` | `Engine::write` and `Engine::actuate` reject edges referencing missing, superseded-only, `InitialState::Pending`, `LifecycleState::Pending`, `LifecycleState::Deleted`, or in-batch-invalidated/deleted endpoints with `EngineError::WriteValidation` / `WriteValidationError` (`ActuationOutcomeV1::Refused { reason: ReferenceUnavailable }` on `actuate`) and zero persisted rows across all canonical/derived tables after `close()` + reopen; same-batch edge-before-active-node batches and active self-loops succeed with `dangling_edge_endpoints == 0`; syntactic/provenance refusal order remains unchanged. |
| `AC31-30-02` | `R31-04`, `X3` | In-batch superseded dangling edges (both G0 `logical_id` and G11 `(from_id, to_id, kind)` supersession) are caught and refused in `Engine::write`; SQLite `Authorizer` confirms node-only batches execute zero `canonical_nodes` reads; `EXPLAIN QUERY PLAN` verifies endpoint probe uses `canonical_nodes_logical_active_idx` via `SEARCH`. |
| `AC31-30-03` | `R31-02`, `X3` | Current-time `graph_expand` pushes `superseded_at IS NULL`, `kind`, `t_valid`, and `t_invalid` into SQL and splits `Both` into `UNION ALL` (without duplicating self-loops); superseded, wrong-kind, expired (`t_invalid <= now`), and future-valid (`t_valid > now`) edges consume zero `LIMIT`/`work_units`; edges failing `edge_predicates` do not poison `visited_by_seed` for parallel edges; `L895` terminal skip never drops a strictly better `origin_cmp` target (`L942-955`) while keeping O(1) RSS (`slice60_fix4_rss`, `slice60_fix5_rss`); `EXPLAIN QUERY PLAN` verifies `canonical_edges_{from,to}_active_idx` `SEARCH` with zero `SCAN`. |
| `AC31-30-04` | `R31-02`, `X1` | `GraphExpandRequestV1` is `#[non_exhaustive]` with `::new(...)`, accepting closed `edge_predicates` and `target_predicates` (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte` over `String`, `Int`, `Float`, `Bool`); missing keys evaluate `false` for all 6 operators, cross-type comparisons evaluate `false` (no coercion), `Int` comparisons preserve exact `i64` precision (`> 2^53`), `Bool` ordering (`Lt..Gte`) / malformed predicates return typed `GraphExpansionErrorV1`; ineligible edges/targets never enter paths or displace `result_limit` at depths `1..=3` or distort `max_work_units`; `GraphTargetV1.properties`, `GraphOriginV1.terminal_edge_properties`, and `resolve_graph_evidence` match exact revision properties and pass closed wire + `encode_graph_evidence_request` commitment authentication. |
| `AC31-30-05` | `R31-02` | `proptest` disabled-optimization equivalence proves identical target sets, origins, and hydrated properties over random multigraphs and predicate combinations whether property keys are declared `Filterable` (indexed B-tree `canonical_property_values` path) or undeclared (inline evaluation). |
| `AC31-30-06` | `R31-02` | Adversarial corpus with `>= 12` FTS-matching edges where top-BM25 edges touch ineligible endpoints (`Pending`/`Deleted`/filter-excluded) proves ineligible endpoints claim zero slots and all `SEED_FTS_N = 10` Seed Source A slots go to distinct eligible endpoints without starving later clean matches; Phase 2 traversal uses `UNION ALL` + `graph_edge_kinds`/`graph_edge_predicates` and `canonical_edges_{from,to}_active_idx` `SEARCH`. |
| `AC31-30-07` | `R31-02`, `X3` | Opt-in `graph_seed` (`GraphFts` [default], `FusedTopK`, `Both`), `graph_seed_k` (`1..=100`, default `10`), `graph_edge_kinds`, and `graph_edge_predicates` seed body-less property-graph edges from fused/reranked hits (`logical_id IS NOT NULL`), degrade cleanly without embedder or reranker, merge `Both` deterministically (min-hop first, canonical tie-break), expose `graph_seed_source` (`"fused_top_k"`, `"edge_fts"`, `"entity_fts"`) + `graph_hop_depth` on `PerHitExplain` and `EvidenceGraphOriginV1`/`EvidenceProjectionOriginV1`, and leave `use_graph_arm = false` byte-identical with zero extra SQL/joins. |
| `AC31-30-08` | `X1` | `fathomdb-sdk`, `fathomdb-py`, `src/python`, `fathomdb-napi`, `src/ts`, `python_surface_additions.json`, `test_slice130_package_boundary.py` (`native_stub_sha256`), `fathomdb-sdk/tests/surface.rs`, `fathomdb/tests/slice60_governed_surface.rs`, and `src/ts/tests/sdk-surface-parity.test.ts` match in lockstep (`pyright` 0/0, `ruff` clean, `tsc --noEmit` clean). |
| `AC31-30-09` | `R31-04` | Legacy dangling-edge tests, inactive-intermediate tests, pre-`0.8.31` `slice60_graph_expand.rs` assertions, ~25 convenience-edge tests across Rust/Py/TS, and the 3 Inspector dangling-edge fixtures (`add_graph`, `add_graph_missing_prefix`, `add_graph_highdegree`) are updated and pass `cargo clippy --workspace --all-targets` and `cargo test`. |
| `AC31-30-10` | `R31-06`, `X2`, `X4` | `purge`/`erase_source` wipe target/terminal-edge properties from traversal and evidence resolution; `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md`, `docs/guides/graph-retrieval.md`, and `docs/reference/{python-api,typescript-api,rust-sdk,rust-api,errors}.md` accurately reflect strict integrity, `graph_expand` predicates, `use_graph_arm` seeding, mixed-`source_id` batch atomicity, and zero external consumer-project specifics. |

---

## 3. Architecture & Draft Design

### 3.1 Strict Write Endpoint Integrity & `actuate` Seam Separation (`P4-07`, `P4-08`)

#### Live seam and hazard

- `commit_batch_transaction` (`src/rust/crates/fathomdb-engine/src/write_commit.rs:556,581`) calls `apply_batch_in_transaction(&tx, batch, ...)` once per `Engine::write` batch inside `BEGIN IMMEDIATE`, ending at `count_dangling_edge_endpoints(tx, batch)` (`L921`, `L932-986`).
- However, `actuation.rs` invokes `apply_batch_in_transaction(&tx, &[write], &[plan], ..., SoftClosureMode::Proving)` **one operation at a time** inside `simulate_request` (`actuation.rs:1526,1567`, inside `SAVEPOINT fathomdb_actuation_validation`) and `actuate_new` (`actuation.rs:2172,2238`, inside `SAVEPOINT fathomdb_actuation_domain`). Any `Err(CommitBatchError::Engine(EngineError::WriteValidation))` returned by `apply_batch_in_transaction` propagates out as `Err(EngineError::WriteValidation)` (`actuation.rs:1555,1594,2220,2286`) rather than returning `ActuationOutcomeV1::Refused`. Only **after** the loop finishes does `simulate_request` call `validate_prospective_edge_endpoints(connection, request)` (`actuation.rs:1640,1650-1677`), which checks all `PutDerivedEdge` operations against `superseded_at IS NULL AND state = 'active'` and returns `ActuationRefusalReasonV1::ReferenceUnavailable`.
- Meanwhile, `count_dangling_edge_endpoints` (`write_commit.rs:946-986`) has two gaps: (a) it skips any edge with `last_index[logical_id] > i` (`L964-970`, letting dangling in-batch-superseded edges enter history, and missing G11 `(from_id, to_id, kind)` supersession), and (b) its probe (`L973`) omits `AND state = 'active'`.

#### Design

1. Introduce an internal validation mode enum passed to `apply_batch_in_transaction`:

   ```rust
   pub(crate) enum EndpointValidationMode {
       /// Enforce active, non-superseded endpoints for every edge in `batch`.
       Strict,
       /// Single-operation actuation loop: `validate_prospective_edge_endpoints` runs post-loop in `simulate_request`.
       DeferredToActuationSimulation,
   }
   ```

2. `commit_batch_transaction` (`Engine::write` and `ingest_with_extractor` tx2) passes `EndpointValidationMode::Strict`. `actuation.rs` (`simulate_request` and `actuate_new`) passes `EndpointValidationMode::DeferredToActuationSimulation` so same-batch `[PutDerivedEdge, PutNode]` continues succeeding and dangling actuation edges continue returning `ActuationOutcomeV1::Refused { reason: ReferenceUnavailable, index, path: "/operations/{i}/record/{from|to}" }`.
3. Under `EndpointValidationMode::Strict`, replace `count_dangling_edge_endpoints` with `validate_strict_batch_edge_endpoints(tx, batch) -> rusqlite::Result<Result<(), EngineError>>` (or return `Err(CommitBatchError::Engine(EngineError::WriteValidation))`), running **after** all canonical inserts/supersessions in the batch:
   - **Node-only zero-SQL fast path:** Pre-scan `batch` in memory (`storage_write_shape(w)`). If there are zero `PreparedWrite::Edge` / `ProvenancedEdge` items, return `Ok(())` immediately without preparing or running any SQL query against `canonical_nodes` (preserving `write_commit.rs:1071-1135` `Authorizer` contract).
   - **All-edges-in-batch enforcement:** Remove the `last_index` skip logic completely. Iterate over **every** edge item in `batch` (whether retained, G0 `logical_id`-superseded later in the batch, or G11 `(from_id, to_id, kind)`-superseded later in the batch) and verify both `from` and `to` using one prepared `SELECT 1 FROM canonical_nodes WHERE logical_id = ?1 AND superseded_at IS NULL AND state = 'active' LIMIT 1`. This uses Schema 38's `canonical_nodes_logical_active_idx` via `SEARCH` (`canonical_nodes_logical_active_idx`), checks the true final in-batch node state (rejecting edges whose endpoint was created active and later superseded `Pending`/`Deleted` in the same batch), and fails fast on the first dangling/inactive endpoint with `Err(CommitBatchError::Engine(EngineError::WriteValidation))`, rolling back `tx` atomically before `advance_read_visibility` and `tx.commit()`.
   - Update `WriteReceipt.dangling_edge_endpoints` to remain `0` on every committed `Engine::write`.
4. **Validation ordering (`write_validation.rs`, `write.rs`):** Keep syntactic checks (`empty batch`, empty `kind`/`from`/`to`, `\x1e`/NUL, inverted windows, invalid inline `properties`) and `WriteProvenanceV1` validation running **before** database writes/endpoint probes so existing error precedence (`ProvenanceError` vs `WriteValidationError`) is preserved.
5. **`ingest_with_extractor` (`ingest.rs:204-226, 366-412`):** Keeps tx1 (`new_nodes`, including synthesized unlisted `"entity"` fallback nodes) -> tx2 (`edge_batch`); verify in integration tests that normal extraction commits with `dangling_edge_endpoints == 0` while concurrent endpoint deactivation between tx1 and tx2 refuses tx2 atomically with `WriteValidation`.

---

### 3.2 `GraphExpandRequestV1` `#[non_exhaustive]` & Closed Typed Predicates (`P4-10`, `P4-13`)

#### Rust carrier & constructor ergonomics

- Re-use Slice 20's `PropertyValue { String(String), Int(i64), Float(f64), Bool(bool) }` and `PropertyMap` (`BTreeMap<String, PropertyValue>`) defined in `fathomdb-engine`.
- Define closed traversal predicate types in `graph_expand::types` (and re-export at crate roots `fathomdb-engine`, `fathomdb`, `fathomdb-sdk`):

  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq)]
  pub enum PropertyComparisonOp {
      Eq,
      Ne,
      Lt,
      Lte,
      Gt,
      Gte,
  }

  #[derive(Clone, Debug, PartialEq)]
  pub struct PropertyPredicate {
      pub key: String,
      pub op: PropertyComparisonOp,
      pub value: PropertyValue,
  }
  ```

- Mark `GraphExpandRequestV1` `#[non_exhaustive]` (`graph_expand/types.rs:96`) and add:
  - `pub edge_predicates: Vec<PropertyPredicate>`
  - `pub target_predicates: Vec<PropertyPredicate>`
  - `GraphExpandRequestV1::new(seed, direction, context, max_depth, result_limit, max_work_units) -> Self` initializing `schema_version: 1`, empty `edge_kinds` / `target_kinds` / `edge_predicates` / `target_predicates`, and `false` for `include_explanation` / `include_evidence`, plus fluent `.with_edge_kinds(...)`, `.with_target_kinds(...)`, `.with_edge_predicates(...)`, `.with_target_predicates(...)`, `.with_explanation(...)`, `.with_evidence(...)`.
  - Update all workspace struct literals (`src/inspector/src/graph.rs:572-591`, `src/rust/crates/fathomdb-mcp/src/tools.rs:1468`, `evidence.rs:2804`, `graph_expand/mod.rs:54`, `codec.rs:711`, and all test files in `tests/`) to use `GraphExpandRequestV1::new(...)`.
- Extend output carriers (`graph_expand/types.rs:123,137`):
  - `GraphOriginV1` (`#[non_exhaustive]`): add `pub terminal_edge_properties: Option<PropertyMap>`.
  - `GraphTargetV1` (`#[non_exhaustive]`): add `pub properties: Option<PropertyMap>`.
- Extend `GraphExpansionErrorReasonV1` (`graph_expand/types.rs:188`) with `GraphEdgePredicatesInvalid` and `GraphTargetPredicatesInvalid` (wire strings `"graph_edge_predicates_invalid"`, `"graph_target_predicates_invalid"`).

#### Exact closed predicate evaluation & validation contract (`R31-02`)

To guarantee **disabled-optimization equivalence** (`B-tree == inline`) and close all ambiguity noted in `prework/slice-5.md`:

- **Validation (fail closed before execution):**
  - Bounds: at most `32` predicates per predicate vector (`edge_predicates` / `target_predicates` — excess returns `GraphEdgePredicatesInvalid` / `GraphTargetPredicatesInvalid` at `/edgePredicates` or `/targetPredicates`).
  - Keys: non-empty, `<= 256` bytes, no NUL or `\x1e` delimiter (`/edgePredicates/{i}/key`).
  - Values: reject non-finite `Float` (`NaN`, `±Infinity`); canonicalize `-0.0` to `0.0` matching Slice 20 storage; validate string values for NUL.
  - Operator × value type: allow `Eq | Ne` on all 4 types (`String | Int | Float | Bool`); allow `Lt | Lte | Gt | Gte` on ordered types (`String | Int | Float`) and **refuse `Lt..Gte` on `Bool`** with `GraphEdgePredicatesInvalid` / `GraphTargetPredicatesInvalid` at `.../op` (avoids arbitrary boolean ordering).
- **Runtime comparison semantics (identical in Rust inline evaluator and SQL `canonical_property_values` B-tree lookup):**
  - **Missing key / `properties` `NULL`:** evaluates **`false`** for all 6 operators (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte`) — mirrors SQL `EXISTS(canonical_property_values WHERE attr_name = ? ...)` where absent properties have no row.
  - **Same-family matches:**
    - `String` vs `String`: exact UTF-8 byte comparison (`Ordering`).
    - `Bool` vs `Bool`: exact boolean equality (`Eq` / `Ne`).
    - Numeric (`Int` or `Float` vs `Int` or `Float`): `Int` vs `Int` uses exact `i64::cmp` (`abs(x) > 2^53` never loses precision via `f64` cast); `Int` vs `Float` and `Float` vs `Float` compare mathematically exact values without truncating `i64` (matching SQLite `NUMERIC` column affinity on `canonical_property_values.num_val`!).
  - **Cross-family mismatch (`String` vs numeric/bool, `Bool` vs numeric/string):** evaluates **`false`** for all operators including `Ne` — no implicit type coercion, ensuring identical outcomes between single-`value_type` B-tree queries (`text_val`, `num_val`, `bool_val`) and inline JSON evaluation.
  - **Conjunction:** `Vec<PropertyPredicate>` is strict implicit `AND`; empty list matches all rows.

#### Closed wire codec & graph evidence binding (`codec.rs`, `execution.rs:1114`, `evidence.rs`)

- **Wire request (`codec.rs`, `graph.py`, `graph.ts`):** Add optional `edgePredicates` and `targetPredicates` to `RequestWire` and `decode_graph_expand_request_v1`'s closed field allowlist (`omit` when empty during encoding to preserve pre-`0.8.31` canonical JSON snapshots while decoding omitted or explicit `[]` cleanly). Each wire predicate object is closed to `{"key": string, "op": "eq"|"ne"|"lt"|"lte"|"gt"|"gte", "value": {"type": "string"|"int"|"float"|"bool", "value": ...}}` (using exact stringified decimal integer / JSON float matching `PropertyValue` wire representation from Slice 20 so `i64` `> 2^53` round-trips cleanly across JS/TS without IEEE-754 loss!).
- **Wire response (`codec.rs`):** Extend `OriginWire` with `#[serde(skip_serializing_if = "Option::is_none")] terminal_edge_properties: Option<&'a PropertyMap>` and `TargetWire` with `#[serde(skip_serializing_if = "Option::is_none")] properties: Option<&'a PropertyMap>`, plus corresponding updates in `decode_graph_expand_result_v1`, Python `_decode_graph_expand_result`, and TS `graph.ts` (note: `GraphTargetExplanationV1.origin` shares `OriginWire`).
- **`encode_graph_evidence_request` (`execution.rs:1114`):** Canonicalize and encode `edge_predicates` and `target_predicates` sorted deterministically (matching `edge_kinds`/`target_kinds` set normalization) so frozen evidence commitments authenticate predicate identity regardless of caller slice ordering, while omitting empty sets or encoding explicit counts consistently with token validation.
- **`evidence.rs` (`resolve_graph_evidence` & `GraphEvidenceMaterial`):** Verify/extend `GraphEvidenceMaterial` and `GraphEvidenceArtifactV1` (Node & Edge) so `properties` hydrated by Slice 20/30 are loaded directly from the exact revision `write_cursor` row (`canonical_nodes` / `canonical_edges`) under frozen authority and refuse erased or barrier-blocked artifacts identically.

---

### 3.3 `graph_expand` Execution Refactor (`execution.rs`, `P4-10`)

1. **Indexed SQL Pushdown + `UNION ALL` for `Both` + `t_valid` Gating (`execution.rs:489-554`):**
   - Replace `graph_expand_incident_sql`'s `WHERE (from_id = ?1 OR to_id = ?1) LIMIT ?2` with SQL pushing:
     - `superseded_at IS NULL`
     - `kind IN (?...)` when `request.edge_kinds` is non-empty
     - `(t_valid IS NULL OR t_valid <= ?now)`
     - `(t_invalid IS NULL OR t_invalid > ?now)`
     - Select `write_cursor, kind, from_id, to_id, logical_id, t_valid, t_invalid, properties`.
   - When `direction == TraversalDirection::Both`, emit two indexed `SELECT` arms combined with **`UNION ALL`**:
     - Arm 1: `WHERE from_id = ?1 AND superseded_at IS NULL ...`
     - Arm 2: `WHERE to_id = ?1 AND from_id != ?1 AND superseded_at IS NULL ...` (the `from_id != ?1` residual check prevents duplicating self-loop rows while keeping SQLite on `canonical_edges_to_active_idx`!)
     - Wrap `(Arm 1 UNION ALL Arm 2) LIMIT ?limit` so `remaining + 1` cap applies only to live, matching, in-window edges.
   - Verify via `EXPLAIN QUERY PLAN` (`explain_graph_expand_incident_query`) that Outgoing, Incoming, and Both — both with empty and non-empty `edge_kinds` — plan `SEARCH canonical_edges USING INDEX canonical_edges_{from,to}_active_idx` with zero full `SCAN`.
   - Keep the incident-filter struct modular (e.g., `EdgeViewBoundary::Current { edge_now }`) so Slice 40 adds `AsOf` cleanly without touching Slice 30's current-time SQL path.

2. **B-Tree Acceleration vs. Inline Predicate Evaluation:**
   - Look up declared `Filterable` projections once per `read_graph_expand_with_stop_words_in_tx` snapshot (`load_projection_registry(&tx)`).
   - For `edge_predicates`: when an `edge_predicate` key is declared `Filterable` for `entity_class = 'edge'`, emit a parameterized `AND EXISTS(SELECT 1 FROM canonical_property_values cpv WHERE cpv.entity_class = 'edge' AND cpv.write_cursor = e.write_cursor AND cpv.attr_name = ? AND ...)` clause directly in SQL (using `canonical_property_values_{text,num,bool}_idx` / `canonical_property_values_cursor_idx`), while evaluating all remaining/undeclared predicates inline against fetched `canonical_edges.properties` JSON (never creating indexes automatically). Wait: to ensure `work_units` and `GraphExpansionBoundExceeded` stay consistent for disabled-optimization equivalence near boundaries while accelerating high-fanout declared lookups, both SQL B-tree pre-filter (when declared) and inline filter produce **exact set + origin equivalence** across all random multigraphs (pinned by `proptest`), and dedicated tests cover both `max_work_units` boundary behavior and B-tree index usage.
   - For `target_predicates`: similarly hydrate `n.properties` in `load_node(&tx, next, ...)` (adding `n.properties` to `SELECT n.kind, n.body, n.write_cursor, n.properties FROM canonical_nodes n`) and push any declared node `Filterable` predicates cleanly via `canonical_property_values` (`entity_class = 'node'`), verifying any remaining/undeclared predicates inline before candidate insertion.

3. **Predicate Placement, `visited_by_seed`, and Terminal-Depth `L895` Fix (`execution.rs:877-956`):**
   - **Fix `edge_predicates` vs `visited_by_seed` (`L888`):** Evaluate `edge_predicates` **immediately after `work_units += 1`** (before `visited_by_seed[seed_index].insert(next)` at `L888` and before `L895`). An edge failing `edge_predicates` MUST NOT mark `next` visited; otherwise a second parallel edge `(current -> next)` that *does* satisfy `edge_predicates` would be dropped.
   - **Fix `next_frontier.push` guard (`L905`):** Change `if depth < request.max_depth` (always true inside `0..max_depth`) to `if depth + 1 < request.max_depth`, eliminating dead terminal-depth frontier allocations.
   - **Fix terminal-depth `L895` early-skip soundness (`L895-899` vs `L942-955`):** Before skipping `load_node` at `depth + 1 == request.max_depth && candidates.len() >= request.result_limit as usize`, compare the edge's precomputed structural origin key `(depth + 1, seed.seed_ordinal, current, terminal_direction, edge.kind, next)` directly against the current worst candidate origin in `candidates` (tracked lazily once `candidates.len() == result_limit`): skip `load_node` **only** when the precomputed origin cannot beat `worst` (`!origin_key_lt(...)`). This preserves the exact O(1)-memory zero-`load_node` guarantee required by `slice60_fix4_rss` and `slice60_fix5_rss` while making it impossible for `L895` to ever silently drop a better `origin_cmp` candidate ahead of `L942-955`.
   - **Target eligibility vs intermediate traversal:** Only nodes passing `load_node` (existence, validity, eligibility filter) can traverse (`next_frontier`); failing `target_kinds` or `target_predicates` prevents emitting the node as a `GraphTargetV1` while still permitting the active intermediate node to extend multi-hop walks up to `max_depth` (`result_limit_selects_top_n_only_after_the_walk_is_complete`).

---

### 3.4 `search.rs` Graph Arm: `SEED_FTS_N` Fix, Phase 2 `UNION ALL`, `graph_seed` & Provenance (`P4-11`)

#### Request knobs & zero-diff baseline guarantee

- Introduce:
  - `GraphSeedMode` (`GraphFts` [default] | `FusedTopK` | `Both`; wire/Py/TS strings `"graph_fts" | "fused_top_k" | "both"`).
  - `GraphArmOptions` (internal carrier inside `SearchReaderWork`, defaulting to `GraphFts`, `seed_k = 10`, empty `edge_kinds`, empty `edge_predicates`) exposed on:
    - Rust `fathomdb-sdk`: additive fields on `SearchOptions` and `FrozenSearchOptions` (`#[non_exhaustive]`, `Default` unchanged) and `EvidenceSearchRequestV1` (add constructor or `#[non_exhaustive]` defaults / helper — keep `EvidenceSearchRequestV1` check carefully for existing struct literals!). *(Note: check `EvidenceSearchRequestV1` call sites in Rust tests; if `EvidenceSearchRequestV1` is exhaustive today in `evidence.rs:330`, either add `#[non_exhaustive]` + `::new(...)` + update workspace literals or add additive `graph_*` methods/builder; check exact call sites cleanly in RED setup.)*
    - Python (`engine.py`, `types.py`, `_sdk_search.py`, `_sdk_evidence.py`, `fathomdb-py`): keyword-only defaults `graph_seed: str = "graph_fts"`, `graph_seed_k: int = 10`, `graph_edge_kinds: Sequence[str] | None = None`, `graph_edge_predicates: Sequence[PropertyPredicate] | None = None` on `Engine.search`, `Engine.search_frozen`, and `EvidenceSearchRequestV1`.
    - TypeScript (`search.ts`, `evidence.ts`, `core.ts`, `fathomdb-napi`): additive optional fields `graphSeed?: "graph_fts" | "fused_top_k" | "both"`, `graphSeedK?: number`, `graphEdgeKinds?: string[]`, `graphEdgePredicates?: PropertyPredicate[]` on `SearchOptions`, `FrozenSearchOptions`, and `EvidenceSearchRequestV1` (update `splitSearchOptions` in `src/ts/src/search.ts:299` so `graphSeed`/`graphSeedK`/`graphEdgeKinds`/`graphEdgePredicates` are peeled off cleanly before building `view`!).
- **Validation:** Validate `graph_seed` vocabulary, `graph_seed_k` in `1..=100` (`InvalidArgument` on `0` or `> 100`), `graph_edge_kinds` strings (non-empty, no `\x1e`/NUL), and `graph_edge_predicates` (same closed validator as `GraphExpandRequestV1`, mapping invalid predicates to `EngineError::InvalidFilter`) at request entry regardless of `use_graph_arm`. When `use_graph_arm == false` and defaults hold, `read_search_in_tx` skips all graph-arm queries, allocations, and joins (`hybrid_deferred_identity` / `rank_stream` plans and results stay 100% byte-identical).

#### Mixed-edge `SEED_FTS_N` slot fix (`search.rs:1583-1673`)

- **Root cause:** Today `edge_seed_stmt` queries `WHERE ... AND (EXISTS(ef ...) OR EXISTS(et ...)) LIMIT ?2` (`SEED_FTS_N = 10`) and unconditionally pushes **both** `from_id` and `to_id` into `candidate_seeds`. Mixed edges (`eligible -> ineligible`) consume `LIMIT 10` edge slots and insert ineligible endpoints into `candidate_seeds`.
- **Fix:**
  1. Project `from_eligible` (`EXISTS(SELECT 1 FROM canonical_nodes ef ...)` as boolean) and `to_eligible` (`EXISTS(SELECT 1 FROM canonical_nodes et ...)` as boolean), plus `ce.kind` and `ce.properties` (and `AND ce.kind IN (...)` when `graph_edge_kinds` is non-empty) in SQL, keeping `WHERE ... AND (from_eligible OR to_eligible)` ordered by `bm25(search_index_edges), sei.write_cursor` **without** `LIMIT 10` pre-truncating across mixed/duplicate/predicate-filtered edges (bound stream loop in Rust with optional generous SQL ceiling only after all checks, or stream cursor until `collected_seed_slots == SEED_FTS_N`).
  2. In Rust, verify `graph_edge_predicates` on `(ce.properties)` (and optional `canonical_property_values` pushdown if declared); skip the edge completely if it fails `graph_edge_kinds`/`graph_edge_predicates`.
  3. Push **only** `from_id` when `from_eligible` is true and **only** `to_id` when `to_eligible` is true (never pushing an ineligible endpoint into `candidate_seeds` or `seeds_considered`!). Deduplicate by `logical_id` while counting `SEED_FTS_N = 10` against **distinct newly admitted eligible edge-seed slots** (stops once `distinct_edge_seeds >= SEED_FTS_N`). Result: `>= 10` mixed edges bridging an eligible hub or ineligible counterparties can no longer consume slots for ineligible endpoints or starve eligible edges ranked below them, while legitimate heterogeneous edges (`person -> company` under `kind = "person"`) continue seeding `person`!

#### Phase 2 `UNION ALL` & `graph_edge_kinds` / `graph_edge_predicates` (`search.rs:1840-1989`)

- Rewrite Phase 2 `edge_stmt` `WHERE (e.from_id = ?1 OR e.to_id = ?1)` as:
  - `SELECT e.from_id, e.to_id, e.source_id, e.confidence, e.write_cursor, e.kind, e.properties FROM canonical_edges e WHERE e.from_id = ?1 {edge_existence}{edge_validity}{edge_kinds}{target_exists}`
  - `UNION ALL`
  - `SELECT e.from_id, e.to_id, e.source_id, e.confidence, e.write_cursor, e.kind, e.properties FROM canonical_edges e WHERE e.to_id = ?1 AND e.from_id != ?1 {edge_existence}{edge_validity}{edge_kinds}{target_exists}`
  - Ordered deterministically (`ORDER BY write_cursor` on the wrapped `UNION ALL`), backing both arms with `canonical_edges_{from,to}_active_idx` when `!view.include_superseded`.
- Evaluate `graph_edge_predicates` against `e.properties` (plus optional `canonical_property_values` `EXISTS` acceleration when declared `Filterable`) **before** `visited.insert(neighbor)` and before applying the `64` neighbor-per-hop limit so filtered-out edges neither poison `visited` nor consume the per-hop neighbor budget.

#### Opt-in `FusedTopK` & `Both` Seeding + Deterministic Merge + Provenance (`search.rs`, `evidence.rs`)

- **Fused/reranked seed pool selection:**
  - When `graph_seed` is `FusedTopK` or `Both`: compute `two_arm_fused = fuse_rrf(vector_results, text_results)?`. If `rerank_depth > 0`, obtain `seed_ranking = try_rerank_ranked(raw_query, two_arm_fused.clone(), rerank_depth, alpha, pool_n)?` (which gracefully soft-falls back to `two_arm_fused` when `default-reranker` is absent or model not loaded; likewise if no embedder is loaded, `two_arm_fused` degrades gracefully to lexical FTS results).
  - Filter `seed_ranking` to canonical node hits with **`logical_id IS NOT NULL`** (`matches!(h.ownership, FusionOwnership::CanonicalNode) && h.hit.id.space == IdSpaceKind::Logical`), verify active node eligibility under `view` + `filter` (`active_stmt` so non-active/filtered hits cannot seed), deduplicate `logical_id` preserving rank order, and take up to `graph_seed_k` unique seeds tagged `FusedTopK` (`hop = 0`, no edge confidence).
- **Body-less edge traversal:** Because `FusedTopK` seeds from dense/text/reranked node hits and Phase 2 traverses `canonical_edges` directly (requiring neither `search_index_edges` rows nor `edge.body IS NOT NULL`), body-less property-graph edges (`body = None`, `properties = Some(...)`) now expand seamlessly when `use_graph_arm = true` and `graph_seed` is `FusedTopK` or `Both`.
- **Deterministic `Both` merge & tie-breaking:**
  - Seed Source `FusedTopK` (`rank`-ordered) and Seed Source `GraphFts` (`EdgeFts` `bm25`-ordered, then `EntityFts` `bm25`-ordered) enqueue depth-0 seeds into the shared BFS `frontier`.
  - **Duplicate seed across sources:** First-seen precedence in fixed order `(FusedTopK, EdgeFts, EntityFts)` determines seed attribution when the exact same `logical_id` appears in multiple depth-0 seed sources (preserving `EdgeFts` `source_id`/`confidence` if `EdgeFts` claims an unseeded endpoint). Wait: to preserve 100% backward compatibility with `pr_c1_graph_seeding` when `graph_seed = GraphFts` (the default), `FusedTopK` is not run under `GraphFts`; under `Both`, running `FusedTopK` first (or `GraphFts` first — **pinning exact order `(FusedTopK, EdgeFts, EntityFts)`** at depth 0, with **minimum `hop_count` always winning** across BFS depths `0 < 1 < 2 < 3`, breaking equal-hop ties by frontier insertion order then `e.write_cursor`) guarantees deterministic, order-invariant `(graph_seed_source, graph_hop_depth, source_id, confidence)`.
  - Depth-0 seeds already present in `seed_records` (`two_arm_fused`) are **not** re-emitted as `GraphArm` hits (they retain their `Vector`/`Text` representative arm); reachable neighbors at `hop_count >= 1` (and non-two-arm `EdgeFts` endpoints at `hop_count = 0`) emit `SoftFallbackBranch::GraphArm` hits carrying `CapturedGraphOrigin`.
- **Seed provenance on Explanation & Evidence (`search_types.rs`, `evidence.rs`, Py, TS):**
  - Update `CapturedGraphOrigin` (`search.rs` / `evidence.rs`) to tag root seed provenance `GraphArmSeedSource { FusedTopK, EdgeFts, EntityFts }`:
    - `EdgeSeed { edge_cursor }` (`seed_source = EdgeFts`, `hop_count = 0`)
    - `Traversal { edge_cursor, hop_count, seed_source }` (`seed_source = FusedTopK | EdgeFts | EntityFts`, `hop_count >= 1`)
    - (Note: clean up `evidence.rs:2506` `EntitySeed` token encoding path so entity-seeded traversals flow through `Traversal { ..., seed_source: EntityFts }` rather than invalid bare `EntitySeed`.)
  - Expose `pub graph_seed_source: Option<GraphArmSeedSource>` (`"fused_top_k" | "edge_fts" | "entity_fts"`) and `pub graph_hop_depth: Option<u32>` on:
    1. `PerHitExplain` (`#[non_exhaustive]`, `None` for non-`GraphArm` hits, `Some(...)` for `GraphArm` hits) in Rust, Python (`PyPerHitExplain` + `types.PerHitExplain`), and TS (`fathomdb-napi::PerHitExplain` + `PerHitExplain`).
    2. `EvidenceGraphOriginV1` / `EvidenceProjectionOriginV1` (`evidence.rs:130-143`, payload codec `evidence.rs:2504-2603`, Py `types.EvidenceGraphOriginV1`, TS `EvidenceGraphOriginV1`).

---

### 3.5 Cross-SDK, `Inspector`, & Documentation Alignment (`P3-02`, `P3-08`, `P3-09`, `P6-02`, `P6-04`, `P6-06`)

- **SDK parity (`X1`):**
  - Update `fathomdb-sdk` (`graph.rs`, `options.rs`, `engine.rs`, `lib.rs`), `fathomdb-py` (`engine.rs`, `graph_evidence.rs`, `read_search.rs`, `lib.rs`), `src/python/fathomdb/` (`types.py`, `graph.py`, `engine.py`, `_sdk_search.py`, `_sdk_evidence.py`, `_fathomdb.pyi`, `__init__.py`), `fathomdb-napi` (`engine.rs`, `graph_evidence.rs`, `read_search.rs`), and `src/ts/src/` (`graph.ts`, `search.ts`, `evidence.ts`, `core.ts`, `index.ts`).
  - Zero new governed operations are added in Slice 30 (`graph.expand`, `engine.search`, `engine.write`, etc. already exist in `governed-operation-parity.json`), so `governed-operation-parity.json` count remains unchanged; additive types/fields (`PropertyPredicate`, `PropertyComparisonOp`, `GraphTargetV1.properties`, `GraphOriginV1.terminal_edge_properties`, `PerHitExplain.graph_seed_source`/`graph_hop_depth`, `search` graph-seeding kwargs) must be registered cleanly in `python_surface_additions.json`, `test_slice130_package_boundary.py` (`native_stub_sha256` reverse-patch), `fathomdb-sdk/tests/surface.rs`, and `fathomdb/tests/slice60_governed_surface.rs`.
- **Interface & public doc updates (`X4`):**
  - `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md`: update `Engine::write` / `WriteReceipt.dangling_edge_endpoints` for `0.8.31` strict integrity (`P3-08`), `GraphExpandRequestV1` closed predicates + `GraphTargetV1.properties`/`GraphOriginV1.terminal_edge_properties` + `use_graph_arm` `graph_seed`/`graph_seed_k`/`graph_edge_kinds`/`graph_edge_predicates`/`graph_seed_source`/`graph_hop_depth` + `SEED_FTS_N` fix (`P3-09`), and mixed-`source_id` batch atomicity (`P6-04`).
  - `docs/guides/graph-retrieval.md` (`P6-06`), `docs/reference/{python-api,typescript-api,rust-sdk,rust-api,errors}.md` (`P3-08`, `P3-09`, `P6-02`, `P6-04`, `P6-06`): replace stale `dangling_edge_endpoints` "batch commits" prose, stale `use_graph_arm` "top-10 fused hits" prose, and stale `t_valid` / "no fused-hit seed mode" prose across guides and docstrings (`engine.py:345`, `fathomdb-napi/src/engine.rs:551`). Keep `docs/guides/graph-retrieval.md` clean regarding Slice 40 (`as_of` historical replay lands in Slice 40, so do not document unimplemented `as_of` knobs yet). Keep **zero** external consumer-project specifics in all tracked docs.

---

## 4. Prerequisites Discovered

Audit of live code and `prework/slice-5.md` identified the following concrete prerequisites that **must** be updated in lockstep with Slice 30 implementation so workspace builds and regression suites pass cleanly:

### 4.1 Legacy Dangling-Edge & Pre-`0.8.31` Graph Tests (`P2-05`, `P5-04`)

1. **Explicit dangling-edge acceptance tests -> rewrite to strict atomic refusal + zero-row rollback:**
   - `src/rust/crates/fathomdb-engine/tests/pr_g8_dangling_edges.rs`: `s20_edge_to_one_missing_endpoint_counts_one`, `s20_edge_to_superseded_node_counts`, `s20_default_flag_and_count_commits_the_batch`, `s20_count_is_sum_over_both_endpoints`, `s20_on_supersession_skips_earlier_keeps_final_active` (L225 — update to prove strict rejection of an in-batch superseded dangling edge), `s20_legacy_null_logical_id_batch_writes_with_defined_count`. Retain `s20_same_batch_later_inserted_node_is_not_flagged` and `s20_endpoint_probe_hits_partial_index_no_scan`.
   - `src/rust/crates/fathomdb-engine/src/write_commit.rs` (`L1071-1135`, `node_only_dangling_check_does_not_require_endpoint_reads`, `L1118-1123`): update `Ok(2)` endpoint assertion to `Err(WriteValidation)` while retaining the SQLite `Authorizer` zero-read assertion for node-only writes.
   - `src/python/tests/test_functional_search.py` (`test_functional_dangling_edge_count_across_ffi`, `L203-245`) & `src/ts/tests/functional-search.test.ts` (`functional write: dangling-edge count (Py ≡ TS)`, `L209-230`): retain `clean.dangling_edge_endpoints == 0`; convert the missing-endpoint batch assertions to `WriteValidationError` atomic refusal with zero committed rows.
2. **Inactive-intermediate / erasure setup tests -> convert to create-active -> write-edge -> `transition`/`purge`:**
   - `src/rust/crates/fathomdb-engine/tests/slice20_graph_traversal.rs` (`graph_neighbors_inactive_intermediate_not_traversed`, `L556-587`): write `"MISSING"` active alongside `A` and `C`, write `A -> MISSING` and `MISSING -> C`, then `transition("MISSING", LifecycleState::Deleted)`.
   - `src/rust/crates/fathomdb-engine/tests/slice85_physical.rs` (`absent_logical_root_never_completes_over_retained_touching_edges`, `L279-330`): write `"already-absent"` active + edge, transition `Deleted` + `purge("already-absent")` before testing? *(Note carefully: verify how `slice85_physical.rs` establishes an absent logical root over a touching edge! Because `Engine::purge` cascades to `canonical_edges WHERE from_id=?1 OR to_id=?1`, creating `"already-absent"` and deleting it via `Engine::purge` removes the edge too — instead `slice85_physical.rs` likely tests an edge left over from a prior schema / pre-0.8.31 fixture or requires a fixture-level pre-0.8.31/raw-row delete of `"already-absent"` only after `engine.close()` before re-opening for physical erasure! Verify `slice85_physical.rs:278-330` exact purpose before editing.)*
   - `src/rust/crates/fathomdb-engine/tests/opp12_existence_axis.rs` (`r_ex_2_graph_traversal_excludes_pending_neighbor`, `L192-211`): write `"nbr"` `InitialState::Active`, write `root -> nbr`, then `transition("nbr", LifecycleState::Pending)`.
3. **Pre-`0.8.31` `graph_expand` tests (`src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs`):**
   - `L462` (`superseded_edges_consume_work_but_cannot_supply_a_target`): with SQL `superseded_at IS NULL` pushdown, superseded edges no longer consume `work_units` (`work_units == 1`, not `2`). Update expectation & test name per `ADR-0.8.31-historical-graph-and-fused-seeding.md`.
   - `L489-559` (`node_relaxation_never_relaxes_shipped_edge_recency_and_t_valid_never_gates`): update `L555` (`future-valid-edge`) from `assert!(ids.contains(...))` to `assert!(!ids.contains(...))` and rename test to reflect bitemporal `[t_valid, t_invalid)` gating.
   - `L754-776` (`endpoint_plans_use_shipped_indexes_without_full_edge_scan_and_schema_stays_34`): verify updated Schema 38 partial active indexes (`canonical_edges_from_active_idx`, `canonical_edges_to_active_idx`).

### 4.2 ~25 Convenience-Edge Tests Across Rust / Python / TypeScript (`P4-08`, `P5-04`)

Seed missing active endpoint nodes cleanly alongside edges in all convenience-edge tests identified by `prework/slice-5.md:181-204`:

- **Rust (`src/rust/crates/fathomdb-engine/tests/`):**
  - `erasure_projection_registry.rs:293-329`
  - `erasure_drain_ordering.rs:144, 176`
  - `slice60_store_format.rs:278-317`
  - `pr_g0_identity.rs:111, 278`
  - `source_id_writes.rs:42`
  - `trace_source_ref.rs:52` & `provenance_mandatory.rs:68-102`
  - `slice30_embedding_readiness.rs:101, 137`
  - `slice40_projection_generation.rs:172` & `slice40_projection_completion.rs:140, 362`
  - `tc45_supersession_terminal.rs:251, 273`
  - `tc33_fix1_unrenderable_epoch.rs:131` & `tc33_fix5_iso8601_shape_gate.rs:237`
  - `tc33_fix3_consolidation_timestamp_taxonomy.rs:47`, `consolidate_provider.rs:81`, `rebuild_projections.rs:233`, `tc33_fix6_edge_vector_prune.rs:114`
  - `consolidation_value_test.rs:61`
  - `slice50_lightweight_controls.rs:166`
  - `slice50_record_identity.rs:145, 321, 483, 556`
  - `slice50_refusal_baseline.rs:159-181`
  - `slice50_drain_scan_bound.rs:204, 211, 240`
  - `slice55_data_plane_integrity.rs:1532, 1555, 1837`
  - `slice55_explanation.rs:537`
  - `slice15_identity_provenance.rs:131, 515`
  - `slice80_history.rs:795-810`
  - `slice60_fix4_dependency.rs:73-102` (seed `"derived"` before/with `root -> derived` edge)
  - `write_boundary_atomicity.rs:595-622, 704-716` (seed consolidation endpoint nodes + emit valid node/edge endpoint pairs in `ValidItem::Edge` `proptest` strategy)
  - *(Plus run full `cargo test --workspace --all-targets` immediately after strict enforcement lands in case any additional `PreparedWrite::Edge` tests omitted endpoint nodes.)*
- **Python & TypeScript:**
  - `src/python/tests/test_slice30_embedding_readiness.py:27`
  - `src/ts/tests/slice30-embedding-readiness.test.ts:22`

### 4.3 Three Inspector Dangling-Edge Fixtures & Subtle Forensic Semantic Hazard (`P4-13`, `P5-04`)

Live inspection of `src/inspector/tests/fixtures/mod.rs`, `src/inspector/src/graph.rs`, `src/inspector/src/policy.rs`, and `src/rust/crates/fathomdb-engine/src/erasure.rs` revealed a **critical constraint** on the 3 Inspector fixtures:

1. `add_graph` (`src/inspector/tests/fixtures/mod.rs:527`): edge `"missing"` connects `"derived" -> "not-retained"`. Caller `worker.rs:983` asserts `missing["to"]["resolution"] == "zero"`, and `browser/graph.mjs:49` asserts 5 `path.graph-edge` elements.
2. `add_graph_missing_prefix` (`src/inspector/tests/fixtures/mod.rs:578-597`): edge `"prefix-edge"` connects `5000*'p'` -> `5000*'p' + 'q'`. Callers `integration/graph.rs:330-345` assert `resolution == "zero"` on both endpoints, and `browser/graph.mjs:142-148` asserts `rect.graph-unresolved` count `== 2`.
3. `add_graph_highdegree` (`src/inspector/tests/fixtures/mod.rs:600-619`): writes 160 edges `"alpha" -> "missing-{i}"`. Caller `browser/graph.mjs:184-190` asserts `160` `path.graph-edge` elements, and `browser/comparison.mjs:109-120` asserts **`161 typed entries`** (`1` root node `"alpha"` + `160` edges).

- **Why naive `purge` or `transition(Deleted)` fails on all three:**
  - `Engine::purge` (`erasure.rs:903-914`) deletes `canonical_edges WHERE from_id = ?1 OR to_id = ?1`, which would delete `"missing"` and `"prefix-edge"` and break `worker.rs:973`, `browser/graph.mjs:49`, and `browser/graph.mjs:144`.
  - `Engine::transition(..., Deleted)` retains the `canonical_nodes` row, and Inspector's `graph_scan` (`src/inspector/src/graph.rs:105`, running under `Policy::default()` where `no_show_inactive = false`) resolves `Deleted` node rows (`resolution == "one"` instead of `"zero"`, and adds `39` extra nodes up to `budget` cap `200` in `graph-highdegree`, changing `comparison.mjs:110` from `161` to `200` entries).
- **Clean prerequisite remediation:** In `src/inspector/tests/fixtures/mod.rs`, seed the temporary active endpoint nodes alongside the edges through strict `Engine::write`, close the engine, and run a **fixture-only forensic post-close SQLite helper** (`remove_forensic_dangling_node_rows(&path, &[...])`) that deletes only the targeted endpoint node rows (simulating a pre-`0.8.31` / partially damaged store for Inspector's forensic unresolved-endpoint tests). This keeps `Engine::write` 100% strict while preserving every Inspector worker, integration, and browser assertion (`resolution == "zero"`, `rect.graph-unresolved == 2`, `161 typed entries`) untouched.

### 4.4 `GraphExpandRequestV1` Workspace Sites & Python Package Boundary Pins (`P4-13`, `X1`)

- All in-workspace `GraphExpandRequestV1 { ... }` literal constructions (`src/inspector/src/graph.rs:572-591`, `src/rust/crates/fathomdb-mcp/src/tools.rs:1468`, `evidence.rs:2804`, `graph_expand/mod.rs:54`, `codec.rs:711`, and test files) must switch to `GraphExpandRequestV1::new(...)` when `#[non_exhaustive]` lands.
- `src/python/tests/python_surface_additions.json`, `src/python/tests/test_slice130_package_boundary.py` (`native_stub_sha256` & wrapper declaration count), and `src/python/fathomdb/_fathomdb.pyi` must be updated in lockstep for `PropertyPredicate`, `PropertyComparisonOp`, `GraphTargetV1.properties`, `GraphOriginV1.terminal_edge_properties`, `PerHitExplain.{graph_seed_source, graph_hop_depth}`, `EvidenceGraphOriginV1`, `Engine.search`/`search_frozen`/`EvidenceSearchRequestV1` graph-seeding kwargs, and docstring fixes (`P6-02`).

---

## 5. TDD Implementation & Verification Plan

All work proceeds strictly **RED -> GREEN -> Refactor** (`AGENTS.md` §5): commit failing tests against real SQLite stores first, then implement until green.

### 5.1 RED Tests First (Committed Before Implementation)

1. **`tests/slice38_strict_endpoints.rs` + rewritten `pr_g8_dangling_edges.rs` + `write_commit.rs:1071` (`AC31-30-01`, `AC31-30-02`):**
   - Missing `from` / `to`, `InitialState::Pending`, transitioned `Pending` / `Deleted`, superseded-only, in-batch active-then-deleted, and **in-batch superseded dangling edge (both G0 `logical_id` and G11 `(from_id, to_id, kind)`)** -> `Err(EngineError::WriteValidation)` + zero rows across `canonical_nodes`, `canonical_edges`, `_fathomdb_artifact_revisions`, `canonical_property_values`, FTS, and projection queues after `close()` + reopen.
   - `actuate` with `[PutDerivedEdge, PutNode]` succeeds (`Committed`), while `[PutDerivedEdge(dangling)]` returns `ActuationOutcomeV1::Refused { reason: ReferenceUnavailable }` (`ActuationError` unchanged).
   - Valid same-batch `[Edge, Node(from), Node(to)]`, active self-loop, and `Authorizer` test proving node-only writes issue zero `canonical_nodes` reads.
   - Update `test_functional_search.py` and `functional-search.test.ts` dangling-edge tests to assert `WriteValidationError` + zero-row rollback.
2. **`tests/slice38_query_plans.rs` (Slice 30 `EXPLAIN QUERY PLAN` gates, `AC31-30-02`, `AC31-30-03`, `AC31-30-06`, `AC31-30-07`, `P5-07`):**
   - Write-time endpoint probe -> `SEARCH canonical_nodes USING COVERING INDEX canonical_nodes_logical_active_idx` (no `SCAN`).
   - Current `graph.expand` (`Outgoing`, `Incoming`, `Both`, empty and non-empty `edge_kinds`) -> `SEARCH canonical_edges USING INDEX canonical_edges_{from,to}_active_idx` (no `SCAN`).
   - `use_graph_arm` Phase 2 (`UNION ALL`) -> `SEARCH canonical_edges USING INDEX canonical_edges_{from,to}_active_idx`.
   - Unpredicated default `search` (`hybrid_deferred_identity`) / `search_text_only` -> zero joins on `canonical_edges` or `canonical_property_values`.
3. **`tests/slice38_graph_predicates_and_seeding.rs` + `proptest` + Py/TS parity (`AC31-30-03`–`AC31-30-08`, `AC31-30-10`, `P5-05`):**
   - **SQL pushdown, `t_valid`, self-loop, and `L895`/parallel-edge matrix:** superseded, wrong-kind, expired, and future-`t_valid` edges consume zero `LIMIT`/`work_units`; `Both` does not duplicate self-loops; failing `edge_predicates` on edge #1 does not block parallel valid edge #2 via `visited_by_seed`; terminal-depth `L895` preserves exact `origin_cmp` top-`N` selection (`result_limit` + `slice60_fix4_rss` bounds).
   - **Closed predicate & type matrix (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte` over `String | Int | Float | Bool`):** missing key -> `false` for all 6 ops; cross-type -> `false`; `i64::MIN`/`i64::MAX`/`±(2^53 + 1)` exact `NUMERIC` comparisons; `Bool` with `Lt..Gte` -> `GraphEdgePredicatesInvalid` / `GraphTargetPredicatesInvalid`; non-finite float -> refused; non-displacement of `result_limit` at depths `1..=3` & `max_work_units`; `GraphTargetV1.properties` + `GraphOriginV1.terminal_edge_properties` + `resolve_graph_evidence` parity; `encode_graph_evidence_request` tamper & predicate-order normalization; closed JSON wire decode rejections in Rust/Py/TS.
   - **`proptest` disabled-optimization equivalence:** generate random multigraphs (`String`/`Int`/`Float`/`Bool` + missing keys + multi-edges) and random `edge_predicates`/`target_predicates`; run `graph.expand` with properties **declared `Filterable`** (indexed `canonical_property_values` B-tree path) vs **undeclared** (inline evaluation) and assert exact equality of returned targets, origins, and properties.
   - **Adversarial `>= 12`-edge `SEED_FTS_N` proof:** write `>= 12` FTS-matching edges where the top-10 BM25 edges bridge an active hub and `Pending`/`Deleted`/filter-excluded endpoints (plus disjoint mixed pairs); assert eligible endpoints claim all `SEED_FTS_N = 10` slots with zero ineligible `seeds_considered` waste and edges `11..=12` still seed.
   - **Opt-in `graph_seed` (`GraphFts`, `FusedTopK`, `Both`) & seed provenance:** prove dense/reranked hits (`rerank_depth = 0` and `rerank_depth > 0`) seed body-less property-graph edges under `FusedTopK` & `Both`; prove no-embedder lexical fallback, `graph_edge_kinds`/`graph_edge_predicates` pruning, `graph_seed_k` bounds (`0` / `101` -> `InvalidArgument`), deterministic `Both` min-hop merge, `graph_seed_source` + `graph_hop_depth` on `PerHitExplain` & `resolve_evidence` (`EvidenceGraphOriginV1`), **byte-identical `use_graph_arm = false` baseline**, and **`purge`/`erase_source` non-resurrection** of graph/evidence properties (`AC31-30-10`).
   - **Cross-SDK parity:** `src/python/tests/test_slice38_graph_predicates.py` & `src/ts/tests/slice38-graph-predicates.test.ts` covering closed wire validation, predicate matrix, `FusedTopK`/`Both` body-less edge seeding, `graph_seed_source`/`graph_hop_depth`, and `GraphTargetV1.properties`/`GraphOriginV1.terminal_edge_properties`.

### 5.2 Ordered GREEN Implementation Sequence

1. **WP-30A — Strict Endpoint Integrity & Fixture Remediation (`P4-07`, `P4-08`, `P2-05`, `P5-04`, `P4-13` fixtures):**
   - Parameterize `EndpointValidationMode` in `write_commit.rs` (`Strict` vs `DeferredToActuationSimulation`), enforce all-edges check against `superseded_at IS NULL AND state = 'active'`, keep node-only zero-SQL fast path, thread through `actuation.rs` and `ingest.rs`, update legacy/convenience-edge tests and the 3 Inspector fixtures.
2. **WP-30B — `GraphExpandRequestV1` `#[non_exhaustive]` + Predicates + SQL Pushdown (`P4-10`, `P4-13`):**
   - Add `#[non_exhaustive]` + `::new(...)` + `PropertyPredicate`/`PropertyComparisonOp` + `GraphTargetV1.properties`/`GraphOriginV1.terminal_edge_properties`, update all workspace callers (`src/inspector/src/graph.rs`, `fathomdb-mcp`, tests), implement `graph_expand` SQL pushdown (`UNION ALL`, `superseded_at IS NULL`, `kind`, `t_valid`, `t_invalid`), fix `visited_by_seed` + `L895` + `next_frontier.push`, wire `codec.rs` + `encode_graph_evidence_request` + `evidence.rs` (`resolve_graph_evidence`), and update `slice60_graph_expand.rs` (`L462, L489, L754`).
3. **WP-30C — `search.rs` Graph Arm, `SEED_FTS_N` Fix, Opt-In `graph_seed` & Provenance (`P4-11`, `P5-05`):**
   - Fix Seed Source A mixed-edge slot bug, rewrite Phase 2 as `UNION ALL` + `graph_edge_kinds`/`graph_edge_predicates`, implement `GraphSeedMode` (`GraphFts | FusedTopK | Both`) + `graph_seed_k`, wire `CapturedGraphOrigin` -> `PerHitExplain` + `EvidenceGraphOriginV1` + HMAC token payload, and guarantee `use_graph_arm = false` zero-diff parity.
4. **WP-30D — Cross-SDK Bindings & Package Surface Pins (`X1`):**
   - Wire `fathomdb-sdk`, `fathomdb-py`, `src/python`, `fathomdb-napi`, `src/ts`, update `python_surface_additions.json`, `test_slice130_package_boundary.py`, `_fathomdb.pyi`, `fathomdb-sdk/tests/surface.rs`, and `slice60_governed_surface.rs`.
5. **WP-30E — Interface & Public Documentation Parity (`P3-02`, `P3-08`, `P3-09`, `P6-02`, `P6-04`, `P6-06`, `X4`):**
   - Update `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md`, `docs/guides/graph-retrieval.md`, and `docs/reference/{python-api,typescript-api,rust-sdk,rust-api,errors}.md` (zero external consumer-project specifics), plus `slice-30/status.md` & `release-state-0.8.31.json` advancement when closed.

### 5.3 Proportional Verification Commands

Execute inside the Slice 30 worktree with checkout-owned `.venv` (`AGENTS.md` §3):

1. **Workspace compile, lint & typecheck:**
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `./scripts/agent-lint-migrations.sh`
   - `cargo check --workspace --locked`
2. **Rust tests (targeted Slice 30 + affected suites + full engine/sdk/inspector):**
   - `cargo test -p fathomdb-engine --test slice38_strict_endpoints --test slice38_graph_predicates_and_seeding --test slice38_query_plans --test pr_g8_dangling_edges --test slice60_graph_expand --test slice60_wire --test slice60_fix4_rss --test slice60_fix5_rss --test slice20_graph_evidence --test slice20_graph_traversal --test pr_c1_graph_seeding --test slice35_remaining_arm_matrix --test slice50_evidence --test slice85_physical --test opp12_existence_axis --test write_boundary_atomicity`
   - `cargo test -p fathomdb-engine --lib` (includes `write_commit::tests::node_only_dangling_check_does_not_require_endpoint_reads` and `graph_expand::graph_evidence_request_tests`)
   - `cargo test -p fathomdb-sdk -p fathomdb --all-targets`
   - `cargo test -p fathomdb-inspector` (worker & integration suites covering the 3 updated graph fixtures)
3. **Python rebuild, typecheck, lint & parity tests:**
   - `./scripts/setup-python-owned.sh --build` (rebuilds native extension inside checkout-owned `.venv`)
   - `ruff check src/python`
   - `pyright -p src/python` (must report 0 errors, 0 warnings)
   - `pytest src/python/tests/test_slice38_graph_predicates.py src/python/tests/test_functional_search.py src/python/tests/test_slice60_graph_expand.py src/python/tests/test_slice20_graph_evidence.py src/python/tests/test_slice130_package_boundary.py src/python/tests/test_slice30_embedding_readiness.py`
4. **TypeScript native rebuild, build & tests:**
   - `cd src/ts && npm run build && npm test` (including `slice38-graph-predicates.test.ts`, `functional-search.test.ts`, `sdk-surface-parity.test.ts`, `slice30-embedding-readiness.test.ts`)
5. **Docs, Markdown & Interface Gates:**
   - `./scripts/agent-lint-md.sh`
   - `./scripts/agent-lint-docs.sh`
   - `python3 scripts/check-public-doc-truth.py`
   - `python3 scripts/check-sdk-surface-parity.py`
   - `./scripts/check-governed-surface-pin.sh`
   - `mkdocs build --strict`
   - Grep check confirming zero external consumer-project specifics in any Slice 30 diffed/created file.

---

## 6. Slice 30 Exit Criteria (DoD)

1. Strict endpoint validation (`Engine::write`, `Engine::actuate`), all `GraphExpandRequestV1` predicates + `#[non_exhaustive]` + SQL pushdown + `UNION ALL` + `t_valid` + `L895` fix, mixed-edge `SEED_FTS_N` fix, opt-in `graph_seed` + provenance, and legacy/convenience/Inspector fixture updates land cleanly via **RED -> GREEN** commits.
2. Independent adversarial review (`ROLE_REVIEWER`, `TIER_DEEP`) returns `APPROVE` with zero open P1/P2 findings.
3. All proportional verification checks in §5.3 pass with zero skipped tests or regressions (`use_graph_arm=false` byte-identical; `slice38_query_plans.rs` `SEARCH`-only; `pyright` 0/0; `agent-lint-md.sh` & `agent-lint-docs.sh` green; zero consumer-project specifics).
4. `dev/plans/0.8.31/features/slice-30/status.md` recorded and `dev/plans/release-state-0.8.31.json` advanced to `next_slice: 40`.

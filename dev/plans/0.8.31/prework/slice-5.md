---
title: 0.8.31 Prework Slice 5 — Testing and verification adequacy
status: COMPLETE
target_release: 0.8.31
slice: 5
date: 2026-11-22
---

# 0.8.31 Prework Slice 5 — Testing and verification adequacy

Read-only audit of `dev/plans/0.8.31/plan.md`, `dev/plans/0.8.31/scope.md`,
`dev/notes/fathomdb-quality-testing-roadmap.md`, `dev/plans/0.8.30/coverage-audit.md`,
all four Slice 0 ADRs, `dev/plans/0.8.31/features/slice-0/{w12-fence-review.md,baseline-0.8.30-latency.json,golden-0.8.30-manifest.json}`,
`experiments/sdk_benchmark.py`, `src/conformance/`, and test suites across
`src/rust/crates/fathomdb-{schema,engine,sdk,cli,mcp}/tests/` (and inline `src/`
tests), `src/python/tests/`, `src/ts/tests/`, and `src/inspector/tests/` in
`/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`.

## 1. Quality roadmap fit and `R31-*` falsifiability audit

Per `dev/notes/fathomdb-quality-testing-roadmap.md` and `plan.md`, `0.8.31`
retains **anomaly injection**, **resource-leak checks**, **targeted
branch/mutation checks**, **boundary-value property/state-machine tests**, and
**disabled-optimization equivalence tests**, while broad coverage-guided
fuzzing and power-loss proofs (`dev/plans/0.8.31/quality-backlog.md`) remain
re-slotted to `0.8.32`.

| ID | Current acceptance posture | Concrete falsifiability gaps & unquantified boundaries |
| --- | --- | --- |
| **R31-01** | **Mostly falsifiable** (real-DB TDD + `proptest`/Hypothesis + typed errors) | 1. **Numeric & parser extremes unpinned**: must test `i64::MIN`/`i64::MAX` and `abs(x) > 2^53` exact `NUMERIC` precision (vs. lossy `f64` cast), `-0.0`, `NaN`/`±Inf`, nested object/array, empty key, and `\x1e`/NUL rejection returning `EngineError::WriteValidation` (`WriteValidationError` Py/TS, `ErrorKind::InvalidInput` SDK) with zero-row atomic rollback.<br>2. **Precedence & lifecycle transition**: must pin `properties` precedence over `json_extract(body, ...)` on matching declared keys, `Deleted`/`Pending` purge of `canonical_property_values`, and `Active` re-projection.<br>3. **Reserved `scope` & `vec0`**: `scope TEXT NOT NULL DEFAULT ''` across all 4 tables and `vec0` pre-KNN `attr_<hex>` column wiring for declared node properties lack explicit acceptance checks in `plan.md`.<br>4. **Custom vector kinds (`R1`) vs. `tc67` ambiguity**: must pin **explicit** opt-in enrollment (so unenrolled `"entity"`/`"invoice"` stay in `vector_unsupported_kinds`, preserving `tc67`, while explicitly enrolled custom kinds vector-index cleanly).<br>5. **Op-collection readback (`R12`)**: must assert deterministic sorted listing, internal/audit collection handling (`excise_source_audit`, `erasure_pending_redaction`), and zero-mutation refusal on conflicting `kind` re-registration. |
| **R31-02** | **Partially falsifiable** (operators named, slot & provenance behavior described) | 1. **Missing-key & cross-type comparison matrix**: `plan.md` does not pin behavior for absent keys under `=`, `!=`, `<`..`>=`, cross-type scalar comparisons (`Int` vs. `String`, `Int` vs. `Float` precision), or multi-predicate `AND` conjunction. Requires **disabled-optimization equivalence** (declared `canonical_property_values` B-tree path == inline evaluator path).<br>2. **Non-displacement & bounds**: must prove filtered-out edges/targets do not consume `result_limit` (depth `1..3`) and verify deterministic behavior at `max_work_units` boundary.<br>3. **Mixed-edge `SEED_FTS_N` slot proof**: requires an adversarial corpus with `>= 11` matching edges where outranking edges touch ineligible endpoints, proving eligible edges still populate the `SEED_FTS_N = 10` slots.<br>4. **Fused-hit seeding (`graph_seed`)**: must pin `graph_seed_k` bounds, no-embedder/no-reranker graceful fallback, deterministic merge/priority rules when `Both` hits overlap across hop depths (`graph_seed_source` + `graph_hop_depth`), and byte-identical zero-overhead `use_graph_arm=False` parity. |
| **R31-03** | **Underspecified** (conceptual path-before/after test only) | 1. **Boundary operators unpinned in `plan.md`**: must explicitly pin inclusive creation (`<=`) and exclusive supersession (`>`) for `known_as_of` / `as_of_write_cursor` / `as_of_epoch_ms`, half-open validity `[t_valid, t_invalid)` composition, and interaction when `valid_as_of` or `known_as_of` is omitted.<br>2. **Endpoint-version coherence**: must prove target/intermediate nodes ALSO resolve to their revision at `as_of` (never leaking post-`as_of` edits or later `Deleted` transitions into historical slices, while honoring `Pending`/`Deleted` states that already held at `as_of`).<br>3. **Dual index plans**: must verify via `EXPLAIN QUERY PLAN` that current traversal uses `canonical_edges_{from,to}_active_idx` (`superseded_at IS NULL`) and `as_of` uses `canonical_edges_{from,to}_kind_sup_idx` without full `SCAN`.<br>4. **Subordination to erasure**: must prove `purge`/`erase_source` targets NEVER reconstruct at any historical `as_of` boundary. |
| **R31-04** | **Mostly falsifiable** (atomic refusal after reopen) | 1. **All inactive states**: must explicitly cover missing IDs, `InitialState::Pending`, transitioned `LifecycleState::Pending`, transitioned `LifecycleState::Deleted`, and **in-batch** create-then-delete/supersede ordering (in both `engine.write` and `actuate`), plus valid same-batch reverse ordering and active self-loops.<br>2. **Validation precedence**: malformed provenance / syntactic `WriteValidation` checks must run cleanly (keeping existing error reasons intact) and endpoint probes must use `canonical_nodes_logical_active_idx` via `SEARCH` while node-only batches read zero `canonical_nodes` rows. |
| **R31-05** | **Partially falsifiable** (unblockers named, bounds/faults omitted) | 1. **`resolve_evidence_many`**: needs falsifiable batch limits, empty/duplicate handling, whole-batch failure on malformed `ContextToken` vs. partial per-item errors alongside valid siblings, property hydration parity, and `governed-operation-parity.json` registration.<br>2. **`OpenReport` & `search(hints=)`**: must test compiled/absent reranker & CUDA combinations cleanly across all 3 SDKs and pin exact `{no_results, degraded, more_results}` parity across `search`, `search_frozen`, and `EvidenceSearchRequestV1`.<br>3. **Minimal 37→38 `fathomdb migrate`**: `plan.md` omits fail-closed checks pinned by ADR (`Engine::open` continues rejecting schema 37; `migrate` refuses active lock, `user_version != 37`, unfinished `_fathomdb_dependency_closures`/`_fathomdb_erasure_requests`, and genuinely dangling edges while passing valid historical superseded edges; mid-DDL fault rollback preserves schema-37 byte-for-byte + SHA-256 backup restoration; post-upgrade golden DB passes `doctor check-integrity --full`). |
| **R31-06** | **Strong** (`sqlite_master`, `purge`/`erase_source`, W12) | 1. **Inline property scrubbing & envelope**: `physical_erasure/admission.rs` byte accounting and physical scans (`main`, `-wal`, `-shm`, telemetry sink) must explicitly exercise inline `properties` on **both** nodes and nullable-`body` edges plus `canonical_property_values`, `vec0` (`attr_*` + custom kinds), and `ExportClass` classification.<br>2. **W12 Ask 3a/3b/3c edge cases**: (3a) prove unrelated plain writes pass + advance `safe_boundary` during Autonomous checkpoint BUSY while denied-source/dependency writes remain fenced and `validate_physical_zero` holds after retry; (3b) prove cross-source `finish_redacted_before_request` guard writes `excise_source_audit` whenever `nodes_excised + edges_excised > 0` and suppresses duplicate 0-count rows only on pure completion; (3c) prove production `ErasureIncomplete.detail` classification without `FATHOMDB_WAL_ATTRIBUTION` env set. |
| **R31-07** | **Non-falsifiable as phrased** (**CRITICAL GAP**) | 1. **"No statistically significant p50/p95 regression" is unquantified**: neither `plan.md` nor `experiments/sdk_benchmark.py` defines alpha, bootstrap resamples/CI, noise floor, or warmup protocol (`sdk_benchmark.py` computes only raw `_nearest` quantiles and has no built-in paired test).<br>2. **Scale mismatch (`5k` vs. `10k / 50k / 100k`)**: `features/slice-0/baseline-0.8.30-latency.json` (`capture-baseline-and-golden.py`) captured **only 5,000 nodes / 7,500 edges / 100 queries** (`warm_open.n = 5`, `write_node_batch_200.n = 25`, `write_edge_batch_200.n = 38`), NOT `10k / 50k / 100k`! Slice 60 MUST run same-host interleaved `v0.8.30` (`db271323f`) vs. `0.8.31` pairs across **10k, 50k, and 100k** with explicit bootstrap 95% CI upper-bound gates (e.g., `<= max(5%, 1.0 ms)` on p50/p95 + bounded DB-bytes/RSS), plus new property/predicate/`as_of` micro-gates. |
| **R31-08** | **Mostly falsifiable** (aligned with `prework/slice-7.md` `P7-01`–`P7-06`) | 1. **`0.8.31` Inspector qualification gap**: needs explicit Rust `admission::validate_layout`, worker IPC, and Playwright browser tests for schema-38 columns/tables, node+edge inline `properties`, `as_of` historical graph inspection + 8-flag erasure non-disclosure, and `compiled_schema = 38` (`local-release.py`).<br>2. **Feature-matrix parity**: assert `cargo test -p fathomdb-inspector` runs cleanly on both model-free default and `--no-default-features --features cached-cpu`. |

Cross-cutting DoDs **X1–X4** remain sound as slice checklists provided **X1** binds both `governed-operation-parity.json` and `test_slice130_package_boundary.py` / `surface.rs` / `governed_surface.rs`, and **X3** names concrete SQL statements per slice rather than relying solely on wall-clock timing.

---

## 2. Proposals & required new test matrix (`P5-01` .. `P5-12`)

### Slice 10 — Property graph core, vector kinds, op-collections, W12, `sqlite_master` audit

- **P5-01 — Pin `R1` custom vector-kind opt-in semantics before coding to preserve `tc67` semantics** *(Target: Slice 10, `R31-01`)*
  - Specify in `dev/design/slice-10-property-schema-design.md` that custom kinds become vector-committable (`source_type = kind`) **only** when explicitly enrolled (e.g., dedicated vector-kind registration / `admin.configure` / `configure_projections` parameter or explicit `_fathomdb_vector_kinds` enrollment) — **never** auto-enrolling every arbitrary corpus kind seen during `configure_projections([vector_spec(...)])`. This preserves `tc67_unsupported_vector_kind_report` and `slice22_projection_status` across Rust, Python, and TypeScript (`entity`, `invoice` stay reported in `vector_unsupported_kinds` unless enrolled), while letting apps enroll `"mms1:memory"` / `"custom:chunk"` and verify end-to-end `drain()` + `vec0` search + reopen persistence + purge/erase.

- **P5-02 — Inline properties + declared `canonical_property_values` TDD & boundary/property tests** *(Target: Slice 10, `R31-01`, `X1`, `X3`)*
  - **`fathomdb-schema` (`tests/slice45_step33.rs` + new `tests/step38_property_schema.rs`)**: assert `SCHEMA_VERSION == 38`, additive `ALTER TABLE` columns (`properties`, `scope TEXT NOT NULL DEFAULT ''` across `canonical_nodes`, `canonical_edges`, `operational_mutations`, `operational_state`), `canonical_property_values` schema & 4 composite indexes, `canonical_edges_{from,to}_active_idx`, `canonical_edges_{from,to}_kind_sup_idx`, and `60` read-visibility triggers (`20 * 3`, adding `_fathomdb_read_visibility_cpv_{ai,au,ad}` firing on INSERT/UPDATE/DELETE).
  - **`fathomdb-engine` (new `tests/slice38_properties_and_indexes.rs` + `proptest`)**:
    - Real-DB round-trip (`String`, `Int`, `Float`, `Bool`) outside `body` for nodes + edges + `actuate`; verify O(1) hydration on `SearchHit`, `read_get`, `read_get_many`, `read_list`, and resolved evidence, and verify `body`-FTS & 512-wordpiece embedding never index property keys/values.
    - **`proptest` codec & numeric boundary suite**: fuzz `BTreeMap<String, PropertyValue>` + `i64::MIN`/`i64::MAX`/`±(2^53 + 1)` exact `NUMERIC` comparisons (proving `2^53 + 1 != 2^53`), `-0.0`, `NaN`/`±Inf`, nested JSON, empty keys, `\x1e`/NUL keys & strings -> `EngineError::WriteValidation` + zero-row atomic rollback.
    - **Declared `Filterable` B-tree & `vec0` sync + lifecycle**: assert precedence (`properties` > `json_extract(body, ...)`), immutable revisioning in `_fathomdb_artifact_revisions`, cleanup on node supersession + edge G0 (`logical_id`) & G11 (`(from_id, to_id, kind)`) supersession, purge on `Deleted`/`Pending` transition & re-projection on return to `Active`, typed error on undeclared `SearchFilter` keys, and `vec0` pre-KNN filtering for declared node properties + custom vector kinds.
  - **`R12` op-collections**: test `list_operational_collections` / `read.operational_collections` / `read.operational_collection(name)` parity across Rust/Py/TS and atomic `WriteValidation` error on conflicting `kind` re-registration.
  - **Cross-SDK parity (`src/python/tests/test_slice38_properties.py` w/ Hypothesis + `src/ts/tests/slice38-properties.test.ts`)**: shared fixture proving identical typed property round-trips, `SearchFilter` pre-filtering, custom vector kind indexing, and op-collection readback/refusal across Rust, Python (`hypothesis`), and TS.

- **P5-03 — `sqlite_master` + `ExportClass` completeness audit, `properties` erasure, and W12 regression suite** *(Target: Slice 10, `R31-06`, `X2`)*
  - **`fathomdb-engine/src/lib.rs` & new `tests/slice38_erasure_and_audit.rs`**:
    - Verify `canonical_property_values` is registered in `ROW_OWNED_PROJECTIONS` and triggers `guard_row_owned_registry` / `slice15d_attribute_projections_registered_and_guard_bites` style negative checks if hypothetically removed.
    - Add an automated `sqlite_master` completeness test asserting every non-internal table in a schema-38 DB has an explicit classification (`CANONICAL_TABLES` vs `ROW_OWNED_PROJECTIONS` vs internal `_fathomdb_*`), read-visibility trigger coverage (`VISIBILITY_TRIGGER_TABLES`), erasure reachability, `doctor` coverage, Inspector exposure, and `ExportClass` (`DurableAuthority | OptInVector | OptInAudit | DerivedProjection | InternalRuntime`) — plus a negative test ensuring an unclassified dummy table fails the audit.
    - **RED→GREEN erasure proof (Autonomous + ExplicitMaintenance)**: write nodes + nullable-`body` edges with inline `properties` (both declared & undeclared) and custom-kind vectors; run `purge` and `erase_source`; assert **zero rows** in `canonical_property_values`, `canonical_nodes`, `canonical_edges`, `vector_default`, zero unique byte substrings across raw SQLite pages (`main`, `-wal`, `-shm`) and telemetry sink, and `physical_erasure/admission.rs` (`envelope`) properly charges `properties` bytes even when `body IS NULL`.
    - **W12 Ask 3a/3b/3c tests**:
      - (3a) Hold `wal_checkpoint(TRUNCATE)` BUSY in Autonomous mode: assert unrelated plain writes succeed + advance `safe_boundary`, denied-source/identity and dependency/actuation writes still fail closed, checkpoint completion succeeds once reader releases, and `validate_physical_zero` holds; verify ExplicitMaintenance sequence unchanged; audit `ActualCheckpointObserver` assertions (`test_slice65_wal_attribution_installed.py:218`, `src/rust/crates/fathomdb-engine/src/lib.rs`).
      - (3b) Test `completed_pending_erasure` (`completedPendingErasure` in TS) = `true` on both `finish_redacted_before_request` and `root_retry || !pending.is_empty()` paths, `false` on fresh/already-erased no-ops, **no duplicate zero-count `excise_source_audit` row** on pure completion, AND **cross-source audit retention**: completing prior Source A's redacted closure while excising real rows in Source B sets `completed_pending_erasure = true` AND writes Source B's `excise_source_audit` row!
      - (3c) Without setting `FATHOMDB_WAL_ATTRIBUTION`, trigger `ErasureIncomplete { stage: "wal_checkpoint" }` via owned reader snapshot vs. external connection; verify detail omits false `"a concurrent reader is pinning"` blame on external holders, names accurate `WalAttributionCollector` classification/roles, preserves `"wal_checkpoint(TRUNCATE)"` + `"BUSY"` + `"frames still in the log"`, and distinguishes initial vs. retry wording.

### Slice 20 — Strict endpoint integrity, typed traversal predicates, fused seeding, mixed-edge slot fix

- **P5-04 — Strict write endpoint integrity & stateful atomicity suite** *(Target: Slice 20, `R31-04`)*
  - Rewrite `tests/pr_g8_dangling_edges.rs` and `write_commit.rs:1071` + Python/TS dangling-edge tests (`test_functional_search.py`, `functional-search.test.ts`), and add `tests/slice38_strict_endpoints.rs`:
    - Test `Engine::write` AND `Engine::actuate` across every failure case: missing `from`/`to`, `InitialState::Pending` endpoint, transitioned `LifecycleState::Pending` / `Deleted` endpoint, superseded-only endpoint with no active revision, and **in-batch node written active then invalidated/deleted before commit** -> atomic `EngineError::WriteValidation` (`WriteValidationError` Py/TS) and zero committed rows in `canonical_nodes`, `canonical_edges`, `_fathomdb_artifact_revisions`, `canonical_property_values`, FTS, and projection queues verified after `close()` + reopen.
    - Test positive cases: edge listed *before* its active target node in the same batch (`dangling_edge_endpoints == 0`), valid self-loop on active node, and node-only fast-path reading zero `canonical_nodes` rows via SQLite authorizer.
    - Pin syntactic/provenance validation ordering before DB endpoint lookup so `test_slice15_identity_provenance.py`, `test_provenance_mandatory.py`, and their TS counterparts keep exact `ProvenanceError` / `WriteValidationError` precedence.

- **P5-05 — Typed traversal predicates, `proptest` disabled-optimization equivalence, `SEED_FTS_N` slot fix, and `FusedTopK` seeding** *(Target: Slice 20, `R31-02`, `X1`, `X3`)*
  - **`fathomdb-engine/tests/slice38_graph_predicates_and_seeding.rs` + Py/TS parity (`test_slice38_graph_predicates.py`, `slice38-graph-predicates.test.ts`)**:
    - Test closed `PropertyPredicate` (`Eq`, `Ne`, `Lt`, `Lte`, `Gt`, `Gte`) over `edge_predicates` & `target_predicates` on `graph.expand` and `graph_edge_kinds` / `graph_edge_predicates` on `use_graph_arm=True`, covering absent keys, type mismatches, extreme ints, inline `GraphTargetV1.properties` / `GraphOriginV1.terminal_edge_properties`, and proving ineligible edges/targets never enter paths or consume `result_limit` (`depth = 1..3`, `max_work_units`).
    - **`proptest` disabled-optimization equivalence** (per `dev/notes/fathomdb-quality-testing-roadmap.md`): generate random multigraphs + random predicates, execute with properties **declared `Filterable`** (indexed path) vs **undeclared** (inline scan) on the same corpus, and assert set equality of returned target/edge cursors.
    - **Mixed-edge `SEED_FTS_N` slot regression**: create `>= 12` matching edges where top BM25 edges connect ineligible (e.g. `Pending`/`Deleted`) endpoints and assert eligible edges still claim `SEED_FTS_N = 10` seed slots.
    - **Opt-in `graph_seed` (`GraphFts | FusedTopK | Both`) & seed provenance**: test body-less edges seeded from vector/reranked top-k hits (`graph_seed_source`, `graph_hop_depth` on hits/explanation/`resolve_graph_evidence`), deterministic tie/min-hop merge under `Both`, `graph_seed_k` validation, no-embedder lexical fallback, and **zero-diff equivalence** when `use_graph_arm=False`.

### Slice 30 — Historical graph reconstruction (`as_of`)

- **P5-06 — Bitemporal `as_of` graph reconstruction, boundary `proptest`, and erasure non-resurrection** *(Target: Slice 30, `R31-03`, `R31-06`, `X1`, `X3`)*
  - **`fathomdb-engine/tests/slice38_historical_graph.rs` + Py/TS parity (`test_slice38_historical_graph.py`, `slice38-historical-graph.test.ts`)**:
    - **Four-quadrant bitemporal matrix**: test divergent `t_valid`/`t_invalid` vs. write-supersession boundaries (`as_of_write_cursor` / `as_of_epoch_ms` + `valid_as_of`), proving exact chain reconstruction *before* and *after* edge supersession, boundary equality (`created <= known_as_of` inclusive, `superseded_at > known_as_of` exclusive; `[t_valid, t_invalid)`), and **node+edge temporal coherence** (reconstructing at `T` returns target node's body, `properties`, and lifecycle state as of `T`, ignoring post-`T` updates or deletions while respecting pre-`T` `Deleted`/`Pending`).
    - **`proptest` reference oracle**: execute random sequences of node/edge writes, property updates, edge supersessions, validity edits, and lifecycle transitions recording state snapshots at cursors `1..N`, then compare `graph.expand(as_of)` at random `(cursor, valid_as_of)` pairs against the exact snapshot oracle.
    - **Erasure subordination**: run `purge(target)` and `erase_source(sid)` across superseded/historical chains and assert zero rows/paths reconstruct at ANY `as_of` boundary (including `as_of_write_cursor` from before erasure!).

### Slices 10–30 Cross-cutting SQLite Planner (`EXPLAIN QUERY PLAN`) Pins

- **P5-07 — `EXPLAIN QUERY PLAN` `SEARCH`-only regression pins for all schema-38 query paths** *(Target: Slices 10, 20, 30, `X3`, `R31-07`)*
  - Extend `slice19_join_indexes.rs` / `perf_gates.rs` / `slice60_graph_expand.rs` patterns (`explain_graph_expand_incident_query` helper) into a unified `tests/slice38_query_plans.rs` gate asserting zero unexpected `SCAN`:
    1. Write-time edge endpoint validation -> `SEARCH ... USING COVERING INDEX canonical_nodes_logical_active_idx` (and node-only batches skip SQL).
    2. `SearchFilter` property predicates -> `SEARCH canonical_property_values USING COVERING INDEX canonical_property_values_{text,num,bool}_idx` + `SEARCH canonical_nodes USING INDEX canonical_nodes_write_cursor_idx`.
    3. Current `graph.expand` (`Outgoing`, `Incoming`, `Both`, with and without `edge_kinds`) -> `SEARCH canonical_edges USING INDEX canonical_edges_{from,to}_active_idx`.
    4. Historical `as_of` `graph.expand` -> `SEARCH canonical_edges USING INDEX canonical_edges_{from,to}_kind_sup_idx`.
    5. **Unpredicated fast-path invariance**: capture `EXPLAIN QUERY PLAN` strings for unpredicated `search_text_only`, `search` (`hybrid_deferred_identity`), and default `graph.expand`, asserting **zero joins** on `canonical_property_values` and `superseded_at` historical scans when property/history options are absent.

### Slice 40 — Memex SDK unblockers, `fathomdb migrate` 37→38 helper, and PyPI `fathomdb-cli` wheel

- **P5-08 — `resolve_evidence_many`, `search(hints=)`, `OpenReport` booleans & parity manifest updates** *(Target: Slice 40 (fields from Slice 10), `R31-05`, `X1`)*
  - Lockstep update of `src/conformance/governed-operation-parity.json` (bumping live op count from `53` in `sdk-surface-parity.test.ts:98`), `src/conformance/governed-surface-allowlist.json`, `src/rust/crates/fathomdb-sdk/tests/surface.rs`, `src/rust/crates/fathomdb/tests/governed_surface.rs`, `src/python/fathomdb/_fathomdb.pyi`, `src/python/tests/python_surface_additions.json`, `test_slice130_package_boundary.py`, and `test_openreport.py` / `openreport.test.ts` fixtures.
  - Behavioral tests across Rust/Py/TS for:
    - `resolve_evidence_many`: empty batch, over-limit refusal, duplicate refs, mixed valid + missing/erased refs returning partial per-ref successes + typed errors in order, property hydration parity, and whole-batch refusal on tampered `ContextToken`.
    - `search(hints=)` across `search`, `search_frozen`, `EvidenceSearchRequestV1` (`no_results`, `degraded`, `more_results` matching `ADR-0.8.29-agent-hints-sidecar.md`, default omitted/`None` unchanged).
    - `OpenReport`: `reranker_compiled`, `cuda_compiled`, `rerank_available` matching feature flags (`--features default-reranker` vs. default) and runtime capability.

- **P5-09 — `fathomdb migrate` 37→38 fault-injection, golden DB upgrade, and PyPI `fathomdb-cli` wheel tests** *(Target: Slice 40, `R31-05`, `P7-05`)*
  - **CLI tests (`src/rust/crates/fathomdb-cli/tests/slice38_migrate_cli.rs` + `parser.rs` preserving 17 `doctor` subcommands in `slice55_composed_recovery.rs:318-339`)**:
    - Assert public `Engine::open` continues rejecting schema 37 directly (`IncompatibleSchemaVersion { seen: 37, supported: 38 }`).
    - Precondition refusals (zero target mutation verified by pre/post SHA-256): missing `--backup-dir` / occupied backup, active sidecar lock held, `user_version != 37` (`36` and `38`), incomplete physical erasure in `_fathomdb_dependency_closures` (`phase != 'complete'`) or `_fathomdb_erasure_requests`.
    - **Dangling-endpoint preflight**: prove a schema-37 DB containing genuinely dangling edges fails closed with diagnostic JSON listing dangling refs, while a schema-37 DB whose edges touch previously valid-then-superseded historical nodes **succeeds**.
    - **Crash/anomaly rollback test** (per quality roadmap): inject a DDL failure before `PRAGMA user_version = 38` inside `BEGIN IMMEDIATE`, assert exit non-zero, target DB remains byte-for-byte schema 37, and SHA-256-verified backup is valid.
    - **End-to-end golden DB verification**: run `fathomdb migrate` on a copy of `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite` (created in Slice 0); verify `user_version == 38`, `doctor check-integrity --full` clean, `req:1`/`ver:1`/`edge:verifies:1` + `agent_checkpoints`/`agent_audit` + `vec0` preserved, `secret:1` stays erased, and subsequent `0.8.31` property writes + `as_of` traversal + erasure succeed.
  - **PyPI binary wheel test (`P7-05`)**: install built `fathomdb-cli` (+ `fathomdb[cli]`) wheel into a clean venv with no Rust/Cargo on `PATH` and execute `fathomdb --version`, `fathomdb doctor check-integrity`, and `fathomdb migrate` end-to-end.

### Slice 50 — Inspector `0.8.31` upgrade & CI/harness hardening

- **P5-10 — Inspector schema-38 admission, property + `as_of` worker/browser qualification, and scoped CI/timeout tests** *(Target: Slice 50, `R31-08`, `P7-01`–`P7-04`, `P7-06`)*
  - **Core Inspector updates & tests (`src/inspector/src/admission.rs`, `src/inspector/tests/{worker.rs,integration/,browser/}`)**:
    - Update `validate_layout` (`admission.rs`) and `compiled_schema: 38` (`local-release.py`, `test_local_release.py`), plus update dangling fixtures (`add_graph`, `add_graph_missing_prefix`, `add_graph_highdegree`) cleanly via purge-after-write.
    - Add worker + Playwright browser specs verifying: (1) inline typed node & edge `properties` display/copy/XSS-escaping (`img` injection safe, truncation preview vs. chunk distinction), (2) `as_of` historical graph slider/cursor reconstruction showing superseded edges before vs. after supersession and preserving non-disclosure of erased rows across all 8 `(n, e, t)` policy flags.
    - Verify `cargo test -p fathomdb-inspector` green under **both** `--no-default-features` and `--no-default-features --features cached-cpu`, plus `P7-01` (`local-ci.sh --scoped` / `--full-workspace` in `test_local_ci.py`), `P7-02` (`timeout --kill-after` bounds in `run-installed.sh` / `installed-harness.mjs` / `launch.rs`), and `D4 = B` native checks.

### Slice 60 — Full-workspace gate & Paired 10k / 50k / 100k latency gate

- **P5-11 — Quantified paired `v0.8.30` vs. `0.8.31` benchmark protocol at 10k / 50k / 100k** *(Target: Slice 60, `R31-07`, `X3`)*
  - Resolve the **5k vs. 10k/50k/100k** and statistical-significance gap in `plan.md`:
    1. Build paired artifacts on the same host (exact `v0.8.30` baseline @ `db271323f` vs. `0.8.31` candidate) using `experiments/sdk_benchmark.py` (with explicit seed/pinning/warmup) plus paired `capture-baseline-and-golden.py` parity at `5k` against `baseline-0.8.30-latency.json`.
    2. Measure **10k, 50k, and 100k** scales (nodes + 1.5x edges) across all five required unpredicated paths: **open** (cold & warm), **batch write** (node + edge batches), **streamed FTS**, **optimized hybrid (`hybrid_deferred_identity`)**, and **current-time 2-hop `graph.expand`**, plus `database_bytes` and `peak_rss_bytes`.
    3. Quantify "no statistically significant p50/p95 regression" using **10,000-resample paired bootstrap** (`comp-03` precedent) over repeated measured iterations: gate fails if the lower/upper 95% CI of `(0.8.31 - 0.8.30)` degrades `p50` or `p95` by more than `max(5.0%, 1.0 ms)` (or `database_bytes` / `peak_rss_bytes` grows beyond declared schema-38 DDL overhead budget).
    4. Add dedicated `0.8.31` performance sanity checks for high-fanout predicate evaluation (`canonical_property_values` B-tree vs. unindexed threshold) and `as_of` composite seeks (`historical_reconciliation` scaling check style).

- **P5-12 — Full `./scripts/agent-verify.sh`, `AGENT_LONG=1`, security & archive closure** *(Target: Slice 60, `R31-07`, `R31-08`)*
  - Execute full `./scripts/agent-verify.sh` (unconfined if ptrace sandbox restricts `AC-036`), `AGENT_LONG=1 ./scripts/check.sh`, `local-ci.sh --full-workspace` + `D4 = B` Inspector qualification, `fathomdb-cli` PyPI wheel & `fathomdb[cli]` installed clean-venv smoke, security `timeout`-wrapped `strace`/`unshare` probes (`P7-03`), and archive a closed schema-38 golden `0.8.31` DB + manifest for `0.8.34`.

---

## 3. Existing tests expected to break intentionally in `0.8.31`

### 3.1 `SCHEMA_VERSION` 37 → 38, visibility triggers (57 → 60), & Inspector `compiled_schema` (Target: Slice 10 / 50)

| File | Exact test / line(s) | Reason & remediation |
| --- | --- | --- |
| `src/rust/crates/fathomdb-schema/tests/slice45_step33.rs` | `L27`: `assert_eq!(SCHEMA_VERSION, 37)` | Bump constant assertion to `38`. |
| `src/rust/crates/fathomdb-engine/src/lib.rs` | `L1189-1190` unit test (`schema_version_after`, `step_id` `== 37`) | Bump to `38`. |
| `src/rust/crates/fathomdb-engine/tests/slice40_fresh_database_cutover.rs` | `L79, 146-147, 156, 182, 203-205, 211-212, 216-217, 257, 279, 285` | Bump `37 -> 38`; at `L257`, `38` is no longer future (`("future", 39)`), and schema `37` becomes rejected prior version (`IncompatibleSchemaVersion { seen: 37, supported: 38 }`). |
| `src/rust/crates/fathomdb-engine/tests/slice60_store_format.rs` | `L26, 51`: `user_version == 37`, `supported: 37` | Bump to `38`. |
| `src/rust/crates/fathomdb-engine/tests/truncate_wal.rs` | `L75, 213, 223, 230, 239, 249, 297, 310` | Bump `PRAGMA user_version=38` and `supported: 38`. |
| `src/rust/crates/fathomdb-engine/tests/slice95_visibility_coalescing.rs` | `L36-40, 73, 168`: `sql.len() == 57`, `37` | Step 38 adds `cpv_{ai,au,ad}` (`canonical_property_values`), increasing triggers `57 -> 60` (`VISIBILITY_TRIGGER_TABLES` `[..19]` -> `[..20]`) and schema `37 -> 38`. |
| `src/rust/crates/fathomdb-engine/tests/slice85_policy.rs` | `L8`: `PRAGMA user_version == [37]` | Bump to `vec![38]`. |
| `src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs` | `L754-755`: `assert_eq!(SCHEMA_VERSION, 37)` | Bump to `38`. |
| `src/rust/crates/fathomdb-engine/src/stored_coverage.rs` & `stored_coverage_tests.rs` | `stored_coverage.rs:149` (`user_version != 37`), `stored_coverage_tests.rs:105` (`PRAGMA user_version=37`) | Advance to `38` (`SCHEMA_VERSION`). |
| `src/rust/crates/fathomdb-cli/tests/slice55_composed_recovery.rs` | `L268`: `assert_eq!(schema["user_version"], 37)` | Bump to `38`. |
| `src/inspector/tools/local-release.py` & `src/inspector/tests/tools/test_local_release.py` | `local-release.py:70`, `test_local_release.py:34` (`'compiled_schema': 37`) | Update to `38` in Slice 50 alongside Inspector `admission.rs` layout update. |

### 3.2 `ADR-0.8.31-strict-endpoint-integrity.md` (Target: Slice 20 / Slice 50 for Inspector)

#### A. Explicit dangling-edge acceptance tests (rewrite to strict refusal + atomic rollback)

| File | Test / line(s) | Remediation |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/pr_g8_dangling_edges.rs` | `s20_edge_to_one_missing_endpoint_counts_one`, `s20_edge_to_superseded_node_counts`, `s20_default_flag_and_count_commits_the_batch`, `s20_count_is_sum_over_both_endpoints`, `s20_on_supersession_skips_earlier_keeps_final_active` (`L225`), `s20_legacy_null_logical_id_batch_writes_with_defined_count` | Convert to atomic `EngineError::WriteValidation` refusal + zero-row rollback assertions; retain `s20_same_batch_later_inserted_node_is_not_flagged` and `s20_endpoint_probe_hits_partial_index_no_scan`. |
| `src/rust/crates/fathomdb-engine/src/write_commit.rs` | `L1071-1127` (`node_only_dangling_check_does_not_require_endpoint_reads`, `L1123`) | Update endpoint check assertion from `Ok(2)` to typed validation refusal while keeping SQLite authorizer check proving node-only batches do zero `canonical_nodes` reads. |
| `src/python/tests/test_functional_search.py` & `src/ts/tests/functional-search.test.ts` | `test_functional_dangling_edge_count_across_ffi` (`L203-245`) & `functional write: dangling-edge count (Py ≡ TS)` (`L205-230`) | Keep `clean.dangling_edge_endpoints == 0` check; convert `GHOST_A -> GHOST_B` assertion to `WriteValidationError` atomic refusal (zero committed rows). |

#### B. Graph/erasure/Inspector tests intentionally exercising absent/inactive intermediates via pre-commit dangling writes (convert to create-active -> write-edge -> transition/purge)

| File | Test / line(s) | Dangling write & fix |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice20_graph_traversal.rs` | `graph_neighbors_inactive_intermediate_not_traversed` (`L556-587`) | Create `"MISSING"` active, write `A->MISSING` + `MISSING->C`, then `transition("MISSING", Deleted)`. |
| `src/rust/crates/fathomdb-engine/tests/slice85_physical.rs` | `absent_logical_root_never_completes_over_retained_touching_edges` (`L279-330`) | Create `"already-absent"` active, write edge, then `purge("already-absent")` before physical test. |
| `src/rust/crates/fathomdb-engine/tests/opp12_existence_axis.rs` | `r_ex_2_graph_traversal_excludes_pending_neighbor` (`L192-211`) | Write `"nbr"` `Active` + `root->nbr`, then `transition("nbr", Pending)`. |
| `src/inspector/tests/fixtures/mod.rs` + callers (`worker.rs:952`, `integration/graph.rs:330`, `browser/graph.mjs:42, 142, 184`) | **Not listed in Slice 0 ADR:**<br>- `add_graph` (`L527`: `"derived" -> "not-retained"`)<br>- `add_graph_missing_prefix` (`L578-597`: `5000*p -> 5000*p+q`)<br>- `add_graph_highdegree` (`L600-619`: `"alpha" -> "missing-{i}"`) | Seed endpoint nodes active alongside edges, then `purge` `"not-retained"` and the two `5000`-char prefix nodes (yielding genuine `"zero"` resolution in `0.8.31` stores so `worker.rs`, `integration/graph.rs`, and `browser/graph.mjs` assertions keep passing without modifying browser semantics). |

#### C. Unrelated Rust / Python / TS tests using convenience edges without creating nodes (NOT listed in Slice 0 ADR — seed missing endpoint nodes cleanly)

- `src/rust/crates/fathomdb-engine/tests/`:
  - `erasure_projection_registry.rs:293-329` (`bob->acme`, `bob->carol`)
  - `erasure_drain_ordering.rs:144, 176` (`source->target`)
  - `slice60_store_format.rs:278-317` (`a->b`, `a->c`)
  - `pr_g0_identity.rs:111, 278` (`a->b`, `a->c`)
  - `source_id_writes.rs:42` (`a->b`)
  - `trace_source_ref.rs:52` & `provenance_mandatory.rs:68-102` (node `logical_id: None`, edge `a->b`)
  - `slice30_embedding_readiness.rs:101, 137` (`memex-a->memex-b`)
  - `slice40_projection_generation.rs:172` & `slice40_projection_completion.rs:140, 362` (`a->b`)
  - `tc45_supersession_terminal.rs:251, 273` (`a->b`, `a->c`)
  - `tc33_fix1_unrenderable_epoch.rs:131` & `tc33_fix5_iso8601_shape_gate.rs:237` (`A->B`)
  - `tc33_fix3_consolidation_timestamp_taxonomy.rs:47`, `consolidate_provider.rs:81`, `rebuild_projections.rs:233`, `tc33_fix6_edge_vector_prune.rs:114` (`bob->acme`, `bob->globex`)
  - `consolidation_value_test.rs:61` (12 edges with zero nodes)
  - `slice50_lightweight_controls.rs:166` (`node->missing`)
  - `slice50_record_identity.rs:145, 321, 483, 556` (`missing`, `missing-a`, `missing-b`)
  - `slice50_refusal_baseline.rs:159-181` (`a->b` while only `anchor`/`target` seeded)
  - `slice50_drain_scan_bound.rs:204, 211, 240` (`a-{i}->b`, nodes had `logical_id: None`)
  - `slice55_data_plane_integrity.rs:1532, 1555, 1837` (`from->to`)
  - `slice55_explanation.rs:537` (`a->b`)
  - `slice15_identity_provenance.rs:131, 515` (`L131` missing `"entity-logical"`; `L515` `a->b`)
  - `slice80_history.rs:795-810` (`history->old/new`, `first->first`, `second->second` with no nodes)
  - `slice60_fix4_dependency.rs:73-102` (`root->derived` written *before* raw SQL inserts `"derived"`; insert/write `"derived"` prior to or with the edge)
  - `write_boundary_atomicity.rs:595-622, 704-716` (`consolidate_out_of_cluster_verdict_rolls_back_applied_invalidate` missing `carol`/`acme`/`globex`; `proptest` `ValidItem::Edge` strategy emits `prop-from-* -> prop-to-*` without matching nodes)
- `src/python/tests/test_slice30_embedding_readiness.py:27` & `src/ts/tests/slice30-embedding-readiness.test.ts:22` (`memex-a->memex-b` with no nodes).

### 3.3 SDK surface, `OpenReport`/`EraseReport`, `tc67`, & W12 pins (Target: Slices 10 / 40)

| File | Impact & required lockstep update |
| --- | --- |
| `src/python/tests/test_slice130_package_boundary.py`, `src/python/tests/python_surface_additions.json`, `src/python/fathomdb/_fathomdb.pyi` | Exact declaration count/dict (`L380-383`) and SHA-256 stub reverse-patch proof (`L385-566`, `native_stub_sha256`) will fail on `EraseReport.completed_pending_erasure`, `OpenReport.{reranker_compiled, cuda_compiled, rerank_available}`, `Engine.search(hints=)`, `Engine.resolve_evidence_many`, `read.operational_collections`, and property/historical graph types until registered in `python_surface_additions.json` + test delta. |
| `src/ts/tests/sdk-surface-parity.test.ts`, `src/conformance/governed-{operation-parity,surface-allowlist}.json`, `src/rust/crates/fathomdb-sdk/tests/surface.rs`, `src/rust/crates/fathomdb/tests/governed_surface.rs` | Adding governed operations (`read.operational_collections`, `resolve_evidence_many`) changes the live canonical operation count from `53/53` (`sdk-surface-parity.test.ts:98`). Update all four conformance pins in lockstep. |
| `src/python/tests/test_openreport.py:90-120` & `src/ts/tests/openreport.test.ts:88-115, 153-209` | Hand-crafted mock native `OpenReport` literals passed to `_map_open_report` / `mapOpenReport` fail (`AttributeError` / `tsc --noEmit`) unless supplied with `reranker_compiled`, `cuda_compiled`, `rerank_available`. |
| `tc67_unsupported_vector_kind_report.rs`, `slice22_projection_status.rs` (+ Py & TS counterparts) | **Condition-dependent**: remain **PASSING** if custom vector kinds (`R1`) use explicit opt-in enrollment (`P5-01`); ALL 6 break if `configure_projections([vector_spec(...)])` auto-enrolls arbitrary corpus kinds. |
| `src/rust/crates/fathomdb-engine/tests/erasure_completeness.rs:302` & `src/python/tests/test_slice65_wal_attribution_installed.py:139-140, 218` | W12 Ask 3c rewrites `ErasureIncomplete.detail` (`erasure.rs:1285-1288`): preserve `"wal_checkpoint(TRUNCATE)"`, `"BUSY on all 5 attempts"`, and `"frames still in the log"` substrings; audit Ask 3a two-truncate checkpoint counts against `ActualCheckpointObserver`. |

---

## 4. Verification ordering summary for Slices 10–60

1. Implement Slices **10–40** with **RED** commits preceding **GREEN** per `AGENTS.md` §5; run `cargo clippy --workspace --all-targets`, `cargo test` on touched crates, rebuilt wheel (`scripts/setup-python-owned.sh --build`) + `pyright -p src/python` (0/0) + `ruff`, TS `npm run build && npm test`, `scripts/agent-lint-migrations.sh`, and `scripts/agent-lint-md.sh`.
2. Close **Slice 50** on Inspector `local-ci.sh --scoped` (both default model-free and `cached-cpu`), `D4 = B` browser suite, plus `test_local_ci.py` / `test_local_release.py`.
3. Close **Slice 60** on paired 10k/50k/100k latency gate (`P5-11`) + unconfined `./scripts/agent-verify.sh` + `AGENT_LONG=1 ./scripts/check.sh` + `local-ci.sh --full-workspace` + clean-venv `fathomdb-cli` PyPI wheel smoke.

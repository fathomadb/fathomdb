---
title: Slice 41 Plan — 0.8.32 Engine Query Scaling, Purge Lineage, Evidence Eligibility, and Partitioned Frozen-Read Stability
status: APPROVED
target_release: 0.8.32
slice: 41
date: 2026-11-26
---

# Slice 41 Plan — `ENGINE-QUERY-AND-EVIDENCE-HARDENING`

- **Target branch / worktree:** `dev/0.8.32` at `/mnt/data/worktrees/release-0.8.32` (never mutate `/home/coreyt/projects/fathomdb`).
- **Requirements covered:** `R32-09` (purge-lineage & shared-bucket receipt hardening `#268`, FTS/evidence & `read.get(include_superseded)` `O(N)` elimination `#267` Parts 1 & 2 Option 2B, artifact-scoped `property_predicates` on evidence `source_filter` `#269` Option A, and Schema 39 partitioned frozen-read additive stability `#267` Part 3 / `ADR #103`) and `R32-06` (`TABLE_MANIFEST`, `sqlite_master` completeness, erasure integrity, and zero latency regression).
- **Authoritative ADRs & rulings:**
  - `dev/adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md` (`#103`, accepted 2026-11-26 — amends `#46`, `#47`, `#48`; extends `#70`, `#102`)
  - `dev/plans/0.8.32/features/slice-41/frozen-read-stability-experiment.md` (empirical evidence grounding `ADR #103`)
  - `dev/plans/release-state-0.8.32.json` (`Gate1-D11`, 2026-11-26)
- **Hard invariants:**
  - **Governed surface invariance:** `scripts/governed-surface-pin.json` (`58` ops / `97` allowlist, `core = 5`, `recovery_denylist = 5`) and `src/conformance/governed-{operation-parity,surface-allowlist}.json` remain strictly **unchanged** (`0` new governed verbs; zero new flags on `ReadContextV1`).
  - **`MR-070.2` & `rmcp` confinement:** `fathomdb-mcp` links ONLY `fathomdb-sdk` in runtime dependencies (zero `rusqlite` or direct `fathomdb-engine` deps).
  - **Unreleased Schema 39 in-place extension:** `SCHEMA_VERSION` stays `39` (`STEP_39_SQL` extended in place; no Schema `40` bump prior to `0.8.32` release; golden `0.8.31` Schema `38 -> 39` migration and Schema `31`/`32` frozen-read normative fixtures remain green).
  - **Single-source `TABLE_MANIFEST` & admission parity:** `_fathomdb_scope_visibility_watermarks` and `_fathomdb_read_visibility_state.(global_struct_gen, global_struct_nonce)` stay synchronized across `fathomdb-schema/src/lib.rs`, `fathomdb-engine/src/export_manifest.rs`, `src/inspector/src/admission.rs`, `projection_generation::bootstrap_is_fresh`, and `frozen_read::validate_on_open`.
  - **`lib_rs_blob` sync:** If `fathomdb-engine/src/lib.rs` changes, update `lib_rs_blob:` in `dev/plans/0.8.27/features/slice-90/phase3-root-reconciliation.md:4` in the same commit.
  - **Per-stage adversarial review discipline (`Gate1-D11`):** Every stage (Stage 1 ADR + Experiments → Stage 2 Requirements & ACs → Stage 3 Architecture → Stage 4 Detailed Design → Stage 5 TDD RED/GREEN + Adversarial Code Review → Stage 6 Full Unconfined Verify & Local Closeout) requires an independent `pro` adversarial `VERDICT: APPROVE` before advancing.
  - **Product-generic documentation & git discipline:** Zero external consumer-project specifics in tracked files; TDD failing test first (`red -> green -> refactor`); no `--no-verify` / `--no-gpg-sign`; no `prettier` or unguarded `markdownlint-cli2 --fix` on `.md` (`npm run format:md` only); local commits only (no `git push` without explicit owner request).

---

## 1. Acceptance Criteria Matrix (`R32-09`)

### A. Purge Lineage & Shared-Bucket Receipt Hardening (`WP-41A`, `#268`)

| AC ID | Requirement & Falsifiable Verification Signal |
| --- | --- |
| `AC32-PURGE-LINEAGE-01` | Executing `actuate -> transition("deleted") + purge() -> actuate` on a shared `source_id` bucket — covering both sole-node purge (empty canonical table after purge) and trailing-cursor purge (surviving lower-cursor nodes present), both in-process and across `close()` / `open()` — succeeds with zero `StorageError` (`SQLITE_CONSTRAINT_PRIMARYKEY`), strictly monotonic cursors, and zero `doctor data-plane-integrity` findings. |
| `AC32-PURGE-LINEAGE-02` | Single-node `purge(logical_id)` redacts actuation receipts referencing the purged `logical_id` / purged revision cursor ONLY and MUST NOT pass `("source_id", shared_bucket)` into `redact_actuation_receipts_for_refs`; replaying an `operation_id` owned by a surviving sibling entity in the same shared `source_id` bucket returns its cached receipt cleanly (never `OperationIdErased`), while `erase_source(source_id)` continues to redact all receipts bound to that `source_id`. |

### B. FTS/Evidence & `read.get` `O(N)` Elimination (`WP-41B`, `#267` Parts 1 & 2, Option 2B)

| AC ID | Requirement & Falsifiable Verification Signal |
| --- | --- |
| `AC32-QUERY-SCALE-01` | `search_with_evidence(..., include_explanation = false)` decouples lightweight `EvidenceContributionV1` capture from full `explain = true` so `status_cache` / `status_in_snapshot`, `structural_*` explain hydration, and `vec_distance_cosine` SQL evaluation are skipped completely when `include_explanation = false`, while `EvidenceContributionV1` remains bit-identical to `include_explanation = true`. |
| `AC32-QUERY-SCALE-02` | Filtered FTS-only `search_frozen` and `search_with_evidence` push `search_index.kind = ?` before joining `canonical_nodes`; never apply an `ORDER BY`-less `{limit_clause}` on `defer_full_hydration` (requiring FTS5 `ORDER BY rank` streaming / `collect_complete_fusion_rank_boundary` for deterministic `(bm25 score, write_cursor)` prefix ordering); cap `fts_only_limit` when `_fathomdb_vector_kinds` is empty ONLY under the exact safety preconditions (`!use_graph_arm && !recency_enabled && !importance_enabled`, respecting `rerank_depth` / rerank pool bounds when `rerank_depth > 0`); and move `build_importance_confidence_maps` strictly after `truncate(final_limit)` whenever `!importance_enabled` (preserving `EngineError::Storage` propagation on row decode failure for all surviving post-truncation hits `<= final_limit`, and preserving identical ranking/error semantics when `importance_enabled = true`). |
| `AC32-QUERY-SCALE-03` | Inside `empty_physical_fast_path` (`projection_generation.rs`), skip the internal `canonical_nodes` aggregation (`node_sql`) when neither `node_arm_declared` nor `has_explicit_node_enrolment` holds, eliminating the unnecessary `canonical_nodes` scan while preserving `empty_physical_fast_path`'s `edge_count` and `edge_count != 0 && !edge_enrolled` corruption checks verbatim. |
| `AC32-QUERY-SCALE-04` | `store_single_unscoped` (`fts_fast_path.rs`) replaces `WHERE scope != ''` full-index scanning with `EXISTS(... WHERE scope < '' LIMIT 1) OR EXISTS(... WHERE scope > '' LIMIT 1)` gated behind cheap booleans, verified `SEARCH`-only under `EXPLAIN QUERY PLAN`; `detect_vector_soft_fallback` (`vector_arm.rs`) short-circuits in `O(1)` via `EXISTS(SELECT 1 FROM _fathomdb_vector_kinds LIMIT 1)`. |
| `AC32-QUERY-SCALE-05` | Both `frozen_read::mint_inner` and `graph_expand/execution.rs` (`Current` snapshot pin) replace `SELECT COUNT(*) FROM canonical_nodes` with `O(1)` `_fathomdb_read_visibility_state` lookups (`singleton = 1`); `historical_dense_incomplete` (`fusion_stage.rs`) short-circuits immediately when `query_vector.is_none()`; `evidence/ranked.rs` / `dependency.rs` eliminate duplicate SHA-256 hashing of `source_body` on the same evidence resolution path. |
| `AC32-QUERY-SCALE-06` | `read.get` and `read.get_many` with `include_superseded = true` execute strictly `SEARCH`-only query plans (`EXPLAIN QUERY PLAN` contains zero `SCAN canonical_nodes` and zero `USE TEMP B-TREE FOR ORDER BY`) via zero-schema-change `UNION ALL` over existing `canonical_nodes_scope_logical_active_idx` and `canonical_nodes_scope_logical_sup_idx`, performing max-`write_cursor` tiebreaking in Rust and gating `read_eligibility_sql` on `has_source_dependencies`, preserving exact returned revisions, ordering, and dependency-revocation semantics across single and multi-ID lookups (including mixed active + superseded + missing IDs). |

### C. Artifact-Scoped `property_predicates` on Evidence `source_filter` (`WP-41C`, `#269` Option A)

| AC ID | Requirement & Falsifiable Verification Signal |
| --- | --- |
| `AC32-EVIDENCE-PROP-01` | Under `ReadContextV1(eligibility = SearchFilter(kind = "Doc", attributes = [("acl", "team-a")], property_predicates = [confidence >= 0.8]))`, resolving a derived `"Doc"` artifact (`acl = "team-a"`, `confidence = 0.95`) backed by a canonical `"SourceDoc"` (`acl = "team-a"`, **no `confidence` property**) succeeds across `search_with_evidence`, `resolve_evidence`, `resolve_evidence_many`, and `graph_expand(include_evidence = true)` + `resolve_graph_evidence` across Rust, Python, and TypeScript, while artifacts failing `confidence >= 0.8` remain excluded before truncation. |
| `AC32-EVIDENCE-PROP-02` | Access-bearing `attributes`, `created_after`, `status`, `ReadView` validity/lifecycle, dependency closures, and Schema 39 `ScopePredicate` remain strictly source-enforced: mutating the canonical source's `acl` attribute to `"team-b"` or moving it outside `ScopePolicy` fails closed with non-disclosing `EvidenceErrorReasonV1::EvidenceUnavailable` at `/evidenceRef` (preserving `slice50_evidence.rs` nondisclosure and extending `slice38_historical_graph.rs:2418-2480` to decouple and verify both artifact-only `property_predicates` — target matches `score >= 40` while source omits `score` — and source-enforced `attributes` independently). |
| `AC32-EVIDENCE-PROP-03` | `trace_dependency` preserves `ADR-0.8.25 #49` symmetric endpoint enforcement (Rust `visible_artifact` in `authorize_candidate_chain` and `include_candidate` continues applying full `context.eligibility`, including `kind`, `source_type`, and `property_predicates`, to both root and counterpart endpoints, while only its SQL pre-queries omit `property_predicates`; intentionally unchanged by `#269` Option A); `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md` and `docs/` document Option A (`ADR #103` §2.1), `trace_dependency` symmetry, and the intentional `GraphArm` `SearchHit.source_id` (traversed edge) vs. `ResolvedEvidenceV1.source_id` (returned node artifact provenance) distinction (`ADR #103` §2.2). |

### D. Schema 39 Partitioned Frozen-Read Additive Stability (`WP-41D`, `#267` Part 3, `ADR #103`)

| AC ID | Requirement & Falsifiable Verification Signal |
| --- | --- |
| `AC32-FROZEN-STAB-01` | `8` concurrent readers running `freeze_read_context(SearchFilter(kind = "Doc")) -> search_with_evidence -> resolve_evidence_many` (and `search_frozen`, `read_canonical_page("Doc")`, `graph_expand`) while a writer commits `kind = "AppTurn"` nodes via `Engine.write` and `Engine.actuate` achieve **`0%` unintended drift**. |
| `AC32-FROZEN-STAB-02` | High-frequency `operational_mutations` (`AppendOnlyLog`) appends produce **`0%` drift** on canonical, search, evidence, graph, trace, and `operational_state` contexts (while retaining `('up_to_date', 'non_canonical')` terminal cursors and `advance_projection_cursor` with zero scheduler deadlocks); `operational_state` (`LatestState`) upserts produce **`0%` drift** on canonical/search/evidence/graph/trace contexts (including `SearchFilter::default()`) while drifting `ReadSurface::OperationalState`. |
| `AC32-FROZEN-STAB-03` | Additive writes, `transition` (`record_lifecycle::transition` under `VisibilityCoalescingGuard`), `purge`, `erase_source`, and `physical_erasure` in `"tenant-b"` produce **`0%` drift** on `"tenant-a"` `FrozenReadContextV1` tokens across close/reopen and within `fathomdb-mcp` cached sessions (removing global `minted_state` pre-gates, premature `entry.epoch != guard.epoch` rejection on reused `frozen_context` handles across MCP close/reopen, and `after != visibility` post-error checks across both reused and freshly minted contexts in favor of `MR-070.2`-compliant frozen SDK probes covering `search_with_evidence`, `resolve_evidence_many`, `resolve_graph_evidence` `graph_refs`, and `graph_expand`). |
| `AC32-FROZEN-STAB-04` | Background `projection_worker` commits embedding `"AppTurn"` (or `"tenant-b"`) vectors produce **`0%` drift** on frozen `"Doc"` contexts in `"tenant-a"`, whereas embedding `"Doc"` vectors in `"tenant-a"` or pinning `_fathomdb_embedder_profiles.mean_vec` produces deterministic drift. |
| `AC32-FROZEN-STAB-05` | Same-`(scope, kind)` canonical write, matching-`kind` `TextEdge` / `edge_fact` insert, same-scope edge write on `GraphOrEdge` / `search_expand_frozen` / `EvidenceGraph` / `TraceDependency` / post-decode `Edge`/`GraphArm` slot (`resolve_evidence_many` returns per-slot `Refused(EvidenceUnavailable)` while unaffected node-only slots resolve), destructive mutation on any kind in `scope`, `source_dependencies` / `_fathomdb_dependency_closures` write in `scope`, `global_struct` update, or raw `coalescing = 0` SQL across all 20 trigger-bearing tables ALL fail closed (`StateDrifted` / `EvidenceUnavailable`); `validate_snapshot` continues returning the monotonic global `load_visibility_generation` clock for `Engine::read_visibility_generation.fetch_max`. |
| `AC32-FROZEN-STAB-06` | All existing `slice20`, `slice35`, `slice38`, `slice40`, `slice45`, `slice50`, `slice75`, MCP, Inspector admission, and `38 -> 39` migration suites pass; `fdbfr1` token size stays `<= 1,024` bytes (`492`–`556 B` across unscoped/scoped Schema `39+`); Schema `31`/`32` normative fixtures stay bit-identical; `validate_snapshot` `p50 <= 0.012 ms` at `50k` rows; `dev/interfaces/{rust,rust-sdk,python,typescript,wire,mcp}.md` and `docs/` accurately document partitioned additive stability and `BM25` non-freeze honesty (`ADR #103` §1.7). |

---

## 2. Sequenced Work Packages (`WP-41A`–`WP-41D`)

All 4 work packages execute sequentially in `/mnt/data/worktrees/release-0.8.32` (one writer per checkout) strictly after Stage 4 Design `VERDICT: APPROVE`:

1. **`WP-41A` (`#268` Purge Lineage & Shared-Bucket Receipt Hardening):**
   - RED → GREEN regression tests for `actuate -> transition("deleted") + purge() -> actuate` (sole-node and trailing-cursor, in-process & reopened, plus `doctor data-plane-integrity`).
   - Narrow `purge_inner` (`src/rust/crates/fathomdb-engine/src/erasure.rs`) receipt redaction to purged entity refs only (excluding shared `source_id` bucket while keeping `erase_source` full source redaction) + regression test proving surviving shared-bucket sibling `operation_id` replay succeeds.
2. **`WP-41B` (`#267` Parts 1 & 2 — Zero-Schema-Change FTS/Evidence & `read.get` `O(N)` Elimination):**
   - Implement all verified FTS-only / evidence `O(N)` hotspot fixes (`AC32-QUERY-SCALE-01..05`) across `search.rs`, `search/{fts_fast_path,vector_arm,fusion_stage}.rs`, `filter.rs`, `projection_generation.rs`, `frozen_read.rs`, `graph_expand/execution.rs`, `evidence/ranked.rs`, and `dependency.rs`.
   - Implement Option 2B in `src/rust/crates/fathomdb-engine/src/read.rs` (`UNION ALL` over `canonical_nodes_scope_logical_{active,sup}_idx`, drop SQL `ORDER BY write_cursor`, Rust max-`write_cursor` tiebreak, `has_source_dependencies`-gated `read_eligibility_sql`) with `EXPLAIN QUERY PLAN` `SEARCH`-only assertions and semantic equivalence tests (`AC32-QUERY-SCALE-06`).
3. **`WP-41C` (`#269` Option A — Artifact-Scoped `property_predicates` on Evidence `source_filter`):**
   - Clear `source_filter.property_predicates.clear();` across `evidence/ranked.rs` (`authorize_source_access`) and `evidence/graph.rs` (`query_graph_evidence_batch`, `resolve_graph_evidence`).
   - Extend `src/rust/crates/fathomdb-engine/tests/slice38_historical_graph.rs:2418-2480` to decouple and test both target-only `property_predicates` and source-enforced `attributes`, add RED → GREEN Rust, Python, and TypeScript tests covering `AC32-EVIDENCE-PROP-01..03`, and document Option A & `GraphArm` `source_id` semantics in `dev/interfaces/` and `docs/`.
4. **`WP-41D` (`#267` Part 3 — `ADR #103` Schema 39 Partitioned Frozen-Read Additive Stability):**
   - Extend unreleased `STEP_39_SQL` + `TABLE_MANIFEST` in `fathomdb-schema/src/lib.rs`, `src/inspector/src/admission.rs`, `projection_generation::bootstrap_is_fresh`, and `frozen_read::validate_on_open`.
   - Implement 20-table trigger classification, generalized non-deadlocking `VisibilityCoalescingGuard` across non-`AdminSchema` batches and scope-owned mutation paths (`write_commit.rs`, `actuation.rs`, `record_lifecycle.rs`, `projection_commit.rs`, `erasure.rs`, with `physical_erasure.rs` verified uncoalesced / zero `global_struct` bump), and 8-surface `ReadSurface` + `validate_snapshot` + `0x53 0x39` wire codec (`frozen_read.rs`, `search_api.rs`, `search/fusion_stage.rs`, `evidence/ranked.rs`, `graph_expand/execution.rs`, `read.rs`, `dependency_trace.rs`).
   - Update `fathomdb-mcp/src/tools.rs` to remove global `minted_state` pre-gates, premature `entry.epoch != guard.epoch` rejection on reused `frozen_context` handles across MCP close/reopen, and `after != visibility` post-error checks across both reused and freshly minted contexts, implementing `MR-070.2`-compliant frozen SDK probes (`search_frozen` / `search_expand_frozen`) including whole-call `FetchEvidenceExec::Drifted` attribution for drifted `resolve_graph_evidence` (`graph_refs`).
   - Update `dev/interfaces/{rust,rust-sdk,python,typescript,wire,mcp}.md` and `docs/`, rebuild `.venv` wheel (`bash scripts/setup-python-owned.sh --build`) & TS addon, verify `scripts/governed-surface-pin.json` (`58`/`97`) untouched, and prove `AC32-FROZEN-STAB-01..06` via RED → GREEN Rust, MCP, Python, and TypeScript tests.

---

## 3. Verification & Exit Gate

- Independent `pro` adversarial reviews (`VERDICT: APPROVE`) on:
  1. Stage 1 ADR `#103` + Experiment (`78c88d14d`, **DONE**),
  2. Stage 2 Requirements & Acceptance Criteria (this plan + release state + `scope.md` + `plan.md` + `STATUS-0.8.32.md`),
  3. Stage 3 Architecture docs (`fathomdb-data-plane-architecture-v2.md`, `retrieval.md`, `engine.md`, `actuation.md`, `mcp-adapter/design.md`, `mcp-adapter/requirements.md`, plus lifecycle/authority catalogs),
  4. Stage 4 Detailed Design (`dev/plans/0.8.32/features/slice-41/design.md`),
  5. Stage 5 Implementation (`WP-41A`–`WP-41D` full diff, remediated & re-reviewed until `VERDICT: APPROVE`).
- Final Stage 6 verification: full unconfined `./scripts/agent-verify.sh` (**EXIT `0`**, zero skipped unexpected suites, zero security violations) and `mkdocs build --strict -d /tmp/fathomdb-slice41-mkdocs` (**EXIT `0`**, zero warnings), followed by durable local Slice 41 closeout (`features/slice-41/status.md` + `release-state-0.8.32.json` -> `next_slice: 50`, no push).

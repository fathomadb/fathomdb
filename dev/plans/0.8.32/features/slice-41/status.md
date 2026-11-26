---
title: Slice 41 Status — Engine Query Scaling, Purge Lineage, Evidence Eligibility, and Partitioned Frozen-Read Stability
status: LANDED
target_release: 0.8.32
slice: 41
date: 2026-11-26
---

# Slice 41 Status — `ENGINE-QUERY-AND-EVIDENCE-HARDENING`

## Outcome

**LANDED locally on `dev/0.8.32`** (`/mnt/data/worktrees/release-0.8.32`). Delivers `R32-09` (`Gate1-D11`, `ADR-0.8.32 #103`) and `R32-06` invariants across four pre-code reviewed stages (ADR + Experiments → Requirements/ACs → Architecture → Design), four sequentially executed TDD (`RED -> GREEN`) work packages, a 34-file 4-tier documentation refresh, and full workspace verification:

1. **Pre-Code Stages (`78c88d14d`, `0b8273690`, `d4cefbda4`, `756c8d0e8`):**
   - **Stage 1 (`78c88d14d`):** Ratified [ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md](../../../../adr/ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md) (`#103`, amending `#46`, `#47`, `#48` and extending `#70`, `#102`), registered `#103` in `ADR-0.6.0-decision-index.md`, and grounded the design with [frozen-read-stability-experiment.md](./frozen-read-stability-experiment.md) (independent `pro` review `VERDICT: APPROVE`).
   - **Stage 2 (`0b8273690`):** Updated `release-state-0.8.32.json` (`Gate1-D11`), `scope.md`, `plan.md` (`R32-09`), `STATUS-0.8.32.md`, and approved [plan.md](./plan.md) (`AC32-PURGE-LINEAGE-01..02`, `AC32-QUERY-SCALE-01..06`, `AC32-EVIDENCE-PROP-01..03`, `AC32-FROZEN-STAB-01..06`; independent `pro` review `VERDICT: APPROVE`).
   - **Stage 3 (`d4cefbda4`):** Updated maintained architecture and subsystem design authorities (`fathomdb-data-plane-architecture-v2.md`, `retrieval.md`, `engine.md`, `actuation.md`, `mcp-adapter/design.md`, `mcp-adapter/requirements.md`, `document-lifecycle.json`, `current-owner-authority.json`; independent `pro` review `VERDICT: APPROVE`).
   - **Stage 4 (`756c8d0e8`):** Authored and approved [design.md](./design.md) covering all SQL DDL/triggers, coalescing deltas, wire codec layouts, query plans, and TDD matrices (independent `pro` review `VERDICT: APPROVE`).
2. **`WP-41A` (`bbfbedaeb`) — `#268` Purge-Lineage & Shared-Bucket Receipt Hardening (`AC32-PURGE-LINEAGE-01..02`):**
   - Narrowed `purge_inner` actuation-receipt redaction in `erasure.rs` so single-node `purge(logical_id)` redacts only the purged entity's revision refs without over-redacting surviving siblings sharing a `source_id` bucket (while preserving full `source_id` bucket redaction on `erase_source`).
   - Hardened monotonic `current_floor` (`self.next_cursor.max(load_next_cursor(&tx))`) across all erasure branches and verified `actuate -> transition("deleted") + purge() -> actuate` (sole-node & trailing-cursor, in-process & reopened, zero `doctor data-plane-integrity` findings) via `slice41_purge_lineage.rs` (`6/6`).
3. **`WP-41B` (`7065068bc`, `FIX-2` `2039910c8`) — `#267` Parts 1 & 2 Option 2B Zero-Schema-Change Query & `read.get` `O(N)` Elimination (`AC32-QUERY-SCALE-01..06`):**
   - Decoupled lightweight `EvidenceContributionV1` capture from full `explain = true` on `search_with_evidence(include_explanation = false)` (skipping `status_cache`, `structural_*`, and `vec_distance_cosine`).
   - Pushed `search_index.kind = ?` + `CROSS JOIN canonical_nodes cn` (post-`ANALYZE` plan safe), enforced FTS5 `ORDER BY rank` streaming (`collect_complete_fusion_rank_boundary` with `.min(4096)` preallocation OOM cap), removed `ORDER BY`-less `{limit_clause}` truncation from `defer_full_hydration`, safely guarded `_fathomdb_vector_kinds`-empty `fts_only_limit` capping, and moved `build_importance_confidence_maps` post-truncation when `!importance_enabled`.
   - Skipped `node_sql` inside `empty_physical_fast_path` when `!(node_arm_declared || has_explicit_node_enrolment)` preserving `edge_count` corruption checks; replaced `store_single_unscoped` `WHERE scope != ''` with `SEARCH`-only `scope < '' OR scope > ''` covering range probes; short-circuited `detect_vector_soft_fallback` and `historical_dense_incomplete`; replaced `SELECT COUNT(*) FROM canonical_nodes` snapshot pins in `frozen_read::mint_inner` and `graph_expand/execution.rs` with `O(1)` `_fathomdb_read_visibility_state` lookups; eliminated duplicate `source_body` SHA-256 hashing (`VerifiedSourceBody`).
   - Rewrote `read.get` / `read.get_many(include_superseded = true)` via zero-schema-change `UNION ALL` over `canonical_nodes_scope_logical_{active,sup}_idx` with Rust-side max-`write_cursor` tiebreaking and `has_source_dependencies`-gated eligibility SQL, verified `SEARCH`-only via `slice41_query_scale.rs` (`11/11`).
4. **`WP-41C` (`698746542`) — `#269` Option A Artifact-Scoped `property_predicates` on Evidence `source_filter` (`AC32-EVIDENCE-PROP-01..03`):**
   - Cleared `source_filter.property_predicates` across `evidence/ranked.rs` (`authorize_source_access`) and `evidence/graph.rs` (`query_graph_evidence_batch`, `resolve_graph_evidence`) while retaining source-enforced `attributes`, lifecycle/validity, dependencies, Schema 39 `ScopePredicate`, and symmetric `trace_dependency` enforcement (`dependency_trace.rs` unchanged).
   - Verified across Rust (`slice41_evidence_property_predicates.rs`, extended `slice38_historical_graph.rs`), Python (`test_slice41_evidence_property_predicates.py`), and TypeScript (`slice41-evidence-property-predicates.test.ts`), and documented intentional `GraphArm` `SearchHit.source_id` vs. `ResolvedEvidenceV1.source_id` semantics.
5. **`WP-41D` (`4b4d33b4e`, `FIX-1` `b8d44f90f`) — `#267` Part 3 `ADR #103` Schema 39 Partitioned Frozen-Read Additive Stability (`AC32-FROZEN-STAB-01..06`):**
   - Extended unreleased `STEP_39_SQL` in place (`_fathomdb_read_visibility_state.{global_struct_gen, global_struct_nonce}`, `_fathomdb_scope_visibility_watermarks WITHOUT ROWID`, `TABLE_MANIFEST` `35 -> 36` tables, all 60 triggers across 20 trigger tables with paired `(OLD.*, NEW.*)` `_au` upserts) and synchronized `src/inspector/src/admission.rs`, `bootstrap_is_fresh`, and `validate_on_open`.
   - Generalized `VisibilityCoalescingGuard` & `VisibilityDelta` across non-`AdminSchema` batches (`write_commit.rs` retaining `('up_to_date', 'non_canonical')`), `actuation.rs`, `record_lifecycle.rs::transition`, `projection_commit.rs`, `projection_registry.rs`, `erasure.rs`, and `index_projector.rs` (keeping `physical_erasure.rs` uncoalesced / zero `global_struct` bump).
   - Implemented 8-variant `ReadSurface`, universal `SERVING_DOMAIN_V4` `serving_digest`, compact `0x53 0x39` wire codec (`492 B` unscoped / `556 B` scoped `<= 1,024 B`; Schema `< 39` bit-identical), global monotonic `load_visibility_generation` return from `validate_snapshot`, all 13 call sites including per-slot `Refused(EvidenceUnavailable)` for post-decode `Edge`/`GraphArm` drift in `resolve_many_raw`, and `fathomdb-mcp` global pre-gate/epoch-gate removal with `MR-070.2`-compliant cold probes (`scripts/governed-surface-pin.json` strictly unchanged at `58` ops / `97` allowlist).
   - Verified across Rust (`slice41_frozen_read_stability.rs`), MCP (`mcp_frozen_evidence.rs`), Inspector (`admission.rs`/`scope.rs`), CLI (`migrate_cli`), Python (`test_slice41_frozen_read_stability.py`), and TypeScript (`slice41-frozen-read-stability.test.ts`).
6. **4-Tier Documentation Refresh (`40dbdfa0c`):**
   - Updated 34 documentation files across Project (`10`), Contributor (`2`), Maintainer/Interface/Design (`10`), and User-Facing `docs/` (`12`) tiers with all 9 doc/truth/lifecycle/lint/MkDocs validators exiting `0`.

## Verification & Adversarial Review Receipts

- **Full unconfined `./scripts/agent-verify.sh` on `40dbdfa0c`:** **EXIT `0`** — `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)` (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` PASS); `agent-test.sh: 246/249 suites passed (skipped=3 excluded=0)`.
- **`mkdocs build --strict -d /tmp/fathomdb-slice41-docs-check`:** **EXIT `0`** (`0` warnings, built in `0.57s`).
- **Independent `pro` adversarial reviews (all 5 gates `VERDICT: APPROVE`, `0` P1 / `0` P2):**
  - Stage 1 ADR `#103` + Experiment (`78c88d14d`): `VERDICT: APPROVE`
  - Stage 2 Requirements & Acceptance Criteria (`0b8273690`): `VERDICT: APPROVE`
  - Stage 3 Architecture & Subsystem Designs (`d4cefbda4`): `VERDICT: APPROVE`
  - Stage 4 Detailed Design (`756c8d0e8`): `VERDICT: APPROVE`
  - Stage 5 Code Review (`WP-41A`–`WP-41D` + `FIX-1` `b8d44f90f`): `VERDICT: APPROVE`

## Commits on `dev/0.8.32`

- `78c88d14d` — `docs(0.8.32): ratify ADR #103 and Slice 41 frozen-read stability experiments`
- `0b8273690` — `docs(0.8.32): approve Slice 41 requirements, release state, and acceptance criteria`
- `d4cefbda4` — `docs(0.8.32): update architecture and subsystem designs for Slice 41`
- `756c8d0e8` — `docs(0.8.32): approve Slice 41 detailed design`
- `bbfbedaeb` — `fix(0.8.32): WP-41A narrow purge_inner receipt redaction and harden purge cursor floor`
- `7065068bc` — `perf(0.8.32): WP-41B eliminate FTS/evidence and read.get(include_superseded) O(N) hotspots`
- `698746542` — `fix(0.8.32): WP-41C make evidence source_filter property_predicates artifact-scoped`
- `4b4d33b4e` — `feat(0.8.32): WP-41D implement ADR #103 Schema 39 partitioned frozen-read additive stability`
- `b8d44f90f` — `docs(0.8.32): Slice 41 FIX-1 clarify physical_erasure coalescing and MCP test path`
- `2039910c8` — `fix(0.8.32): Slice 41 FIX-2 reuse engine connection in read_get_by_id_explain_plan_for_test`
- `40dbdfa0c` — `docs(0.8.32): sync project, contributor, maintainer, and user-facing docs through Slice 41`

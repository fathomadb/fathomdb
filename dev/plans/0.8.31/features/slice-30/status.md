---
title: Slice 30 Status — Strict Endpoint Integrity, Typed Traversal Predicates & Opt-In Fused-Hit Seeding
status: CLOSED
target_release: 0.8.31
slice: 30
date: 2026-11-22
---

# Slice 30 Status — `0.8.31` Strict Graph (`STRICT-GRAPH`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31` (moved from `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31` after Layer 1)
- **Plan:** [`dev/plans/0.8.31/features/slice-30/plan.md`](./plan.md)
- **Commits:** `0a43d66b7` (Layer 1 — `WP-30A`), `3f8dbc470` (Layer 2 — `WP-30B`), `87cd6a633` (Layer 3 — `WP-30C`), `eda733775` (Layer 4 — `WP-30D` + `WP-30E`), `8e44a47ad` (`FIX-1`)
- **Verdict:** **CLOSED / LANDED** — all 4 implementation layers + `FIX-1` verified green across Rust, Python, TypeScript, and Markdown/doc gates; independent adversarial code review returned **`APPROVE`** at HEAD `8e44a47ad`.

## 1. Delivered Scope

1. **Layer 1 (`WP-30A`, `0a43d66b7`) — Strict Write Endpoint Integrity & Fixture Remediation:**
   - Added `EndpointValidationMode::{Strict, DeferredToActuationSimulation}` in `write_commit.rs`, enforcing atomic all-edges active endpoint validation (`superseded_at IS NULL AND state = 'active'`, including G0 and G11 in-batch superseded edges) on `Engine::write` and `ingest_with_extractor` tx2 while preserving `actuate`'s single-op loop (`DeferredToActuationSimulation` + post-loop `validate_prospective_edge_endpoints` returning `ReferenceUnavailable`) and the node-only zero-SQL-read `Authorizer` fast path.
   - Added `tests/slice38_strict_endpoints.rs`, rewrote legacy dangling-edge tests (`pr_g8_dangling_edges.rs`, `write_commit.rs`, `test_functional_search.py`, `functional-search.test.ts`), updated 32 convenience-edge test targets across `fathomdb-engine` and `fathomdb-mcp` plus Python/TS readiness tests, and updated the 3 Inspector forensic fixtures in `src/inspector/tests/fixtures/mod.rs` using post-close forensic node removal (all 66 Inspector tests passing).
2. **Layer 2 (`WP-30B`, `3f8dbc470`) — `GraphExpandRequestV1` `#[non_exhaustive]` + Predicates + SQL Pushdown:**
   - Un-gated `properties::eval_property_predicate`, marked `GraphExpandRequestV1` `#[non_exhaustive]` with `::new(...)` + builders + `edge_predicates`/`target_predicates` + `GraphEdgePredicatesInvalid`/`GraphTargetPredicatesInvalid`, and migrated all workspace struct literals across all 13 crates.
   - Implemented `EdgeViewBoundary::Current`, parameterized `GraphExpandIncidentQuery`, valid SQLite `UNION ALL ... LIMIT ?limit` for `Both` (`from_id != ?1`), `superseded_at IS NULL` / `kind` / `[t_valid, t_invalid)` SQL pushdown, sound `canonical_property_values` (`source.is_none()`) pre-filtering + mandatory inline `eval_property_predicate` verification, `visited_by_seed` non-poisoning, terminal-depth early-skip soundness (`origin_key_lt`), untagged scalar `PropertyPredicateWire` reuse, `encode_graph_evidence_request` deterministic predicate framing, `tests/slice38_graph_predicates_and_seeding.rs` (including 32-case `proptest` disabled-optimization equivalence), and `tests/slice38_query_plans.rs` pins.
3. **Layer 3 (`WP-30C`, `87cd6a633`) — `search.rs` Graph Arm, `SEED_FTS_N` Fix, Opt-In `graph_seed` & Provenance:**
   - Fixed Seed Source A mixed-edge `SEED_FTS_N = 10` slot starvation (`from_eligible`/`to_eligible` projection, no premature SQL `LIMIT 10`, predicate filtering prior to distinct eligible slot counting) and rewrote Phase 2 as indexed `UNION ALL` with `graph_edge_kinds` / `graph_edge_predicates`.
   - Added `GraphSeedMode` (`GraphFts` | `FusedTopK` | `Both`), `GraphArmSeedSource` (`FusedTopK` | `EdgeFts` | `EntityFts`), `GraphArmOptions`, single-pass `CeScoreCache` reuse (zero double CE scoring of two-arm hits), deterministic `(FusedTopK, EdgeFts, EntityFts)` min-hop merge, body-less edge traversal, `#[non_exhaustive]` `EvidenceSearchRequestV1::new(...)`, explain-only origin map preserving `size_of::<NoEvidenceCapture>() == 0`, `PerHitExplain.{graph_seed_source, graph_hop_depth}`, and `EvidenceGraphOriginV1.seed_source` authenticated in the binary HMAC token codec while keeping `use_graph_arm = false` byte-identical.
4. **Layer 4 (`WP-30D` + `WP-30E`, `eda733775`) — Python/TS Bindings, Surface Pins & Docs:**
   - Wired full Python (`fathomdb-py`, `src/python/`, `python_surface_additions.json`, `_fathomdb.pyi` + `test_slice130_package_boundary.py` reverse-patch ledger, `test_slice38_graph_predicates.py`) and TypeScript (`fathomdb-napi`, `src/ts/`, `TextSearchOptions` decoupled from `SearchOptions`, `slice38-graph-predicates.test.ts`) bindings, plus Rust SDK surface pins (`surface.rs`, `slice60_governed_surface.rs`).
   - Updated `dev/interfaces/{rust,rust-sdk,python,typescript,wire}.md`, `docs/guides/graph-retrieval.md`, and `docs/reference/{python-api,typescript-api,rust-sdk,rust-api,errors}.md` (zero external consumer-project specifics; zero unimplemented Slice 40 `as_of` claims; governed-surface files unchanged at `55` live ops / `91` signed members).
5. **Adversarial Review Remediation (`FIX-1`, `8e44a47ad`):**
   - Gated `pushed_target_predicates` defensively off under `include_superseded || include_inactive` (`execution.rs:1067-1081`) with `wp30b_relaxed_view_declared_target_predicate_matches_inline_parity`; aligned Python `graph.py` closed-wire `"unknown_field"` and per-field RFC 6901 pointer semantics with Rust/TS; narrowed TS `searchProjectedText`'s `view` type to `BaseSearchOptions`; registered the 4 new `Engine.search` graph-arm kwargs in `src/python/eval/earp/knobs.py` `CATALOG`.

## 2. Verification Summary

- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings, incl. `default-reranker`), `slice38_strict_endpoints` (8/8), `slice38_graph_predicates_and_seeding` (16/16 incl. `proptest`), `slice38_query_plans` (4/4), `pr_g8_dangling_edges`, `pr_c1_graph_seeding` (5/5), `ep03_degradation_bytes` (1/1), `slice60_graph_expand` (19/19), `slice60_wire` (9/9), `slice60_fix4_rss` (1/1), `slice60_fix5_rss` (1/1), `slice20_graph_evidence` (34/34), `slice50_evidence` (29/29), `slice55_explanation` (17/17), 30 updated engine convenience-edge suites (302/302), `fathomdb-mcp` (21/21), `fathomdb-inspector` (66/66), `fathomdb-sdk` + `fathomdb` (all green).
- **Python:** Owned wheel built via `scripts/setup-python-owned.sh --build`; `ruff` clean, `pyright` (0 errors, 0 warnings), full `pytest` (**1,789 passed**, 0 failed, incl. `test_slice38_graph_predicates.py`, `test_slice130_package_boundary.py`, `test_sdk_surface_parity_oracle.py`, and `earp/test_config_resolver.py` 51/51).
- **TypeScript:** `npm run build`, `npm run typecheck`, full `npm test` (**653/653 passed**, 0 failed) + `slice38-graph-predicates.test.js` (3/3).
- **Governance & Docs:** `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, `python3 scripts/check-public-doc-truth.py`, `python3 scripts/check-sdk-surface-parity.py`, `./scripts/check-governed-surface-pin.sh` — **ALL PASS**; `dev/acceptance.md` untouched; zero external consumer-project leaks.
- **Adversarial Code Review:** Independent `slice-reviewer` returned **`APPROVE`** at HEAD `8e44a47ad` (0 open P0/P1/P2 findings).

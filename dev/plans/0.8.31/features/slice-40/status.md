---
title: Slice 40 Status — Historical Graph Reconstruction (HISTORY-GRAPH)
status: CLOSED
target_release: 0.8.31
slice: 40
date: 2026-11-22
---

# Slice 40 Status — `0.8.31` Historical Graph (`HISTORY-GRAPH`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-40/plan.md`](./plan.md)
- **Commits:** `039bc7f88` (Layer 1 — historical `graph_expand` + `EdgeViewBoundary::AsOf` + inline eligibility + `proptest` oracle), `9ca376e57` (`search.rs` `EntityFts` depth-0 revision emission & `selected_source` fix), `e00a3b359` (Layer 2 — historical frozen graph evidence), `9f362ed1f` (Layer 3 — Rust SDK surface pins, Python/TS bindings & interface/guide/reference docs), `fe5e7d8a1` (`FIX-1` — `SearchOriginCapture::requires_edge_origin()` guard + `slice50_evidence` regression tests)
- **Verdict:** **CLOSED / LANDED** — all 3 implementation layers + `FIX-1` verified green across Rust, Python, TypeScript, and Markdown/doc gates; independent adversarial code review returned **`APPROVE`** at HEAD `fe5e7d8a1`.

## 1. Delivered Scope

1. **Layer 1 (`039bc7f88`) — Historical `graph_expand` Request Carriers, Closed Codec, `EdgeViewBoundary::AsOf`, Inline Eligibility & Historical Explanation:**
   - Added `as_of_write_cursor: Option<u64>` and `as_of_epoch_ms: Option<i64>` + fluent builders (`with_as_of_write_cursor`, `with_as_of_epoch_ms`) on `GraphExpandRequestV1` (`types.rs`), closed wire codec with canonical base-10 string `"asOfWriteCursor"` / signed 64-bit integer `"asOfEpochMs"`, explicit JSON `null` refusals, and mutual exclusion (`GraphContextInvalid` at `/asOfEpochMs`).
   - Extended `encode_graph_evidence_request` with disjoint prefix-free trailing tags `2u8`/`3u8` preserving byte-identical encoding when unset; implemented O(log N) `_fathomdb_commit_timestamps` reverse seek (`ORDER BY committed_at_ms DESC, write_cursor DESC LIMIT 1`, resolving pre-upgrade timestamps to cursor `0`) and saturating `as_of_sql_i64`.
   - Implemented `EdgeViewBoundary::AsOf` (direct 2/4-arm compound `UNION ALL ... LIMIT ?limit` with `from_id != ?1`, `(temporal_fallback IS NULL OR temporal_fallback = 0)`, derived-edge closure barriers via `read_eligibility_sql_historical_for_cursor`), disabled active-only `cpv` pushdown under `AsOf`, restored `EdgeRow.superseded_at: Option<i64>`, added two-arm historical `load_node` (`ORDER BY write_cursor DESC LIMIT 1`) + current-head active check (`Engine::transition` semantics per amended ADR `#75`), historical `resolve_seeds`, `filter.rs::append_node_eligibility_sql_historical` (`properties > body` for both `attributes` and `property_predicates`), `structural_dependency_state_as_of` / `registered_dependency_for_cursor_as_of`, and production `EXPLAIN QUERY PLAN` helpers (`slice38_query_plans` 5/5 — zero `SCAN`, zero `TEMP B-TREE`) plus `proptest` snapshot BFS oracle (`slice38_historical_graph`).
2. **Layer 2 (`e00a3b359`) — Historical Frozen Graph Evidence (`evidence.rs`, `filter.rs`, `execution.rs`):**
   - Threaded `history_as_of: Option<u64>` into `preflight_graph_evidence` and token minting; preserved `GRAPH_SELECTOR_BYTES = 300` and `GRAPH_TOKEN_BYTES = 704` (`fdbgev1.` + 696 hex chars) using reserved selector byte `payload[7]` (`0` = current-time unchanged, `1` = historical `as_of`, any other value rejected as `EvidenceUnavailable`).
   - Extended `INTRINSIC_{NODE,EDGE}_PREFLIGHT_SQL`, `query_graph_evidence_batch` (`TARGET_ELIGIBILITY` and `SOURCE_ELIGIBILITY` both routed through inline historical `properties > body`), `authorize_graph_evidence`, `materialize_graph_evidence_batch` (`write_cursor <= c` and `source_cursor <= c`), `load_stored` (`artifact_head_inactive` / `source_head_inactive`), and `resolve_graph_evidence` (`filter::text_hit_passes_filter_historical`, accurate `artifact_lifecycle.superseded`, unconditional `EvidenceUnavailable` on erase/purge/closure-barrier/inactive-head).
3. **Layer 3 (`9f362ed1f`) — Cross-SDK Parity & Documentation:**
   - Added Rust SDK builder pins (`fathomdb-sdk/tests/surface.rs`), Python bindings (`types.py`, `graph.py`, 4 entries in `python_surface_additions.json`, `test_slice38_historical_graph.py`), TypeScript bindings (`src/ts/src/graph.ts` with `Number.isSafeInteger` & JSON `null` refusal, `slice38-historical-graph.test.ts`), and updated `dev/interfaces/{wire,rust,rust-sdk,python,typescript}.md`, `docs/guides/{graph-retrieval,frozen-evidence}.md`, and `docs/reference/{rust-api,rust-sdk,python-api,typescript-api}.md` (zero external consumer-project mentions).
4. **Regressions & Adversarial Review Remediation (`9ca376e57`, `fe5e7d8a1`):**
   - Restored `EntityFts` depth-0 revision emission & `selected_source` in `search.rs` (`slice80_history` 17/17 green) and added `SearchOriginCapture::requires_edge_origin()` (returning `true` for `EvidenceCapture`) so `search_with_evidence(..., use_graph_arm: true)` never emits unbacked `EntityFts` depth-0 hits while preserving depth `>= 1` `Traversal` seeding (`slice50_evidence` 30/30 green).

## 2. Verification Summary

- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings across all 13 crates), `slice38_historical_graph` (12/12 incl. `proptest`), `slice38_query_plans` (5/5), `slice20_graph_evidence` (36/36), `slice50_evidence` (30/30), `slice60_graph_expand` (19/19), `slice60_wire` (10/10), `slice60_fix4_rss` (1/1), `slice60_fix5_rss` (1/1), `slice38_graph_predicates_and_seeding` (16/16), `pr_c1_graph_seeding` (5/5), `slice80_history` (17/17), `evidence::tests` (9/9), `fathomdb-sdk` + `fathomdb` + `fathomdb-inspector` + `fathomdb-mcp` (468 passed, 0 failed).
- **Python:** Owned wheel rebuilt via `scripts/setup-python-owned.sh --build`; `ruff` clean, `pyright` (0 errors, 0 warnings), full `pytest` (**1,794 passed**, 44 skipped, 0 failed, incl. `test_slice38_historical_graph.py` and `test_slice130_package_boundary.py`).
- **TypeScript:** `npm run build`, `npm run typecheck`, full `npm test` (**657/657 passed**, 0 failed, incl. `slice38-historical-graph.test.js`).
- **Governance & Docs:** `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, `python3 scripts/check-public-doc-truth.py`, `bash scripts/check-design-refs.sh` — **ALL PASS**; `dev/acceptance.md` untouched; zero external consumer-project leaks.
- **Adversarial Code Review:** Independent `slice-reviewer` (`95490fe6`) returned **`APPROVE`** at HEAD `fe5e7d8a1` (0 open P0/P1/P2 findings).

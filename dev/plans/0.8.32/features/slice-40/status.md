---
title: Slice 40 Status — MCP-RW, Gate1-D9 Fixes, and Gate1-D10 Status Cache + Vector Cosine
status: LANDED
target_release: 0.8.32
slice: 40
date: 2026-11-25
---

# Slice 40 Status — `MCP-RW`

## Outcome

**LANDED on `dev/0.8.32`** (`/mnt/data/worktrees/release-0.8.32`). Delivers `R32-02`, `R32-06` (`Gate1-D9`), and `R32-08` (`Gate1-D10`) across four sequentially executed, independently reviewed work packages:

1. **`WP-40A1` (`ab1e8b704` + `FIX-1` `50618998d`) — `Gate1-D9` (`0.8.31` Bug 1 + Bug 2 Option B2-strict):**
   - Fixed the empty-DB reopen cursor regression by folding `_fathomdb_commit_timestamps` and `_fathomdb_projection_terminal` into `load_next_cursor()`, unifying boundary reads, adding O(log N) boundary indexes in `STEP_39_SQL`, and adding explicit `owner_kind` (`'canonical' | 'non_canonical'`) on `_fathomdb_projection_terminal` with accurate Schema 38 → 39 backfill so `data_plane_integrity` has zero false positives on `AdminSchema`/`OpStore` terminal cursors and zero false negatives on deleted canonical batch tails.
   - Upgraded `_fathomdb_projection_registry` in unreleased Schema 39 to `PRIMARY KEY(entity_class, name)` preserving all Schema-38 columns and 3 visibility triggers (`pr_ai`/`pr_au`/`pr_ad`), introduced strictly typed `ProjectionTarget { name, entity_class }` (zero bare-string shims) across `Engine::configure_projections` `drop` and `ProjectionDelta.{built, dropped, deferred}`, and added `entity_class` to `ProjectionRuntimeStatusEntry` across Rust, `fathomdb-sdk`, Python, and TypeScript.
2. **`WP-40A2` (`8e18ba9b4`, pin SHA `cc538539b`, `FIX-1` `50618998d`) — `Pin C` Seams (`Gate1-D4`):**
   - Added atomic `ActuationOperationV1::PutLatestState` (`LatestStateActuationV1`) with operation-aware refusal field paths (`/collection`, `/recordKey`, `/payloadJson`), bound caller `scope`, and collision-free `simulation_cursor_base` near `i64::MAX`.
   - Added `EP-07` `OpenOptions::create_if_missing` (default `true`) + `DatabaseNotFound` (`49` shared error classes) acquiring the sidecar lock and checking dangling symlinks / missing paths before embedder or CUDA initialization with zero artifact creation on refusal.
   - Added `EP-01` `read::visibility_state` (`VisibilityStateV1`) on one deferred reader transaction with zero writer lock and monotonic `fetch_max` regression guard.
   - Updated governed surface pin (`57 → 58` live governed ops, `95 → 97` signed allowlist members; `core = 5`, `recovery_denylist = 5`, `runtime_controls = 2` invariant) with lockstep Rust/SDK/Python/TypeScript parity and regenerated `docs/generated/sdk-operation-map.md`.
3. **`WP-40B` (`cafb6bf49`, `FIX-1` `50618998d`) — Production Read+Write `fathomdb-mcp`:**
   - Preserved `MR-070.2` (runtime dep solely `fathomdb-sdk`, zero `rusqlite`) and exact-pinned `rmcp = "=3.5.1"` confined to `src/edge.rs` and `src/main.rs`; detached and excluded `examples/locomo_*` from `cargo package`.
   - Enforced default read-only posture (`allow_writes: false`, exact **7-tool** read-only `tools/list`, deterministic `writes_disabled` refusal with zero DB mutation), opt-in per-store scoped writes (`actuate`, `put_latest_state`, **9** tools when write-enabled), fail-fast `SC-01`–`SC-10` startup conflict matrix (including duplicate canonical-path `SC-06` and hosted `HostedStorePolicy` / `SC-10`), `AC32-MCP-SAFETY-01`–`09`, live `EP-01` `state` and `EP-07` open, scoped frozen evidence integrated into `search`/`expand`/`fetch`, per-class `(entity_class, name)` `orient`, blocking drain fence, and `m_wire.py` sync (`test-slice30-m-cli` `24/24`).
4. **`WP-40C` (`5597453e8`, `FIX-1` `50618998d`, `FIX-2` `2baaa08b9`) — 3-Channel Distribution, Release Gates & MCP Docs:**
   - Flipped `publish = true` in `fathomdb-mcp/Cargo.toml` (keeping `default = []`), added `LICENSE` + `README.md`, created lockstep PyPI binary wheel `src/python-mcp/` (`--features default-embedder`, `fathomdb[mcp]` optional extra, base `src/python/pyproject.toml` `[project.scripts]` strictly empty per `AC-041`), and wired 5-target GH Release binaries, PyPI wheels, parallel `publish-rust-t7-mcp` (tag-scoped `v0.8.32` token bootstrap), `post-publish-smoke`, `promote-npm-release.yml`, `docs-pages.yml`, and all 12+ release/contract gates.
   - Updated `AGENTS.md`, `dev/interfaces/mcp.md`, `dev/design/mcp-adapter/{design,requirements}.md`, design lifecycle catalogs (`29` maintained design docs), and created `docs/guides/mcp-server.md` and `docs/reference/mcp.md`.
5. **`WP-40D` (`655f6e5a4`, `7cc8ed1c5`, `a2da98bf5`, `FIX-1` `c1bc4a6ef`, `FIX-2` `2baaa08b9`) — `Gate1-D10` (`R32-08`):**
   - Completed reviewed Phase 1 requirements/AC/architecture updates (`655f6e5a4`) and reviewed Phase 2 design (`wp-40d-design.md`, `7cc8ed1c5`).
   - Unified `projection_generation::snapshot_observed_boundary` and implemented slot-verified, monotonic pre-snapshot-epoch-guarded `ProjectionGenerationStatusCache` across `Engine` and all 8 `ReaderWorkerPool` workers (`AC32-PERF-STATUS-01..03`), eliminating `O(N)` steady-state completion scans on `search(..., explain=true)`, `search_frozen(..., explain=true)`, `search_with_evidence`, and query-seeded `graph.expand` while preserving `stored_head_coverage` and `vector_default` external-tamper detection.
   - Exposed additive `PerHitExplain.vector_cosine: Option<f64>` (`1.0 - vec_distance_cosine` on uncentered f32 vectors; `Some` iff scored by `execute_vector_arm` / `vector_rank.is_some()`, `None` on zero-norm/`NULL`/`NaN` or vector-absent hits, finite clamped to `[-1.0, 1.0]`, `-0.0 -> +0.0`, zero SQL cost when `explain=false`, unchanged L2 ordering) across Rust, `fathomdb-sdk`, Python, and TypeScript (`AC32-EXPLAIN-COSINE-01..04`) while keeping `SearchHit`, `EvidenceContributionV1`, `fathomdb-mcp` `PerHitOut`, Schema 39 DDL, and governed surface pin (`58`/`97`) unchanged.

## Verification & Adversarial Review Receipts

- **Full unconfined `./scripts/agent-verify.sh` on `2baaa08b9`:** **EXIT `0`** — `agent-security: 0 violation(s), 0 blocker(s), 0 downgrade(s)` (`AC-036`, `AC-037`, `AC-038`, `AC-050a`, `AC-050c` PASS); `agent-test.sh: 246/249 suites passed (skipped=3 excluded=0)`.
- **`mkdocs build --strict -d /tmp/fathomdb-slice40-final-mkdocs`:** **EXIT `0`** (`0` warnings, built in `0.53s`).
- **Independent `pro` adversarial reviews:**
  - Slice 40 plan: `VERDICT: APPROVE` (`00d2f0dc3`)
  - `WP-40A1..WP-40C + FIX-1` (`50618998d`): `VERDICT: APPROVE`
  - `Gate1-D10` Phase 1 requirements/AC/architecture (`655f6e5a4`): `VERDICT: APPROVE`
  - `Gate1-D10` Phase 2 `WP-40D` design (`7cc8ed1c5`): `VERDICT: APPROVE`
  - `WP-40D + FIX-1` (`c1bc4a6ef`): `VERDICT: APPROVE`

## Commits on `dev/0.8.32`

- `00d2f0dc3` — `docs(0.8.32): approve Slice 40 plan and record Gate1-D9`
- `ab1e8b704` — `feat(0.8.32): WP-40A1 reopen cursor fix and per-class projection namespace`
- `8e18ba9b4` — `feat(0.8.32): WP-40A2 Pin C PutLatestState, EP-07 create_if_missing, and EP-01 visibility_state`
- `cc538539b` — `chore(0.8.32): pin governed surface SHA for WP-40A2 Pin C`
- `cafb6bf49` — `feat(0.8.32): WP-40B production read+write fathomdb-mcp with scoped safety`
- `5597453e8` — `feat(0.8.32): WP-40C fathomdb-mcp 3-channel distribution, release gates, and docs`
- `50618998d` — `fix(0.8.32): Slice 40 FIX-1 review P1/P2 and shell lint remediations`
- `655f6e5a4` — `docs(0.8.32): Gate1-D10 Phase 1 requirements, acceptance criteria, and architecture`
- `7cc8ed1c5` — `docs(0.8.32): approve WP-40D design (Gate1-D10)`
- `a2da98bf5` — `feat(0.8.32): WP-40D pooled-reader status cache and PerHitExplain.vector_cosine`
- `c1bc4a6ef` — `fix(0.8.32): WP-40D FIX-1 pre-snapshot epoch guard and review test polish`
- `2baaa08b9` — `fix(0.8.32): Slice 40 FIX-2 inspector admission, EARP delta, and TS CI cache split`

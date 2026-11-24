---
title: 0.8.32 Prework Slice 2 — Repo-Wide Cruft Review
status: COMPLETE
target_release: 0.8.32
slice: 2
date: 2026-11-24
---

# 0.8.32 Prework Slice 2 — Repo-Wide Cruft Review

## 1. Coverage, delta vs. `0.8.31`, and scope boundary

Read-only audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`).
Tracked file counts at audit vs. the `0.8.31` Slice 2 baseline:

| Subtree | `0.8.31` baseline | `0.8.32` (`38a1594bf`) | Delta / notes |
| --- | ---: | ---: | --- |
| `dev/` | 11,434 | 11,470 | +36 (`0.8.31` slice plans/statuses, `dev/agent-memory/`, `0.8.32` draft) |
| `dev/plans/` | 9,922 | 9,954 | +32 (`0.8.31` features + `0.8.32` scaffold) |
| `dev/design/` | 242 | 242 | Unchanged (`cross-cutting-change-checklist.md` absent) |
| `scripts/` | 1,113 | 1,122 | +9 (`0.8.31` retro/release gates) |
| `docs/` | 52 | 54 | +2 (`0.8.31` release notes / docs) |
| `src/` | 1,268 | 1,299 | +31 (`0.8.31` engine/SDK/Inspector) |

**What `0.8.31` already resolved from `0.8.31/prework/slice-2.md`:**
`P2-02` (`AGENTS.md` crate count -> 13 and `DEPRECATED` banners on `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`), `P2-03` (`0.8.31` public doc truth & API refs), `P2-04` partial (`fathomdb-data-plane-architecture-v2.md` updated to `0.8.31` / 13 members; `ADR-0.6.0-decision-index.md` #72 updated), `P2-05` (strict dangling-edge test assertions), and `P2-13` (`src/inspector/tools/local-ci.sh` & `local-release.py` decoupled).

**What remains explicitly out of scope for `0.8.32` (scheduled in `dev/plans/0.8.33/scope.md`):**
Major public OSS doc/IA overhaul, bulk `dev/` pruning (`dev/plans/0.8.27/features/` ~6,915 files, `dev/plans/runs/` ~1,508 files, historical `dev/design/` ~215 slice docs, `dev/interfaces/` refactoring, `dev/DOC-INDEX.md` slimming, `dev/hygeine-roadmap.md` rename), repo-root working-file cleanup (`0.8.23-release-todo.md`, `CORPUS-01-supersession-questions.txt`, `plan-slice-85-{recovery,review}.md`), retired Steward/Orchestrator seat & `scripts/repo-prune/` removal, and `test/` / `experiments/` / `dev/experiments/` bulk archival.

Accordingly, this review focuses on **cruft and structural debt that directly impacts `0.8.32`** (Slice 0 orientation aids, Slice 10/20 engine splits & `module-boundary-policy.txt`, `fathomdb-mcp` graduation, Dependabot alert noise from orphaned historical lockfiles, and `0.8.31`/`0.8.32` currency).

---

## 2. Candidate inventory by area

### A. `fathomdb-engine/src/` test-file relocation, hotspot splits, and boundary policy

| Candidate / path | Observation | Action | Release target |
| --- | --- | --- | --- |
| `src/rust/crates/fathomdb-engine/src/provider_admission_tests.rs`, `slice20_fix1_tests.rs`, `slice20_open_hook_tests.rs`, `slice90_close_tests.rs`, `slice90_concurrent_close_tests.rs`, `slice90_close_review_tests.rs`, `slice90_post_probe_real_error_tests.rs`, `stored_coverage_tests.rs`, `visibility_admission_tests.rs`, `visibility_coalescing_tests.rs` | 10 pure `#[cfg(test)]` files (2,421 lines total) sit at top level in `src/rust/crates/fathomdb-engine/src/` and are declared in `lib.rs` (lines 117, 892–911, 4114–4117). All use `super::*` / `crate::*` internal items, and all 10 are explicitly listed in `dev/tools/module-boundary-policy.txt` (lines 118–153, plus `unparsed-macro visibility_coalescing_tests` on line 265). | `archive` (relocate inside `src/tests/` or subsystem test submodules; update `lib.rs` + `dev/tools/module-boundary-policy.txt`) | `0.8.32` (Slice 20 entry) |
| `src/rust/crates/fathomdb-engine/src/search.rs` (3,847 lines), `src/rust/crates/fathomdb-engine/src/evidence.rs` (3,628 lines) | Monolithic hotspots slated for zero-behavior-change submodule split (`search/{mod,fts_fast_path,vector_arm,fusion_stage,hydration}.rs` alongside existing `search/{paired50_tests,ranking_tests}.rs`; `evidence/{mod,ranked,graph}.rs`) plus `dev/tools/module-boundary-policy.txt` & `dev/tools/module-boundary-gate/src/main.rs` (`REQUIRED_FORBIDDEN_*` / `owner`) updates before `scope` edits. | `deprecate-in-place` (split into submodules before threading `scope`) | `0.8.32` (Slice 20 entry) |
| `src/rust/crates/fathomdb-engine/src/slice30_observer.rs` + `slice30_observer/transport_tests.rs` | Feature-gated (`slice30-observer`) runtime diagnostic observer referenced by `search.rs`, `search_api.rs`, `wal_attribution.rs`, and `fathomdb-sdk` (`RUST_QUALIFICATION_DIAGNOSTICS`). Not a pure `#[cfg(test)]` file; do not relocate with `*_tests.rs` (`transport_tests.rs` already lives under `slice30_observer/`). | `keep` | `0.8.32` / `keep` |

### B. Governance indexes, `dev/design/`, `AGENTS.md`, and gate diagnostics

| Candidate / path | Observation | Action | Release target |
| --- | --- | --- | --- |
| `dev/adr/ADR-0.6.0-decision-index.md` | 226-line chronological phase table mixing active and superseded rulings; has duplicate row `#66` (`ADR-0.8.28-store-format-35.md` and `ADR-0.8.29-agent-hints-sidecar.md`); omits 20 existing `ADR-*.md` files (`ADR-0.7.0-*`, `ADR-0.8.0-filter-grammar.md`, `ADR-0.8.6-*`, `ADR-0.8.11-*`–`0.8.22-*`, `ADR-0.8.27-typescript-subscriber-delivery.md`, `ADR-0.8.30-explicit-module-model-release.md`); mixes `status: draft` / `NO-STATUS` entries without clear active vs. historical separation. | `deprecate-in-place` (split into **Active / Governing** vs. **Superseded / Historical** tables, fix `#66`, index missing ADRs) | `0.8.32` (Slice 10) |
| `dev/design/README.md` & `dev/design/cross-cutting-change-checklist.md` & `dev/design/document-lifecycle.json` | `dev/design/README.md` (46 lines) currently declines to list live subsystem owners; `dev/design/cross-cutting-change-checklist.md` does not exist yet; `dev/design/document-lifecycle.json` still classifies `dev/design/orchestration.md` as `"class": "maintained"` despite `AGENTS.md` §§ 7, 9 marking it deprecated. Any new doc in `dev/design/` must be added to `document-lifecycle.json` (`check-design-lifecycle.py`). | `deprecate-in-place` (add 1-page owner map to `dev/design/README.md`, create `cross-cutting-change-checklist.md`, link in `AGENTS.md`, update `document-lifecycle.json` including `orchestration.md` -> `historical`/`superseded`) | `0.8.32` (Slice 10) |
| `AGENTS.md` (line 31) | Still describes `fathomdb-mcp` as `[MCP server, publish = false]` and does not link `dev/design/cross-cutting-change-checklist.md`. Must stay `<= 300` lines and pass `scripts/check-agents-md-paths.py`. | `deprecate-in-place` (link checklist in Slice 10; flip `publish = false` note when `Cargo.toml` flips) | `0.8.32` (Slices 10, 40) |
| `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, `scripts/check-module-boundaries.sh` / `dev/tools/module-boundary-gate/src/main.rs` | Failure paths do not consistently print the exact registry/config file to update (per `0.8.32/scope.md` § 6 / `R32-07`). | `deprecate-in-place` (tighten failure diagnostics to name exact files) | `0.8.32` (Slice 10) |
| `dev/DOC-INDEX.md` (486 lines / 63.7 KB), `dev/hygeine-roadmap.md`, `dev/update-docs.md` | Exceeds 12 KB target (`DOC-INDEX.md`), carries filename typo (`hygeine`). Scheduled in `0.8.33/scope.md`. | `keep` in `0.8.32` (`archive`/`delete` in `0.8.33`) | `0.8.33` |

### C. Orphaned lockfiles and Python requirement files (150 open Dependabot alerts)

`prework/slice-0.md` records **150 open Dependabot alerts** (`rust: 86`, `pip: 63`, `npm: 1`). Inspecting all **56** `Cargo.lock` files and **7** Python manifest/lock files in the checkout reveals:

| Candidate / path | Count | Observation | Action | Release target |
| --- | ---: | --- | --- | --- |
| `dev/plans/0.8.27/features/slice-135/results/**/Cargo.lock` | 33 | `0.8.27` Slice 135 provenance snapshots. Their sibling `Cargo.toml` files were already renamed to `Cargo.toml.snapshot` (enforced by `scripts/tests/test_slice135_cargo_receipt_snapshots.py`), but `Cargo.lock` was left with a live lockfile filename, causing GitHub Dependabot to emit stale Rust alerts. Note: `SHA256SUMS` and `audit.py` inside `results/` reference `"Cargo.lock"` by name (none are executed by `agent-test.sh`). | `archive` (rename to `Cargo.lock.snapshot` + update local `SHA256SUMS`/`audit.py` refs in `0.8.32` Slice 10, OR `delete` with `0.8.27/features/` in `0.8.33`) | `0.8.32` (Slice 10) or `0.8.33` |
| `dev/experiments/0.8.29-slice60-projected-metadata/driver/Cargo.lock`, `dev/experiments/current-history-search/Cargo.lock`, `dev/experiments/current-history-search/followups/Cargo.lock`, `dev/experiments/erasure-availability/Cargo.lock` | 4 | Historical experiment lockfiles not used by any shipped crate or CI gate (`materialize_driver.py` / `operator-sources.json` references `driver/Cargo.lock` locally). | `archive` (`Cargo.lock.snapshot` or delete with `dev/experiments/` in `0.8.33`) | `0.8.32` (Slice 10) or `0.8.33` |
| `dev/plans/refactor-background-check/structure/dev_requirements_md.txt` | 1 | Unreferenced historical pip requirements snapshot (`git grep dev_requirements_md` returns zero hits) contributing to pip Dependabot alerts. | `delete` (or `archive` with `refactor-background-check/` in `0.8.33`) | `0.8.32` (Slice 10) or `0.8.33` |
| `src/python/uv.lock` | 1 | Stale lockfile frozen at `fathomdb = 0.8.20` (`maturin 1.14.1`, `pytest 9.0.3`), ignored by `setup-python-owned.sh --provision` (already flagged in `0.8.31` `P1-09` and `0.8.31` `prework/slice-1.md`). | `delete` or refresh in `0.8.32` Slice 10 (coordinate with Slice 1 Dependabot sweep) | `0.8.32` (Slice 10) |
| `Cargo.lock` (root), `dev/tools/module-boundary-gate/Cargo.lock`, `scripts/tests/fixtures/{cudarc-lock-set,single-cudarc}/**/Cargo.lock` | 17 | Active workspace/gate lockfiles (2) and intentional CI unit-test fixtures (15). | `keep` | `keep` |
| `experiments/comp_drivers/surrealdb/Cargo.lock`, `experiments/sdk_drivers/rust/Cargo.lock` | 2 | Referenced by `experiments/` benchmark harnesses (`0.8.35`). | `keep` (`0.8.35` review) | `0.8.33` / `0.8.35` |

### D. `fathomdb-mcp` evaluation-era leftovers and packaging gaps

| Candidate / path | Observation | Action | Release target |
| --- | --- | --- | --- |
| `src/rust/crates/fathomdb-mcp/Cargo.toml` | Has `# Evaluation build; not published until the MCP evaluation is done.`, `publish = false`, `description = "FathomDB MCP server: read-only tools over fathomdb-sdk, served over stdio."`, and `[[example]]` targets `locomo_load` / `locomo_eval` (`test = true`, pointing at `dev/experiments/mcp-hybrid-vs-keyword/`). (`slice30_m_helper` IS active in `scripts/test-slice30-m-*.sh` and must be kept.) | `deprecate-in-place` (update description, flip `publish = true` per `HANDOFF-06` Option 1, retire/separate `locomo_*` eval examples from crate manifest or move to `experiments/` so published crate metadata/sources stay clean) | `0.8.32` (MCP slice) |
| `src/rust/crates/fathomdb-mcp/LICENSE` (missing) | Every other publishable crate under `src/rust/crates/` carries a byte-identical `LICENSE` copy. Flipping `publish = true` in `Cargo.toml` will immediately trigger `scripts/check-license-consistency.sh` to require `src/rust/crates/fathomdb-mcp/LICENSE`. | `keep` / add `LICENSE` copy when flipping `publish = true` | `0.8.32` (MCP slice) |
| `src/rust/crates/fathomdb-mcp/src/{config,lib,edge,error}.rs` & `dev/interfaces/mcp.md` | Evaluation-era read-only guards and prose: `config.rs` hard-rejects `write: true` (`"must be false in this release (read-only)"`), hard-rejects `scope` (`"must be null in this release (MR-014.4)"`), rejects `max_open_stores != 1`, and documents `snippet_head_bytes` with a LoCoMo header example; `lib.rs` (`//! A read-only...`) and `edge.rs` (`INSTRUCTIONS = "FathomDB read-only memory stores..."`) assert read-only posture; `error.rs` says `"only l: (logical id) references are accepted in this release"`; `dev/interfaces/mcp.md` says `"read-only and unscoped in this evaluation build"`. | `deprecate-in-place` (update contract, config validation, startup conflict checks, and error/instruction text for governed read+write + scope) | `0.8.32` (Slice 10 ADR & MCP slice) |
| `src/rust/crates/fathomdb-mcp/examples/{locomo_load.rs,locomo_eval/}` | Only referenced by `experiments/mcp_agent_loop.py` and `dev/experiments/mcp-hybrid-vs-keyword/` (not wired in `agent-test.sh`). Note `locomo_eval` currently has `test = true` in `Cargo.toml`. | `archive` (relocate out of `fathomdb-mcp/examples/` or drop from `fathomdb-mcp` crate before `crates.io` publish, keeping `experiments/mcp_agent_loop.py` working if needed) | `0.8.32` (MCP slice) or `0.8.33` |

### E. `0.8.31` / `0.8.32` plans, boards, CI baseline, and `.claude` / `.codex`

| Candidate / path | Observation | Action | Release target |
| --- | --- | --- | --- |
| `dev/plans/0.8.31/` | Cleanly closed (`prework/slice-0..9`, `features/slice-0..80`, `plan.md`, `scope.md`, `quality-backlog.md`, `cuda-delivery-plan.md`); no stray `TODO`/`WIP`/scratch files found. | `keep` (until `0.8.33` archival) | `0.8.33` |
| `dev/plans/runs/STATUS-0.8.31.md` & `dev/plans/release-state-0.8.31.json` | `release-state-0.8.31.json` has `published` receipt (`v0.8.31`), so `scripts/release-current.py` exits `0` cleanly. `STATUS-0.8.31.md` lacks the `closed — historical record` marker line (shared with 6 other state-backed boards: `0.8.20`, `0.8.23`, `0.8.25`, `0.8.26`, `0.8.28`, `0.8.29`). | `keep` (optionally prepend `closed — historical record` banner to `STATUS-0.8.31.md` when commissioning `STATUS-0.8.32.md` in Slice 10) | `0.8.32` (Slice 10) |
| `dev/plans/0.8.32/{plan,scope}.md` | Still `status: PROPOSED` (expected in prework until HITL Gate 1). `plan.md` § "Open HITL decisions" is now superseded by the owner rulings recorded in `prework/slice-0.md`. Note `prework/slice-0.md` also records 2 failing baseline `main` CI jobs (`windows-wal-checkpoint-diagnosis` hardcoded `golden-0.8.30-schema37.sqlite` path in `migrate_cli.rs`, plus 4 `verify-fast` suites). | `deprecate-in-place` (promote `plan.md`/`scope.md`, add `release-state-0.8.32.json` + `STATUS-0.8.32.md`, and fix baseline CI failures in Slice 10) | `0.8.32` (Slice 10) |
| `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`, `.claude/hooks/{seat-path-guard,sealed-worktree-guard}.sh`, `.claude/commands/decisions.md`, `.codex/skills/fathomdb-decisions/` | `decisions.md` and `fathomdb-decisions/` are active. The 3 legacy agent specs already carry `DEPRECATED` headers and remain only because `scripts/tests/test_seat_path_guard.sh` parses their frontmatter. | `keep` in `0.8.32` (`archive` with `test_seat_path_guard.sh` in `0.8.33`) | `0.8.33` |
| Repo root stray files (`0.8.23-release-todo.md`, `CORPUS-01-supersession-questions.txt`, `plan-slice-85-{recovery,review}.md`) | Inbound links from `dev/plans/0.8.27/`; explicitly assigned to `0.8.33` Slice 10 (`0.8.33/scope.md` § 1). | `keep` in `0.8.32` (`archive` in `0.8.33`) | `0.8.33` |

---

## 3. Numbered Proposals (`P2-01`–`P2-10`)

| ID | Proposal | Action | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation (include / postpone / reject) | Target Slice / Rationale |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `P2-01` | Relocate the 10 top-level `*_tests.rs` files out of `fathomdb-engine/src/` (`lib.rs`, `dev/tools/module-boundary-policy.txt`); keep feature-gated `slice30_observer.rs` in place | `archive` | H | L | M | **include** | `0.8.32` Slice 20 entry (`R32-07`, `HANDOFF-05`). Zero-behavior-change cleanup before `scope` edits; preserves `pub(crate)` access via `src/tests/` or subsystem modules. |
| `P2-02` | Split `fathomdb-engine/src/search.rs` (3,847 lines) and `evidence.rs` (3,628 lines) into cohesive submodules and update `dev/tools/module-boundary-policy.txt` / gate | `deprecate-in-place` | H | M | M | **include** | `0.8.32` Slice 20 entry (`R32-07`, `HANDOFF-05`). Reduces merge conflicts and diff blast radius across Slices 20, 30, and 40. |
| `P2-03` | Split `dev/adr/ADR-0.6.0-decision-index.md` into **Active / Governing** and **Superseded / Historical** tables, fix duplicate `#66`, and index the 20 unindexed `ADR-*.md` files | `deprecate-in-place` | H | L | S | **include** | `0.8.32` Slice 10 (`R32-07`). High agent/contributor orientation payoff with negligible risk. |
| `P2-04` | Author `dev/design/cross-cutting-change-checklist.md`, add 1-page live subsystem owner map to `dev/design/README.md`, link from `AGENTS.md`, and update `dev/design/document-lifecycle.json` (incl. `orchestration.md` -> `historical`) | `deprecate-in-place` | H | L | S | **include** | `0.8.32` Slice 10 (`R32-07`). Prevents `check-design-lifecycle.py` and `check-agents-md-paths.py` failures while fulfilling `scope.md` § 6. |
| `P2-05` | Tighten failure diagnostics in `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, and `scripts/check-module-boundaries.sh` / gate to name exact files to update | `deprecate-in-place` | H | L | S | **include** | `0.8.32` Slice 10 (`R32-07`). Directly speeds up Slices 20/30/40 when governed surface or module layout changes. |
| `P2-06` | Neutralize orphaned historical lockfiles/manifests generating Dependabot noise: rename 33 `slice-135/results/**/Cargo.lock` (+ 4 `dev/experiments/**/Cargo.lock`) to `Cargo.lock.snapshot`, delete unreferenced `dev_requirements_md.txt`, and retire/refresh stale `0.8.20` `src/python/uv.lock` | `archive` / `delete` | H | L | S | **include** | `0.8.32` Slice 10 (`Prework Prep` / Dependabot triage). Eliminates 86 Rust alerts and stale pip alerts without touching active fixtures (`scripts/tests/fixtures/`), avoiding wait until `0.8.33`. |
| `P2-07` | Scrub evaluation-era read-only/unscoped/LoCoMo remnants in `fathomdb-mcp` (`Cargo.toml` `# Evaluation build` + `publish = true` + `LICENSE` copy, `config.rs`/`lib.rs`/`edge.rs`/`error.rs`, `dev/interfaces/mcp.md`, `AGENTS.md` line 31) and detach `locomo_load`/`locomo_eval` examples from the published crate | `deprecate-in-place` | H | M | M | **include** | `0.8.32` Slice 10 (`mcp.md` ADR/contract) & MCP feature slice (`fathomdb-mcp` graduation & `HANDOFF-06` publish). Must keep `slice30_m_helper` (`scripts/test-slice30-m-*.sh`). |
| `P2-08` | Fix `STATUS-0.8.31.md` historical-closed marker when commissioning `release-state-0.8.32.json` / `STATUS-0.8.32.md`, reconcile `0.8.32/{plan,scope}.md` with HITL Gate 1 rulings, and fix the 2 failing baseline `main` CI jobs (`migrate_cli.rs` golden path, `verify-fast`) | `deprecate-in-place` | H | L | S | **include** | `0.8.32` Slice 10 (`Prework Prep`). Ensures green baseline CI and `scripts/release-current.py` clarity before feature work. |
| `P2-09` | Bulk `dev/` pruning (`dev/plans/0.8.27/`, `dev/plans/runs/`, historical `dev/design/`, `dev/DOC-INDEX.md` slimming, `dev/hygeine-roadmap.md`), root stray files, and `test/` / `dev/experiments/` archival | `archive` / `delete` | H | M | L | **postpone** | `0.8.33` (`scope.md` § 1). Explicitly out of scope for `0.8.32`; premature moves risk breaking `agent-test.sh` / link gates. |
| `P2-10` | Retire deprecated `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`, `scripts/steward-orient.sh`, `scripts/tests/test_seat_path_guard.sh`, and `scripts/repo-prune/` | `archive` | H | M | M | **postpone** | `0.8.33` Slice 10 (`0.8.31` `P2-02`, `P2-09`). Actively coupled to `agent-test.sh` today; remove cleanly alongside `0.8.33` repo hygiene. |

---

## 4. Verification notes (read-only)

- Verified on `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`); no files were modified.
- `python3 scripts/release-current.py` exited `0` (empty stdout: all tracked releases currently published).
- Checked `dev/plans/0.8.31/` (no stray temp/WIP files), `src/rust/crates/fathomdb-mcp/LICENSE` (absent today; required by `scripts/check-license-consistency.sh` once `publish = true`), and `dev/design/document-lifecycle.json` (`236` total entries, `27` marked `maintained`).

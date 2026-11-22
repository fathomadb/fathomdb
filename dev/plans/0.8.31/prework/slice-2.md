---
title: 0.8.31 Prework Slice 2 — Repo-Wide Cruft Review
status: COMPLETE
target_release: 0.8.31
slice: 2
date: 2026-11-22
---

# 0.8.31 Prework Slice 2 — Repo-Wide Cruft Review

## Coverage and scope boundary

Read-only inventory on `release/0.8.31` (`6ee16b9e8`, incorporating `origin/main`
`6795dc902` and Inspector `0.2.1` via `086d5af0a`). Tracked file counts at audit:
`dev/` **11,434** (`dev/plans/` **9,922**, `dev/experiments/` **544**, `dev/design/`
**242**, `dev/performance-benchmarking/` **147**, `dev/inspector/` **136**,
`dev/adr/` **103**, `dev/archive/` **56**), `scripts/` **1,113** (`scripts/tests/`
**681**, top-level **194**), `docs/` **52**, `test/` **5**, `tests/` **332**,
`experiments/` **686**, `src/` **1,268**.

Full OSS repo/`dev/` declutter and community-file creation are explicitly
scheduled in [`dev/plans/0.8.33/scope.md`](../../0.8.33/scope.md) and
[`dev/plans/0.8.33/plan.md`](../../0.8.33/plan.md) (HITL-gated prune manifest in
`0.8.33` Slice 0/10). Accordingly, proposals below separate **quick, safe
`0.8.31` cleanups** (factual drift, Slice 0/20/40/50 contract transitions) from
**`0.8.33`-deferred bulk archival/deletion** (where premature moves would break
active `agent-test.sh` gates, markdown/state validators, or historical links).

## Summary disposition table

| ID | Candidate group | Action | Target |
| --- | --- | --- | --- |
| `P2-01` | Repo-root stray working files (`0.8.23-release-todo.md`, `CORPUS-01-supersession-questions.txt`, `plan-slice-85-{recovery,review}.md`) | `archive` (defer; `keep` in `0.8.31`) | `0.8.33` (Slice 10) |
| `P2-02` | `AGENTS.md` crate count & retired Steward/Orchestrator seat files (`.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`, `scripts/steward-orient.sh`, legacy handoffs) | `deprecate-in-place` (`AGENTS.md` + `.claude/agents/` headers in `0.8.31`); `archive` legacy seats/orient in `0.8.33` | `0.8.31` (Slice 10) / `0.8.33` (Slice 10) |
| `P2-03` | Public docs stale `0.8.30` / schema-37 / `use_graph_arm` / CUDA-CE / Inspector `0.2.1` text (`docs/index.md`, `docs/reference/python-api.md`, `docs/install/python.md`, `docs/compatibility/index.md`, `docs/inspector/`) | `deprecate-in-place` (update in-place to `0.8.31` / schema 38 facts) | `0.8.31` (Slices 40, 50, 60) |
| `P2-04` | Internal `dev/` indexes & headers (`dev/DOC-INDEX.md`, `dev/design/fathomdb-data-plane-architecture-v2.md`, `dev/adr/ADR-0.6.0-decision-index.md` #72, `dev/hygeine-roadmap.md`, `dev/update-docs.md`) | `deprecate-in-place` (factual `0.8.31` rows/headers); `archive` / `delete` typo & bloat cleanup in `0.8.33` | `0.8.31` (Slice 10/60) / `0.8.33` (Slice 10) |
| `P2-05` | Legacy dangling-edge write tests (`fathomdb-engine/tests/pr_g8_dangling_edges.rs`, `slice20_graph_traversal.rs`, `slice85_physical.rs`) & write-commit comments | `deprecate-in-place` (update to assert strict refusal per `ADR-0.8.31-strict-endpoint-integrity.md`) | `0.8.31` (Slice 20) |
| `P2-06` | Historical `dev/plans/` bulk tree (`dev/plans/0.8.27/` [6,915], `dev/plans/runs/` [1,508], `dev/plans/0.8.24`–`0.8.30`, `plan-0.8.*.md`, `refactor-background-check/` [122]) | `archive` (keep closed `release-state-*.json` & audit receipts via HITL prune manifest) | `0.8.33` (Slice 0/10) |
| `P2-07` | Frozen/extracted subtrees (`dev/progress/` [5], `dev/memex-note-on-0.6.0.md`, `dev/turso-watch/` [2], `dev/prototypes/l2-router/` [5], `dev/long-file-investigation/` [8], `dev/roadmap/` [8], `dev/releases/0.8.0.md`, `dev/archive/` [56], `dev/AGENT-HARNESS-EXTRACTED-2026-07-20.md`) | `keep` `dev/archive/` & extraction notice; `archive` remaining frozen/superseded subtrees | `0.8.33` (Slice 10) |
| `P2-08` | Integrated `dev/inspector/` historical plans (`plans/0.1/`, `plans/0.2.0/`) & `dev/design/db-inspector/*` redirect stubs | `keep` for `0.8.31` Slice 50; `archive` `0.1`/`0.2.0` plans & `delete` redirect stubs in `0.8.33` | `0.8.33` (Slice 10) |
| `P2-09` | `scripts/repo-prune/` (24 files from 2026-06-26 + `test_md_generators.sh` & shellcheck ratchet hooks) | `archive` (retire together with `test_md_generators.sh` hooks & ratchet entries) | `0.8.33` (Slice 10) |
| `P2-10` | `scripts/slice135_*` & `scripts/tests/test_slice135_*` (171 files) + `scripts/test-slice30-*` (5 scripts) | `keep` in `0.8.31` (actively wired in `scripts/agent-test.sh` & `.github/workflows/ci.yml`); `archive` / `delete` once `0.8.33`/`0.8.35` audits replacement gates | `0.8.33` / `0.8.35` |
| `P2-11` | `scripts/sbom-survey/` (30 files), `scripts/perf-experiments/` (33), `scripts/gcp-lab/` (3) | `keep` `perf-experiments/` & `gcp-lab/`; `keep` or `archive` `sbom-survey/` in `0.8.33` | `0.8.33` (review `sbom-survey/`) |
| `P2-12` | Empty `test/` placeholder tree (5 READMEs), unwired `tests/experiments/` (75 of 85 `test_*.py` not in `agent-test.sh`), `experiments/` (686), `dev/experiments/` (544) | `delete` empty `test/` (unless populated for `0.8.33` recipes); `archive` unwired experiment suites after `0.8.31` Slice 60 paired benchmark | `0.8.33` / `0.8.35` |
| `P2-13` | `src/inspector/tools/local-ci.sh` & `local-release.py` (coupled to full `agent-verify.sh` / standalone `0.2.1` branch flow) | `deprecate-in-place` (decouple to Inspector-scoped default + release CI wiring per R31-08) | `0.8.31` (Slice 50) |
| `P2-14` | `dev/steward/steward-ledger.jsonl`, `dev/todos-and-considerations-ledger.jsonl`, `dev/agent-tools/{ledgerwatch,ledgerwrite}`, `dev/adr/`, `dev/interfaces/` | `keep` (authoritative append-only ledgers, ADRs, and interface contracts) | `keep` |

## Detailed proposals (`P2-01`–`P2-14`)

- **`P2-01` — Repo-root stray working files (`archive`, target `0.8.33` Slice 10)**
  - **Paths:** `0.8.23-release-todo.md`, `CORPUS-01-supersession-questions.txt`, `plan-slice-85-recovery.md`, `plan-slice-85-review.md`.
  - **Evidence:** `plan-slice-85-{recovery,review}.md` have inbound links from `dev/plans/0.8.27/features/slice-85/{plan.md,recovery-receipt.md,status.md}` and `dev/plans/plan-0.8.27.md`; `0.8.23-release-todo.md` has a stale historical banner calling `0.8.25` active. `dev/plans/0.8.33/scope.md` explicitly assigns root cleanup to `0.8.33` Slice 10 (`R33-01`).
  - **Action:** `keep` undisturbed during `0.8.31` to avoid breaking `0.8.27` plan links; `archive` (and update inbound links) in `0.8.33` Slice 10.

- **`P2-02` — `AGENTS.md` workspace shape & retired Steward/Orchestrator remnants (`deprecate-in-place` in `0.8.31` Slice 10; bulk `archive` in `0.8.33`)**
  - **Paths:** `AGENTS.md` (line 31), `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`, `dev/plans/prompts/0.8.x-{STEWARD,RELEASE-ORCHESTRATOR}-HANDOFF.md`, `scripts/steward-orient.sh`, `scripts/tests/test_steward_orient.sh`.
  - **Evidence:** `086d5af0a` added `fathomdb-inspector` to `Cargo.toml` (13 workspace members), making `AGENTS.md` line 31 ("12 crates") stale. `6ee16b9e8` removed `/steward`, `/orchestrate`, `/orch`, and Codex wrappers and noted legacy `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md` as deprecated in `AGENTS.md`, but the three `.claude/agents/*.md` files themselves still read as active agent specs without a deprecation notice, and `scripts/steward-orient.sh` remains wired in `scripts/agent-test.sh` (`test-steward-orient`).
  - **Action:** In `0.8.31` Slice 10, `deprecate-in-place`: update `AGENTS.md` crate count `12` → `13` (`fathomdb-inspector`) and prepend a one-line `DEPRECATED` banner to `.claude/agents/{orchestrator,sealed-orchestrator,steward}.md`. Defer moving/deleting retired agent definitions, legacy handoffs, and `steward-orient.sh` (plus its `agent-test.sh` hook) to `0.8.33`.

- **`P2-03` — Public docs stale `0.8.30` / schema-37 / `use_graph_arm` / CUDA-CE / Inspector `0.2.1` assertions (`deprecate-in-place`, target `0.8.31` Slices 40/50/60)**
  - **Paths:** `docs/index.md`, `docs/reference/python-api.md`, `docs/install/python.md`, `docs/compatibility/index.md`, `docs/install/{rust,typescript}.md`, `docs/concepts/index.md`, `docs/operations/index.md`, `docs/inspector/{index.md,ROADMAP.md}`.
  - **Evidence:** `python3 scripts/check-public-doc-truth.py` currently fails (`FAIL public-doc-truth: docs/index.md lacks documentation target 0.8.31`) because `PUBLIC_DOCS` still declare `0.8.30` / schema `37`. `0.8.31` R31-05 explicitly requires updating `docs/reference/python-api.md` (stale `use_graph_arm` text + `Engine.write(batch)` atomicity) and `docs/install/python.md` (CUDA + CE wheel coverage), and Slice 50 updates `docs/inspector/` from `0.2.1`/`0.8.30` to `0.8.31`.
  - **Action:** `deprecate-in-place` (targeted factual updates in `0.8.31` Slices 40, 50, 60). Defer new architecture docs, IA overhaul, Material theme, auto-doc upgrades, and recipes to `0.8.33`.

- **`P2-04` — Internal `dev/` indexes, architecture header & hygiene typo (`deprecate-in-place` in `0.8.31`; `archive`/`delete` in `0.8.33`)**
  - **Paths:** `dev/DOC-INDEX.md` (62,237 bytes / 471 lines), `dev/design/fathomdb-data-plane-architecture-v2.md`, `dev/adr/ADR-0.6.0-decision-index.md` (row #72), `dev/hygeine-roadmap.md`, `dev/update-docs.md`.
  - **Evidence:** `dev/DOC-INDEX.md` has zero `0.8.31` rows (missing `ADR-0.8.31-*`, `dev/inspector/`, and `0.8.31` plans) and exceeds its own 12 KB target by >5×. `fathomdb-data-plane-architecture-v2.md` still says `target_release: 0.8.30` (and lists 12 crates). `ADR-0.6.0-decision-index.md` row #72 still says "implementation and qualification pending" despite `0.8.30` delivery. `dev/hygeine-roadmap.md` has the filename typo and 0.8.27/0.8.28 scheduling (`0.8.33` `R33-01` explicitly owns fixing it). `dev/update-docs.md` carries an unverified `2026-06-14` epoch (`3a09571`).
  - **Action:** In `0.8.31` (Slice 10/60), `deprecate-in-place` only factual metadata (`DOC-INDEX.md` 0.8.31 rows, `fathomdb-data-plane-architecture-v2.md` target/crate count, row #72 status if verified). Defer slimming `DOC-INDEX.md` back to ≤12 KB, renaming/refreshing `dev/hygeine-roadmap.md`, and retiring `dev/update-docs.md` to `0.8.33` Slice 10.

- **`P2-05` — Legacy dangling-edge write tests & write-commit comments (`deprecate-in-place`, target `0.8.31` Slice 20)**
  - **Paths:** `src/rust/crates/fathomdb-engine/tests/pr_g8_dangling_edges.rs`, `src/rust/crates/fathomdb-engine/tests/slice20_graph_traversal.rs`, `src/rust/crates/fathomdb-engine/tests/slice85_physical.rs`, `src/rust/crates/fathomdb-engine/src/write_commit.rs` (and corresponding Python/TS dangling-edge tests).
  - **Evidence:** `ADR-0.8.31-strict-endpoint-integrity.md` explicitly mandates strict atomic endpoint validation on every `0.8.31` write path and requires updating existing tests that expect `Engine::write` to commit dangling edges (while preserving `WriteReceipt::dangling_edge_endpoints` as `0` for `0.8.32` bulk opt-in). Note: `src/rust/crates/fathomdb-query/tests/property_template.rs` (flagged in `0.8.30` Slice 2) is already removed.
  - **Action:** `deprecate-in-place` (rewrite legacy dangling-edge test assertions to verify strict atomic refusal or transition-after-create setup in Slice 20).

- **`P2-06` — Historical `dev/plans/` bulk tree (`archive`, target `0.8.33` Slice 0/10)**
  - **Paths:** `dev/plans/0.8.27/` (6,915 files), `dev/plans/runs/` (1,508 files, pre-`0.8.31` boards/logs/codex dumps), `dev/plans/0.8.24`–`0.8.30`, `dev/plans/plan-0.8.*.md`, `dev/plans/0.6.*`–`0.8.1*`, `dev/plans/refactor-background-check/` (122 files), `dev/plans/memex-0.6.0-collaboration/` (7 files).
  - **Evidence:** Accounts for ~9,890 of `dev/plans/`'s 9,922 tracked files. `scripts/check-release-state-views.sh` and `scripts/lint-plans-status.sh` actively scan `release-state-*.json` and top-level `dev/plans/*.md`. `0.8.33/plan.md` requires a HITL-approved pruning manifest before relocating/removing historical runs.
  - **Action:** `keep` intact during `0.8.31`; `archive` (retaining closed `release-state-*.json`, boards, and golden/audit receipts per `0.8.33` HITL decision #1) in `0.8.33`.

- **`P2-07` — Frozen, archived, and extracted engineering subtrees (`keep` / `archive`, target `0.8.33` Slice 10)**
  - **Paths:** `dev/archive/` (56 files), `dev/AGENT-HARNESS-EXTRACTED-2026-07-20.md`, `dev/progress/` (5 files, frozen 0.6.x), `dev/memex-note-on-0.6.0.md`, `dev/turso-watch/` (2 files, 2026-06-21), `dev/prototypes/l2-router/` (5 files), `dev/long-file-investigation/` (8 files), `dev/roadmap/` (8 files, `0.8.0`–`0.8.5`), `dev/releases/0.8.0.md`.
  - **Evidence:** `dev/archive/` is already the designated archive; `dev/AGENT-HARNESS-EXTRACTED-2026-07-20.md` explains intentional dangling refs to `~/projects/agent-aware`. `dev/roadmap/` and `dev/releases/0.8.0.md` predate `ROADMAP.md` and `docs/release-notes/`. `dev/prototypes/l2-router/build_registry.py` reads `dev/plans/runs/{expb-joint-tune,gate2-oracle}-output.json`.
  - **Action:** `keep` `dev/archive/` and `dev/AGENT-HARNESS-EXTRACTED-2026-07-20.md`; `archive` the rest together in `0.8.33` Slice 10.

- **`P2-08` — Integrated `dev/inspector/` historical plans & redirect stubs (`keep` in `0.8.31`; `archive`/`delete` in `0.8.33`)**
  - **Paths:** `dev/inspector/plans/0.1/` and `dev/inspector/plans/0.2.0/` (part of 113 files under `dev/inspector/plans/`), plus 5 `SUPERSEDED` redirect stubs in `dev/design/db-inspector/*.md`.
  - **Evidence:** Merged in `086d5af0a` (`origin/inspector/dev/0.2.1`). `dev/inspector/{README,needs,requirements,acceptance,ROADMAP,BACKLOG}.md` and `plans/0.2.1/` are live context for `0.8.31` Slice 50; `0.1/` and `0.2.0/` are completed history and `dev/design/db-inspector/*` are pure redirect stubs.
  - **Action:** `keep` in `0.8.31`; `archive` `dev/inspector/plans/{0.1,0.2.0}/` and evaluate deleting `dev/design/db-inspector/` stubs (checking `dev/design/document-lifecycle.json` + inbound links) in `0.8.33`.

- **`P2-09` — `scripts/repo-prune/` mini-project (`archive`, target `0.8.33` Slice 10)**
  - **Paths:** `scripts/repo-prune/` (24 tracked files from 2026-06-26: `prompts/`, `runs/`, `measurements/`, `bin/`, `tests/`, `backups/`).
  - **Evidence:** Still exercised by `scripts/tests/test_md_generators.sh` (lines 48–134) and listed in `scripts/shellcheck-sc2312-ratchet.txt` and `scripts/shell-early-consumer-ratchet.txt`. Removing it in `0.8.31` would fail `agent-test.sh` unless `test_md_generators.sh` and ratchets are updated simultaneously.
  - **Action:** `keep` in `0.8.31`; `archive` (alongside removing its `test_md_generators.sh` / ratchet coupling) after `0.8.33` executes the `dev/` declutter.

- **`P2-10` — `scripts/slice135_*` (171 files) & `scripts/test-slice30-*` (5 scripts) (`keep` in `0.8.31`; `archive`/`delete` in `0.8.33` or `0.8.35`)**
  - **Paths:** `scripts/slice135_*` (90 top-level files), `scripts/tests/test_slice135_*` (81 files), `scripts/test-slice30-{d-control,g-cli,g-control,m-cli,m-control}.sh`.
  - **Evidence:** Actively invoked by `scripts/agent-test.sh` (`heavy` tier: lines 746–774, 842–850) and referenced in `.github/workflows/ci.yml` (lines 241–242). Deleting without updating `agent-test.sh` / `ci.yml` breaks `agent-verify.sh`.
  - **Action:** `keep` in `0.8.31`; audit in `0.8.33`/`0.8.35` to `archive` or `delete` superseded qualification scripts alongside `agent-test.sh` cleanup.

- **`P2-11` — `scripts/sbom-survey/` (30 files), `scripts/perf-experiments/` (33 files), `scripts/gcp-lab/` (3 files) (`keep` in `0.8.31`; review `sbom-survey/` in `0.8.33`)**
  - **Paths:** `scripts/sbom-survey/`, `scripts/perf-experiments/`, `scripts/gcp-lab/`.
  - **Evidence:** `scripts/perf-experiments/` and `scripts/gcp-lab/` are wired into `agent-test.sh` (`test-gcp-cuda-lab`, `test-gcp-lab-remote-driver`, `tests/experiments/test_perf_gauntlet*.py`). `scripts/sbom-survey/` is an un-wired 0.8.20 dependency-survey tool (`dev/design/0.8.20-slice-31-sbom-survey-tool.md`).
  - **Action:** `keep` `perf-experiments/` and `gcp-lab/`; review `sbom-survey/` in `0.8.33` (`keep` if still used for library sweeps, else `archive`).

- **`P2-12` — Empty `test/` placeholders (5 files), unwired `tests/experiments/` (75 files), `experiments/` (686), `dev/experiments/` (544) (`delete` / `archive`, target `0.8.33` / `0.8.35`)**
  - **Paths:** `test/{README.md,cross-language/README.md,fixtures/README.md,performance/README.md,smoke/README.md}`; `tests/experiments/test_*.py` (85 files total, 75 not invoked by `scripts/agent-test.sh`); `experiments/`; `dev/experiments/`.
  - **Evidence:** `test/` holds only 5 empty-directory READMEs with no tests and zero code references. Only 10 of 85 `tests/experiments/test_*.py` files are wired in `scripts/agent-test.sh` (lines 824–834). `0.8.31` Slice 60 actively uses `experiments/sdk_benchmark.py` (R31-07), and `0.8.33` adds runnable CI recipes (`R33-06`).
  - **Action:** `keep` in `0.8.31`; in `0.8.33`, `delete` `test/` if not adopted for `R33-06` recipe harnesses, and `archive` unwired historical `tests/experiments/` + completed `dev/experiments/` campaigns across `0.8.33`/`0.8.35`.

- **`P2-13` — Inspector CI/release tooling coupling (`src/inspector/tools/local-ci.sh`, `src/inspector/tools/local-release.py`) (`deprecate-in-place`, target `0.8.31` Slice 50)**
  - **Paths:** `src/inspector/tools/local-ci.sh`, `src/inspector/tools/local-release.py`.
  - **Evidence:** `0.8.31` R31-08 specifically replaces full-monorepo default execution in `local-ci.sh` with an Inspector-scoped source gate when shared core crates are unchanged, and integrates Inspector build/release artifacts into the main FathomDB release pipeline.
  - **Action:** `deprecate-in-place` (update in `0.8.31` Slice 50 per `plan.md`).

- **`P2-14` — Authoritative ledgers, ADRs, and interface contracts (`keep`)**
  - **Paths:** `dev/steward/steward-ledger.jsonl`, `dev/todos-and-considerations-ledger.jsonl`, `dev/agent-tools/{ledgerwatch,ledgerwrite}/`, `dev/adr/` (103 files, including `ADR-0.8.31-*`), `dev/interfaces/` (8 files), `dev/design/document-lifecycle.json` (236 documents, passing `check-design-lifecycle.py`).
  - **Evidence:** Load-bearing authoritative records and active CI validator inputs.
  - **Action:** `keep` permanently (never delete or hand-rewrite).

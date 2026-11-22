---
title: Slice 0 — Preflight, Inspector 0.2.1 merge, bootstrap fix, ADRs, W12 review and 0.8.30 baseline
status: ACTIVE
target_release: 0.8.31
slice: 0
date: 2026-11-22
---

# Slice 0 — Preflight, Inspector 0.2.1 merge, bootstrap fix, ADRs, W12 review and 0.8.30 baseline · `[design-adr]` · depends-on: `0.8.30 (CLOSED)`

> Instantiated from `dev/plans/prompts/0.8.0-SLICE-TEMPLATE.md`.

## 0. Worktree and branch

- Release worktree: `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- Branch: `release/0.8.31` cut off `origin/main` (`6795dc902`)
- Canonical `/home/coreyt/projects/fathomdb` remains read-only.

## 1. Required reading

1. `AGENTS.md` — TDD, no mocked DBs, no `--no-verify`, worktree isolation.
2. `dev/plans/0.8.31/scope.md` and `dev/plans/0.8.31/plan.md` — authoritative
   `0.8.31` scope, settled HITL decisions, and slice ladder.
3. `dev/notes/fathomdb-quality-testing-roadmap.md` and
   `dev/design/overflow-chunking-reevaluation.md`.
4. `scripts/lib/agent-python-env.sh`, `scripts/tests/test_select_python_for_venv.sh`,
   `scripts/tests/test_dev_environment_tools.sh`, `scripts/bootstrap.sh`.

## 2. Mandate

Deliver all prerequisites required before Slice 10 opens:

1. Merge completed `origin/inspector/dev/0.2.1`
   (`d6501cc5ac9b4fd1747be28f1896f7f7670ff39a`) into `release/0.8.31`.
2. Fix worktree Python/TS bootstrap (`select_python_for_venv` prefers bare PATH
   Python >=3.11 over `uv` standalone Python; `test_dev_environment_tools.sh`
   preflights checkout-owned `src/ts/node_modules`) via strict RED→GREEN TDD,
   and bootstrap `release/0.8.31`.
3. Activate `dev/plans/release-state-0.8.31.json`,
   `dev/plans/runs/STATUS-0.8.31.md`, and `dev/plans/0.8.31/plan.md`
   (`status: ACTIVE`) passing `release-current.py` and
   `check-release-state-views.sh`.
4. Author and index the four Slice 0 ADRs (`ADR-0.8.31-*`) plus the W12
   checkpoint-fence review (`w12-fence-review.md`).
5. Capture the `v0.8.30` baseline latency measurements and a closed golden
   schema-37 fixture database for Slice 40 (`37->38` upgrade helper), Slice 50
   (Inspector), and Slice 60 (paired latency gate).

**Deferred / out of scope:** All schema-38 engine/SDK code changes (Slice 10+),
`local-ci.sh` decoupling and harness timeouts (Slice 50), caller scope and
read+write MCP (`0.8.32`), CUDA 13 (deferred).

## 3. Self-check and Definition of Done

- `bash scripts/tests/test_select_python_for_venv.sh` PASS (including Arm D & Arm G)
- `bash scripts/tests/test_create_venv_with_selected_python.sh` PASS
- `bash scripts/tests/test_dev_environment_tools.sh` PASS in bootstrapped `release-0.8.31`
- `python3 scripts/release-current.py` resolves `0.8.31`
- `./scripts/check-release-state-views.sh` PASS
- `./scripts/agent-lint-md.sh` PASS (0 errors)
- `v0.8.30` baseline latency JSON and closed golden schema-37 DB with SHA-256
  manifest captured before schema 38 begins
- Independent `slice-reviewer` PASS on Slice 0 diff.

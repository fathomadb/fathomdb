---
title: Slice 0 status — preflight, Inspector 0.2.1 merge, bootstrap fix, ADRs, W12 review and 0.8.30 baseline
status: COMPLETE
target_release: 0.8.31
slice: 0
date: 2026-11-22
---

# Slice 0 status

## Deliverables completed

1. **Inspector `0.2.1` merged cleanly into `release/0.8.31`**:
   - Merge commit `086d5af0a` (`origin/inspector/dev/0.2.1` @
     `d6501cc5ac9b4fd1747be28f1896f7f7670ff39a`), zero conflicts.
2. **Worktree Python/TS bootstrap fix (RED→GREEN TDD)**:
   - RED commit `20269cee4` (`scripts/tests/test_select_python_for_venv.sh` Arm D
     & Arm G + `scripts/tests/test_dev_environment_tools.sh` `src/ts/node_modules`
     preflight).
   - GREEN commit `db271323f` (`scripts/lib/agent-python-env.sh` prefers bare
     PATH Python `>=3.11` over `uv` standalone Python; verified
     `HOST_SELECTED=/usr/bin/python3.12` on Ubuntu 24.04).
   - Bootstrapped `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
     with owned `.venv` (CPython 3.12), noneditable `fathomdb` wheel,
     `node_modules`, and `src/ts/node_modules`; `test_dev_environment_tools.sh`
     and `pyright` (0/0) passed.
3. **Slice 0 ADRs (`#73`–`#76`) and W12 checkpoint-fence review**:
   - `dev/adr/ADR-0.8.31-property-graph-inline-and-declared-indexes.md`
   - `dev/adr/ADR-0.8.31-strict-endpoint-integrity.md`
   - `dev/adr/ADR-0.8.31-historical-graph-and-fused-seeding.md`
   - `dev/adr/ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md`
   - `dev/plans/0.8.31/features/slice-0/w12-fence-review.md`
   - Indexed in `dev/adr/ADR-0.6.0-decision-index.md`.
4. **Release tracking activated**:
   - `dev/plans/release-state-0.8.31.json`, `dev/plans/runs/STATUS-0.8.31.md`,
     and `dev/plans/0.8.31/plan.md` (`status: ACTIVE`).
5. **`v0.8.30` baseline latency & golden schema-37 fixture DB captured**:
   - Golden DB: `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite`
     (`634,880` bytes, SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`),
     manifest at `dev/plans/0.8.31/features/slice-0/golden-0.8.30-manifest.json`.
   - Baseline latency (5k nodes, 7.5k edges, 100 queries):
     `dev/plans/0.8.31/features/slice-0/baseline-0.8.30-latency.json`
     (`search_text_only` p50 `13.80 ms`, `search_hybrid_cpu_no_embedder` p50
     `6.32 ms`, `graph_expand_2hop` p50 `0.70 ms`, `write_node_batch_200` p50
     `39.38 ms`, `write_edge_batch_200` p50 `19.86 ms`).

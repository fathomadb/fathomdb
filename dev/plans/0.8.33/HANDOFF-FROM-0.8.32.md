---
title: FathomDB 0.8.33 Handoff from 0.8.32
status: ACTIVE
target_release: 0.8.33
date: 2026-11-28
---

# FathomDB `0.8.33` Handoff (from `0.8.32`)

Read [AGENTS.md](../../../AGENTS.md), [dev/agent-memory/README.md](../../agent-memory/README.md),
and [dev/plans/0.8.33/scope.md](./scope.md) before starting Slice 0.

## Handoff checklist (`HANDOFF-01` – `HANDOFF-08`)

| ID | Topic | Actionable guidance | HITL? |
| --- | --- | --- | --- |
| `HANDOFF-01` | `0.8.32` published baseline & Schema 39 golden archive | Start `0.8.33` from `origin/main` (`v0.8.32` published on `06abc24c6`, closed at `88435cd8a` across crates.io incl. `fathomdb-mcp`, PyPI `fathomdb`/`fathomdb-cli`/`fathomdb-mcp`, npm `latest`, GitHub Releases `27` assets incl. Inspector `0.3.0`, and Tegra Pages run `46094326695` with `13` retained artifacts). Read-only golden Schema-39 archive lives at `/mnt/data/worktrees/fathomdb-0.8.32-golden/golden-0.8.32-schema39.sqlite` (`1,200,128 B`, `sha256: e232232e2c669a53400bdd78e655efad16c3d92b2e6305ba6d6431d41e3ffaa8`). | No |
| `HANDOFF-02` | `0.8.32` 100k latency & storage floor | Hold or beat `0.8.32` 100k-node baselines ([paired-latency-report.json](../0.8.32/features/slice-60/paired-latency-report.json)): `process_open` `224.28 ms`, `warm_reopen` `218.94 ms`, `write_node_batch_200` `29.41 ms`, `write_edge_batch_200` `19.40 ms`, `search_text_only` `79.02 ms`, `search_hybrid` `79.19 ms`, `graph_expand_2hop` `1.48 ms`, `database_bytes` `+6.25%` vs. `0.8.31`, `peak_rss` `-1.86%` vs. `0.8.31`, `Gate1-D10` `10k` `search_with_evidence` `10.59–10.99 ms` p50. | No |
| `HANDOFF-03` | Audit `fathomdb` npm Trusted Publisher in Slice 0 | Check `https://www.npmjs.com/package/fathomdb/access` to reconcile why `release.yml`'s `promote-npm-latest` hit `E403` (`46080698565`) while `promote-npm-release.yml` (`46088912921`) succeeded via OIDC, so `release.yml` completes 53/53 on the first attempt. | **Yes** *(npm web UI)* |
| `HANDOFF-04` | 2-core constrained concurrency stress before tagging | Run `bash scripts/check-low-core-concurrency.sh` (`taskset -c 0,1`) during qualification (and on any slice touching WAL/frozen-read/concurrent-reader state) so 2-core hosted-CI preemption races surface locally before tagging. | No |
| `HANDOFF-05` | New-package pre-flight & Windows artifact globs | Before tagging any release that introduces a new crate/wheel/npm package, verify crates.io crate existence (`0.0.0-reserve` + yank if brand-new) + OIDC binding to `release.yml`, PyPI Pending Publisher (`release.yml`, `pypi`), and workspace-relative forward-slash paths in `upload-artifact-retry`. | **Yes** *(if new package)* |
| `HANDOFF-06` | Tegra Pages publication timing & SHA deduplication guard | Gate `jetson-tegra-cuda-evidence.yml` `publish_to_pages=true` on green `ci.yml` + `release.yml` `all-builds-passed` (use `publish_to_pages=false` if running Jetson early). Never deploy `docs-pages.yml` and `jetson-tegra-cuda-evidence.yml` from the same `GITHUB_SHA`; `scripts/release/check-tegra-pages-route.sh <ver> fathomadb/fathomdb <sha>` now fails closed on already-deployed `github-pages` SHAs. | No |
| `HANDOFF-07` | CI baseline materialization & worktree hygiene | `.github/workflows/ci.yml` now invokes `scripts/materialize-release-baseline.sh`, which materializes `active_ref` (`refs/heads/dev/<ver>` or `refs/heads/release/<ver>`) from `origin/` so `test_preflight_release_state.py` stays green on pre-tag `main` pushes. Keep `/mnt/data/worktrees/release-0.8.33` isolated with its own `.venv` and never mutate `/home/coreyt/projects/fathomdb`. | No |
| `HANDOFF-08` | Deferred `0.8.33` scope items | Rust `1.95.0 -> 1.100.0` MSRV/toolchain upgrade (isolated early slice) and reviewing/relaxing `0.8.32`'s single-agent-per-database default guidance alongside `0.8.33`'s public OSS hygiene, auto-docs, architecture diagrams, and runnable recipes (`dev/plans/0.8.33/scope.md`). | **Yes** *(Slice 9 Gate 1)* |

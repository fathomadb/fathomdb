---
title: 0.8.31 Prework Slice 0 — Environment, infrastructure and release scaffold
status: COMPLETE
target_release: 0.8.31
slice: 0
date: 2026-11-22
---

# 0.8.31 Prework Slice 0 — Environment, infrastructure and release scaffold

## Release parameters

- **Project:** FathomDB
- **Development branch:** `release/0.8.31` (cut from `origin/main` @ `6795dc902`)
- **Development worktree:** `/home/coreyt/projects/fathomdb-worktrees/release-0.8.31`
- **Durable plan directory:** `dev/plans/0.8.31/`
- **Active harness + model pairing:** Pairing 1 · Antigravity + Gemini

## Scaffold and baseline status

- Completed `origin/inspector/dev/0.2.1` (`d6501cc5ac9b4fd1747be28f1896f7f7670ff39a`)
  merged cleanly at `086d5af0a` (zero conflicts; `Cargo.lock` `+17/-0`).
- Canonical cross-harness release planning template
  (`dev/plans/prompts/RELEASE-AND-SLICE-PLANNING-TEMPLATE.md`) and legacy
  Orchestrator/Steward deprecations committed at `6ee16b9e8`.
- Four accepted Slice 0 ADRs (`ADR-0.8.31-*`), W12 checkpoint-fence review,
  golden schema-37 fixture DB
  (`/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite`,
  SHA-256 `59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`),
  and `v0.8.30` baseline latency JSON committed at `ede350976`.

## Environment and infrastructure inventory

| Area | Observed host state (`2026-11-22`) | Notes / decisions |
| --- | --- | --- |
| Python | `/usr/bin/python3.12` (Python 3.12.3); `uv` at `/home/coreyt/.local/bin/uv`; owned `.venv` provisioned with `ruff 0.15.17`, `pyright 1.1.410`, `pytest 9.2.0`, `hypothesis 6.170.17`, `maturin 1.16.0` | Fixed `select_python_for_venv` (`db271323f`) to prefer bare PATH Python `>=3.11` over `uv` standalone Python 3.13; `pyright` reports 0 errors / 0 warnings. |
| Node / TypeScript | Node `v26.8.2`, npm `12.0.2`; root `node_modules` (`markdownlint-cli2 0.24.0`, `js-yaml`) and `src/ts/node_modules` (`typescript`, `@napi-rs/cli`) provisioned | `test_dev_environment_tools.sh` now preflights `src/ts/node_modules` upfront (`20269cee4`). |
| Rust | `cargo` at `/home/coreyt/.cargo/bin/cargo`; 13 workspace members (12 core crates + `fathomdb-inspector 0.2.1`) | Owned noneditable wheel builds cleanly (`release` profile ~57s). |
| Security & lint tools | `shellcheck 0.11.0`, `gitleaks 8.30.1`, `actionlint 1.7.12` installed and passing | Verified by `scripts/tests/test_dev_environment_tools.sh`. |
| GPUs | GPU 0 & GPU 1: NVIDIA GeForce RTX 3090 (24 GiB each, idle `2 MiB / 24576 MiB` at check); GPU 2: Quadro K620 (host display — never modify driver/primary CUDA 12.6) | Follow GPU Availability & Backoff Protocol via native `schedule` timer if busy; CUDA 13 deferred out of `0.8.31`. |
| Disk & storage | `/tmp` / NVMe: 173 GiB free (81% used) | Adequate for builds and temporary test databases; clean up temporary slice worktrees at phase close. |
| External platforms / CI | 5 generic native targets (Linux x64/ARM64, macOS x64/ARM64, Windows x64) + Jetson Orin Tegra Pages via GitHub Actions; `fathomdb-cli` PyPI binary wheels added in `0.8.31` | Release publication remains separately HITL-gated. |

---
title: 0.8.32 Prework Slice 0 — Environment, infrastructure and release scaffold
status: COMPLETE
target_release: 0.8.32
slice: 0
date: 2026-11-24
---

# 0.8.32 Prework Slice 0 — Environment, infrastructure and release scaffold

## Release parameters

- **Project:** FathomDB
- **Development branch:** `dev/0.8.32` (cut from `origin/main` @ `38a1594bff7c11b12468581da70eef55e6884373`)
- **Development worktree:** `/mnt/data/worktrees/release-0.8.32`
- **Durable plan directory:** `dev/plans/0.8.32/`
- **Active harness + model pairing:** Pairing 1 · Antigravity + Gemini

## Recorded owner HITL rulings (2026-11-24, pre-Slice 0)

1. **`fathomdb-mcp` write capability gating (`HANDOFF-03`, Option 1 approved + owner safety addenda):**
   - Disabled by default (`allow_writes: false`); require explicit per-store `allow_writes: true` plus a bound caller scope policy to enable write/checkpoint tools.
   - **Runtime startup conflict check (owner addendum):** `0.8.32` must validate and reject conflicting `fathomdb-mcp` configurations at runtime startup (fail-fast launch validation across `allow_writes`, caller scope policies, store/root definitions, and write grants).
   - **Write-safety test bar (owner addendum):** Because `fathomdb-mcp` is expected to become the primary access point for many agents, `0.8.32` must include comprehensive safety tests proving writes via `fathomdb-mcp` are safe under scope enforcement, idempotent `operation_id` replay, `expected_write_boundary` preconditions, concurrent admission/timeouts, malformed inputs, and partial/error recovery.
2. **Slice 40 disposable power-loss fault venue (`HANDOFF-03`, Option 1 approved):**
   - Local disposable Linux VM / loopback `dm-flakey` block device exercising real dropped, torn, and reordered writes and flush behavior (strictly stronger than process `SIGKILL`, no cloud dependency or host-reset risk).
3. **`fathomdb-mcp` distribution channels (`HANDOFF-06`, Option 1 approved):**
   - GitHub Releases prebuilt stdio binaries (5 native targets) + `crates.io` (`fathomdb-mcp`, flipping `publish = true`) + lockstep PyPI wheel (`fathomdb-mcp`, mirroring `fathomdb-cli` from `0.8.31`); npm wrapper deferred.
4. **Prework authorization:** Slices `0–9` approved up to **HITL Gate 1** human review.

## Scaffold and baseline status

- Worktree `/mnt/data/worktrees/release-0.8.32` checked out clean on `dev/0.8.32` at `38a1594bf` (leaving `/home/coreyt/projects/fathomdb` untouched).
- Draft release scope (`dev/plans/0.8.32/scope.md`), draft release plan (`dev/plans/0.8.32/plan.md`), and handoff (`dev/plans/0.8.32/HANDOFF-FROM-0.8.31.md`) present on `dev/0.8.32`.
- Golden `0.8.31` Schema-38 archive verified at `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (`984 KB`, read-only; `SCHEMA_VERSION` stays `38` for `0.8.32`).
- `0.8.31` 100k latency/storage baseline recorded at `dev/plans/0.8.31/features/slice-70/paired-latency-report.json`: `process_open` `239 ms`, `warm_reopen` `232 ms`, `write_node_batch_200` `32.8 ms`, `write_edge_batch_200` `15.5 ms`, `search_text_only` `75.5 ms`, `search_hybrid` `126.9 ms`, `graph_expand_2hop` `1.41 ms`, `database_bytes` `200.4 MB`, `peak_rss` `165.0 MB`.
- `dev/plans/release-state-0.8.32.json` and `dev/plans/runs/STATUS-0.8.32.md` are deferred to Slice 10 / post-HITL Gate 1 once the renumbered ladder is approved (no mutating product/config edits in Slices 0–9).

## Environment and infrastructure inventory

| Area | Observed host / CI state (`2026-11-24`) | Decisions / prerequisites needed (execute in Slice 10+) |
| --- | --- | --- |
| Python | `/usr/bin/python3.12` (`3.12.3`), `/home/coreyt/.local/bin/uv`, `/home/coreyt/.local/bin/python3.11`; worktree `.venv` not yet provisioned | Provision checkout-owned `.venv` in `/mnt/data/worktrees/release-0.8.32` via `bash scripts/setup-python-owned.sh --provision` at Slice 10 start (never share `.venv` across checkouts). |
| Node / TypeScript | Node `v26.8.2`, npm `12.0.2`; root `node_modules` and `src/ts/node_modules` not yet provisioned in this worktree | Provision via `npm install` (root) and `cd src/ts && npm install` at Slice 10 start. |
| Rust | `rustc 1.95.0` / `cargo 1.95.0` (active default); installed toolchains also include `nightly-2026-04-24`, `nightly-2026-08-01`, `1.99.1`, `1.100.0`; 13 workspace members | Retain `rust-version = "1.95"` default; evaluate `cargo-fuzz` installation (missing today: `cargo fuzz` not installed) with existing nightly toolchain for Slice 40 coverage-guided fuzzing. |
| Lint & security tools | `shellcheck 0.11.0`, `gitleaks`, `actionlint 1.7.12`, `lychee 0.24.2` installed | Ready for Slice 10 verification; keep `check-agents-md-paths` and `check-release-smoke-sdk-compile` green. |
| Disk & storage | `/mnt/data`: `576 GiB` free (`38%` used); `/`: `167 GiB` free (`82%` used) | Keep all release worktrees, temporary slice worktrees, fuzzing corpora, and VM images under `/mnt/data/worktrees/` (never on root SSD). |
| GPUs (`windchill3`) | GPU 0 & GPU 1: RTX 3090 (`24 GiB` each, `2 MiB` used / idle); GPU 2: Quadro K620 (`833 MiB / 2048 MiB`, driver `580.178.04`) | **Never alter NVIDIA driver or `/usr/local/cuda` (12.6).** Use `schedule` backoff protocol if GPUs are busy; spin temporary org group-3 GPU0 runners in `/tmp/` only at release time. |
| Power-loss fault venue (Slice 40) | Host kernel has `dm-flakey.ko.zst` (`7.0.0-43-generic`), `dmsetup`, `losetup`, `qemu-system-x86_64`, `virsh`, `docker`, `podman`; `dm-dust` module absent; `libvirtd` socket inactive in sandbox | Plan disposable QEMU/KVM Linux VM (preferred for zero-sudo/zero-host-mutation `dm-flakey` control inside guest) or owner-approved loopback `dm-flakey` harness; specify exact guest image/invocation before Slice 40. |
| Fuzzing toolchain (Slice 40) | `cargo-fuzz` not installed; `nightly-2026-08-01` toolchain installed | Install `cargo-fuzz` (`cargo install --locked cargo-fuzz`) or verify `libFuzzer` / sanitizer linkability in Slice 10/40 prep. |
| External registries (`fathomdb-mcp`) | PyPI `fathomdb-mcp` returns HTTP `404` (unregistered); `crates.io` `fathomdb-mcp` currently `publish = false` (unregistered) | Configure PyPI Pending Publisher (`fathomdb-mcp`) and crates.io bootstrap / Trusted Publisher early (Slice 10/30, never at tag time). |
| Jetson Orin Tegra runner | `10.83.10.13` (`jetson-fathomdb`, agentId `22`, intentionally idle) | Leave stopped until release qualification / publication; start/stop only that listener. |
| Baseline `main` CI (`45414522686` @ `38a1594bf`) | `verify`, 5-target native validation & wheel-size gates, `security`, `docs`, `shell-lint`, `windows-wal-attribution` passed; **2 jobs failed:** (1) `windows-wal-checkpoint-diagnosis` (`migrate_cli.rs` hardcoded `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite`), (2) `verify-fast` (4 suites: `test-tegra-publication-operator-path`, `test-pages-site`, `test-tegra-pages-retention`, `test-agent-typecheck-pyright-version`) | Triage and fix both CI regressions in Slice 10 (`Prework Prep`) so `main` / `dev/0.8.32` CI is fully green before feature slices land. |
| Open Dependabot alerts | `150` open alerts captured from GitHub API (`rust`: 86, `pip`: 63, `npm`: 1) | Triaged in detail in `prework/slice-1.md`. |

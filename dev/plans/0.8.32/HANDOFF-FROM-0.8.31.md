---
title: FathomDB 0.8.32 Handoff from 0.8.31 (Antigravity + Gemini)
status: ACTIVE
target_release: 0.8.32
date: 2026-11-24
---

# FathomDB `0.8.32` Handoff (from `0.8.31`)

Read [AGENTS.md](../../../AGENTS.md), [dev/agent-memory/README.md](../../agent-memory/README.md),
[dev/plans/0.8.32/plan.md](./plan.md), and [dev/plans/0.8.32/scope.md](./scope.md)
before starting Slice 0.

## Handoff checklist (`HANDOFF-01` – `HANDOFF-10`)

| ID | Topic | Actionable guidance | HITL? |
| --- | --- | --- | --- |
| `HANDOFF-01` | `0.8.31` published baseline & golden archive | Start `0.8.32` from `origin/main` (`v0.8.31` published on `8965af93d`, closed at `97580d39f` across crates.io, PyPI `fathomdb`+`fathomdb-cli`, npm `latest`, GitHub Releases incl. Inspector `0.3.0`, and Tegra Pages). Read-only golden Schema-38 archive lives at `/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.31-schema38.sqlite` (`sha256: e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036`). | No |
| `HANDOFF-02` | `0.8.31` 100k latency & storage floor | Hold or beat `0.8.31` 100k-node baselines ([paired-latency-report.json](../0.8.31/features/slice-70/paired-latency-report.json)): `process_open` `239 ms`, `warm_reopen` `232 ms`, `write_node_batch_200` `32.8 ms`, `write_edge_batch_200` `15.5 ms`, `search_text_only` `75.5 ms`, `search_hybrid` `126.9 ms`, `graph_expand_2hop` `1.41 ms`, `database_bytes` `200.4 MB`, `peak_rss` `165.0 MB`. Watch DB-size headroom closely (`0.8.31` was `+13.09%` vs. `0.8.30`). | No |
| `HANDOFF-03` | Resolve `0.8.32` open HITL gates first | Before starting Slice 0, present the two open HITL decisions in [plan.md](./plan.md) (`fathomdb-mcp` write capability gating and Slice 40 power-loss fault venue) **clearly and concisely, with concrete numbered options plus a recommendation and justification** for each. | **Yes** |
| `HANDOFF-04` | Non-breaking Schema-38 `scope` activation | Activate the already-reserved Schema-38 `scope TEXT NOT NULL DEFAULT ''` column and reserved `vec0` filter slot **without bumping `SCHEMA_VERSION`** — existing `0.8.31` stores must open cleanly, and single-scope (`scope == ""`) workloads must take a zero-overhead fast path. | No |
| `HANDOFF-05` | Strict Slice 10 refactor-before-scope order | Land `fathomdb-engine/src/` test relocation, `search.rs`/`evidence.rs` submodule splits, and single-source `TABLE_MANIFEST` as zero-behavior-change commits **before** threading `scope` across read, search, graph, frozen, write, actuate, and erasure paths. | No |
| `HANDOFF-06` | Early `fathomdb-mcp` packaging & publishers | Decide `fathomdb-mcp` distribution channels (GitHub Releases vs. crates.io/PyPI/npm) and configure any new Trusted/Pending Publishers plus `set-version.sh`, `check-license-consistency.sh`, and `release.yml` wiring in Slice 0/30 rather than at tag time. | **Yes** |
| `HANDOFF-07` | Worktree, venv, hygiene & boundary discipline | Cut `/mnt/data/worktrees/release-0.8.32` off `origin/main` with its own `.venv` (`bash scripts/setup-python-owned.sh --provision`); do not mutate `/home/coreyt/projects/fathomdb`; triage GitHub Dependabot alerts (150 alerts as of `0.8.31` close) in Slice 0 prework; never write external consumer-project specifics into tracked FathomDB files. | No |
| `HANDOFF-08` | Local GPU, Orin & runner operating map | Keep `windchill3` NVIDIA driver and `/usr/local/cuda` (12.6) untouched; spin temporary group-3 GPU0 runners (`GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`) out of `/tmp/` and deregister immediately after `build-cuda-linux-x64-gnu`; reach Jetson Orin at `10.83.10.13` (`jetson-fathomdb` agentId `22`, leaving unrelated runner services untouched). | No |
| `HANDOFF-09` | Antigravity delegation, watchdog & review discipline | Use `define_subagent` (`slice-implementer`, `slice-reviewer`) with one writer per worktree, mandatory independent adversarial review on every plan and slice diff, act first within agent-appropriate scope (`AGENTS.md` § 7/§ 11), and use `schedule` (`TimerCondition`) with ≤15-min progress/ETA updates instead of tight polling or silent waits. | No |
| `HANDOFF-10` | Pre-tag & post-publish verification gates | Wait for cross-OS `ci.yml` on `main` to finish before pushing `v0.8.32`, use `scripts/release/configure-tag-cuda-route.sh` (never raw group-3 `PATCH`), never run `git fetch --depth=1` in a full checkout, and directly verify live registry installs, GitHub Release assets, and Docs Pages URLs before closing Slice 60. | **Yes** *(final `v0.8.32` tag & publication approval)* |

## Open HITL Decisions Template (`HANDOFF-03`)

When opening the `0.8.32` session, present these two items concisely with
options + recommendation + justification before executing Slice 0:

1. **`fathomdb-mcp` write capability gating:**
   - *Option A (Recommended):* Disabled by default (`allow_writes: false`); require explicit per-store `allow_writes: true` plus a bound caller scope policy to enable write/checkpoint tools. *Justification:* Prevents accidental mutations from read-only agent configs while making write-enabled profiles explicit and auditable.
   - *Option B:* Enabled by default whenever a caller scope grants write permission.
2. **Slice 40 disposable power-loss fault venue:**
   - *Option A (Recommended):* Local disposable Linux VM / loopback `dm-flakey` (or `dm-dust`) block device exercising real dropped/torn/reordered writes. *Justification:* Reproducible locally without external cloud dependencies and strictly stronger than process `SIGKILL`.
   - *Option B:* Userspace FUSE/interposed block fault simulator in CI containers.

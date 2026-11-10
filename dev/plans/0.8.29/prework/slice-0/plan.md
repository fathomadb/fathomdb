---
title: 0.8.29 Slice 0 — environment and project infrastructure proposal
status: PROPOSED
target_release: 0.8.29
slice: 0
---

# Slice 0 — environment and project infrastructure proposal

## Goal, authority and maturity

Propose a reproducible, isolated release workspace and bounded platform/resource
inventory for Slice 9 to decide and Slice 10 to prepare. This prework changes no
Git state, tools, environments, runners, dependencies or product contracts.
The [planning brief](../../planning-brief.md) controls proposal-only sequencing;
the [release plan](../../../plan-0.8.29.md) now places state/board activation in
approved Slice 10 preparation after Slice 9 decisions.
Existing MCP, queue, Block 1 and format-37 source remain the integrated baseline.

## Current observations and inspected sources

Read-only local observations on November 9, 2026; availability can change.
No product or preparation test campaign was run for this draft.

| Area | Observed evidence | Proposal consequence |
| --- | --- | --- |
| Git | `llm/0.8.29-chat` tracks `origin/dev/0.8.29`; both at `6edc1155f039f87cd0a37660cb27ae4373bbce71`; completed .28 `f4343524198f15cc78ec006ea4e1a4e48cf9f04e` is an ancestor | Preserve the integrated .29 tip; recheck refs before any execution |
| Ownership | Separate worktree already holds `dev/0.8.29`; current checkout contains parent-owned planning changes | Establish ownership before reuse; no branch switch in this drafting checkout |
| Versions | Workspace W, Python and TS packages are 0.8.28; embedder-api E is independently 0.8.0 | Allocate coordinated W bump to 0.8.29; do not advance E for lockstep |
| Rust | `rust-toolchain.toml` pins 1.95.0; manifest minimum 1.95; active rustc/cargo 1.95.0 | Retain reviewed Rust; confirm rustfmt/clippy in target workspace |
| Node/npm | `.nvmrc` 25.9.0, TS `>=25 <26`, packageManager npm11.12.1; active Node26.8.2/npm12.0.2 | Propose command-local reviewed tools; no global tool replacement |
| Python/native | System Python3.12.3; no checkout `.venv` | Provision owned venv/tools/native artifacts only after approval |
| npm ownership | Root `node_modules` symlinks to primary checkout; TS `node_modules` absent | Propose checkout-owned installs from locks; no mutation through shared symlink |
| Host/resources | `windchill3`, Linux7.0.0-42 x86_64, Threadripper5945WX/24 logical CPUs; RAM62GiB, available21GiB; swap7.9/8GiB used | Bound build concurrency and remeasure pressure before gates |
| Disk | Checkout and `/tmp`: 18GiB free,98% used; `/mnt/data`:914GiB free,2% used | Choose an approved owned build/scratch directory and disk budget before builds |
| CUDA | Primary resolves `/usr/local/cuda-12.6`; nvcc12.6.68; `nvidia-smi` cannot communicate with driver in this context | GPU availability/driver version unverified here; no driver repair or upgrade inference |
| Data/state | `release-current.py` exits successfully with empty output; current `data/0.8.26` contains evidence READMEs; primary cold-data symlink targets unavailable here | Resolve actual corpus/model paths and digests before measurement; activate state only in 10 |

Inspected [reconciliation](../../reconciliation-plan.md),
[forward integration](../../forward-integration.md),
[Slice95 status](../../../0.8.28/features/slice-95/status.md) and
[qualification matrix](../../../0.8.28/features/slice-95/qualification.md).
Tool contracts: [Cargo](../../../../../Cargo.toml),
[Python](../../../../../src/python/pyproject.toml),
[TS](../../../../../src/ts/package.json), [bootstrap](../../../../../scripts/bootstrap.sh),
[preflight](../../../../../scripts/preflight.sh),
[verification](../../../../../scripts/agent-verify.sh), and
[bindings](../../../../design/bindings.md). Memory index and verified entries
`windchill3-never-change-driver-or-primary-cuda.md`,
`agent-worktree-stale-base-trap.md`, `session-must-enter-own-worktree.md`,
`windows-vm-for-fathomdb-testing.md` and
`0.8.28-slice42-sqlite-bump-lessons.md` informed isolation and host constraints.

## Draft needs, requirements and acceptance

Release-local proposal IDs only; canonical acceptance is unchanged.

- N29-0-1: contributors need reproducible tools and native bindings without disturbing other sessions.
- N29-0-2: reviewers need a truthful platform/resource inventory before allocating qualification work.
- R29-0-1 / AC29-0-1: list each version axis, declared pin, observed version and proposed retain/prepare/defer action; every mismatch has an owner and decision destination.
- R29-0-2 / AC29-0-2: propose exact base-ref verification, checkout ownership, branch/worktree creation, session switch and native-artifact identity checks; reject stale or borrowed environments.
- R29-0-3 / AC29-0-3: distinguish documented venues, current observations and unmeasured availability; every selected platform has a route, feature/artifact scope and unmet prerequisite.
- R29-0-4 / AC29-0-4: propose disk, memory, GPU and corpus budgets with stop thresholds, isolated paths and preservation exclusions; unknown digests stay explicitly unknown.
- R29-0-5 / AC29-0-5: provide a decision-bound Slice10 preparation order that resolves state discovery before dependent worktree preflight; this draft contains no executed-preparation claim.

## Architecture/design preparation and platform inventory

Keep one writer per checkout and checkout-owned venv/native outputs. Separate
shared immutable download/model caches from mutable installs, build outputs and
receipts; cache reuse needs exact inputs, never borrowed binding ownership.
Python package floor remains 3.10; developer interpreter must satisfy current
bootstrap/scripts, preferably >=3.11; retain Ruff 0.15.17/Pyright 1.1.410.
Propose noneditable wheel installation into the owned venv for linked worktrees;
resolve bootstrap's editable-install guidance against ownership rules before use.

- Browser applicability: the published JS binding targets Node native addons; propose Chrome/Firefox checks only for a selected documentation or tooling flow, and explicitly record other browser cells as not applicable rather than adding a product browser matrix.
- CPU native matrix: Linux x64/ARM64, macOS x64/ARM64 and Windows x64; use existing [CI](../../../../../.github/workflows/ci.yml) and [release](../../../../../.github/workflows/release.yml) routes, without dispatch or publication in prework.
- Recorded self-hosted venues: GPU 168/group 3 on windchill3, Windows 31 on `MEMEX-AFE8V4I00` using the existing `gh-runner-wonl-win11` VM, and Jetson 22 on `ubuntu`. Slice 95 records identity run 42726255745 success; that establishes historical delivery, not present access or qualification.
- [Runner-readiness workflow](../../../../../.github/workflows/runner-readiness.yml) is main-only, identity-only and three-minute bounded; do not repurpose it as .29 candidate qualification.
- Preserve standing GPU installation `/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`; use a separately owned path if `/mnt/data` is selected. No runner/configuration cleanup is implicit.
- GPU contract: R580 and primary 12.6 fixed; RTX 3090s are documented venues, never K620. Map selected GPU UUID/ordinal before future `cuda:0` work. No reset/fault injection belongs in this slice.
- MCP CUDA build proposal uses existing `fathomdb-mcp --features embed-cuda`; package remains `publish=false`. Distinguish compiled binary, runtime/provider smoke and installed-consumer evidence.
- Keep classic Tegra and genuine non-Tegra ARM driverless cells separate. CUDA13 library qualification does not establish a second product artifact; Q5 Tegra CUDA 13 remains unmeasured.

Read [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
apply truthful skips, artifact identity, lifecycle/resource checks and bounded
wrong/missing-tool controls; defer new broad fuzz/exhaustive environment matrices.
The [CUDA procedure](../../../../design/cuda-version-support-review.md) and
[accepted policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md) constrain
host/artifact choices. Fresh quantitative adoption research remains incomplete;
propose primary NVIDIA/JetPack/tool/runtime data collection in 120, with accessible
raw sources and owner ratification, rather than inventing fresh shares here.

## Proposed tasks, dependencies and deliverables

1. Produce `environment-register.md`: commands, source pins, observed outputs/limits, owners and proposed actions; reconcile version/tool conflicts with Slice 1.
2. Produce `requirements.md` and `design.md` drafts mapping the local IDs to workspace ownership, platform routes, resource budgets and artifact/corpus identity checks.
3. Produce `preparation-proposal.md`: approve ownership of an existing isolated .29 checkout for initial state/view bootstrap; then require strict preflight before any new worktree spawn. Propose `git worktree add -b llm/0.8.29-preparation <approved-path> <verified-.29-sha>` when a new workspace is selected, verify its base and switch the execution session cwd into it; provision approved owned tools/artifacts and validate scoped setup.
4. Produce `platform-resource-register.md`: required versus conditional CPU/GPU/Tegra/Windows cells, availability checks, exact data/model digests still needed, resource reservations and deferred venues.
5. Submit proposals and disputed bootstrap/install wording to 9; independent review checks feasibility and failure cases before 10 executes any approved subset.

Hard edges: this proposal requires only current evidence;9 requires 0–8 outputs;
10 requires recorded 9 decisions. State discovery must succeed before future
`preflight --worktree/--expect-closed` can pass. Conditional edges: approved
cleanup from 2 may supply disk; 1 supplies pin decisions; 7 supplies build/CI cost
recommendations. Coordinate GPU/VM/Jetson reservations with 30/100/110/120/130;
serialize mutable environment and release-state writes. Preserve other worktrees.

## Review, validation, stop points and completion

Proposal completion means all five AC29-0 checks have inspectable mappings,
unknowns and decision owners, and relative source links resolve. Parent imports
the draft and runs scoped Markdown validation; no full product gate is due now.
Later script behavior changes require failing-test-first controls for missing
owned venv, wrong pins and stale base, followed by independent review and the
applicable final gate; mechanical version bumps retain their test exception.
Stop preparation on unknown workspace ownership, insufficient approved capacity,
unavailable corpus, borrowed native identity, failed strict preflight or changed
host CUDA invariants. Carry blocked platform cells as unmeasured to 9/160.
No new uninterrupted green claim is inherited from .28's assembled campaign.

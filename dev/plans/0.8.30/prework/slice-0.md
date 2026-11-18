---
title: Slice 0 environment and infrastructure proposals
status: COMPLETE
target_release: 0.8.30
slice: 0
date: 2026-11-18
---

# Slice 0 environment and infrastructure proposals

The requested environment investigation and minimal Git isolation are complete.
Only branch/worktree administration and read-only inventory occurred in this
slice; the setup changes below remain proposals for9/10. The
[overall plan](../plan.md) and seven feature drafts exist and passed independent
Astra-medium review after Sol6.2-high authoring; see [review](../planning-review.md).

## Actual checkout and baseline

- Dedicated linked worktree: `/tmp/fathomdb-0830-testing-review`.
  Created for this planning task, reused rather than duplicating it.
- Created and switched to `release/0.8.30`; current baseline
  `18427b7a8e6c9635282d010bc8494fdc6a579cf3` merges reviewed planning commits
  `f7fb1da9a`/`07e80e207` with observed `origin/main` `42629c2a1`.
  Main is untouched. The general `scripts/preflight.sh` passed, with clean
  tracked source and148GiB available on the worktree filesystem.
- Existing `dev/0.8.30` at `7d3637657` has its own worktree at
  `/home/coreyt/projects/fathomdb-worktrees/dev-0.8.30` and Inspector work.
  It was neither switched nor merged. Its placement and integration are a
  Slice9 scope choice; prework3/4 reviews this overlap.
- Latest main changes .29 Tegra install/docs and retained artifacts. Its
  release-state still says `CORE_PUBLISHED_TEGRA_DELIVERY_PENDING` while public
  docs advertise .29 Tegra. Treat this as a source/evidence reconciliation
  item for6/7, not proof that public delivery failed or completed.

## Tooling and proposed choices

| Area | Observed state | Proposal and owner |
| --- | --- | --- |
| Rust | `rustc`/Cargo1.95.0; root toolchain1.95.0 and workspace rust-version1.95. | Keep exact toolchain and clippy/rustfmt. Revisit only for a justified dependency change in1/9/60. |
| Python | System3.12.3 available; product minimum3.10, main CI3.12; no checkout `.venv`. `python3.13` is not on PATH. | Select3.12 for developer environment in9, provision this checkout only in10 via the owned setup path. Keep public version claims separate; no borrowed venv/native modules. |
| Node | Ambient PATH chooses26.8.2; declared `.nvmrc` and current supported release route25.9.0. Pinned25.9.0 binary is present under `/home/coreyt/.nvm/versions/node/v25.9.0/bin`. | Use existing25.9.0 path for declared checks; do not qualify26 by inference. Environment switch for this read-only inventory was command-local. |
| npm | Root and TS packageManager11.12.1; ambient12.0.2; npm alongside installed25.9.0 reports11.19.0. | Select declared project npm in9/10 or explicitly justify a pin change. Keep the separately pinned production OIDC CLI purpose distinct. No npm version was installed here. |
| Local packages | Root checkout-owned `node_modules` exists from earlier Markdown tooling setup; no `.venv`. | Provision only consumed tools/native packages in10 after decisions. Tool-only/native-build split is proposed20 work; current `setup-python-owned.sh --provision` also builds a wheel. |
| Storage | `/tmp` filesystem148GiB available; `/mnt/data`290GiB. | Use `/mnt/data` for any later owned build/scratch needing capacity, with ownership and retained-evidence checks. No cache cleanup or allocation now. |

The managed `uv python list --only-installed` lookup could not acquire its cache
lock under the sandbox. This does not prove a Python version absent outside
PATH; no tool/cache permissions were changed.3.12 suffices for the proposed
current developer setup. No full verifier was run without the required venv.

## Platforms and compute

| Need | Existence/location and current evidence | Later preparation |
| --- | --- | --- |
| Ordinary CPU source work | Local `windchill3`, Linux x86_64,24logical CPUs. | Use ordinary shared host; retain load context for timing, no quiet-host or CPU-controller prerequisite. |
| Existing CUDA venue | Read-only unconfined inventory sees two RTX3090,24576MiB each, driver580.178.04. Also K6202048MiB, excluded from work. | Keep driverR580 and primaryCUDA12.6. This inventory reserves no GPU or session and qualifies no reset. Check exclusive availability only for consumed approved runs. |
| Windows | Actual `virsh list --all` finds `gh-runner-wonl-win11`, shut off. | Preserve the existing VM;10/70 select an authorized ordinary local or hosted route. No VM start, disk change or runner registration occurred. |
| Five generic targets | Current workflows declare Linuxx64, `ubuntu-24.04-arm`, `macos-15-intel`, `macos-15` and Windows runners. | Keep existing supported CPU/native jobs; inspect actual job/worker availability when selected. Labels are configuration, not current successful execution. |
| Tegra | Existing seq-398 .29 route names `10.83.10.13`; .29 records identify Orin runtime/soak and private-pool proof. | Reconcile .29 public delivery before final .30 Tegra baseline. No SSH session, new access or device mutation was attempted. GenericARM CPU is separate. |
| GPU release workers | Existing release workflow uses self-hosted Linux/X64/gpu/cuda-12 labels. | Before production, inspect actual matching workers/services and trusted exact-tag route, review/lease and bounds. No runner was provisioned or queried as ready by assumption. |
| Browsers | Selected mainline quality plan has no browser product requirement. Unmerged Inspector work may change this. |3/4/9 decide Inspector scope and browser/runtime needs before any Chrome/Firefox provisioning or new platform claim. |

Initial sandbox GPU/libvirt probes could not access their drivers/socket. The
same read-only queries succeeded with executor permission; they establish the
inventory above, not runtime test results. Hardware/tool availability does not
satisfy source, package, live-fault or publication acceptance.

## Decisions proposed for Slice 9

Keep Rust1.95.0 and current public platform boundaries as the baseline; choose
checkout-owned Python3.12 and reconcile project npm before provisioning. The
completed [library sweep](slice-1.md) subsequently flags Node25 end-of-life and
a verified26LTS candidate:9 must choose a coordinated migration and qualification
plan or explicitly retain the current25 baseline with its limitation. The installed
ambient26 binary does not itself qualify that migration. Preserve existing
compute instead of creating new venues. Decide Inspector placement explicitly.
Only10 implements approved setup;20 owns measured CI/bootstrap changes. No
accepted architecture, dependency, host or release-state setting was changed.

---
title: FathomDB 0.8.27 prework Slice 0 - environment and infrastructure
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 0 - environment and project infrastructure

## Plan and delta reconciliation

Outcome: verify the already-created release workspace and inventory every
environment or project-infrastructure decision needed by the later correction
and refactor slices. This is evidence-only: no tool, runner, credential,
environment, workflow, release state, or package is mutated.

Changes since the 2026-09-02 draft intake:

- 0.8.25 and 0.8.26 shipped; schema 34 and the fresh-database boundary are now
  the baseline.
- `release/0.8.27` and its linked worktree already exist at clean
  `origin/main` commit `2332242848d6135a1737d1abcc767c99620a9f53`.
- Main added the performance-gauntlet/graph-benchmark program and the F27-01
  roadmap intake after 0.8.26. Neither changes local build prerequisites.
- The worktree already contained a draft release plan, refactor-test approach,
  and two index edits. They are preserved as planning inputs.

The draft plan is adjusted to the observed host: Node 26.8.2/npm 12.0.2 is
installed, not the declared Node 25.9.0/npm 11.12.1 release combination. The
host combination is therefore not admissible package evidence.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-0A | The release work happens only in the linked release worktree from the intended main commit. | Branch, worktree, HEAD, `main`, and `origin/main` are recorded; general and `--landing` preflight pass. |
| PW27-0B | Development and artifact evidence cannot silently consume tools or native bindings from another checkout. | No worktree `.venv` or `node_modules` exists; later slices use checkout-owned tools and fresh non-editable artifacts. |
| PW27-0C | Every required platform or capability has an owner and honest availability state. | The matrix below identifies local, hosted, VM, accelerator, and unavailable evidence without treating absence as pass. |

## Findings and design allocation

Observed local facts:

- Linux x86_64, 24 logical CPUs, about 170 GiB free.
- Rust/Cargo 1.95.0 with rustfmt and clippy; only the x86_64 GNU target is
  installed.
- Python 3.12.3; the package remains Python >=3.10 and abi3-py310.
- Node 26.8.2/npm 12.0.2; manifests require Node >=25 <26 and npm 11.12.1.
- GCC, CMake, pkg-config, strace, actionlint, lychee, gitleaks, Docker, and
  Podman clients are present. Clang, sqlite3 CLI, and nvcc are absent.
- The NVIDIA driver was not visible from the prework sandbox. Correction
  (2026-09-26, Slice 70): the host `windchill3` has NVIDIA driver 580.173.02
  with two RTX 3090s (`cuda:0`, `cuda:1`) and a display-only Quadro K620. It
  ran `scripts/test-feature-complete.sh` with CUDA-selected legs; `nvcc` is
  still absent and was not required. Libvirt access to the Windows VM is
  unavailable in this sandbox. The VM remains an external named executor, not
  a local pass.
- `gh auth status` reports the default token invalid. Read-only public access
  may still work, but no privileged or hosted-release action may rely on it.

| Surface | Current | Required owner/allocation |
| --- | --- | --- |
| Rust/source | Host toolchain is exact; local target only | Every source slice uses a checkout-local target/build root; Slice 150 owns native target qualification. |
| Python | 3.12.3; no worktree venv | Tool-only venv may be checkout-local; native claims use a disposable wheel installed non-editably in a fresh environment. |
| Node/TypeScript | Host runtime is outside the declared range; no worktree install | Use declared Node 25.9.0/npm 11.12.1 for Slice 110/120 and artifact qualification. |
| ptrace | `strace` present | AC-036 must run unchanged on a capable executor if the sandbox denies ptrace. |
| user namespaces | Ubuntu 24.04 sets `kernel.apparmor_restrict_unprivileged_userns=1`, so `unshare -rUn` fails and the AC-037 live netns layer is unavailable by default | Run AC-037 live on an executor that permits rootless user namespaces. On `windchill3` (2026-09-26, Slice 70), a per-binary AppArmor profile granting `userns` to `/usr/bin/unshare` only was installed temporarily for that run, and strict security passed 0/0/0 with both live AC-037 layers. The profile is not a standing host configuration. |
| platforms | Linux x64 local | Slice 150 owns Linux ARM64, macOS x64/ARM64, Windows x64, and any Jetson evidence warranted by moved paths. |
| accelerators | Corrected 2026-09-26: `windchill3` has two RTX 3090s (driver 580.173.02; K620 display-only) and is the named CUDA feature-complete executor; `nvcc` absent and not required; no Metal host | Slice 70/150 use this named host for CUDA evidence where affected paths warrant it; Metal remains unavailable; driverless CPU remains mandatory. |
| credentials/network | GitHub default token invalid | No privileged operation before exact auth revalidation; dependency evidence records network limits honestly. |
| storage | 170 GiB free | Recheck before broad native/package matrices; every cache/artifact cleanup needs proven ownership. |

Browsers are not part of any assigned surface and remain N/A. The absence of a
host sqlite3 CLI is not a prerequisite because the product uses bundled SQLite.

## Implementation, review, verification, and status

Implementation is this durable inventory only. Behavioral TDD and code review
are not applicable. General preflight and linked-worktree landing preflight
both passed; branch/worktree/base/tool/platform facts were read directly.
Independent prework design review and closeout verification passed.

No temporary branch, worktree, environment, package, or artifact was created.
Next: Slice 1 dependency evidence.

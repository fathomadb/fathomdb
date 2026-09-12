---
title: 0.8.26 Slice 0 — environment and infrastructure draft notes
status: COMPLETE
observed_on: 2026-09-12
---

# Slice 0 draft design notes

## Established facts

- Branch `release/0.8.26` and worktree
  `/home/coreyt/projects/fathomdb-worktrees/release-0.8.26` were created from
  `40f807f5198cf826ef08ffd50658c8bb23f6f0f0`; subsequent commits are planning
  records only.
- The primary checkout is stale/dirty and is not a valid release workspace.
- Observed host tools: Rust/Cargo 1.95.0, Python 3.12.3, Node 25.9.0, npm
  11.19.0.
- Declared contracts: Rust 1.95, Python >=3.10/abi3-py310, Node >=25 <26,
  package manager npm 11.12.1, CI Python primarily 3.11/3.12.
- The release matrix includes Linux x86_64/aarch64, macOS x86_64/arm64, and
  Windows x86_64 native artifacts. CUDA is Linux x86_64-specific; CPU-mode
  driverless loading remains mandatory.
- No browser runtime is part of P0–P2. Chrome/Firefox testing is N/A unless
  Slice 3 identifies a browser-owned public-doc or SDK requirement.

## Decisions Slice 0 must prepare

| ID | Decision | Draft recommendation |
| --- | --- | --- |
| E26-01 | Release-bound Python environment | Use an external or cloned clean build venv bound to release sources; ignored symlinks are planning-only. |
| E26-02 | Node/npm environment | Select declared npm 11.12.1 for reproducible install/package checks; do not rewrite it to the host version. |
| E26-03 | Rust environment | Retain Rust 1.95 unless Slice 1 finds a security/toolchain driver. Install required targets/components explicitly. |
| E26-04 | Platform matrix | Run feature-local native parity on five CPU targets; use CUDA/Metal only where touched code or packaging justifies it. |
| E26-05 | Operator CLI target matrix | Slice 30 must decide whether Memex needs Linux-only CLI delivery or the full native target set. |
| E26-06 | Release-state authority | Create a 0.8.26 JSON/board only after current historical state files and generator ownership are reconciled. |
| E26-07 | Package witness | Build from release sources into isolated directories; never use editable install from the worktree. |

## Project-infrastructure inventory

Inspect CI path filters, release candidate inputs, CLI archives, package smoke
manifests, public-doc truth checks, DOC-INDEX ownership, plan/design linters,
and the generated release-state views. Known baseline concern: the current
public-doc truth test has been observed to demand a stale 0.8.23 statement even
though the README names published 0.8.25; Slice 0 records it and Slice 5 decides
the test repair allocation.

No decision in this note is accepted until Slice 8.

## Completed inventory and corrections

- `40f807f5198cf826ef08ffd50658c8bb23f6f0f0` is the planning-input
  commit, not the completed Slice 0 tip. The branch forked from the published
  0.8.25 line and its merge base with `origin/main` is `a563362d`.
- The host is Linux x86_64 with 24 logical CPUs. Only the Rust host target is
  installed. Docker, Podman, GCC, CMake, `pkg-config`, `strace`, actionlint,
  lychee, and gitleaks are present; clang, sqlite3, and nvcc are absent.
  `nvidia-smi` cannot reach a driver. Cross-target, CUDA, Metal, macOS, and
  Windows evidence must come from their named executors.
- The filesystem has about 50 GB free but is 95% used. That passes the present
  10 GB preflight threshold but requires a release-matrix disk budget and
  monitoring before package rehearsal.
- This worktree has no `.venv` or TypeScript-local `node_modules`. Its ignored
  root `node_modules` symlink targets the primary checkout and contains
  markdownlint-cli2 0.23.0 rather than the locked 0.23.2. It is not admissible
  build or verification evidence. Bootstrap must create checkout-owned tools;
  release claims must use fresh wheel/package environments.
- The current authority selector still returns 0.8.25. Its state lacks the
  top-level publication receipt and its board lacks the historical-closure
  marker expected by the selector. This also leaves public-document truth tied
  to 0.8.23. Close 0.8.25 authority before creating 0.8.26 state or board.
- GitHub authentication provenance is uncertain: `gh auth status` rejects the
  stored default token although some public and Dependabot API queries worked.
  Revalidate the exact auth source before any later privileged operation.

## Slice 8 proposals

1. Put the 0.8.25 authority correction and clean worktree bootstrap in Slice 9.
2. Allocate platform executors and fresh artifact environments to their owning
   feature or Slice 50, rather than installing all cross-targets locally.
3. Require a disk budget before full native/package rehearsal.
4. Keep browsers out of scope. No browser-owned 0.8.26 surface was found.

## Current, required, and unknown matrix

| Surface | Current | Required for implementation/release | Unknown or allocation |
| --- | --- | --- | --- |
| Python | 3.12.3; no worktree venv | checkout-owned development environment; fresh external wheel venv for artifact claims | exact cache sizing belongs to Slice 50 |
| Rust/native | 1.95 host target; shared default Cargo cache; GCC/CMake/pkg-config present; bundled SQLite path | checkout-local target/build roots, clippy/rustfmt, native linker, isolated writable advisory cache | cross-linkers/targets absent; install only in owning platform slice |
| Node/npm | Node 25.9.0, host npm 11.19.0, shared drifted root modules | declared npm 11.12.1 and clean checkout-owned install | npm cache is user-shared; isolate or prove content-addressed use before package evidence |
| ptrace | `strace` installed; repository records sandbox denial and unchanged unconfined success | unchanged strict AC-036 route on a ptrace-capable executor | current sandbox is not accepted as capability proof |
| containers | Docker and Podman clients present | usable socket/runtime only where package or cross-build plan requires it | execution/socket viability not established; test in owning slice |
| accelerators | no usable NVIDIA driver/nvcc; no Metal host | driverless CPU locally; CUDA/Metal only on registered capable hosts | model/cache capacity and runner availability remain Slice 50 inputs |
| CI/platform | Linux host only; platform manifest names five native CPU targets | named hosted/self-hosted executors and exact artifact matrix | Windows/macOS/CUDA availability must be revalidated before scheduling |
| network/credentials | registry reads succeeded selectively; default gh token status is invalid | isolated read caches for sweeps; explicit auth validation before privileged operations | exact credential source and write scopes intentionally unresolved |
| storage | about 50 GB free, filesystem 95% used | preflight minimum plus per-matrix budget and cleanup ownership | package/model/cache peak size must be measured before Slice 50 |

Project infrastructure must keep bundled SQLite and the canonical linker path;
the absence of a host `sqlite3` CLI is not a product prerequisite. Package
caches may accelerate work but cannot substitute for a clean install witness.

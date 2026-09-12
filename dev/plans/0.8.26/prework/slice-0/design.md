---
title: 0.8.26 Slice 0 — environment and infrastructure draft notes
status: DRAFT
observed_on: 2026-09-12
---

# Slice 0 draft design notes

## Established facts

- Branch `release/0.8.26` and worktree
  `/home/coreyt/projects/fathomdb-worktrees/release-0.8.26` exist at
  `40f807f5198cf826ef08ffd50658c8bb23f6f0f0`.
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

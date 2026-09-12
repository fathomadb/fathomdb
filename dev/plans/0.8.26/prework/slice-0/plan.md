---
title: 0.8.26 Slice 0 — environment and project-infrastructure prework
status: DRAFT
target_release: 0.8.26
---

# Slice 0 plan

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is evidence-only after the already requested workspace setup: reconcile
draft deltas, complete and review its findings, verify the records, write
status, and mark behavioral TDD/code review not applicable unless scope changes.

## Outcome

Establish the isolated release workspace and produce a durable, read-only
inventory of environment and project-infrastructure decisions required before
0.8.26 work. Branch/worktree creation and draft planning records are the only
authorized setup actions.

## Work

1. Verify `release/0.8.26` and its worktree are based on the intended clean
   commit and that the primary checkout remains untouched.
2. Inventory Python versions, supported floor, native wheel ABI, venv ownership,
   tooling pins, and a worktree-safe build/test arrangement.
3. Inventory Rust version/targets/components, Cargo caches, native libraries,
   cross-compilation, ptrace, SQLite, and linker requirements.
4. Inventory Node/npm/TypeScript versions, N-API targets, package-manager pin,
   package caches, and clean install strategy.
5. Map Linux x86_64/aarch64, macOS x86_64/arm64, Windows x86_64, CUDA, Metal,
   and driverless CPU verification. Record browsers as N/A unless a concrete
   browser-owned surface is found.
6. Inventory disk, ignored data/model caches, credentials, network/registry,
   CI runner, release workflow, documentation, and package rehearsal needs.
7. Inspect the release-state/board convention and propose the 0.8.26 authority
   transition without creating or editing generated state prematurely.

## Durable output

Update [`design.md`](design.md) with observed facts, required decisions,
recommendations, owner, and setup status. Take no environment, workflow,
version, runner, or release-state action.

## Acceptance

- The exact branch, worktree, and base commit are recorded.
- Every language/tool/platform surface has current, required, and unknown state.
- Worktree-local tools cannot silently build against the primary checkout.
- Project-infrastructure proposals are allocated by Slice 8 to Slice 9 or a
  feature/hardening slice.
- No product or environment configuration changed.

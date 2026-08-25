---
title: 0.8.24 Slice 40 — status
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 status

## Current state

**BLOCKED — READY FOR OWNER DECISION.** Preparation, product-contract review,
repository discovery, and conditional architecture review are complete. No
implementation has started because both required owner decisions remain open:
P24-09 (SDK surface) and P24-10 (real Windows CUDA executor).

## Work completed

- Reviewed the Slice 40 draft plan, Slice 0–6 allocations, current release
  workflow, Python/npm Windows CPU surfaces, ADR-0.8.22, and local Windows
  environment boundary.
- Recorded the existing-versus-net-new map and local contract drafts.
- Produced a conditional design and exact executor/evidence contract.
- Confirmed that no current Windows CUDA artifact, selector, GPU/toolchain
  evidence, loader identity, or installed GPU smoke exists.

## Explicit non-substitutions

- The self-hosted CUDA runner inventory is Linux-only.
- The local Windows VM has a virtual display and no NVIDIA host-device
  passthrough; it is CPU-only validation, not Windows CUDA proof.
- GitHub-hosted `windows-latest` build/publish/smoke jobs are CPU routes and
  are not an approved CUDA executor.
- No local Windows compile, hosted CPU substitution, runner/VM operation,
  publication, workflow dispatch, or GitHub setting change occurred.

## Next owner action

Record P24-09 and P24-10 using `decision.md`. With both decisions, Slice 40
can promote the applicable draft contracts, write the selected ADR/interface
change, add RED tests, and implement/prove the route on the approved executor.

## Handoff

Slice 60 will require the selected identity/version/index or registry,
artifact digest, source and executor provenance, CPU-preservation assertions,
candidate install command, and successful Windows GPU smoke. Slice 70 consumes
that evidence for release integration; it must not manufacture an absent
executor or package topology.

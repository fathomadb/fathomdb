---
title: 0.8.24 Slice 30 — status
status: BLOCKED
target_release: 0.8.24
---

# Slice 30 status

## Outcome

Slice 30 planning and design reconciliation are complete, but Slice 30 itself
is **BLOCKED before implementation**. D-80.6-3 already resolves package
identity: CPU remains `fathomdb==0.8.24` on PyPI; Tegra is the same `fathomdb`
distribution/import at exact `0.8.24+tegra` on a first-party PEP 503 index.
The stale `fathomdb-tegra` / separate-import draft premise is retired.

The supported target is narrowed to Jetson Orin, L4T R36 / JetPack 6, CUDA
12.6, glibc 2.35, and Python abi3. The bare `linux_aarch64` wheel is
host-bound, detection-gated, and not PyPI-uploadable.

## Evidence reviewed

- Authoritative D-80.6-3 and the release-independent Tegra platform reference.
- Existing host-native build, co-install/displacement detection, CUDA contract,
  witness, and manual evidence workflow.
- Successful dedicated-Orin run 32296395639 at exact
  `59c1033e229838632b5d0fe1ecd48845f3007fa0`.
- Owner clarification that the repository is `fathomadb/fathomdb`.
- Already-collected GitHub metadata: environments `pypi` and
  `cuda-unmerged-preflight`; no Actions variables; repository secret names
  `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`.

## Blocker

No concrete first-party PEP 503 URL/service owner or publisher deployment route
is declared. The repository name does not imply a hostname. The publisher
environment and authentication path also do not exist in the observed GitHub
configuration. These are required architecture inputs, so no RED test, code,
workflow, build, runner, registry, or publication action was started.

## Durable records

- `prep.md` — reviewed goals, changes, matrix, and exists/net-new map.
- `draft-contracts.md` — corrected slice-local need, requirements, and ACs.
- `research.md` — settled packaging/runner facts and endpoint research boundary.
- `design.md` — D-80.6-3-aligned architecture and trust boundary.
- `decision.md` — resolved rulings and exact remaining owner inputs.
- `plan.md` — future RED/GREEN sequence and implementation allowlist.

## Completion state

This is a blocked status, not a completed Slice 30 claim. After the endpoint and
deployment route are supplied, the design requires independent re-review before
implementation. Slice 60 receives no registry-installed Tegra handoff yet.

No hosted workflow, Jetson build, registry query/mutation, runner operation,
environment change, secret access, push, tag, or publication occurred.

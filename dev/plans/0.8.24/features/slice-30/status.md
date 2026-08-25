---
title: 0.8.24 Slice 30 — status
status: IN_PROGRESS
target_release: 0.8.24
---

# Slice 30 status

## Outcome

Slice 30 planning and design reconciliation are complete and implementation is
now **IN_PROGRESS**. D-80.6-3 already resolves package
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
- Historical workflow gap resolved in Slice 30: both Jetson jobs now admit
  only `refs/heads/release/0.8.24`; the candidate build also fails closed
  until the normal version bump sets `project.version` to `0.8.24`.
- Successful dedicated-Orin run 32296395639 at exact
  `59c1033e229838632b5d0fe1ecd48845f3007fa0`.
- Owner clarification that the repository is `fathomadb/fathomdb`.
- Already-collected GitHub metadata: environments `pypi` and
  `cuda-unmerged-preflight`; no Actions variables; repository secret names
  `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`.

## Authorized interim route

On 2026-08-25 the owner authorized GitHub Pages as an interim first-party PEP
503 route. Pages is now enabled in Actions mode at
`https://fathomadb.github.io/fathomdb/`; Slice 30's PEP 503 base is
`https://fathomadb.github.io/fathomdb/tegra/simple/`. The planned hosted
publisher deploys only after the credentialless Jetson artifact is verified,
uses the `github-pages` environment with `pages: write` and `id-token: write`,
and carries no package-registry credential. Each Pages redeploy is explicitly
owner-authorized and revalidates its input; this interim static deployment does
not claim durable multi-version immutability.

This closes the prior endpoint/publisher-design blocker. It does not publish a
wheel, dispatch a runner, or make GitHub Pages the permanent distribution
decision. Before a later Tegra release, hosting and distribution must be
re-reviewed for durable multi-version retention and endpoint policy.

Slice 30 has corrected the workflow-ref gap under RED/GREEN tests for both
jobs. The slice remains open for the normal 0.8.24 version bump and an
owner-authorized real-Jetson candidate run; neither has been performed by this
implementation change.

## Durable records

- `prep.md` — reviewed goals, changes, matrix, and exists/net-new map.
- `draft-contracts.md` — corrected slice-local need, requirements, and ACs.
- `research.md` — settled packaging/runner facts and endpoint research boundary.
- `design.md` — D-80.6-3-aligned architecture and trust boundary.
- `decision.md` — resolved rulings and exact remaining owner inputs.
- `plan.md` — future RED/GREEN sequence and implementation allowlist.

## Completion state

This is an in-progress status, not a completed Slice 30 claim. Slice 60 receives
no registry-installed Tegra handoff until explicit publication and its clean
Jetson installed-package smoke complete.

No hosted workflow, Jetson build, registry query/mutation, runner operation,
environment change, secret access, push, tag, or publication occurred.

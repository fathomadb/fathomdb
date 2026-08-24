---
title: 0.8.24 Slice 30 — decision reconciliation
status: PARTIAL-BLOCKED
target_release: 0.8.24
---

# Slice 30 decision reconciliation

## Decisions already owned by D-80.6-3

- Repository: `fathomadb/fathomdb`.
- Distribution and import package: `fathomdb`.
- Generic CPU release: `fathomdb==0.8.24` on PyPI.
- Tegra release: exact `fathomdb==0.8.24+tegra` on a first-party PEP 503 index.
- Wheel: honest host-bound `linux_aarch64`; not PyPI-uploadable and not
  relabelled manylinux.
- Selection: exact pin and classic-Tegra detection gate; no naked floating
  `--extra-index-url` instruction.
- Public target: Jetson Orin, L4T R36 / JetPack 6, CUDA 12.6, glibc 2.35,
  Python abi3. Generic SBSA, Thor, and npm Tegra are excluded.

The former proposal for `fathomdb-tegra` or a separate import topology is
**rejected as superseded**. It must not return as an unresolved option.

## Proven executor fact

The dedicated `jetson-fathomdb` runner and manual evidence route are real.
Run 32296395639 passed on Orin against
`59c1033e229838632b5d0fe1ecd48845f3007fa0`, retaining the bare
`linux_aarch64` `+tegra` wheel and CPU/auto/forced-CUDA installed proof. This
resolves executor feasibility, not public endpoint readiness.

The executor route has one proven source gap: both jobs in its workflow are
hard-coded to `refs/heads/release/0.8.23`, so `release/0.8.24` would skip. The
future implementation must correct both predicates under a bounded
release-candidate contract; this does not reopen runner selection.

## Still required from the owner

1. Concrete first-party PEP 503 HTTPS root and owning service/account.
2. Deployment route: publisher workflow/job, GitHub environment,
   authentication method, path/project scope, and immutable/retry policy.

The repository and GitHub metadata do not declare either. Existing environment
and secret names are unrelated to a new PEP 503 host. The hostname and service
must not be inferred from `fathomadb/fathomdb`.

## Gate

Status remains BLOCKED. Once the two inputs are supplied, update this record,
perform endpoint-specific primary-source research, obtain independent design
re-review, and then begin the RED/GREEN sequence in `plan.md`, including the
two-job 0.8.24 workflow-ref correction.

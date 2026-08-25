---
title: 0.8.24 Slice 30 — decision reconciliation
status: READY
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

## Authorized interim Pages route

The owner authorized the following interim route on 2026-08-25 and GitHub Pages
was enabled in Actions mode for the repository.

| Decision | Ruling |
| --- | --- |
| PEP 503 base | `https://fathomadb.github.io/fathomdb/tegra/simple/` |
| Owner | `fathomadb/fathomdb` GitHub Pages site |
| Publisher | The hosted publisher job in `.github/workflows/jetson-tegra-cuda-evidence.yml`, after the credentialless Jetson build/evidence job |
| Environment/authentication | `github-pages`; ephemeral repository `GITHUB_TOKEN` restricted to `pages: write` plus Pages OIDC `id-token: write`; no registry secret |
| Scope | Explicit opt-in only, exact `release/0.8.24` candidate and `fathomdb==0.8.24+tegra`; generic CPU PyPI release remains untouched |
| Retry | Verify the exact wheel filename, metadata version, and SHA-256 before redeploying; never substitute another artifact for the same local version |

The static Pages deployment hosts the PEP 503 HTML and the wheel for the
interim 0.8.24 release. Before any later Tegra release, the owner and a design
review must revisit durable multi-version retention, endpoint/domain policy,
and distribution support. No later release may inherit this interim route by
default.

## Gate

Status is READY for the scoped RED/GREEN implementation in `plan.md`, including
the two-job 0.8.24 workflow-ref correction. Publication and the resulting
installed-package smoke remain explicit release actions, not a consequence of
merging the workflow.

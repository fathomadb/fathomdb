---
title: 0.8.24 Slice 1 — Dependabot and library sweep
status: COMPLETE
target_release: 0.8.24
---

# Slice 1 — Dependabot and library sweep

**Observed:** 2026-08-23. This is an inventory and disposition; no dependency,
lockfile, Dependabot, Action, or registry change was made.

## Dependabot

`.github/dependabot.yml` covers every tracked shipped/tooling manifest:
Python, Cargo, TypeScript npm, root npm, and GitHub Actions. All five entries
set `open-pull-requests-limit: 0`. The checked-in rationale says this is a
deliberate owner-standing posture after 0.8.20 recovery, not an accidental
configuration omission.

**Disposition: keep paused.** This slice does not re-enable automatic PRs.
The consequence is explicit: current GitHub Dependabot alert/PR state is
unknown because authenticated GitHub API requests returned rate-limit HTTP 403.
Manual, evidence-led reviews remain required while the pause stands.

## Concrete npm findings

| Package | Locked | Available | Signal | Pin/source rationale | Disposition |
| --- | ---: | ---: | --- | --- | --- |
| `markdownlint-cli2` | 0.23.0 | 0.23.2 | Root `npm audit` reports transitive `js-yaml` 5.2.0 moderate/high advisories. Candidate 0.23.2 declares `js-yaml` 5.2.2. | Root Markdown verifier; its fix path is AST-guarded. | **Recommend Slice 7** contained security remediation after owner acceptance. |
| `prettier` | 3.9.4 | 3.9.6 | No audit finding. It is not used by an active command; only bootstrap installs it and manifest comments retain it for hypothetical non-Markdown use. | It must never be used for Markdown because it corrupts this repository’s prose. | **Propose Slice 2 review** for removal/deprecation, not an automatic version bump. |
| `@napi-rs/cli` | 2.18.4 | No `npm outdated` candidate | TypeScript audit clean. | N-API build tooling; major migration is not a routine patch. | Keep. |
| `@types/node` | 26.1.0 | No `npm outdated` candidate | TypeScript audit clean. | Already on the currently declared major. | Keep. |
| `typescript` | 6.0.3 | No `npm outdated` candidate | TypeScript audit clean. | TS 6 transition already landed with its `types` fix. | Keep. |

The root audit has exactly two advisories, both through `markdownlint-cli2 →
js-yaml@5.2.0`:

- moderate GHSA-724g-mxrg-4qvm (`>=5.0.0 <=5.2.0`);
- high GHSA-pm4m-ph32-ghv5 (`>=5.0.0 <=5.2.1`).

The TypeScript audit reports zero vulnerabilities. The repository’s offline
`check-pinned-override-rot.py --root .` passes, so no unrecorded npm override
or Cargo git-source exception was introduced.

## Python findings

Python uses ranges for ordinary test/eval tools and exact pins only where the
repository has a reproducibility contract.

| Package | Declared | Latest observed | Classification | Disposition |
| --- | --- | ---: | --- | --- |
| `ruff` | `==0.15.17` | 0.16.4 | Exact reviewed clean-clone/tool-output pin. | Postpone pending explicit reproducibility review and matching guard update. |
| `pyright` | `==1.1.410` | 1.1.411 | Exact selected typechecker pin. | Candidate for a small Slice 7 review only with its version-guard/typecheck proof. |
| `maturin` | `>=1.9,<2` | 1.14.1 | Minimum is justified by PEP-639 build behavior; resolver already permits current 1.x. | Keep range; no manifest bump required. |
| `pytest` | `>=8` | 9.1.1 | Flexible test dependency. | Do not manufacture a version update; verify compatibility only if a clean environment exposes a failure. |
| `hypothesis`, `pyyaml`, `numpy`, `scipy`, `networkx`, `tomli` | minimum/range | current metadata queried | Flexible non-shipped/test-or-eval dependencies. | Keep; no evidence of a release-blocking upgrade. |

## Rust and Action findings

Public crates.io queries returned a 403 access-policy response, so current
upstream availability and advisories are **unknown**, not “no updates.” The
current direct dependency set was still inventoried from Cargo metadata.

| Cohort | Current constraint | Why it is pinned or coupled | Disposition |
| --- | --- | --- | --- |
| Fathom Candle fork | three `=0.10.2` crates plus one exact Git revision | Preserves Linux ARM64 CPU GEMM/F16 fallback and dynamic CUDA driver loading; core/nn/transformers types must remain unified. | Keep; do not update without target-platform compatibility evidence. |
| `sqlite-vec` | `=0.1.9` in engine/schema | Couples vector extension ABI/semantics to schema and migration tests; 0.1.9 was the accepted remedy for the earlier vec0 metadata-delete issue. | Keep exact; a new version is an engine/schema migration, not a sweep bump. |
| `rusqlite` | `^0.40` in engine/schema/CLI | Bundled SQLite and sqlite-vec are coupled. | Keep; any major/minor migration is a database feature decision. |
| `ort` | `=2.0.0-rc.10` | ONNX dynamic-load contract is tied to the expected runtime ABI. | Keep; pre-release API/ABI changes require dedicated ONNX review. |
| `napi` family | major 2 | Native binding ABI/build surface. Historical review identified major 3 as a migration. | Keep; do not bundle with Windows CUDA. |
| `pyo3` | `^0.29` | Python binding/abi3 surface. | Keep absent concrete registry/security evidence. |
| GitHub Actions | SHA-pinned actions with version comments | Supply-chain pinning is intentional; Dependabot’s Actions updates are paused. | Unknown current upgrades due GitHub API rate limit; no change. |

## Proposed actions and allocation

| Item | Risk | Recommendation | Destination |
| --- | --- | --- | --- |
| Update root `markdownlint-cli2` and lockfile to 0.23.2 | Low/contained; dev tooling only, but fixes a high advisory transitive path. | Include only after owner accepts the Slice 7 maintenance item. | 7 |
| Update Pyright 1.1.410 → 1.1.411 | Low but governed by version guard. | Postpone pending explicit check. | 7 or later sweep |
| Update Ruff 0.15.17 → 0.16.4 | Medium; exact reproducibility pin. | Postpone pending clean-clone review. | Later sweep |
| Remove/deprecate unused Prettier | Low but documentation/bootstrap impact. | Do not couple to dependency remediation. | 2 |
| Candle, sqlite-vec, rusqlite, ORT, N-API major work | High/wide product/runtime impact. | Reject from 0.8.24 dependency maintenance. | Future dedicated release |
| Dependabot re-enable | Policy and notification-volume decision. | Keep paused; revisit only with owner direction. | Slice 6 decision register |

## Completion

The library sweep is complete: supported ecosystems are enumerated, each known
pin has a rationale/disposition, concrete public metadata was captured where
available, and service-limit gaps are visible. It intentionally took no action.

## Evidence

- `.github/dependabot.yml`.
- Root and TypeScript `npm outdated`/`npm audit` on 2026-08-23.
- PyPI metadata queries on 2026-08-23.
- `python3 scripts/check-pinned-override-rot.py --root .` (pass).
- `cargo metadata --format-version 1 --no-deps`; crates.io HTTP 403 response.

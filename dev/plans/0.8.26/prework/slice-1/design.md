---
title: 0.8.26 Slice 1 — dependency sweep draft notes
status: COMPLETE
observed_on: 2026-09-12
---

# Slice 1 draft design notes

## Initial state

The authenticated GitHub query found no open PRs. This does not prove currency:
`.github/dependabot.yml` sets `open-pull-requests-limit: 0` for Cargo, Python,
root npm, TypeScript npm, and GitHub Actions. A local `cargo audit` attempt could
not lock the read-only shared advisory cache; Slice 1 must use an isolated
writable cache and fail closed on refresh failure.

## Required enumeration targets

| ID | Current constraint | Why pinned / prior candidate | Slice 1 question |
| --- | --- | --- | --- |
| D26-L1 | Candle family `=0.10.2` at FathomDB Git rev `cf02edbc...` | AArch64 F16/GEMM and static-CUDART/driverless compatibility; transitive `paste` debt | Is a maintained compatible revision available without CPU/CUDA/Metal behavior drift? |
| D26-L2 | `ort =2.0.0-rc.10` | Dynamic ONNX load contract; later RC previously had provider mismatch | Has a stable/RC successor passed API, ABI, CPU, and provider tests? |
| D26-L3 | `rusqlite 0.40` + `sqlite-vec =0.1.9` | Coupled SQLite/extension, migration, metadata-delete, query-plan contract | Is there a security/correctness driver strong enough for a dedicated migration? |
| D26-L4 | PyO3 0.29 / abi3-py310 | Published Python ABI and binding surface | Retain unless a concrete security/platform driver exists. |
| D26-L5 | N-API major 2 / `@napi-rs/cli ^2.18.4` | Published native loader/package topology | Inventory current compatible patches; treat major migration separately. |
| D26-L6 | Pyright 1.1.410 | Exact reproducible diagnostic guard; prior candidate 1.1.411 | Compare complete diagnostics and guard together. |
| D26-L7 | Ruff 0.15.17 | Exact clean-clone lint contract; prior candidate 0.16.5 | Upgrade only after intentional output/fix review. |
| D26-L8 | TypeScript `^6.0.3`, `@types/node ^26.1.0` | Node 25 SDK/tooling surface | Resolve actual lock/current versions and test emitted declarations. |
| D26-L9 | `markdownlint-cli2 ^0.23.2` | AST-guarded Markdown toolchain; raw fixer/prettier prohibited | Audit compatible patch only; preserve neutrality guard. |
| D26-L10 | GitHub Actions full SHAs | Supply-chain pinning | Map each SHA to official current release and preserve SHA form. |

Also refresh RustSec findings for transitive `crossbeam-epoch`, `anyhow`,
`event-listener`, `memmap2`, `async-std`, and `paste`; 0.8.25 may already have
resolved some, so prior findings are inputs rather than current claims.

No item is approved for upgrade in this draft.

## Completed live sweep

The dated registry sweep covered every tracked manifest class. Five
Dependabot ecosystems are paused, so zero open pull requests is not evidence
of currency.

| Priority | Finding | Draft disposition and proof |
| --- | --- | --- |
| urgent | Root npm reports a high-severity `smol-toml` advisory through markdownlint-cli2. Current 0.23.2 still pins affected 1.7.0; npm proposes an unsafe blind downgrade. | Slice 9 investigation: prefer a bounded override or fixed upstream release, and prove the Markdown AST-neutrality/lint contract before adoption. |
| urgent | `dev/tools/mermaid` is outside Dependabot coverage and has Mermaid/DOMPurify advisories; CLI 11.17.0 is the first bounded candidate. | Slice 9 or a dedicated housekeeping slice with rendered-diagram and audit proof. |
| high | Seven `release.yml` references label the pinned `actions/download-artifact` v4.3.0 SHA as v8.0.1; the repository already uses the actual v8.0.1 SHA elsewhere. | Reviewed in Slice 7; Slice 8 must place it. Validate affected runners and release artifact flow non-publishing before changing the pin. |
| medium | Rust lock refresh offers compatible updates, while RustSec reports zero vulnerabilities and one unmaintained `paste 1.0.15`. | Investigate the Candle/transitive path as a coupled change; do not force it into feature work. |
| medium | Pyright 1.1.410 to 1.1.414 and Ruff 0.15.17 to 0.16.7 are available. | Compare complete diagnostics and fixer behavior in isolation; retain exact pins until evidence is green. |
| low | `@types/node` 26.1.0 to 26.5.1 is a compatible patch candidate; TypeScript and N-API otherwise report no direct outdated packages or advisories. | Optional bounded housekeeping, not a release blocker. |

The fresh isolated RustSec database was at `b50980aa...`, covered 440
packages, and found no vulnerabilities. The root Cargo dry run proposed 126
compatible lock changes; that breadth is evidence against bundling a blind
lock refresh with the Memex contract work. The TypeScript audit was clean.
Python range dependencies already admit current releases and need no manifest
edit merely to chase latest versions.

## Protected boundaries and evidence limits

Retain the Candle fork, ORT release candidate, rusqlite/sqlite-vec pair, PyO3
ABI, N-API major, exact lint/typecheck pins, and action SHA form until their
documented compatibility witnesses pass. The online SBOM sweep could not run
because its isolated environment was absent; action-wide latest-ref
reconciliation also remains incomplete. Slice 8 must not interpret either as
green. Alerts against ignored, unshipped `python/uv.lock` are tracked
separately from shipped-product exposure.

## Complete disposition by source

| Source | Result | Classification |
| --- | --- | --- |
| Cargo manifests/lock | zero RustSec vulnerabilities; unmaintained `paste`; 126 compatible resolver changes with no aggregate driver | investigate `paste` with Candle; postpone broad refresh |
| root npm | high `smol-toml` advisory through current markdownlint-cli2 | investigate now; accept no blind downgrade |
| TypeScript npm | clean audit; only bounded `@types/node` patch | postpone/optional update-now after declaration proof |
| Python package | range dependencies admit current versions; Pyright/Ruff exact-pin candidates | retain exact pins; investigate diagnostics separately |
| Mermaid tool | actionable Mermaid/DOMPurify advisories and no Dependabot coverage | investigate/update-now candidate with render proof |
| SBOM survey tool | standalone Python ranges: CycloneDX 8.x, packageurl <1, packaging <26, semver <4, pytest <10 | retain ranges; live self-survey unavailable and fail-closed |
| GitHub Actions | SHA form retained; one confirmed download-artifact label/SHA mismatch | investigate/update-now candidate after runner/artifact proof |

“Update-now” remains a proposal class for Slice 8, not authorization. Coupled
native boundaries are retained; broad Cargo, Ruff/Pyright, and major
TypeScript/N-API changes are postponed absent a concrete driver.

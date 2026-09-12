---
title: 0.8.26 Slice 1 — dependency sweep draft notes
status: DRAFT
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

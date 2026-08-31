---
title: 0.8.25 Slice 1 — dependency and pinning sweep
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 1 — dependency and pinning sweep

## Outcome

The manifests do not require a broad refresh. The Rust lock contains one
vulnerability and three unsoundness advisories with patch releases available;
those lock-compatible remediations should be evaluated together in Slice 7.
The npm trees audit clean. Python has two reviewed-tool patch candidates, but
neither is a security requirement. No manifest, lockfile, workflow, source, or
installed dependency changed in this slice.

## Dependabot and source coverage

`.github/dependabot.yml` covers Cargo, Python, root npm, TypeScript npm, and
GitHub Actions. All five sources deliberately set
`open-pull-requests-limit: 0`, so the repository has chosen manual dependency
maintenance. The file's comment that GitHub reports six alerts, two high, is
stale: the most recent authenticated push banner reported ten default-branch
alerts, seven moderate and three low. Neither number is a package-level
inventory, so Slice 7 must reconcile the comment with current authenticated
alert data before claiming closure.

The root and TypeScript `npm audit --json` queries both reported zero
vulnerabilities. `npm outdated` reported no TypeScript candidate. The root
manifest already declares `markdownlint-cli2 ^0.23.2`; the shared ignored
`node_modules` link still contains 0.23.0. This is environment drift, not a
manifest upgrade, and reinforces Slice 0's rule that shared installs are not
release evidence.

## Required and candidate updates

| Item | Current | Candidate | Role and evidence | Proposed disposition |
| --- | --- | --- | --- | --- |
| `crossbeam-epoch` | 0.9.18 | >=0.9.20 | RustSec RUSTSEC-2026-0204; transitive through Rayon and the Candle fork | **Include in Slice 7.** Attempt a lock-compatible patch, run all-feature CPU/CUDA build and tests, and retain only with a documented upstream block. |
| `anyhow` | 1.0.102 | >=1.0.103 | RustSec RUSTSEC-2026-0190 unsoundness; transitive through `jsonschema` | **Include in Slice 7.** Patch the lock and verify schema-validation paths. |
| `event-listener` | 5.4.1 | >=5.4.2 | RustSec RUSTSEC-2026-0221 unsoundness; test-only path through `httpmock`/`async-std` | **Include in Slice 7.** Patch if resolver-compatible and rerun embedder provider tests. |
| `memmap2` | 0.9.10 | >=0.9.11 | RustSec RUSTSEC-2026-0186 unsoundness; transitive through the Candle fork | **Include in Slice 7.** Patch and verify model-loading CPU/CUDA paths. |
| `async-std` | 1.13.2 | replacement dependency | RUSTSEC-2025-0052 says discontinued; test-only through `httpmock` | Investigate in Slice 7. Prefer upgrading/replacing `httpmock`; do not migrate product runtime code because none uses this path. |
| `paste` | 1.0.15 | maintained replacement | RUSTSEC-2024-0436 says unmaintained; transitive through Candle/GEMM/tokenizers | Retain temporarily unless the Candle stack can remove it safely; record as coupled Candle debt, not a blind lock edit. |
| Pyright | 1.1.410 | 1.1.411 | Exact typechecker pin and guard; official PyPI metadata | Optional Slice 7 maintenance item. Update only with guard, typecheck, and clean-environment proof. |
| Ruff | 0.15.17 | 0.16.5 | Exact reproducibility pin; official PyPI metadata | Postpone unless Slice 6 explicitly accepts the output-change review. No security motivation was found. |
| npm package manager | manifest 11.12.1; host 11.19.0 | n/a | Slice 0 environment mismatch | Retain the manifest contract; release commands should select its declared version rather than rewriting it to match one host. |

Registry metadata also reported Maturin 1.15.0, pytest 9.1.1, Hypothesis
6.167.1, PyYAML 6.0.3, NumPy 2.5.2, SciPy 1.18.1, NetworkX 3.6.1, and
tomli 2.4.1. Their declared minimum/range constraints already permit current
compatible releases; no version edit is required merely because a newer
release exists.

## Pins and coupled stacks

| Constraint | Verified reason | Disposition |
| --- | --- | --- |
| Candle fork at Git revision `5719d90e` and crates `=0.10.2` | Preserves ARM64 CPU GEMM/F16 fallback and dynamic CUDA behavior; core/NN/transformer types must stay unified | Keep. The RustSec transitive fixes must first be attempted within semver-compatible lock resolution; changing the fork is a separate platform migration. |
| `sqlite-vec =0.1.9` plus `rusqlite 0.40` | Couples extension behavior, bundled SQLite, schema, and migration tests; 0.1.9 carries the accepted metadata-delete fix | Keep exact. Any upgrade is an Engine/schema change with migration and query-plan evidence. |
| `ort =2.0.0-rc.10` | Dynamic ONNX Runtime load contract and prerelease ABI/API | Keep until a dedicated ONNX compatibility slice. |
| `pyo3 0.29`, N-API major 2 | Shipped native binding and ABI surfaces | Keep; cross-SDK feature work must preserve them, but a major migration is not dependency housekeeping. |
| Ruff and Pyright exact pins | Agent gates fail closed on selected versions to make diagnostics reproducible | Keep until the pin, guard, and full output are reviewed together. |
| GitHub Actions by full SHA | Supply-chain integrity; human-readable tags are comments only | Keep SHA pinning. Reconcile current versions through Dependabot/official release metadata during the accepted Slice 7 maintenance change. |

`scripts/check-pinned-override-rot.py --root .` passed. No unrecorded Cargo
source exception or npm override was found.

## Proposed Slice 7 sequence

1. Capture authenticated Dependabot alert identities and action-update
   candidates; correct only stale configuration commentary.
2. Write a failing dependency-policy/advisory fixture that proves the accepted
   security boundary without depending on live network state.
3. Update only the four patchable Rust advisory packages in `Cargo.lock`; if
   resolution crosses the Candle, SQLite, ORT, or binding pins, stop and return
   that item to Slice 6.
4. Run RustSec, pinned-source checks, all-feature Rust verification, default and
   CUDA embedder/model-load tests, and the repository verification gate.
5. Treat Pyright 1.1.411 as an independent optional change. Do not couple Ruff,
   native-stack migrations, or Dependabot re-enablement to the security patch.

## Evidence

- `cargo audit --json`, refreshed RustSec database commit
  `ba9db2a77a6a0fe93bc63a3d9b730e08b145aff5`.
- `cargo tree --all-features -i` for every advisory-affected Rust package.
- Root and `src/ts` `npm outdated --json` and `npm audit --json`.
- Official PyPI JSON metadata for the direct Python tool/test dependencies.
- `.github/dependabot.yml`, manifests, lockfiles, workflows, prior 0.8.24
  dependency sweep, and the pinned-override check.

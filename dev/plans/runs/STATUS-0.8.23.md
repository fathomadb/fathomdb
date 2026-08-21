# STATUS — FathomDB 0.8.23

> **Board of record.** The single writer is
> `dev/plans/release-state-0.8.23.json`; the release plan is
> `dev/plans/plan-0.8.23.md`.

## Current state

**0.8.23 is released.** Tag `v0.8.23` resolves to `e9cf8763`; the release
branch and every recorded slice are reachable from `origin/main` (currently
`f854b30a`). The owner retired external Slice 10/20/70/71 evidence as a
release and publication gate; canceled workflows provide no receipt and none
is claimed.

crates.io and PyPI publication, npm publication under `next`, and the
GitHub Release completed in run `32505367254`. The owner then promoted the
main npm package directly: both `latest` and `next` resolve to `0.8.23`.
The required registry-installed npm smoke passed locally on windchill3
(install plus open/write/search/close/process exit). Slice 65 remains
**UNATTRIBUTED / NO REMEDY** and changed no production behavior.

## Slice ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 0 | ENVIRONMENT — identify environment setup and change preconditions | LANDED (`916023fe`). |
| 1 | DEPENDENCIES — Dependabot needs and library-sweep disposition | LANDED (`2167a0cd`). |
| 2 | CRUFT-REVIEW — repository and documentation cruft proposal | LANDED (`b363af85`). |
| 3 | DRAFT-CONTRACTS — draft needs, requirements, and acceptance-criteria allocation | LANDED (`91e162c2`). |
| 4 | ARCHITECTURE — architecture and high-level code-alignment review | LANDED (`a7df1590`). |
| 5 | VERIFICATION — verification-adequacy review | LANDED (`00f865f3`). |
| 6 | PREPARATION-WORKPLAN — hygiene and in-flight release preparation workplan | LANDED (`e98f727d`). |
| 30 | MEMEX-INTEGRATION — Memex readiness and graph-integration contract | LANDED on `origin/main` (`776d2c20`). |
| 50 | GITLEAKS-GUARDS — staged pre-commit and always-on CI secret scanning | LANDED (`ae7cef0e`). |
| 60 | WINDOWS-WAL — Windows WAL checkpoint reader-conflict diagnosis | LANDED on `origin/main` (`423baf6a`). |
| 65 | WAL-ATTRIBUTION — Windows WAL checkpoint root-cause attribution | LANDED on `origin/main` (`6b57557c`) — **CLOSED_UNATTRIBUTED_NO_REMEDY**; no retry, binding-lifetime, reader-pool, public-surface, Windows-cause, or production behavior change was selected. |
| 10 | CUDA-CONTRACT — CUDA environment, artifact contract, and protected runner gate | LANDED from `78fe1969` in main merge `346767b5`; external CUDA evidence is historical only and not a release or publication gate. |
| 20 | CUDA-PACKAGE — CUDA package, rehearsal, and installed-artifact smokes | LANDED from `78fe1969` in main merge `346767b5`; external package-rehearsal evidence is historical only and not a release or publication gate. |
| 70 | DUAL-RUNTIME-TC5 — supported dual CPU/GPU runtime policy, diagnostics, artifacts, and exact pre-fusion TC-5 controls | LANDED from `aa29152b` in main merge `346767b5`; external CUDA/TC-5 evidence is historical only and not a release or publication gate. |
| 71 | RERANK-DUAL-RUNTIME — cross-encoder reranker CPU/GPU runtime parity | LANDED from `a49ac989` in main merge `346767b5`; external installed-artifact receipts are historical only and not a release or publication gate. |
| 72 | CONCURRENT-DUAL-RUNTIME — concurrent embedding and cross-encoder GPU coexistence characterization | LANDED on `origin/main` (`824b1d4b`) with independently verified local CUDA basic/moderate/stress receipts; no performance threshold claim. |
| 80 | AARCH64-TEGRA — AArch64 correctness and Jetson Orin CUDA | LANDED on `origin/main` (`68b696a3`): 80.1–80.7 and approved 80.6.5 are complete. This is nonpublishing functionality evidence only. |

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | None — 0.8.23 is released. |

## Stop gate

Slice 65 is closed **UNATTRIBUTED / NO REMEDY**. The owner retired the former
Slice 10/20/70/71 external-evidence stop gate; it must not be recreated from
historical status text. Slice 40 remains outside this release at
`TC-8ccd1cf1-8e7d-4b7c-8efa-176663aed553`. Do not reopen Slice 65 for a remedy
without new scope. There is no remaining release stop gate.

The report is [0.8.23 Slice 6 preparation report](0.8.23-slice-6-hitl-package.md).
Its companion is the [Slice 6 workplan](0.8.23-slice-6-workplan.md).

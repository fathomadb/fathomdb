# STATUS — FathomDB 0.8.23

> **Board of record.** The single writer is
> `dev/plans/release-state-0.8.23.json`; the release plan is
> `dev/plans/plan-0.8.23.md`.

## Current state

<!-- BEGIN GENERATED release-state:0.8.23:status-current-state -->**Next is Slice 10 (CUDA-CONTRACT), IN_PROGRESS.** Completed on `origin/release/0.8.23`; `origin/main` integration is PENDING: 0 (`916023fe`) · 1 (`2167a0cd`) · 2 (`b363af85`) · 3 (`91e162c2`) · 4 (`a7df1590`) · 5 (`00f865f3`) · 6 (`e98f727d`) · 30 (`776d2c20`) · 50 (`ae7cef0e`) · 60 (`423baf6a`) · 65 (`6b57557c`) — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.23:status-current-state -->

0.8.23 remains planning-first and publication-held. Slices 30, 50, 60, and 65
are completed on `origin/release/0.8.23`; integration to `origin/main` remains
pending. Slice 65 closed **UNATTRIBUTED / NO REMEDY** after independent review
of its three enriched hosted-Windows binding repetitions; it changed no
production behavior. Optional diagnostics are separate scope. The remaining
candidate slices require explicit commission.

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
| 30 | MEMEX-INTEGRATION — Memex readiness and graph-integration contract | COMPLETED on `origin/release/0.8.23` (`776d2c20`); `origin/main` integration is PENDING. |
| 50 | GITLEAKS-GUARDS — staged pre-commit and always-on CI secret scanning | LANDED (`ae7cef0e`). |
| 60 | WINDOWS-WAL — Windows WAL checkpoint reader-conflict diagnosis | COMPLETED on `origin/release/0.8.23` (`423baf6a`); `origin/main` integration is PENDING. |
| 65 | WAL-ATTRIBUTION — Windows WAL checkpoint root-cause attribution | LANDED on `origin/release/0.8.23` (`6b57557c`) — **CLOSED_UNATTRIBUTED_NO_REMEDY**. Enriched binding jobs `95369168206`, `95373316503`, and `95375745327` completed the 3/3 discriminator; the normal actual series is 4 clean / 1 BUSY and broad binding is 3 busy-child-clean / 2 clean. Independent evidence review found no holder attribution and selected no retry, binding-lifetime, reader-pool, public-surface, Windows-cause, or production behavior change. Optional diagnostics are separate scope. |
| 10 | CUDA-CONTRACT — CUDA environment, artifact contract, and protected runner gate | IN PROGRESS — local controls include the reviewed dry-run provenance correction through `4ff8d146`; reviewed main provenance, runner-policy/environment attestation, and retained trusted x64 hardware evidence remain required. |
| 20 | CUDA-PACKAGE — CUDA package, rehearsal, and installed-artifact smokes | PENDING_EXTERNAL — local reviewed control plane landed at `91a5ddd8`; retained Slice 10 receipt/witness plus one real nonpublishing package-rehearsal bundle and independent evidence review block release completion and publication only. |
| 70 | DUAL-RUNTIME-TC5 — supported dual CPU/GPU runtime policy, diagnostics, artifacts, and exact pre-fusion TC-5 controls | PENDING_EXTERNAL — local release implementation through `aa29152b`; real CUDA/trusted-runner/rehearsal and TC-5 characterization evidence remains required. |
| 71 | RERANK-DUAL-RUNTIME — cross-encoder reranker CPU/GPU runtime parity | PENDING_EXTERNAL — local release implementation through `a49ac989`; compatible-GPU and visible-incompatible installed-artifact receipts remain required. |
| 72 | CONCURRENT-DUAL-RUNTIME — concurrent embedding and cross-encoder GPU coexistence characterization | LANDED on `origin/release/0.8.23` (`824b1d4b`) with independently verified local CUDA basic/moderate/stress receipts; no performance threshold claim. |
| 80 | AARCH64-TEGRA — AArch64 correctness and Jetson Orin CUDA | IN PROGRESS — 80.1–80.7 landed; `68b696a3` completes approved 80.6.5 schema/UUID work. Supplied transfer scripts and pinned windchill3 RTX 3090 preflight evidence remain. |

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.23:status-next-action -->**Continue Slice 10 (CUDA-CONTRACT)** — CUDA environment, artifact contract, and protected runner gate. **Remaining ladder:** 10 → 20 → 70 → 71 → 80.<!-- END GENERATED release-state:0.8.23:status-next-action --> |

## Stop gate

Slice 65 is closed **UNATTRIBUTED / NO REMEDY**. Slice 10 is
`PENDING_EXTERNAL`: it must not dispatch the unmerged release branch to the
protected runner, and its external evidence blocks release completion and
publication only. Slice 20 is likewise `PENDING_EXTERNAL`: its valid retained
package rehearsal is required for release completion and publication, but its
local control-plane completion permits Slice 70 work. Slice 40 is no longer in
this release: HITL `seq:254` moved its preserved V2 design to repository backlog
`TC-8ccd1cf1-8e7d-4b7c-8efa-176663aed553`. Do not reopen Slice 65 to infer a Memex
root cause or retry, binding, or reader-pool remedy; any optional diagnostic
requires separate scope. Slice work remains on the release branch; this board
does not require per-slice integration to `main`.

The report is [0.8.23 Slice 6 preparation report](0.8.23-slice-6-hitl-package.md).
Its companion is the [Slice 6 workplan](0.8.23-slice-6-workplan.md).

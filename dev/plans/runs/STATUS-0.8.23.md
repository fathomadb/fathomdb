# STATUS — FathomDB 0.8.23

> **Board of record.** The single writer is
> `dev/plans/release-state-0.8.23.json`; the release plan is
> `dev/plans/plan-0.8.23.md`.

## Current state

<!-- BEGIN GENERATED release-state:0.8.23:status-current-state -->**Next is Slice 65 (WAL-ATTRIBUTION), IN_PROGRESS.** Completed on `origin/release/0.8.23`; `origin/main` integration is PENDING: 0 (`916023fe`) · 1 (`2167a0cd`) · 2 (`b363af85`) · 3 (`91e162c2`) · 4 (`a7df1590`) · 5 (`00f865f3`) · 6 (`e98f727d`) · 30 (`776d2c20`) · 50 (`ae7cef0e`) · 60 (`423baf6a`) — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.23:status-current-state -->

0.8.23 remains planning-first and publication-held. Slices 30, 50, and 60 are
completed on `origin/release/0.8.23`; integration to `origin/main` remains
pending. Slice 65 is active with implementation and focused first-party
Windows evidence; the remaining candidate slices require explicit commission.

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
| 65 | WAL-ATTRIBUTION — Windows WAL checkpoint root-cause attribution | IN_PROGRESS / UNATTRIBUTED — direct Rust/current installed Python serial has two clean paired real-attempt observations (`32014441608`/`95340828268`, `32015551136`/`95344203972`) versus one five-attempt BUSY/25-frame observation (`95332307148`); one fresh serial measurement remains. In `95344203972`, the distinct binding diagnostic had five intentional owned-reader BUSY attempts, then direct completion ack/autocommit/idle/full inventory, but raw and same-Engine sampler BUSY at 195/176 frames until a native child was clean after Engine.close. That is one variable live-Engine/process-lifetime-associated observation, with its own three-measurement gate—not a binding-lifetime conclusion. Multiple controls still block completion, landing, attribution, and remedies. |
| 40 | SCALE-CHARACTERIZATION — fixture-scoped scale characterization | NOT_STARTED. |
| 10 | CUDA-CONTRACT — CUDA environment, artifact contract, and protected runner gate | NOT_STARTED. |
| 20 | CUDA-PACKAGE — CUDA package, rehearsal, and installed-artifact smokes | NOT_STARTED. |

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.23:status-next-action -->**Continue Slice 65 (WAL-ATTRIBUTION)** — Windows WAL checkpoint root-cause attribution. **Remaining ladder:** 65 → 40 → 10 → 20.<!-- END GENERATED release-state:0.8.23:status-next-action --> |

## Stop gate

Slice 65 is in progress and partially validated. Do not declare it complete or
landed, or infer a Memex root cause or retry, binding, or reader-pool remedy,
until its remaining controls complete. Do not start Slices 40, 10, or 20
without the applicable explicit instruction. Slice work remains on the release
branch; this board does not require per-slice integration to `main`.

The report is [0.8.23 Slice 6 preparation report](0.8.23-slice-6-hitl-package.md).
Its companion is the [Slice 6 workplan](0.8.23-slice-6-workplan.md).

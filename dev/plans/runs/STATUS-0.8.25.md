---
title: FathomDB 0.8.25 status
status: ACTIVE
target_release: 0.8.25
---

# STATUS — FathomDB 0.8.25

The single writer is `dev/plans/release-state-0.8.25.json`. Edit release facts
there, then regenerate this board's fenced view. The release plan is
`dev/plans/plan-0.8.25.md`.

## Current state

<!-- BEGIN GENERATED release-state:0.8.25:status-current-state -->**Next is Slice 79 (AC020-RUNTIME), IN_PROGRESS.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.25:status-current-state -->

Prework and Slices 10 through 60 are complete on the durable `release/0.8.25`
worktree. Slice 60 closed minimal constrained graph expansion, deterministic
bounded traversal, one-context reads, lifecycle and projection handling,
cross-SDK parity, and exact Linux/Windows verification. The owner-approved
2026-09-02 scope adjustment removes Slices 65/70. The owner-approved 2026-09-08
adjustment creates focused Slices 71–73 before final integrated closure. Direct
agents execute this release without Steward or Orchestrator roles. Slices
71–73 are complete; AC-072 and 71B recovery remain protected. The owner-directed
[2026-09-10 adjustment](../0.8.25/scope-adjustment-2026-09-10-ac020.md)
closes Slice 75 as an evidence checkpoint at safe product SHA `5056db9e`,
not passing release acceptance. AC-020 remains unresolved; other original
verification obligations transfer without being waived. Slices 76/77 own
bounded statistics-enabled experiments, Slice 80 requires results and further
owner consultation, and Slice 85 owns final verification/CI/package rehearsal.

## Slice ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 0 | Environment and infrastructure | Complete on release branch (`321ca576`) |
| 1 | Dependencies and pins | Complete on release branch (`51043e20`) |
| 2 | Repository cruft | Complete on release branch (`51043e20`) |
| 3 | Draft contracts | Complete on release branch (`51043e20`) |
| 4 | Architecture/code alignment | Complete on release branch (`51043e20`) |
| 5 | Verification adequacy | Complete on release branch (`51043e20`) |
| 6 | Proposal scoring and interactive HITL | Complete on release branch (`3a35c1e6`; approved `seq-274`) |
| 7 | Approved repository preparation | Complete on release branch (`fdbae48a`) |
| 10 | Measurement classification | Complete on release branch (`f383ec82`) |
| 15 | Identity and source provenance | Complete on release branch (`9a53e26a`) |
| 20 | Core dependency registration | Complete on release branch (`7766d4c4`) |
| 25 | Core atomic semantic actuation | Complete on release branch (`131053da`) |
| 30 | Lifecycle and erasure closure | Complete on release branch (`75617521`) |
| 35 | Eligibility and optional frozen reads | Complete on release branch (`071ff7d1`) |
| 40 | Core projection generation/readiness | Complete on release branch (`ccf7c695`) |
| 45 | Minimal pagination and operational state | Complete on release branch (`2f48e657`) |
| 50 | Compact source-complete evidence | Complete on release branch (`e741542d`) |
| 55 | Basic tracing and integrity | Complete on release branch (`5a6942bc`) |
| 60 | Minimal constrained graph parity | Complete on release branch (`59208028`) |
| 71 | AC-072 read-latency resolution and 71B write recovery | Complete on release branch (`84c056c6`) |
| 72 | Generic preflight and installed CE profiles | Complete on release branch (`2e14f5ba`) |
| 73 | Windows Node/N-API CI coverage | Complete on release branch (`6eb7cd18`) |
| 75 | Evidence checkpoint; unfinished release closure transferred | Closed with carry-forward (`5056db9e`); AC-020 unresolved |
| 76 | AC-020 attribution and statement reuse | Complete (`8027546d`) |
| 77 | Adaptive experiments and candidate selection | Complete inconclusive (`5484cd17`) |
| 79 | Explicit runtime configuration and statement reuse | In progress; authorized at `seq-276` |
| 80 | Remaining AC-020 disposition | Planning contract; depends on 79 evidence |
| 85 | Final verification, CI and non-publishing packaging | Planning brief; final candidate matrix pending |

## Decisions and blockers

- `seq-272` and `seq-273` rule every proposal. `seq-274` approves the reviewed
  Slice 7 plan and closes Slice 6. P25-17 keeps all runs/data; P25-20 remains
  narrow.
- The owner-approved
  [scope adjustment](../0.8.25/scope-adjustment-2026-09-02.md) is the current
  feature boundary. Bubble work is allocated to 0.8.26–0.8.28 or Parked;
  experimental work is assigned to 0.8.29/0.8.31/0.8.33 reviews or Parked.
- The owner-approved
  [closing-ladder adjustment](../0.8.25/scope-adjustment-2026-09-08-slices-71-73.md)
  moves the focused investigations, preflight/CE profile, and Windows Node/N-
  API coverage into Slices 71–73. Its former Slice 75 final-verification
  allocation is superseded by the 2026-09-10 adjustment: Slice 85 now owns it.
- Slice 71 is complete. AC-072 now passes three exact 10k/384d/1,000-query
  candidate repetitions at p50 69 ms and p99 75–77 ms, versus a stable
  p50 161–164 ms and p99 169–173 ms baseline. Limits and search semantics were
  unchanged; Slice 72 is unblocked.
- 71B remains complete at product candidate `eda95b07`. Scale-02 10k median
  acknowledgement/total improve from historical 1,833.777/1,834.571 ms to
  1,403.217/1,407.768 ms; projection-active AC-013 10k total improves from
  2,337.097 ms to 1,311.089 ms. Focused verification and independent code and
  evidence reviews pass. Candidate-only no-loss guards after the AC-072 product
  change also pass.
- Slice 72 is complete at measured candidate `2e14f5ba`. Generic preflight uses
  tracked release state and exact Git ancestry. The installed CPU/CUDA CE
  campaign passes correctness, pinned RTX 3090 allocation, and all four 10%
  median-p95 comparisons; the maximum ratio is `1.0962`. Slice 73 is unblocked.
- Slice 73 is complete at `6eb7cd18`. The Windows 11 installed N-API campaign
  passes its fixed 13-module matrix at 180/180 with no failures, cancellations,
  skips, or todos. This unblocked the original Slice 75 campaign.

- Slice 75 remains a closed checkpoint, not a passing gate. Slices 76 and 77
  completed bounded attribution and selection without resolving AC-020.
- The owner allocated Slice 79 at `seq-276` and authorized `MEMSTATUS=0` for
  the application-owned SQLite runtime. Slice 79 is in progress; 78/81–84
  remain reserved. One final broad Slice 85 round is planned; an exceptional
  second requires owner authorization.

- CUDA, NVIDIA tools including `nvidia-smi`, and ptrace are standing-authorized,
  including unconfined execution when needed. Sandboxed probe failures do not
  establish host absence.
- Release-branch Python/native behavior is certified through the isolated
  release-wheel verifier; the primary-checkout `.venv` is not evidence.
- Generic release-state completion now distinguishes release-branch completion
  from `origin/main` reachability. The `landed` set remains empty until an
  independently authorized push and integration.
- Publication, tags, registry mutation, and `main` integration remain outside
  the completed Slice 72 scope.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.25:status-next-action -->**Continue Slice 79 (AC020-RUNTIME)** — explicit SQLite runtime configuration and statement reuse. **Remaining ladder:** 79 → 80 → 85.<!-- END GENERATED release-state:0.8.25:status-next-action -->

## Verification

Every transition must pass the release-state renderer, developer Markdown
lint, and `git diff --check`. Each Slice 1–5 record is proposal-only.

- Slice 7 fast verification passes 103/103 suites; the appropriately sized
  heavy route passes 2/3 applicable suites with one explicit exclusion.
- A fresh serial Rust workspace run passes and contains no removed-worktree
  target provenance. The isolated release-built Python wheel passes real
  open/write/search/close and property checks.
- Dependency policy, loader behavior, RustSec, protected pins, traceability,
  strict ptrace, documentation, and release-state views pass.
- The CUDA feature suite passes all 79 tests on RTX 3090 GPU 0 with CUDA 12.6,
  including CPU/GPU logit agreement and real GPU load/score.
- Independent implementation review passes after FIX-1/FIX-2 with no remaining
  P1/P2/P3 finding; a separate read-only verifier confirms the evidence.
- Slice 10 passes independent design and implementation review after the
  bounded FIX-3 cycles. Its 59 focused tests, portable and historical audits,
  fast tier, and applicable heavy suites pass; the sole heavy skip is the
  unavailable TypeScript dependency tree.
- Slice 15 passes independent design review at cycle 3 and implementation
  review at cycle 4. Independent focused verification covers schema 27,
  atomic versioned writes, corruption-safe erasure, SDK error parity, and
  packaged-smoke contracts. Its between-slice memory checkpoint is complete.
- Slice 20 passes independent design review at cycle 5 and implementation
  review at cycle 3. Its 55 focused Rust/schema tests, 103 fast suites, and
  packaged Python/TypeScript dependency smokes pass.
- Slice 25 passes independent design review at cycle 6 and implementation
  review at cycle 7. Its 76 focused Rust/schema tests, 387 TypeScript tests,
  fresh-wheel Python tests, rollback/projection proofs, and governed-surface
  gate pass.
- Slice 30 passes design reconciliation and implementation review. Verification
  FIX-1 closed the sole post-review P2 through committed RED/GREEN. Its 26
  focused tests, 103 fast suites, full serial Rust workspace, heavy Rust and
  TypeScript routes, operator route, and fresh-wheel corruption replay pass.
- Slice 35 passes independent design review, implementation review at FIX-7,
  and separate verification. Its 46-test focused correction suite and 103/103
  fast gate pass. The clean authoritative receipt records 5,500
  `Engine.search_text_only` calls per arm and passes the preregistered 3%
  p50/p95 upper-regression boundary; four false-label historical receipts are
  quarantined without rewriting them.
- Slice 40 passes independent design review at cycle 4, implementation review
  at cycle 8, and a separate full-gate fixture-reconciliation review. Its
  preregistered 10k write/storage/reopen and 50k CPU/CUDA-linked status
  campaigns pass all bounds; Linux CPU/CUDA and Windows native artifacts pass
  status/reopen smokes, and real RTX 3090 allocation is separately witnessed.
- Slice 45 passes independent design review at cycle 4, implementation review
  at cycle 6, and separate Linux/Windows verification. Its authenticated
  frozen pages and operational-state reads pass all functional gates. The
  registered 10k/50k receipt finds no material latency or memory effect; the
  largest steady p95 increase is 0.116 ms and the largest median peak-RSS
  increase is 1.17 MiB. The final unconfined fast gate passes 103/103 suites.
- Slice 50 passes independent design and implementation review at cycle 4.
  Its stateless content-free evidence reference, exact eligibility-bound
  resolver, nondisclosure matrix, and unchanged ordinary-search path pass
  focused Rust/Python/TypeScript, property, Linux package, Windows native,
  optional-reranker feature, and repository gates.
- Slice 55 passes design review at cycle 4 and implementation review at cycle
  15. Separate exact Linux and Windows verification passes focused tracing,
  integrity, explanation, startup, cancellation, WAL, package, cross-SDK,
  performance, 103/103 fast, 3/3 heavy, 106/106 all, canonical serial
  workspace, and terminating parallel-report routes without a startup or
  pause deadlock.
- Slice 60 passes design review at cycle 4 and implementation review at cycle
  15. Linux verification passes both 50/50 feature matrices, 110 compatibility
  tests, fresh Python 24/24 and Node 15/15, AC-059b 1,000/1,000, and the serial
  and terminating parallel routes. Windows passes the corrected lifecycle
  selector 1/1, the six unreached targets 31/31, fresh Python 24/24, and fresh
  Node 15/15. No broad test route was repeated during the final narrow close;
  normal commit and push hooks remained enabled.
- A diagnostic release-wide long gate exposed the pre-existing AC-013 vector
  latency failure and was stopped after that failure. It is release debt, not
  a Slice 60 graph-contract failure; Slice 71 owns its classification.
- Slice 71B code review passes on exact product candidate `eda95b07`; a separate
  read-only evidence audit passes the retained prospective recovery run. The
  affected visibility, rollback, frozen-read, projection, capacity, and drain-
  deadline checks pass, together with affected-crate check and clippy. Zero
  broad-verification rounds completed during 71B.
- Slice 71 AC-072 RED/GREEN completes at `de8da0f9`/`84c056c6`. Forty-nine
  focused blast-radius tests and changed-target clippy pass. The final exact
  campaign passes all candidate thresholds and spreads; the two candidate-only
  71B write guards preserve the recovered margins. Independent review and
  evidence audit pass. Zero broad-verification rounds completed for closure.

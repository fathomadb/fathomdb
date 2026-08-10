# STATUS — FathomDB 0.8.22

> **CLOSED — historical record.** v0.8.22 was published 2026-08-10. Current
> release state is 0.8.23.

> **Board of record.** The single writer is
> `dev/plans/release-state-0.8.22.json`; the release plan is
> `dev/plans/plan-0.8.22.md`.

## Current state

**Published and complete.** Tag `v0.8.22` points to
`93cd1a14e35bbf68a57dd97aac76794f3bf1d887`; npm `latest` and `next` both
resolve `fathomdb` to 0.8.22.

| | |
| --- | --- |
| Stable target matrix | Linux glibc x64/ARM64, macOS x64/ARM64, and Windows x64. |
| npm policy | Publish `fathomdb` under `next`; promote only the main package to `latest` after all registry smokes and co-tagging. |
| Explicitly unsupported | Linux musl, Windows ARM/32-bit, and every triple outside the matrix. |
| Publish authority | The trusted-publishing bootstrap and normal explicit HITL authorization remain required before a real tag or registry publish. |

## Slice ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 0 | Contract, acceptance, and trusted-publishing runbook | Landed — `55792858b2adce00d3d87193d02b23a5d8d52dd7`. |
| 5 | SQLite dependency migration and TC-76 proof | Landed — `55792858b2adce00d3d87193d02b23a5d8d52dd7` (folded into Slice 0). |
| 10 | Five-target platform-package topology | Landed — `4c7bb26b`. |
| 12 | Current-authority and document-debt inventory | Landed — `72a83049`; Phase 2 remains unauthorized. |
| 17 | Pre-register 0.8.23 scale-measurement protocol | Landed — `5a7f2484`; protocol only, no scale run or scale claim. |
| 15 | Native build and validation matrix | Landed — `13341688fca3d02d11c10bb10eb26232156f8032`; CI run #31186535382 passed the full heavy verifier, five-runner native runtime matrix, and five wheel-size gates. |
| 18 | Ranked retrieval result limits and SDK parity | Landed — `8fdb27dbf00a0663772ffc8e27a243ac1e7dcd74`; default 10 and validated 1..=100 limits across Rust, Python, and TypeScript. |
| 19 | Canonical FTS join indexes and planner proof | Landed in PR #207 at `e95afd292561d203d1001ea992ecbc191e129536`; reviewed source ends at `550c4b03`, and fixture-scoped closure evidence is recorded in `0.8.22-slice-19-join-index-measurement-20260808.json`. |
| 21 | Truthful projection runtime state and safe boot graft | Landed in PR #207 at `e95afd292561d203d1001ea992ecbc191e129536`; FIX-1 through FIX-5 and the post-integration test-only FIX-1 through FIX-3 correction are closed. |
| 22 | Governed pure projection-status read | Landed in PR #207 at `e95afd292561d203d1001ea992ecbc191e129536`; C5 seq-247, RED→GREEN→FIX-1, isolated verification, independent re-review, and refreshed CI are complete. |
| 23 | Direct FTS result-prefix stability | Landed in PR #209 at `f1ccf2694087e1da4cee2204fe7b80389420a4b0`; RED→GREEN, cross-SDK verification, independent FIX-1 closure, repeat documentation review, and CI run #31265399431 are complete. |
| 20 | Ordered publication and registry smokes | Published from `93cd1a14` after recovery verification. |
| 25 | `next` to `latest` promotion and release truth | Complete: `fathomdb@0.8.22` is npm `latest` and `next`. |

## Immediate next action

| | |
| --- | --- |
| **Final action** | Published and closed; do not reopen this release board. |

## Slice 22 pickup gate

The independent pickup review is recorded in
`dev/plans/runs/0.8.22-slice-22-pickup-review-20260807.md`. The C5 signature
is recorded at steward-ledger seq-247, and the independently approved
governed-surface preparation is recorded in
`dev/plans/runs/0.8.22-slice-22-c5-prep-review-20260808.md`. Runtime review
and FIX-1 re-review are recorded in their paired Slice 22 review files. The
implementation ended at `6aeee48e` and landed through PR #207 at `e95afd29`.

## Slice 20 publish hold

The local release-safety preparation is closed at `2f94085c`; its pickup and
three review records document the RED→GREEN→FIX-2 closure. No production
workflow changed. The remaining Slice 20 action is real ordered publication and
registry smoke, which remains held pending new explicit release authorization.

## Slice 23 landed P2 repair

HITL ruling seq-248 required a direct FTS-only result-prefix repair before
publication. The completed design and executable plan are
`dev/design/0.8.22-slice-23-text-limit-prefix-stability.md` and
`dev/plans/prompts/0.8.22-slice-23-text-limit-prefix-stability.md`. They limit
the work to `search_text_only`; hybrid/vector candidate-fanout semantics are a
separate 0.8.23 architecture/documentation follow-up. The independent design
review and FIX-1 closure are recorded in
`dev/plans/runs/0.8.22-slice-23-design-review-20260808.md`.
Its pickup review, RED→GREEN proof, cross-SDK verification, and independent
FIX-1 review closure are recorded in `dev/plans/runs/`. PR #209 landed the
repair at `f1ccf2694087e1da4cee2204fe7b80389420a4b0` after CI run #31265399431
passed; the repeat documentation-correctness review is approved.

## Completion documentation gate

Before any 0.8.22 completion claim, independently verify that the release-state
JSON, rendered plan and STATUS board, affected `dev/` records, and affected
public `docs/` match the final implementation and release witnesses. Repair
material drift and run the applicable documentation checks; code or CI success
does not bypass this gate.

**Local result (2026-08-08): PASSED.** The independent final review of
`cbd2b725..282bfe45` found one P2: its completion witness stopped at
`ad14c879`, before the integrated Slice 19 closure record and CI-contract
correction review. Final-FIX-1 at `c68016b9` corrected that state, and its
focused independent re-review found no P1/P2. Release-state rendering,
developer Markdown checks, and public-doc checks passed. This is a local
integration and landing result: the explicitly held real publication/smokes
remain outstanding.

**Post-landing reconciliation (2026-08-08): PASSED.** The state JSON, rendered
views, prompts, designs, developer indexes, and public-document indexes now
record Slices 19, 21, and 22 as landed in PR #207. Two P2 stale-status rounds
were corrected and the final independent re-review found no P1/P2.

**Slice 23 repeat review (2026-08-08): PASSED.** The independent review of
the Slice 23 candidate found no P1/P2 in its lifecycle/status documentation,
rendered views, developer indexes, or public contracts. Its durable record is
`dev/plans/runs/0.8.22-slice-23-documentation-correctness-review-20260808.md`.
Slice 20 publication/registry smokes and Slice 25 promotion remain held.

---
title: FathomDB 0.8.28 Slice 50 — status
status: provider subunit complete; scan Unit S remains open
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 status

> **Current direction (2026-11-01, owner `seq-359`):** the narrowed
> built-in-provider contract is implemented at `db2baf1ec420dae6680315fcb911ca30e94ddc32`, with separate
> Astra-medium APPROVE and full verification coverage plus scoped repairs.
> Existing cache behavior is preserved. See the
> [implementation record](built-in-provider-implementation-record.md).
> Scan Unit S remains open; D50-1/D50-2 remain held and unimplemented.

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-50-drain-scan-bound` (base `544bf74f8`, `dev/0.8.28`). The
provider subunit is recorded below; the scan work remains open. Phase-1 docs
were pushed at `ac83688a1`; the subsequent
owner-requested characterization does not implement D50-1 or D50-2.

| Step | State (2026-11-01) |
| --- | --- |
| P1 phase 1 docs | done at ac83688a1 |
| R-DES design review | narrow provider contract independently reviewed; scan Unit S review not started |
| O replacement contract | complete at db2baf1ec; separate Astra-medium APPROVE; D50-1/D50-2 held unselected |
| H hook RED/GREEN | not started |
| S scan RED/GREEN | not started |
| T scaling tool | not started |
| D drain RED/GREEN (and D2 pairs) | not started |
| DOC records | provider implementation record complete; scan records pending |
| R-CODE independent review | provider subunit Astra-medium APPROVE; scan review pending |
| V verification | provider full-run coverage plus scoped repairs complete; scan qualification pending |
| L source | provider subunit db2baf1ec; scan work remains open |

## Owner-requested characterization

The owner requested baseline tests and committed tools/data before ruling on
the drain options. Tools: `5fb45c24a`; purge fixture correction: `caecfb831`.
[Experiment and analysis](../../../../experiments/slice50-refusal-baseline/README.md)
record the refused/empty, refused/pending, absent/pending and accepted/ready
matrix, recovered original probe output, and the design history. That characterization changed no engine source. D50-1/D50-2 remain held;
the later approved provider implementation is recorded below.
This investigation is not a slice close or release verification.
Owner direction following characterization: steward ledger `seq-350`,
release-state decision `slice-50-successful-open-investigation`. The replacement
open contract was subsequently approved at `seq-359` and implemented as recorded below.
The [replacement contract proposal](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
was subsequently accepted at `seq-359`. The owner-requested
[fresh-open timing results](../../../../experiments/fresh-open-verification/findings.md)
quantify the earlier fresh-check proposal (steward `seq-352`); `seq-357` preserves
the existing persistent cache and cadence. Production pooling stays
Mean under the existing ruling. The later narrowed implementation preserves that policy.

## Historical unselected drain options

- **D50-1.** On a dense-disabled store with pending vector rows, `drain`
  returns promptly. What it returns: A `Err(Scheduler)` (recommended),
  B `Ok(())`, or C a new typed error. See design § 2.3.
- **D50-2.** Whether lifecycle `transition`, `configure_projections`,
  lifecycle actuation and erasure on such a store proceed, as they do
  for an absent runtime (recommended: yes, in this slice). Today they
  wait 30 s and fail. See design § 2.4.

## Phase 1 findings

- **Todo 289 is not in the two queries.** It is in `Engine::drain`
  (`runtime_lifecycle.rs` 57–95), which sends a refused runtime to
  `wait_for_idle`. The master-plan bullet is corrected.
- **On SQLite 3.54.1 the arms already stream in index order.** The cost
  is the top-level sort, which consumes the whole `UNION ALL`
  co-routine before `LIMIT`. Per-arm `LIMIT` bounds it. In a scratch
  probe, page VM steps went from 38,110 and 287,110 (1k and 8k pending)
  to 20,825 and 28,394, with 496 identical pages (design § 1.3).
- **The 33 s to 5.7 s figure is CS64 (Slice 60's layout).** This slice
  measures on format 34 under a rule stated in advance (test-plan § 4.3).
- **The same wait breaks lifecycle mutations on a refused store.**
  `transition` took 30.000 s, then returned `Err(Scheduler)` (scratch
  probe). That is the reason for D50-2.

Original phase-1 scratch probes ran in a `git archive` export of `544bf74f8`.
The later recovered probe record, characterization tools/data and fresh-open
measurements are committed in the experiment directories linked above.

## Built-in-provider design proposal

Owner scope `seq-355`, prior proposal `seq-356`, and scope correction `seq-357`.
Astra-medium revised the [contract](built-in-provider-design.md),
[requirements/AC](built-in-provider-requirements.md) and
[plan](built-in-provider-plan.md) to preserve existing checks/cache behavior and
close production provider gaps, with typed failed-open behavior proposed only on
actual refusal. Prior [integration review](built-in-provider-astra-review.md)
and [reuse review](verification-reuse-astra-review.md) are superseded historical
records, not current approval. Separate [independent Astra-medium review](built-in-provider-narrow-review.md)
returned APPROVE_FOR_OWNER_REVIEW with no P1/P2 findings; contract and release
placement were subsequently approved at `seq-359`. Source implementation is
recorded below; no schema or release-ladder completion change is implied. The known provider-free
dense rebuild defect remains deferred for separate owner disposition.

## Approved narrow implementation

Owner approval: steward `seq-359`, following the separate narrow design review.
Implementation worktree: `../slice-0.8.28-50-provider-contract`, branch
`impl/0.8.28-built-in-provider-contract`, source baseline `2d63cee3c`.
[Handoff](built-in-provider-implementation-brief.md) pins scope and TDD. Existing
cache settings and behavior remain fixed. Separate Astra-medium implementation
review and final strict verification gate landing on `dev/0.8.28`. This does not
close scan Unit S, advance the Slice 50 ladder or authorize later slice work.

## Provider subunit completed

Source `db2baf1ec420dae6680315fcb911ca30e94ddc32` implements the approved narrow contract.
[Implementation, TDD, review and verification record](built-in-provider-implementation-record.md).
The separate Astra-medium review approved the exact source; all failures in the
205-suite full run have scoped repairs and passing reruns. The original full
run remains nonzero, so this is not a whole-gate pass at final HEAD.
Slice 50 remains IN_PROGRESS pending scan Unit S.

## Scan Unit S execution (2026-11-01)

Reconciled requirements, design and test plan against `630af0bfc`; retired the
obsolete refused-handle options from active scope. Independent Astra design
review: APPROVE, no P1/P2 findings. Clarified that bounds limit accepted arm
output, not rejected index visits; scaling remains a measured qualification.
Implementation proceeds with baseline-equivalence guards and scan-cost RED/GREEN.

Scan RED `4d1134da6`: plan lacked bounded arm co-routines; VM ratios were
7.7782 scheduler and 7.8626 candidate. GREEN `65b8fd281`: all five focused
tests pass, including 64 generated every-cursor cases and three public-write
ordering/drain layouts. Both VM ratios are exactly 1.0 (26,978 scheduler;
24,314 candidate). LIMIT=1 fails randomized equivalence; DESC fails the
600-row arm-dominant ordering guard. Mutations and their regression artifact
were removed. Module-boundary validation passes. Scaling qualification and
independent final verification remain pending.

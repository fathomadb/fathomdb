---
title: FathomDB 0.8.28 Slice 50 — status
status: design investigation; built-in-provider replacement proposed; no implementation authorized
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 status

> **Current direction (2026-11-01):** owner scope `seq-355` limits production
> providers to the built-in implementation before 1.x. Read the revised
> [user needs, requirements and AC](built-in-provider-requirements.md),
> [design and extension inventory](built-in-provider-design.md), and
> [design work plan](built-in-provider-plan.md) before the original phase-1
> material below. Unit S is unchanged. Unit D / D50-1 / D50-2 are historical,
> unselected options, held without implementation; their recommendations are
> not the current proposal. The replacement contract and release placement
> require owner review before production changes.

Step log for [plan.md](plan.md). Commits are on
`slice/0.8.28-50-drain-scan-bound` (base `544bf74f8`, `dev/0.8.28`). Not
landed. Phase-1 docs were pushed at `ac83688a1`; the subsequent
owner-requested characterization does not implement D50-1 or D50-2.

| Step | State (2026-10-31) |
| --- | --- |
| P1 phase 1 docs | done (this commit) |
| R-DES Fable design review | not started |
| O replacement contract | built-in-provider proposal awaiting owner review; D50-1/D50-2 held unselected |
| H hook RED/GREEN | not started |
| S scan RED/GREEN | not started |
| T scaling tool | not started |
| D drain RED/GREEN (and D2 pairs) | not started |
| DOC records | not started |
| R-CODE codex review | not started |
| V verification | not started |
| L land | not started |

## Owner-requested characterization

The owner requested baseline tests and committed tools/data before ruling on
the drain options. Tools: `5fb45c24a`; purge fixture correction: `caecfb831`.
[Experiment and analysis](../../../../experiments/slice50-refusal-baseline/README.md)
record the refused/empty, refused/pending, absent/pending and accepted/ready
matrix, recovered original probe output, and the design history. Engine source
is unchanged. D50-1/D50-2 implementation and slice design review remain paused.
This investigation is not a slice close or release verification.
Owner direction following characterization: steward ledger `seq-350`,
release-state decision `slice-50-successful-open-investigation`. The replacement
open contract remains undecided; existing ADRs remain in force.
The [replacement contract proposal](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
is submitted for owner review before implementation. The owner-requested
[fresh-open timing results](../../../../experiments/fresh-open-verification/findings.md)
now quantify the cache decision (steward `seq-352`); production pooling stays
Mean under the existing ruling. No policy change is implemented.

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

Owner scope `seq-355` and proposal record `seq-356`. Astra-medium developed the
[built-in-provider contract](built-in-provider-design.md); the integrated
[requirements/AC](built-in-provider-requirements.md) and
[work plan](built-in-provider-plan.md) accompany it. Final
[Astra integration review](built-in-provider-astra-review.md): coherent proposal
for owner review; the three consistency corrections were applied. No production
source, dependency, schema or ladder-completion change. Detailed contract and
release placement remain pending owner review before implementation.

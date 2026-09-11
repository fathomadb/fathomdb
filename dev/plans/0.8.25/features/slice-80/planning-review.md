# Slice 80 planning reconciliation

## Changes since the draft

1. Slice 79 closed at `323db678`; its shipping candidate keeps performance-mode
   MEMSTATUS=0 and statement reuse, while its old AC-020 campaign remained 0/7.
2. Slice 77 established that more ratio tuning lacked adequate residual
   attribution. No treatment was selected.
3. Owner ruling seq-277 retired AC-020 and fixed independent absolute budgets,
   warning semantics, the seven-run campaign and the three-run AC-072 obligation.
4. The acceptance registry has no AC-081 entry, so AC-081a/b/c are the next
   unused identifiers. The split preserves the one-assertion-per-AC rule:
   sequential budget, concurrent budget and reader independence. Existing
   AC-080 belongs to erasure completeness.
5. Existing reader-pool tests prove worker count, lifetime and loss-free routing,
   but not same-engine progress while a peer owns a live SQLite snapshot. Existing
   test hooks can prove that gap without changing shipping dispatch.
6. Slice 79's AC-072 results are numerically green but environment-invalid.
   Its six protected write cells remain applicable unless product/build inputs
   change.
7. Historical AC-020 scripts and workflow labels remain in the tree. Historical
   result parsers stay intact; active selectors move to AC-081.

## Verdict

Approve the draft with these bounded adjustments:

- allocate AC-081a/b/c and seal the exact commands/counts in
  `execution-manifest.json`;
- add a pure, nanosecond-precise local oracle plus a small receipt validator;
- add one real-database same-engine reader-independence witness;
- preserve historical AC-020 evidence while removing it from active release
  acceptance; and
- run only the focused tests, seven AC-081 processes and three valid AC-072
  cells. No product optimization or broad verification is authorized.

The plan is complete without another experiment, platform campaign or packaging
change. Slice 85 owns final broad verification.

## Contract-migration inventory

The active migration covers `dev/acceptance.md`, `dev/test-plan.md`,
`dev/traceability.md`, `dev/design/perf-gates.md`, the reader-pool and obsolete
AC-020-lever ADRs, the ADR index, active runner/workflow selectors, changelog,
release state and the Slice 85 handoff. Historical Slice 75–79 receipts and
the Slice 76/77 AC-020 evidence parsers remain byte-preserved history.

---
title: FathomDB 0.8.27 Slice 85 - execution plan
status: APPROVED
target_release: 0.8.27
baseline: 8b2a9edaf
---

# Slice 85 execution plan

## Reconciliation since the drafted plan

The binding Slice 85 design was reviewed at `d1918892` and approved by the
independent GPT-6 Astra medium review recorded in `design-review.md`. The
following changes landed between that candidate and this execution baseline:

1. `ceffa56d` recorded the independent design review.
2. `82bde3de`, `4c75bfec`, and `07941a47` recorded owner authorization,
   capable-host entry receipts, and execution commission.
3. `1ca95ba1`, `f7f60fd6`, and `8b2a9eda` completed prospective Slice 90
   planning and accepted the D27 runtime-topology successor ADR.

The reconciliation inspected the current release state and board, Slice 80
plan/design/status/reviews, the complete Slice 85 master-plan section and
review records, the Slice 90 design and accepted ADR, current engine sources,
the affected source-scraping checks, and normal lint/dependency/license entry
points. There are no production, test, script, workflow, manifest, lockfile,
or `AGENTS.md` changes after `d1918892`. The assigned Slice 85 functions and
carriers therefore have not drifted. The Slice 90 documents consume Slice 85's
settled boundaries and do not add Slice 85 work.

Two preallocated files whose names also contain `slice85` were reviewed:
`scripts/release/verify-slice85-manifest.py` and
`scripts/tests/test_slice85_final_gate.py`. They are the closed 0.8.25 final
evidence gate (their paths, schemas, versions, and obligations are explicitly
0.8.25), not drafts for 0.8.27 ENGINE-BOUNDARIES. Reusing or revising them is
rejected as cross-release scope contamination. The 0.8.27 boundary gate gets
its own narrowly named checker and mutation test.

## Plan disposition

The reviewed Slice 85 design is **approved without scope expansion**. Its
needs, R27-85A-G requirements, AC27-85A-G acceptance criteria, final owner
homes, enforced source grammar, and ordered batches remain binding. The D27
runtime ADR is explicitly excluded: configuration forwarding, runtime pool
capacity, embed deadlines, shutdown, and open/facade closure remain Slice 90.

The draft is adjusted only operationally:

- this file is the concise execution and TDD record; the Slice 85 section in
  `dev/plans/plan-0.8.27.md` remains the non-duplicated design authority;
- the already-passing exact baseline in `baseline-verification.md` satisfies
  implementation-order step 1 and must not be rerun merely for ceremony;
- focused compile/tests, the boundary report, and affected source-scraper
  checks run after each cohesive batch; broad verification waits for the
  integrated candidate; and
- no AC-037 claim is made here. Slice 150 retains final live qualification.

No need, requirement, or acceptance criterion is rejected. No additional
public API, schema, runtime behavior, broad import cleanup, or binding work is
authorized.

## TDD and implementation

Implementation follows the six reviewed stages in the master plan:

1. Preserve the recorded exact baseline.
2. Build the standalone `syn` boundary gate in RED/GREEN sub-batches: module
   discovery/classification; imports and item/root edges; Engine fields,
   methods, and callable references; inherent-method policy and negative
   fixtures. Wire the fast gate into normal lint with cache-staleness,
   dependency, and license coverage.
3. Move `structural_state`, `wal_attribution`, then `reader_transaction`.
4. RED-characterize exact filter/graph/read/search error mappings and refusal
   order; GREEN the narrow owner errors, search constructors, and pool request
   factories/families.
5. Move read, graph, and search-expansion facades; make governed imports
   explicit per module and eliminate all four prohibited Slice 80 cycles.
6. Freeze reviewed policy, turn enforcement on, run production mutants and
   focused blast-radius checks, then run the proportionate integrated gate.

For every behavior-bearing change, commit or retain a failing test/fixture
before the implementation that makes it pass. Mechanical moves use compile
failure as RED and preserve bodies byte-for-byte where practical. Tests are
not weakened to accommodate moves. The TDD chronology records RED command and
failure, GREEN command and pass, and any mutant killed.

## Review and closeout

After implementation, an independent GPT-5.6 Sol high reviewer examines the
whole Slice 85 diff against `8b2a9eda`. Findings are fixed with focused
RED/GREEN coverage and rereviewed with GPT-5.6 Sol high. An independent
GPT-5.6 Terra verifier then checks the exact reviewed candidate and runs the
proportionate verification matrix. Closeout writes `tdd-chronology.md`,
`code-review.md`, `review-verification.md`, and `status.md`, updates the
release-state JSON and generated views, and leaves the release worktree clean.

# Follow-up results — 2026-11-02

The experiments support a maintained current lexical family for healthy default
reads. They do **not** support universal per-call historical materialization.
The private native backend remains a correctness reference and a possible
explicitly bounded selected-corpus path; these measurements establish no safe
production cap and no historical request SLO.

Production implementation and accepted contracts remain unchanged. A separate
ordinary-Engine diagnostic exposed live node-FTS term retention after successful
erasure; that is an adoption blocker, not a candidate-only performance finding.

## Functional aspects versus probes

| Functional aspect | Evidence | Remaining qualification |
| --- | --- | --- |
| Default score/candidate/graph-seed isolation | P2 native node collector has exact score/order and VM-work invariance across H0/4/16 | Public fusion/BM25F, edge collector, graph seed and integrated default path not qualified |
| World change, correction, prior belief | P1 real-schema fragment model and P3 actual receipt-cut fixture selectors pass | Arbitrary production batch mapping, fragment API/evidence carrier and source-specific competing claims require successors |
| Temporally coherent graph traversal | Original isolated graph oracle passed | Real Engine temporal graph not implemented or qualified |

| Follow-up probe | Status | Boundary |
| --- | --- | --- |
| P1 mapping | Scoped tests passed | Exclusive adapter; one temporal write per recorded cut; node-only ledger; G11/head/evidence gaps remain |
| P2 certified reads | Healthy native node SCOUT passed; fresh candidate erasure repaired | Not integrated AC-076/081; refresh can depend on history; no P1 fragment composition |
| P3 resources | PARTIAL campaign with native reference parity and real selector witnesses | 5/18 revised cells complete; no production backend selection or historical SLO |

P1 and P2 are not composed: P2 follows canonical current heads, not selected
world-time fragments. The first production integration RED must make old A
remain world-current at V10 after canonical head B becomes effective at V20,
then select B at V30, including source denial, frozen reads and erasure.
A standalone ledger pass plus a current-index pass does not establish that behavior.

## P2 observations

All three T012-derived governed-identity fixtures had 10,000 current nodes and
1,000/1,000 successful measured requests. Healthy writer contacts were zero;
certificate work was 16 VM steps and the per-query ranking VM arrays were equal.

| Prior versions | p50 ms | p99 ms | Bootstrap ms | Boundary refresh ms |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.639309 | 1.642525 | 89.683245 | 93.085927 |
| 4 | 0.615193 | 1.583613 | 123.122256 | 94.360797 |
| 16 | 0.602489 | 1.578474 | 246.910693 | 106.230826 |

Replay on copies of the measured stores compared actual current
(logical_id, body, state, source_id) tuples, then all 1,000 original seeded queries.
Every query was nonempty. Ordered logical IDs and exact f64 BM25 score bits were
identical across H0/4/16. Body lengths/counts were 439/2034, 471/1966, 503/1984,
535/2025 and 567/1991. Original databases were untouched. This establishes numerical
isolation for the tested native node collector, not merely equal query work.

Refresh is conservative: current-head scan plus dirty/frontier/dependency
expansion, potentially history-dependent. Ordinary visibility changes can trigger
full current-family reconciliation. Queue/admission waits have experiment
budgets, but an executing publication runs to completion. Healthy-read timings
do not qualify write-heavy workload latency or integrated AC-076, AC-081a/b/c.
The canonical anonymous T012 identity shape differs from governed versionable
identities here. No unchanged shipping gate is claimed as candidate qualification.

## P3 revised diagnostic

Frozen measurement commit: `2e10301ad95632c1a25165d6868b81086bb4081c`.
Release build, rusqlite 0.41.1 / bundled SQLite 3.54.1, rustc 1.95.0;
Threadripper PRO 5945WX, 12 cores/24 logical CPUs, 65,685,524 KiB RAM.
No competing builds/tests ran during timing. Exact host details and manifests
are in `/tmp/current-history-followups-results/host-build.json` and `p3-stress/`.

Every cell scheduled exactly 1,000 requests with 600 seconds maximum and an
8-GiB sampled whole-process RSS stop. All revised cells use 4,096-byte indexed
scalars. The revised phase consumed 6,640.039 seconds plus the conservatively
charged original 560 seconds: 7,200.039 seconds combined, including shutdown
overshoot. The child was joined; no orphan process remained. There were no request
errors or RSS stops. Eight cells were incomplete and five not run; these are data,
not replaced attempts. Missing percentiles are deliberate.

| H | Callers | View | Family | Success / unfinished | Status | p50 / p99 ms | Peak RSS MiB |
| ---: | ---: | --- | --- | ---: | --- | ---: | ---: |
| 16 | 8 | both | node | 1000 / 0 | complete | 4341.645 / 4920.592 | 1111.96 |
| 16 | 8 | both | edge | 1000 / 0 | complete | 4251.545 / 4726.718 | 1146.45 |
| 16 | 8 | both | property | 463 / 537 | incomplete | — | 2288.98 |
| 16 | 1 | both | node | 139 / 861 | incomplete | — | 146.23 |
| 16 | 1 | both | edge | 146 / 854 | incomplete | — | 146.21 |
| 16 | 1 | both | property | 66 / 934 | incomplete | — | 288.62 |
| 0 | 1 | both | node | 1000 / 0 | complete | 240.932 / 259.156 | 34.39 |
| 0 | 1 | both | edge | 1000 / 0 | complete | 237.083 / 252.928 | 34.61 |
| 0 | 1 | both | property | 1000 / 0 | complete | 486.901 / 529.086 | 51.23 |
| 4 | 1 | both | node | 471 / 529 | incomplete | — | 62.95 |
| 4 | 1 | both | edge | 506 / 494 | incomplete | — | 67.93 |
| 4 | 1 | both | property | 227 / 773 | incomplete | — | 105.11 |
| 16 | 1 | corrected | node | 459 / 541 | incomplete | — | 18.53 |
| 16 | 1 | corrected | edge | 0 / 1000 | not_run | — | — |
| 16 | 1 | corrected | property | 0 / 1000 | not_run | — | — |
| 16 | 1 | prior | node | 0 / 1000 | not_run | — | — |
| 16 | 1 | prior | edge | 0 / 1000 | not_run | — | — |
| 16 | 1 | prior | property | 0 / 1000 | not_run | — | — |

C8 node/edge completing where C1 did not reflects the fixed 1,000-completion
throughput criterion, not superior per-request latency. Their C8 medians are
about 4.3 seconds. Partial C1 completed-duration totals imply similar seconds of
work per request; no qualified partial-cell p99 is reported. H16 property both
selects 343,400 documents and 1,406,566,400 UTF-8 bytes per request. Contentless
FTS removes the private content table, not retokenization or posting construction.

Corrected H16 node selected only 10,000 documents (40,960,000 bytes), completing
459 requests in the residual 299.721 seconds. Its 299,450.938 ms accumulated
successful-request duration includes real temporal selection, population,
ranking, hydration and drop. It is incomplete, not a qualified latency cell.

The original 252-cell campaign is preserved separately in `p3/`: 11 complete,
one interrupted after 535 successes, 240 not run. Fresh cells 0–2 configured
Performance; cached cells used a different implicit runtime profile. Those
heterogeneous observations are not pooled with the corrected diagnostic.
The fresh/cached process defect has its own observed RED and fixed process tests.

Large payloads contain repetitive compressible text, not a vocabulary-entropy
stress distribution. Whole-process RSS includes fixture setup/Engine state and
excludes OS filesystem cache; it is not isolated private-corpus allocation or an
entropy-independent bound. The benchmark has unique bodies; duplicate-heavy
body hydration cost remains unmeasured. Body dedup may hydrate more than returned
K; property revision collection hydrates at most K winners. Resource admission
must account for candidates examined and hydrated, not only returned hits.

Private corpus temp storage is explicitly MEMORY. The main read-only connection
has query_only but no equivalent blanket temp-store policy: the private witness
does not prove every main selector can never spill canonical bytes. Production
must enforce that separately before promising no body-bearing temp files.
Snapshot ownership and error cleanup are tested; process-memory/swap zeroization
and cancellation of active work are not promised. External P3 readers can finish
after Engine.close; this is not integrated reader-pool shutdown qualification.

Native regular-versus-contentless score/order parity uses the production safe
compiler/tokenizer, including Unicode, punctuation, stops, stems, zero tokens and
all-name property statistics. This is not parity of public fusion/BM25F/graph
output. Exact selected ownership and same-snapshot hydration are tested; full
production temporal/evidence routing remains unqualified.

## Erasure findings

The candidate P2 current cache initially retained erased content until a later
refresh. RED `e88742bd3` reproduced that immediately after actual Engine erasure.
GREEN `05a1f254b` enables per-table FTS secure-delete before population and deletes
candidate rows in the canonical deletion transaction. Fresh-fixture tests cover
rollback, content/MATCH/live-shadow absence before search, retained survivor,
pinned-WAL ErasureIncomplete/retry, and no refresh resurrection. This qualifies
fresh fixed candidates; it does not retroactively clean older experimental files.

Separately, opt-in diagnostic `5a12534e9` uses an ordinary fresh Engine with no
candidate schema. A nonstemming target term was present in one live
`search_index_data` block before erasure. `erase_source` returned success;
canonical/content/MATCH target counts became zero, the survivor remained, and WAL
size was zero. The literal target term remained in **two live shadow blocks**.
The physical-erasure assertion fails 2 versus 0. The initial fixture token ended
in `y` and was stemmed, so the prepresence guard correctly refused it; the final
nonstemming `x` fixture passes prepresence and demonstrates retention.

This is a witnessed shipping node-FTS path, not proof about every projection
family or unpopulated v2 index. Production code is unchanged. A separately reviewed
remediation must cover per-table secure-delete policy, cleanup of already retained
terms, registered-family completeness, write/erase cost and format/version
compatibility. Logical row counts and WAL truncation alone do not establish
physical term removal. The default experiment suite excludes this explicitly
opt-in diagnostic; its known FAIL must remain visible beside any suite PASS.

## Decision and next evidence

Keep current-index routing separate from historical-backend choice. Shipping
node/edge body search already ranks retained native FTS; P3 private rebuilding
measures a hypothetical selected-corpus backend, not that shipping path.
First test a certified direct-native full-history fast path where the physical
corpus exactly matches the admitted statistical scope. Certification must cover
source authority, validity, lifecycle, tokenizer and projection membership,
including all declared property names; equal row counts are insufficient.
Historical properties may lack such a maintained corpus.

For arbitrary V/K, compare query-term-scoped persistent statistics or finite
maintained partitions against this private native reference. Avoid full vocabulary
or body scans; exact phrase DF, stemming collisions and literal compiler fallback
remain hard requirements. Finite partitions add writer/erasure/frontier maintenance
and cannot express every temporal selection. No alternative has yet won that probe.

Shipping open limits remain history-coupled body BM25 statistics, unbounded private
property cost and pinned-WAL/run-to-completion behavior. Slice80 fixed historical
dense completeness/false legacy Skip, lexical-only boot visibility and quadratic
FTS inventory checks. G11 competing-claim scope, canonical head versus fragments,
atomic batch cuts and evidence fragment carriers require successor contracts.
Prototype intent/certificate/runtime/cache findings are distinct from shipping
findings; the ordinary-Engine residual-term witness is explicitly shipping.

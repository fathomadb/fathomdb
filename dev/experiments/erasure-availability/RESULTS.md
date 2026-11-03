# Slice 81 results

The bounded investigation supports incremental registered-FTS sanitation as a
successor candidate after successful native erasure. It does **not** qualify
shipping erasure, native availability or a deletion SLA. All 72 scheduled attempts
have dispositions: 34 complete and 38 incomplete, with no replacement or retry.
The shipping physical-erasure finding remains open. Final repository gate and
landing are pending; this report records the frozen local experiment.

## Scope and reproducibility

Actual Engine format-35 files used SQLite 3.54.1, release/performance mode,
porter/unicode61 tables and seed `0x81f00d` on Linux x86_64 host `windchill3`.
Each master had 10,000 owners, four old plus one current revision, 1,024-byte
primary payloads and 1,000 target owners. Edge fixtures used 10,001 isolated
endpoint nodes; property fixtures had 20,000 current property rows. Exact counts,
payload lengths and family-specific positive markers were checked on actual
large inputs. `search_index_v2` was populated, including node copies in property
fixtures. Unused families were recorded empty, never counted as positive proofs.
All nine masters and the binary remained hash-identical across segments.

A fresh enabled FTS secure-delete before population; B fresh retained the default.
Both invoked unchanged native erasure before sanitation. Legacy A rebuilt four
registered FTS tables; B performed negative-first, positive-continuation merges
with durable progress under the real writer mutex (64-page units, 1-ms gaps).
Timed proof required expected survivor MATCH counts, no marker in registered
shadows/main/WAL, and a nonbusy checkpoint after foreground join and projection
drain. Exact survivor IDs/content were verified by separate safety tests, not
recomputed as a full-content oracle in each timed replica. The masters
are independent control inputs outside each disposable replica's erasure scope.
Backups, snapshots, swap, telemetry sinks and already returned bytes are not proved.

Raw root: `/tmp/fathomdb-slice81-campaign`; test/RED/GREEN logs:
`/tmp/fathomdb-slice81-evidence`. Commands are in [README](README.md).
Original probe source: `92bd89d0e2d5e3c168352474c94303254b01a6f0`;
probe SHA256: `77ae0ddbe51011f43080fb44538b942da7788f9dbecd7b2814f8ab01acb144d5`.
Continuation controller: `65625d3953b5415b683299e1067f4fbd9b4887b3`;
recovery controller: `e632d5e9a65379622dbde32cf2591f81a34917a1`.
Each manifest records protocol, source, master and binary hashes; each attempted
replica records raw log/event digests. These temporary raw paths require retention
for later reproduction; committed tools and this summary do not embed all raw data.

| Segment | Active seconds | Complete / incomplete / never started within segment | Manifest SHA256 | Results SHA256 |
| --- | ---: | --- | --- | --- |
| original | 2700.154533 | 10 / 13 / 49 | `708ff15593abef6f5eaa8550888907453d502279d49c69a70604ace5dccacf81` | `549bd6a14634cec843e3eb65af478b403b3869628a81322c124b5d60724362a2` |
| supplement | 908.895636 | 13 / 25 / 11 | `b2337c48ea0be47569522bdfb1e8df10c9b23a73fcdb0cccfee19f1676ccd634` | `ca9dac8be3431f27005710643e81204dcd0e2839e12f10359dfd0740200bf62b` |
| supplement-recovery | 204.941806 | 11 / 0 / 0 | `4aecc84ac65ca8cd0a400a587addfa4b84a9d3456926cf8245249df10a761862` | `d54dfc7eaafa0d76784824e5e842d6a9ab93beb89f528cc4183b39556f21da7a` |

Original 45-minute exhaustion preserved all outcomes. The separately reviewed
25-minute continuation selected only original never-started IDs and retained
90-second cell spending. It stopped conservatively on a `/proc` permission race
at child exit. Recovery selected only the remaining eleven never-started IDs,
carried all spending and used 591.104364 seconds of available allowance; it
consumed 204.941806 seconds. Combined active time was 3,813.991975 seconds;
original-start to final-finish wall span was 4,661.407677 seconds. No threshold,
workload, probe or input changed. The sole controller correction tolerated only
that child's `/proc` permission failure after independently proving child exit;
live-child failures still stop and reap. Original property A idle replica 0 stays
incomplete despite a successful child payload: no retrospective promotion.

## All scheduled cells

Replicas are listed in original order 0/1/2. C = physical completion; T = replica
or cell time limit; G = original campaign limit; M = monitor failure. Every T/G/M
is incomplete, not a physical rejection or a passing sample. All 36 fresh
attempts stopped during native erasure; none reached completed sanitation.

| Cell | Family | Workload | Load | Arm | Replicas 0 / 1 / 2 |
| --- | --- | --- | --- | --- | --- |
| 0 | node | fresh | idle | A | T / T / T |
| 1 | node | fresh | idle | B | T / T / T |
| 2 | node | fresh | loaded | A | T / T / T |
| 3 | node | fresh | loaded | B | T / T / T |
| 4 | node | legacy | idle | A | C / C / C |
| 5 | node | legacy | idle | B | C / C / C |
| 6 | node | legacy | loaded | A | C / C / G |
| 7 | node | legacy | loaded | B | C / C / C |
| 8 | edge | fresh | idle | A | T / T / T |
| 9 | edge | fresh | idle | B | T / T / T |
| 10 | edge | fresh | loaded | A | T / T / T |
| 11 | edge | fresh | loaded | B | T / T / T |
| 12 | edge | legacy | idle | A | C / C / C |
| 13 | edge | legacy | idle | B | C / C / C |
| 14 | edge | legacy | loaded | A | C / C / C |
| 15 | edge | legacy | loaded | B | C / C / C |
| 16 | property | fresh | idle | A | T / T / T |
| 17 | property | fresh | idle | B | T / T / T |
| 18 | property | fresh | loaded | A | T / T / T |
| 19 | property | fresh | loaded | B | T / T / T |
| 20 | property | legacy | idle | A | M / C / C |
| 21 | property | legacy | idle | B | C / C / C |
| 22 | property | legacy | loaded | A | C / C / C |
| 23 | property | legacy | loaded | B | C / C / C |

Native work is not inferred idle from timeout: source inspection shows per-target
projection dispatch and unindexed FTS cursor deletes, but this is not a profiling
claim about dominant cost. Legacy preparation, including historical native erase,
was charged separately and equally before paired arm copies. Preparation took
266.16 seconds for node legacy, 145.04 for edge legacy and 775.21 for property
legacy; property default/secure population took 439.97/510.91 seconds. These costs
explain the original scheduling failure and are not omitted from lifecycle cost.

## Writer occupancy and service

Values below are per-replica maximum maintenance writer-held milliseconds,
not native call durations. A dash is unavailable, never zero. The frozen rule
requires three complete pairs, a median reduction of at least 25%, and consistent
direction in every pair. It passes node idle, edge idle/loaded and property loaded;
node loaded and property idle remain unqualified because one pair is missing.
Fresh occupancy comparison is unqualified. This screening result does not require
or establish refusal-free foreground service.

| Legacy family/load | A held ms (0/1/2) | B held ms (0/1/2) | Screening |
| --- | --- | --- | --- |
| node/idle | 1003.628 / 975.594 / 985.399 | 77.397 / 79.253 / 83.131 | PASS |
| node/loaded | 880.632 / 885.720 / — | 20.294 / 11.236 / 12.745 | UNQUALIFIED: missing pair |
| edge/idle | 500.428 / 511.921 / 505.656 | 64.594 / 89.394 / 66.822 | PASS |
| edge/loaded | 438.135 / 434.509 / 437.683 | 9.835 / 10.456 / 10.055 | PASS |
| property/idle | — / 2162.441 / 2233.689 | 248.594 / 262.515 / 250.420 | UNQUALIFIED: missing pair |
| property/loaded | 1848.472 / 1856.179 / 1880.345 | 14.022 / 12.757 / 13.687 | PASS |

Load was 15 seconds, 100 reads/s total over eight clients and 100 writes/s over
one client. Writes were unique unrelated nodes; edge reads searched endpoint text,
not edge ranking. Identical synchronous logging/polling overhead is included,
not separately estimated. All three no-erasure controls completed 1,500 reads and
1,500 writes with zero refusal/error/unfinished outcomes:

| Control | Read p99 ms | Write p99 ms |
| --- | ---: | ---: |
| node | 49.544 | 2.201 |
| edge | 13.303 | 1.900 |
| property | 20.427 | 1.950 |

All seventeen complete loaded replicas had 1,500 successful reads each. Write refusals below are
recorded missed offers while the synchronous client was busy, not necessarily
Engine admission errors. Zero errors/unfinished operations in complete replicas
does not erase refusals. Qualified read/write quantiles require at least 1,000
successes, all offers reconciled, zero refusals/errors/unfinished and physical
completion. Thus A write p99 is null everywhere; B write p99 is qualified only
for edge replicas 0 and 2 (2.089/2.089 ms). There is no qualified paired write-p99
improvement claim. Read p99s are reported even where occupancy screening is missing.

| Legacy family/arm | Read p99 ms (0/1/2) | Write refusals (0/1/2) |
| --- | --- | --- |
| node/A | 49.472 / 49.725 / — | 112 / 111 / 108 |
| node/B | 49.388 / 49.171 / 49.093 | 2 / 1 / 2 |
| edge/A | 13.574 / 13.904 / 13.318 | 44 / 44 / 44 |
| edge/B | 13.792 / 13.719 / 14.412 | 0 / 1 / 0 |
| property/A | 18.445 / 18.767 / 18.050 | 203 / 202 / 204 |
| property/B | 17.554 / 18.141 / 17.411 | 3 / 6 / 6 |

The node A third replica is interrupted, so its 108 recorded write refusals are
only partial accounting. For all interrupted logs, synchronous recorders may not
emit later scheduled IDs before a blocked call is killed. `not_offered` means
scheduled IDs without recorded disposition, **not** measured admission refusal;
`unfinished` means issued without recorded completion. Interrupted qualified
quantiles remain null; no full-window throughput or refusal rate is inferred.
The controller's per-stream counts and raw IDs retain the exact partial evidence.

## Requirements, hypotheses and successor boundaries

Eighteen Rust tests passed, plus fourteen focused Python controller tests. The
ignored crash child is invoked by parent tests and exits inside actual transactions.
Independent verification replayed a committed RED failure and the final GREEN
suite, checked family proofs and recomputed frozen input/accounting identities.
Final campaign verification reconciled 150 streams and all 150 raw-file hashes.
Astra reviewed design, transaction seam, controller corrections and timing gates.
The final full-workspace gate is recorded separately in slice status.

| Requirement / acceptance | Disposition and evidence |
| --- | --- |
| P81-R01/A01 | PASS scoped inventory and positive ghosts for node/edge/property/v2; dependency-free WAL and registered-dependency native baselines both retained. |
| P81-R02/A02 | PASS sanitation admission/progress atomicity, real process exit before commit, reopen and original deadline. Native erasure precedes sanitation admission; crash in that gap is uncovered. |
| P81-R03/A03 | PARTIAL: pinned native snapshot behavior and new logical absence observed; broad denial/routing/reader cancellation unqualified. |
| P81-R04/A04 | PARTIAL: exact survivor IDs/content and controlled raw-FTS same-cut order/BM25 score bits pass, including large/common, Unicode and zero-token input. Public caps/seeds/graph/evidence/statistical isolation unqualified. |
| P81-R05/A05 | PASS finite markers for A fresh/legacy and B ordinary/preoptimized safety fixtures in all three families; 34 large legacy physical proofs. Main-file read mandatory; busy WAL never completes. Telemetry/sinks and universal-byte erasure unqualified. |
| P81-R06/A06 | PASS atomic rollback, committed progress and process reopen, immutable terminal receipt; reingestion excluded by predeclared workload, not enforced by a native fence. Broad projection races unqualified. |
| P81-R07/A07 | PASS real Engine read/write between committed units and after rollback. Native registered-dependency closure still globally fences unrelated writes; no bypass. Occupancy qualification is limited to table above. |
| P81-R08/A08 | PARTIAL: clock rollback, late proof, overdue persistence, idempotent receipt and pinned checkpoint retry pass. Queue reservation, saturation, leases, starvation and guaranteed scheduler remain unqualified. |
| P81-R09/A09 | PASS separation of legacy/fresh; ordinary expiry supplement deferred outside narrowed sanitation scope, with no authorized-history deletion or migration. |
| P81-R10/A10 | PASS production behavior unchanged; test-only transaction hook documented in Rust interface. Native scoped admission, safe publication, telemetry integration and SLA require successors. |
| E81-R01/A01 | PASS frozen identities, exact fixture checks, controls and alternating paired order; observer costs included. |
| E81-R02/A02 | PASS all 72 final dispositions; original 45-minute failure retained; separately reviewed continuation/recovery never retry attempted IDs. C not implemented: broad staging/retirement redesign outside approved seam. |
| E81-R03/A03 | PARTIAL matrix: physical/large-term/crash/pinned-reader witnesses executed; high-dependency fanout, expiry, telemetry and broad routing explicitly deferred/unqualified. A single derived dependency and the large edge chain do not qualify dependency fanout. |
| E81-R04/A04 | PASS scoped lifecycle/event/raw accounting, lock wait/hold, finalization and proof; native time is call duration, not lock occupancy. No native acceptance/denial timestamp fiction. |
| E81-R05/A05 | PASS finite replica/cell/resource limits retained; explicit budget revisions recorded, interrupted outcomes immutable. No product SLA invented. |
| E81-R06/A06 | PASS committed reproducible tools, raw locators/digests and independent verification; temporary raw retention remains necessary. |
| E81-R07/A07 | PASS bounded conclusions below; no production acceptance inferred. |

| Hypothesis | Node | Edge | Property | Conclusion |
| --- | --- | --- | --- | --- |
| H81-1 physical removal | Safety A/B pass; large legacy passes except incomplete observation | Safety A/B and large legacy pass | Safety A/B pass; one monitor-incomplete legacy A | Supports mechanisms on completed qualified fixtures; all large fresh incomplete. Pre-erasure optimized-input probe succeeded; post-delete segment geometry was not independently asserted. |
| H81-2 occupancy/service | Idle screening passes; loaded missing pair | Idle and loaded screening pass | Loaded screening passes; idle missing pair | B lowers measured maintenance occupancy; writes still refused, no universal service guarantee. |
| H81-3 durable idle cleanup | Scoped recovery/idle pass | Scoped physical idle pass | Scoped physical idle pass | Accepted sanitation survives tested crash/reopen; no finite reserved-service guarantee implemented. |
| H81-4 bounded finalization | Unqualified | Unqualified | Unqualified | Busy-reader/clock safety passes, but no reader leases or worst-case completion bound. |
| H81-5 native availability/isolation | Unqualified | Unqualified | Unqualified | Native genuine dependency closure blocks unrelated writes; broad routing and same-source reingestion unresolved. |
| H81-6 expiry | Deferred | Deferred | Deferred | Separate authorized-history-preserving maintenance work, outside narrowed scope. |

Recommend carrying B's atomic bounded sanitation units into successor design,
with A as physical reference; do not select a shipping policy from these samples.
Required next work is native erase cost/admission design, durable admission across
the erase-to-sanitation gap, same-source reinsertion policy, scoped corpus/routing
correctness, all registered sinks, bounded reader finalization and capacity-backed
service reservations. No numerical SLA or universal maximum is supported.

---
title: FathomDB 0.8.28 Slice 81 — requirements and acceptance
status: PLANNED
target_release: 0.8.28
---

# Slice 81 requirements and acceptance

Sources: [reference index](README.md#references), particularly S81-2/3/4/6/9.
`P81-*` and `E81-*` are slice-local POC/experiment labels. They neither amend
`dev/acceptance.md` nor refer to existing product AC-081.

## POC requirements

| ID | Requirement | Acceptance oracle |
| --- | --- | --- |
| P81-R01 | Use actual Engine-created format-35 files, pinned production SQLite/tokenizer and real transactions; no database mocks. Inventory registered table modes, purge owners, telemetry/WAL and every candidate copy. | P81-A01: inventory distinguishes populated, empty, dormant and unsupported families; ordinary ghost and global-write-fence baseline witnesses reproduced before fixes. |
| P81-R02 | Separate durable admission, committed logical denial and verified physical completion. Specify whether admission and denial share a commit; never call an uncommitted request accepted. | P81-A02: fault/rollback before admission leaves no accepted obligation; after acceptance the original scope/deadline survives restart; pending never reports complete. |
| P81-R03 | New reads after denial cannot disclose targets through any supported candidate view; define pre-denial pinned-read handling. Already returned bytes are not revocable. | P81-A03: held-reader/new-reader race has declared outcomes; stronger drain/cancel semantics require explicit experimental policy, not assumed production behavior. |
| P81-R04 | Pending bytes cannot influence authorized corpus statistics, candidates/caps, graph or relaxed/historical/evidence paths. Publish coherent membership or refuse/await an affected scope. | P81-A04: controlled same-cut survivor IDs/order/score bits and cap/seed witnesses are unchanged when excluded target data varies; unsupported paths explicitly refused or unqualified. No universal scope pass from body-only tests. |
| P81-R05 | Physical completion covers declared live/shadow/freelist/main-file traces, WAL and registered sinks, not just MATCH/content rows. | P81-A05: positive nonstemming marker present before erase, absent after complete in each populated qualified family; survivors retained; legacy already-deleted ghosts tested separately. Finite marker tests are necessary evidence, not proof for every byte encoding. |
| P81-R06 | Preserve idempotent durable progress, no resurrection and scope isolation across projection races, reopen/rebuild and same-source reingestion. | P81-A06: crash/replay at durable transitions converges; uncommitted unit rolls back; no lost obligation or double completion; reingestion blocked until completion in the initial candidate. |
| P81-R07 | Commit/yield between bounded work units; measure writer occupancy. Preserve admission safety before narrowing native global fences. | P81-A07: a real unrelated Engine write/read progresses between candidate units; baseline global-fence refusal retained. Any bypass/adaptor-only witness is labeled and cannot pass the actual-Engine availability claim. |
| P81-R08 | Model reserved service, bounded backlog, reader/finalization policy and an explicit deadline clock. A miss remains a breach with an owed obligation. | P81-A08: deterministic saturation/clock/reader tests refuse unguaranteed admission, preserve deadlines over retries, prevent starvation within declared bounds, and record blocked/overdue without false completion. |
| P81-R09 | Keep legacy sanitation, new source erasure and ordinary validity/supersession maintenance distinct. | P81-A09: a new job cannot certify an unsanitized store; expiry benchmark retains authorized history. No old-format migration or automatic shipped recovery change. |
| P81-R10 | Keep production contracts unchanged and report exact composition limits. | P81-A10: no non-test production behavior/API/schema change; real-writer/pool/public-routing versus emulated coverage matrix; successors listed for every required contract change. |

## Experiment requirements

| ID | Requirement | Acceptance oracle |
| --- | --- | --- |
| E81-R01 | Test explicit hypotheses against frozen paired inputs, positive controls and the same source/runtime profile. | E81-A01: source SHA, actual SQLite/build/host, seeds, family counts, controls, arm order and protocol hash recorded before timing; fresh/cached child configuration checked. |
| E81-R02 | Core A/B first; conditional C only under the stated trigger and remaining budget. | E81-A02: every planned cell is complete, rejected, incomplete, not run or inapplicable with reason; C cannot replace or erase failed A/B data. |
| E81-R03 | Include fresh erasure, legacy ghosts, idle completion, loaded service, pinned readers, high fanout/large terms, crash/reopen and ordinary expiry. | E81-A03: [test matrix](test-plan.md) and [measurement matrix](experiment-design.md) accounted for; each unsupported family/path narrows the conclusion. |
| E81-R04 | Measure foreground service and the entire deletion lifecycle, not just SQL duration. | E81-A04: read/write latency and outcomes, queue/writer wait, writer occupancy, accepted/denied/complete times, progress, finalization and physical proof time reported; observer overhead declared. |
| E81-R05 | Use finite experiment controls without inventing product SLA values. | E81-A05: runtime/disk/RSS/queue limits frozen; stop/refusal/unfinished work retained; no replacement samples or post-hoc threshold widening. |
| E81-R06 | Preserve rerunnability and independent verification. | E81-A06: committed tools/manifests/commands, raw-output locator and digests; verifier reproduces selected RED, physical/rollback/restart witnesses and recalculates counts/quantiles. |
| E81-R07 | Draw bounded decisions from evidence, including honest negative outcomes. | E81-A07: one verdict per hypothesis/family/layer, comparative mechanism recommendation or rejection, proposed supported envelope and unresolved SLA decisions. No extrapolated worst-case guarantee from a sampled maximum. |

## Slice completion rule

Execution is authorized by the owner on 2026-11-03 and follows the reviewed
[execution reconciliation](plan.md#execution-reconciliation-2026-11-03).
Slice 81 execution closes only when required safety proofs and planned
cells have supported dispositions, independent verification and Astra review.
An unsafe mechanism can be rejected without measuring its performance. A
conclusive rejection is a valid POC result; missing safety evidence or campaign
exhaustion is not a passing candidate or an automatically complete slice.

No integrated product latency or physical-erasure defect is qualified/closed by
these local criteria. The eventual production change must preserve existing
AC-076/081 and affected vector/stress criteria under their original protocols.

P81-A05 additionally requires a single-segment legacy counterexample test for B;
merge no-work is not a physical-removal oracle. P81-A07 reports experimental
maintenance writer progress separately from native pending-closure availability.
Scheduler, broad routing and comparative performance obligations dependent on a
safe candidate receive explicit rejected/not-run dispositions if its physical
safety prerequisite fails; they cannot be reported as passing or as an SLA.

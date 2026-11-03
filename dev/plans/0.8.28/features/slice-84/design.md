---
title: Slice 84 — high-level certification and contention design draft
status: PROPOSED
target_release: 0.8.28
---

# Slice 84 high-level design draft

The [plan](plan.md) owns requirements S84-1–7, acceptance, TDD and measurement.
This is a direction for the separate execution session, not an approved concrete
production design. [Slice 83 design](../slice-83/design.md) and the
[accepted ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md) govern
current safety. Preserve the end-of-Slice-83 guarantee/API decision.

## Observed path and unproved attribution

`write_inner` takes the maintenance writer turn and primary connection, calls
`guard_plain_write`, then the canonical commit path calls certification inside
its transaction. `certify_plain_write` performs `envelope`, advances the safe
boundary, stores root debt and runs `validate_zero`. Envelope scans canonical
body sizes/counts, vector mappings and provenance/receipt metadata. Zero proof
checks committed boundaries/generation, all registered erased-cursor copies and
captured mapped vector IDs. Unit/finalization paths also validate authority.

Those operations occur under scarce writer ownership. Root/denial decoding,
per-row provenance checks, SQL preparation, full scans, root serialization and
priority/service-turn waits are separate possible costs. Slice 83 measured long
worker wait, not individual certification component time. Reusing prepared vector
queries did not establish latency improvement. First instrument; do not attribute
all waiting to proof scans without a measured decomposition.

## Preferred design order

1. Remove only measured redundant computation within the same transaction and
   locked connection, preserving identical SQL checks, error precedence and phase
   authority. Examples to evaluate: decode root once per certified batch, reuse
   prepared metadata/probe statements and carry validated transaction-local facts.
   This is the lowest-risk first candidate, but must pass integrated timing.
2. If full scans dominate, propose narrowly scoped transactional accounting or
   proof reuse for the already-certified plain mutation path. For each fact name
   its owner, initial exact check, delta, invalidating events, rollback semantics,
   commit publication and reopen reconstruction. No general proof cache or durable
   counter system is presumed necessary. A changed persisted format needs its own
   reviewed admission/compatibility disposition before code.
3. Only if measured scheduling delay remains dominant, adjust the existing
   worker/foreground handoff, batching certification within existing limits or
   avoiding idle priority acquisitions. Preserve opportunities for both services,
   interruptible backoff, wake-after-commit and worker shutdown. Do not hold the
   primary mutex while sleeping or convert an atomic sanitation transaction into
   a series of unlocked SQL calls. Do not split native canonical deletion.

Astra reviews the selected concrete amendment before implementation. Keep at most
one certification and one scheduling candidate; report a rejected approach rather
than add replacement indexes/generations or a third cleanup mechanism.

## Required proof ledger in the concrete design

A small table in the final design must map each reused fact to authority and
fallback, not merely call it cached:

| Fact | What the concrete design must establish |
| --- | --- |
| Resource envelope | Exact row/body/vector/metadata/page semantics. Count actual effects of inserts and old-revision retention, same-batch supersession, unnamed edges and projection/FTS growth. Check independent page allocation after work; do not assume submitted bytes equal file growth. Unknown/unhandled mutation recomputes fully or refuses. |
| Denied scope/write eligibility | Same source and old/new logical/generated identities/endpoints remain checked, including earlier batch members. Protected provenance and persistent/TEMP trigger fallback retain refusal. No wider mutation class. |
| Structural zero | Explain why every admitted effect cannot recreate any captured cursor/vector/dependent row. Generation/watermark are bound to this exact journal and scope. Stale, corrupt or unproven state runs full exact proof or refuses; finalization keeps authoritative proof. |
| Progress/safe watermark | Update in the same transaction as actual mutations; failed/deferred commit rolls back both. Never publish speculative authority to the worker or a later transaction. |
| Reopen/legacy/redacted phase | Process-local optimization state is disposable and reconstructed only from authoritative DB state. Legacy journals keep conservative validation; redacted finalization retains the persisted global fence and successful checkpoint boundary. |
| Service turn | Lock order and ownership among public caller, foreground writer, maintenance and reader finalization; no lost wake, starvation, spin, leaked guard or join deadlock. |

No optimization may silently skip a safety obligation; an equivalent mechanism
must be explicit and independently tested. Compare optimized versus original full
validation after every generated state transition in test fixtures. Independent
physical markers/survivors remain necessary because both validators can share a
bug. Fail closed on uncertainty rather than let timing success authorize a write.

## Tests and instrumentation

Reuse existing test hooks, SQL-authorizer work counters and physical metrics.
Add no public timing API or production telemetry payload. Keep fault models real:
rollback, deferred commit failure, process exit/reopen, corrupt/missing authority,
new/old endpoint denial, same-batch writes, unbound/missing sinks, vector residue,
reader pin and close. A clock-based race test cannot substitute for a rendezvous.

Separate reduced work from improved service. Profile baseline/candidate with the
same metrics, preserve raw accounting and run the plan's core benefit gates.
Unchanged full-proof fixtures are regression oracles, not tests to rewrite for an
optimization. Work-count RED is required for the selected redundant-work change;
existing safety guards need no invented RED failure.

## Contract and final deliverable

No new successful outcome or weaker refusal/completion meaning. Update relevant
interface/ADR text only if implementation changes a documented mechanism; seek
approval for any contract/envelope/scope change. End-of-83 HITL decisions remain
inputs to reconcile, not choices this slice can invent. Final status must separate
safety pass, work-count reduction, service qualification and residual Phase 82
cost, with measured failures retained.

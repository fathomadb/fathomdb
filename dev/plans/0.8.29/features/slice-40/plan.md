---
title: Slice 40 — eligibility reuse and batch metadata
status: PROPOSED
target_release: 0.8.29
slice: 40
---

# Slice 40 — eligibility reuse and batch metadata

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Current checkpoint scope

Conditional scope approved in Slice9 Batch3. Caching requires30 evidence of benefit and reviewed snapshot safety. Allocation alone does not authorize adoption or schema/counter fallback.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and boundary

Reduce measured redundant eligibility/metadata work while preserving exact snapshot results and publication semantics.
This is a proposal; no cache choice, implementation, product test pass or review approval is claimed.
[Brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md) and [release plan](../../../plan-0.8.29.md) govern scope.
The format-37 baseline includes native vector lifecycle/validity filtering; do not restore the old vector-wide scan.
The remaining direct-node scan gates deferred text identity; source-dependency vector fallback is a separate decision.
Historical scale-search prototypes are research inputs, not merge candidates or current performance evidence.
No maintained counters, storage-format change, migration, public cache interface, ranking/default change or environment work is implied.
Per-batch metadata hoisting is independently selectable and must prove its own semantics and benefit.

## Sources inspected and slice preparation

- [Eligibility implementation](../../../../../src/rust/crates/fathomdb-engine/src/dependency_closure.rs): `all_nodes_directly_eligible`, dependency existence, source fallback and closure predicates.
- [Search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs): reader transaction, native KNN eligibility, dependency fallback and deferred text fast-path conditions.
- [Projection commit](../../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs): IMMEDIATE admission, existing generation/time hoist, per-row attributes, mean pin and commit publication.
- [Attribute fragments](../../../../../src/rust/crates/fathomdb-engine/src/projection_registry.rs), [live vec0 columns](../../../../../src/rust/crates/fathomdb-engine/src/vector_storage.rs) and [frozen bindings](../../../../../src/rust/crates/fathomdb-engine/src/frozen_read.rs).
- Accepted [eligibility](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md), [pagination](../../../../adr/ADR-0.8.25-frozen-pagination-and-operational-state.md), [closure](../../../../adr/ADR-0.8.25-dependency-lifecycle-closure.md) and [format-37 coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md) ADRs.
- [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): apply boundary/stateful properties and disabled-optimization controls; defer general SQLite fuzzing and exhaustive allocation campaigns.
- Relevant verified memory entries: `0.8.25-slice35-eligibility-frozen-read-lessons.md`, `0.8.25-slice30-dependency-closure-lessons.md`, `0.8.25-slice40-projection-readiness-lessons.md`, `tdd-red-must-fail-per-finding.md`.

Create slice-local `requirements.md`, `design.md`, `test-map.md` and `measurement-plan.md` before implementation; these are future deliverables.
Use the draft local IDs below, not additions to locked canonical acceptance.

| Need | Requirement | Local acceptance and evidence |
| --- | --- | --- |
| N40-1: lower repeated-search cost | R40-1: choose reuse only from current read-mostly and write-interleaved evidence | A40-1: paired controls report end-to-end latency, scan/reuse cost, hit/miss rate and memory; selection rationale covers both workloads |
| N40-2: exact eligible candidates | R40-2: eligibility and fallback decisions belong to the consuming read snapshot, effective time and full view | A40-2: optimized/reference results, fallback reasons and pre-cap eligibility agree for the complete invalidation matrix |
| N40-3: efficient projection publication | R40-3: hoist only proven immutable batch metadata after writer admission | A40-3: physical vectors, attributes, spans, terminals, cursor, events and rollback outcomes match the reference |
| N40-4: bounded operational cost | R40-4: bound memory/lifetime and retain a safe uncached route on misses | A40-4: churn/reopen controls show bounded retention; unsupported reuse takes the correct reference route |

## Design draft, challenge and fit revision

Inventory direct-node eligibility, dependency presence and source-fallback costs separately; never substitute one boolean for another.
Compare no cross-call cache, transaction-local reuse and bounded cross-read reuse; do not assume a generation cache wins.
A cross-read candidate must bind database identity, branch-sensitive nonce, visibility generation, exact snapshot authority, effective validity instant and all view flags.
Read key/proof and consumed rows in the same SQLite transaction; an Engine atomic or writer-connection observation cannot authorize another snapshot.
Equal integer generations across copied/diverged stores are insufficient authority; old and new readers must not evict correctness through a shared latest value.
Specify whether filter/route state belongs in a cached answer or remains freshly evaluated; retain every existing fast-path guard.
Time advancing without writes can invalidate eligibility: either key exact effective time or prove a safe validity interval, including both boundary endpoints.
Keep source-dependency presence and fallback separate; closure barriers remain unconditional even when relaxed view flags permit lifecycle history.
Format 37 coalesces relevant writes to one committed advance/nonce rotation; no cache may publish an armed or uncommitted result.
External ordinary SQLite writes retain row-trigger invalidation; retain fresh malformed/committed-armed refusal and frozen reproduce-or-fail behavior.
A miss must not add a window scan on every write without measured benefit; cache errors must preserve existing typed failure semantics.

For batch work, current generation and eligibility time are already read once inside IMMEDIATE; credit existing behavior rather than reimplement it.
Candidate residuals are live attribute-column shape, registry-derived extraction descriptors, SQL fragments and reusable statements.
Prove invariance across the admitted batch, including custom-trigger fallback; bypass hoisting where triggers can mutate relevant registry/state.
Keep body/scalar values, cursor ownership, row lifecycle/validity, terminal checks and physical tuple checks row-specific.
Preserve absent versus present-empty attribute encoding, complete vec0 bindings and the no-attribute path's lack of body lookup.
Mean-centering state can pin mid-batch; retain ordered accumulator/requantization behavior and publish process state/events only after commit.
Do not hoist wall-clock fields or change timestamps simply because other metadata is invariant.

Planned primary research: inspect SQLite transaction/WAL snapshot and trigger documentation, rusqlite transaction/statement behavior, and current vec0 metadata constraints.
Record exact primary references and their applicability in `design.md`; these external sources have not been researched in this draft.
Recheck current writer/reader callsites and selected prototype provenance after Slice 30; test the design against code, not historical architecture alone.
Fit review revises the old two-scan premise to the surviving text gate plus separate source fallback and already-partial batch hoisting.
If evidence rejects cross-read caching, revise scope to justified transaction-local/batch work or explicitly defer it; counters/schema need a separate reviewed proposal and owner decision.

## Dependencies, sequencing and resources

- Hard: approved Slice 10 preparation and dependable Slice 20 harness inputs used by Slice 30.
- Hard: Slice 30 current native/artifact/fixture evidence for both read-mostly and write-then-search workloads, plus accepted local requirements and independently reviewed invalidation design.
- Conditional: any new persisted authority, public contract or trigger change requires its own ADR/interface review before implementation; none is preauthorized here.
- Preferred: land 40 before 50 to stabilize the hot path; 40 is not a semantic prerequisite for Slice 50 fusion identity or Slice 60 MCP work.
- Serialize engine writers with 50/70/100/110; `search.rs`, dependency closure, projection commit, frozen state and shared test seams are conflict hotspots.
- Use one isolated implementation worktree; root owns landing/state writes. Existing compute venues are facts to verify, not reservations.
- No GPU fault venue is needed for cache correctness; attribute/projection controls use real databases and deterministic existing embedder fixtures where applicable.

## Work packages and concrete exits

1. Reconcile Slice 30 profiles and prototype receipts; produce per-path cost inventory and fixed paired measurement cells with a decision rule before candidate selection.
2. Draft local needs/requirements, invariance/key/lifetime design and test mapping; obtain independent adversarial review of snapshot/time/coalescing claims and revise findings.
3. Implement only the selected eligibility treatment through per-behavior RED/GREEN; retain an uncached test control without exposing production environment hooks.
4. Implement selected batch metadata treatment separately through RED/GREEN; retain baseline comparisons so cache and batching effects are attributable.
5. Run correctness and paired current-workload comparisons; record raw outcomes, run identities, uncertainty and regressions, then accept, narrow or defer each treatment.
6. Deliver `status.md`, reviewed requirements/design/test map, rerunnable measurement references and maintained contract/doc updates where behavior warrants them.

Completion requires A40-1–4 evidence and a reviewed disposition for both cache and batch work; a faster repeated-read microbenchmark alone cannot close this slice.
Keep existing AC-083 tracking and numerical release gates unchanged; performance decision tolerances must be proposed/reviewed before measurement, not invented after results.

## TDD, reference controls and validation

Before each behavior change, stage/commit its genuinely failing test and record the assertion at its RED SHA; freeze tests during the fix and retain separate GREEN evidence.
Use real WAL databases and explicit barriers rather than sleeps for old-reader/new-writer snapshots, external commits and rollback/savepoint interleavings.
Cover every view-flag combination, validity start/end without writes, database reopen/copy/divergence, equal-generation different-nonce branches and bounded cache churn.
Cover canonical writes, lifecycle transitions, dependency registration/closure, coalesced projection success/failure/no-op, custom triggers, rebuild/configuration and erase/purge.
Assert rollback, ABORT/ROLLBACK-trigger failures, commit refusal and malformed/armed state cannot seed reusable authority or publish partial batch metadata.
Use stateful properties over write/supersede/validity/dependency/project/rebuild/erase/reopen sequences, with minimized seeds retained.
Compare cached and disabled-cache routes on the same snapshot, but also assert independently specified human eligibility rules so a shared predicate cannot supply its own oracle.
Check eligibility before candidate caps and physical vec0/sidecar/terminal consistency, not merely final hit counts or terminal readiness.
Batch controls cover zero/nonzero attributes, absent/present-empty values, mixed kinds/outcomes, stale generations, mid-batch mean pin, duplicate terminals and injected rollback.
A meaningful work-count or query-count RED can expose redundant metadata reads; output equivalence alone cannot prove an optimization exists.
Review requirements/design before GREEN and obtain independent code review after focused correctness/performance evidence; audit representative RED SHAs.
During implementation use scoped lint/typecheck/tests in latency order; run the required full `scripts/agent-verify.sh` once on the final code phase with checkout-owned tools.
After integration rerun affected controls and contribute to the Block 1 follow-up exit; a focused pass is not full-workspace qualification.
For this planning artifact, root runs Markdown/link checks after integration; no builds or product tests are authorized by drafting.

## Open prerequisites and stop points

Slice 30 benefit evidence, approved performance tolerances, exact cache scope/eviction policy and a complete changed-authority inventory remain open.
Custom-trigger mutation effects on proposed hoists and validity-interval proof require explicit design resolution; retain uncached behavior until resolved.
Retrieve selected historical prototype preregistration/results with original ref/artifact identity before using their findings; this draft inspected current source and reconciliation only.
Stop for a design decision if benefit disappears under writes, an authority cannot be validated inside the reader snapshot, or any shortcut weakens pre-cap/frozen semantics.
A failed cache proposal does not authorize counters, schema changes, fallback weakening or an unreviewed cache default; record the narrower disposition for the next reviewer.

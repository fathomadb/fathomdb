---
title: Testing and robustness findings for FathomDB 0.8.30
date: 2026-11-17
status: RELEASE_PLANNING_REFERENCE
target_release: 0.8.30
---

# Testing and robustness findings for FathomDB 0.8.30

FathomDB already has useful fault controls, real-store regressions, reference
routes and installed-consumer tests. The highest-value .30 work is to use and
strengthen those tools around current correctness and lifecycle risks, profile
the actual expensive paths, and make CI report failures sooner and finish
publication reliably. A new general testing framework or another broad
structural refactor would consume effort without an established product need.

This review uses the .27 implementation and qualification records, .28 repair
and release records, current test sources, and the [.29 retrospective](../../notes/0.8.29-retrospective.md).
Historical results qualify their recorded sources and workloads; this review
does not rerun them or declare the same defects present in .30. The existing
[draft plan](plan.md) and [recommendations](quality-performance-robustness-recommendations.md)
remain proposals. No execution or release state is activated here.

## Findings from 0.8.27

### Behavioral tests made the refactor tractable

The [refactor testing approach](../0.8.27/prework/0.8.27-refactor-test-approach.md)
and [release testing contract](../plan-0.8.27.md#testing-contract) required
stable observable assertions, separate mechanical moves from semantic fixes,
one deep invariant owner with thin binding checks, real databases, exact
survivor/rollback checks, deterministic concurrency and installed artifacts.
They distinguished initially green characterization from genuine defect RED.

This was substantial decomposition: the [large-file review](../0.8.27/features/slice-140/large-file-review.md)
records the engine root falling from 34,184 to 3,905 physical lines, including
its remaining tests. It explicitly retained cohesive large files rather than
making size a CI limit. Temporary surface-oracle machinery was scheduled for
retirement. For .30, refactor only where a reproduced defect, measured cost or
unclear ownership requires it; preserve behavioral tests through the move.

### Broad path coverage did not establish strong error oracles

The final [Phase 1 checkpoint](../0.8.27/features/slice-135/phase1-checkpoint-2026-10-08.md)
reports selected tests hitting 8,121 of 8,134 workload-hit engine lines and all
888 workload-hit branch IDs; two supplemental binaries covered the 13 line
gaps. This was an exact-source workload overlay, not whole-engine coverage.
The [earlier small sample](../0.8.27/features/slice-135/pareto-test-coverage-2026-10-06.md)
already exercised row decoding while missing its error sides.

Targeted corruption probes found real current-schema SQL failures being
discarded in candidate decoding, vector hydration, deferred provenance,
importance/explanation and fallback checks. Successful partial results could
therefore conceal damaged state. The [boundary audit](../0.8.27/features/slice-135/logic-boundary-audit-2026-10-08.md)
links the separate RED/GREEN repairs. Restoring the old provenance-error
suppression compiled and failed the intended assertion in the
[executed mutation check](../0.8.27/features/slice-135/results/2026-10-08-deferred-identity-mutation/README.md).

For .30, review truthfulness of exceptional paths, including `.ok()`, defaulting
and intentionally optional rows, in their control flow. A text search for those
spellings is a lead, not a defect diagnosis. Use a few plausible semantic
mutations to assess important oracles instead of pursuing a global percentage.

### Fault tests checked recovery and resources as well as errors

The [robustness replay](../0.8.27/features/slice-135/results/2026-10-08-robustness-replay-224e/README.md)
covered real concurrent readers/writers, process kills before and after an
acknowledged write, projection-commit faults and persistent provider failure.
The [persistent SQLite-full test](../0.8.27/features/slice-135/results/2026-10-08-persistent-sqlite-full-current/README.md)
kept the capacity fault active across repeated writes, checked exact preserved
state and descriptor counts, lifted the fault, then wrote and reopened.
The [additional crash/refusal probes](../0.8.27/features/slice-135/results/2026-10-08-crash-permission-224e/README.md)
distinguished canonical durability, vector recovery and truthful terminal state.

These are reusable patterns. A typed error alone, or SQLite integrity alone,
does not prove FathomDB composition correct. Conversely, connection `query_only`
does not test OS permissions, and SIGKILL does not test power-loss ordering.
The [final disposition](../0.8.27/features/slice-135/status.md) explicitly retained
limits for sustained mixed load, concurrent close/cancellation, release-binding
panic injection and platform/provider variants. Reassess those leads against
later changes; do not copy every historical omission into .30 scope.

The [Phase 2 correctness study](../0.8.27/features/slice-135/phase2-query-correctness-report-2026-10-08.md)
also used authored deterministic query/reopen contracts and independently
specified retrieval/evidence gold. It separated vector fidelity, retrieval
effectiveness and evidence sufficiency from unsupported generated-answer
quality. A subsequent [611-query CPU/CUDA sample](../0.8.27/features/slice-135/phase2-gpu-sample-2026-10-08.md)
matched ordered hits and gold scores on its installed Python x86_64 candidate
route; it did not establish embedding byte identity or every platform/model.
Reuse those distinctions and small independent oracles rather than inferring
correct answers from either a version-to-version match or retrieval recall.

### Runtime fixes needed deterministic ordering and real completion

The [Slice 90 implementation](../0.8.27/features/slice-90/status.md) records a
provider-error fallback deadlock reproduced in a bounded child and killed by a
restoration mutant. It also repaired retry deadlines shortened by wakeups,
retained capacity after caller timeout, and panic outcomes incorrectly turned
into sparse fallback. These examples favor tests of permit ownership, absolute
deadlines, cancellation, actual stored vectors and reusable workers.

The [Slice 85 recovery](../0.8.27/features/slice-85/recovery-receipt.md) is also a
warning about test infrastructure. It removed 8,431 net lines while retaining
engine ownership and useful architectural witnesses. Its explicit gate
qualification fell from a historical roughly 9.2 minutes to 65.7 seconds.
Those are gate measurements, not database speedups. Preserve the narrow checks;
do not recreate general graph censuses or routine exhaustive mutation scans.

## Findings from 0.8.28

### Correctness and performance improved together when the comparison was exact

[Slice 50](../0.8.28/features/slice-50/status.md) bounded pending-work scans,
compared them with the original SQL over generated cursors/layouts, killed
limit/order mutations and checked independent whole-table vector/mapping
digests. At 100k vectors, its two held-drain comparisons reported 27–28% faster
drain, with approximately flat optimized cost per vector. This applies to that
fixture, not every backlog or search route. The same campaign exposed stale
Cargo artifacts and a lost subprocess diagnostic, demonstrating why the
measurement tool's correctness matters.

[Slice 70](../0.8.28/features/slice-70/status.md) put lifecycle/validity filtering
before the dense candidate cap and maintained all record-mapped vectors
transactionally. Adversarial review found an unmapped vector head surviving a
successful purge; a public RED reproduced it and the fix refused transactionally
without damaging other owners. Its 64 focused tests included unit/property,
boundary, rollback and integration cases. The lesson is to assert both eligible
results and preserved unrelated state, including bounded candidate selection.

The [visibility-coalescing ADR](../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
connects a real reprepare profile to transaction-owned trigger suppression,
rollback and external-writer invariants. Final write throughput passed the
unchanged gates, but the [separate measurements](../0.8.28/features/slice-95/qualification.md#ac011-before-and-after-the-repairs)
are not a paired causal percentage. For .30, first locate current cost, then
compare a selected treatment on matched state without weakening semantics.

### Test failures included product defects, fixture defects and weak assertions

The [Slice 95 designs](../0.8.28/features/slice-95/design.md) distinguish these
classes instead of treating every CI failure as a flake:

| Case | What changed | Lesson for .30 |
| --- | --- | --- |
| CI9 rollback property | The test stopped/joined its own autonomous actor before inspecting foreground rollback. | Attribute mutations to the intended actor and observation point. |
| CI12 subscriber admission | A real producer dropped records on bookkeeping contention despite spare queue capacity. Deterministic RED preserved the delivery contract; production admission was repaired. | Do not relabel ordinary contention as overload or weaken phase-pairing assertions. |
| CI13 provider close | A wall-time proxy included SQLite disposal and scheduling; tests instead observed the real deadline and rejected early-start/renewal mutations. | Assert the contractual clock boundary; changing a timeout threshold can hide the wrong problem. |
| CI14 long cursor test | The old fixture could pass with failed writes and empty searches. Stronger tests required successful concurrent writes, matching rows and advancing hit cursors. | Require positive activity and a progress witness, not loop count alone. |
| CI24 projection interleaving | Seeding with a worker already paused in a WAL transaction created a fixture lock cycle. Quiescent setup preceded arming the intended pause. | Separate setup synchronization from the experiment; release/join actors safely on failure. |

These are completed .28 repairs, not requests to repeat them. Inventory in-crate
`#[cfg(test)]` tests as well as integration targets before declaring a gap.

### CI and publication robustness remained an engineering problem

[Slice 95](../0.8.28/features/slice-95/plan.md) repaired cold pinned-tool setup,
parallel-build disk exhaustion, modular TypeScript export checks, host-dependent
fixtures, empty-diff manual routing, diagnostic spill handling and monotonic
suite timing. Its [completion record](../0.8.28/features/slice-95/status.md)
retains failed original runs separately from passing focused resumptions.

Yet .29 again lost a Rust diagnostic, spent 856.43 seconds in a static Windows
workflow checker, and needed OIDC recovery after normal npm finalization failed.
These are current high-value leads from the [.29 retrospective](../../notes/0.8.29-retrospective.md),
not proof that every .28 repair failed. The suite runner's monotonic-clock fix
does not establish that every other timing wrapper uses that clock. Review
actual current callers and make the normal publishing path consume the proven
mechanism; do not add another permanent recovery implementation.

## Testing approaches for 0.8.30

The [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md) applies
at FathomDB-owned transaction, eligibility, queue, provider and FFI boundaries.
It already records .28's switch to unpatched registry SQLite and retention of
the Windows verbatim-path WAL regression. Retain that supported-platform test;
do not reinstate the retired vendored patch or its old qualification machinery.

| Approach | Selected use | Scope limit |
| --- | --- | --- |
| Behavior-specific TDD | Reproduce confirmed defects before fixing; preserve intended error precedence and post-state. | Passing characterization is coverage, not manufactured RED. |
| Stateful/property tests | Small write/supersede/project/read/reopen sequences, codecs, projection/recovery round trips and bounded filter/cursor combinations. | Human-defined invariants, minimized seeds, real databases; no generic framework or regenerated goldens. |
| Independent reference/metamorphic tests | Exact same-state retrieval identity, eligibility-before-cap, Unicode spans and coverage accounting. | Shared flawed predicates are not independent; moving snapshots cannot support unconditional exact equality. |
| Bounded failure injection | First/persistent failures and selected cancellation/close/reopen at existing seams, with exact state and cleanup. | Actual delivered crash proof when selected; no power-loss claim, host-wide reset or exhaustive allocator/VFS sweep. |
| Targeted branch/mutation review | A few wrong-order, missing-cleanup or swallowed-error variants in high-risk paths. | No global score, MC/DC target, topology freeze or exhaustive source-text mutation matrix. |
| Installed binding/package checks | Thin Rust/Python/TS boundary parity, malformed values, repeated refused open then healthy reopen, actual loader/native dispatch where changed. | One deep semantic owner; do not clone the complete engine matrix into every binding. |
| Cost and usefulness review | Reuse existing suite durations, defects caught, known misses and duplicate claims to retain, consolidate or move checks. | Reduce measured cost while retaining the actual runtime/platform oracle; test count is not quality. |

## Recommended priorities and two bug-hunt slices

1. **Early diagnostics and delivery fixes:** retain the first full useful
   failure through cancellation, expected/observed registry visibility, normal
   OIDC promotion, tool-only setup and the measured static-check hotspot.
2. **Bug hunt and remediation A — correctness:** inspect the preserved S50
   natural-write score observation first, then a small current-state selection
   of record identity/feedback, coverage/UTF-8 and malformed native/error cases.
   Each confirmed defect ends with its failing regression, fix and scoped proof.
3. **Bug hunt and remediation B — concurrency and lifecycle:** select queue/
   cancellation/close/reopen and default Node/native growth cases. Retain
   healthy/OOM/software-loss credit; real reset and unsafe release paths require
   their existing safety prerequisites and concrete authority.
4. **Latency/performance:** profile a representative current 100k hybrid/SDK
   route plus the relevant text/lifecycle route. Separate queue wait, service,
   eligibility, hydration, reranking and serialization; report end-to-end tails,
   throughput, refusals and resources. Promote only demonstrated improvements.
5. **Focused test review throughout, then coherent qualification:** choose
   checks for their claims and cost; use scoped feedback while editing and the
   required broad gate on a stable code phase. Recheck failed/unrun scopes after
   narrow repairs rather than replaying valid work for administrative edits.

Each hunt begins with a bounded selection of named hypotheses and stop rules,
not an open-ended demand to find bugs. Exit classifications are confirmed defect
fixed, expected behavior, bounded non-reproduction or unresolved with its exact
consequence. A critical wrong-result/data-loss finding requires resolution or
explicit owner disposition before release. Fixed-state exactness stays intact;
S50's .29 exception creates no general score tolerance.

Preserve existing [.30 feedback/reset allocations](release-followups.md),
[fork maintenance](fork-maintenance-plan.md) and the draft's safe module-model
release prerequisite. .29 supplemental Tegra delivery stays .29-owned. CUDA13
delivery stays .31-owned; physical erasure is defects-only. No schema migration,
retrieval backend, precision/default change, paid campaign, new support venue or
release activation follows from this review. The draft should expose only the
remaining choices that materially affect implementation or safety.

## Independent Astra review

An independent `gpt-6.2-astra` agent at medium reasoning reviewed these findings,
the cited .27/.28 records and existing .30 needs, then proposed the replacement
[draft plan](plan.md). It confirmed the principal historical comparisons and
recommended two corrections: acknowledge the already-updated SQLite roadmap,
and include .27's Phase 2 correctness and scoped CPU/CUDA evidence. Both are
applied above. Its proposal absorbs test-value review into the engineering
slices and retains two bug-hunt/remediation slices and the existing allocations.
The review was read-only; no runtime or external qualification was performed.

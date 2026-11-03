---
title: Current and historical search — decisions and staged follow-up
status: PROPOSED
date: 2026-11-02
target_release: unassigned
---

# Current and historical search — staged follow-up

Owner decisions follow the [reviewed results](../experiments/current-history-search/followups/RESULTS.md).
This records decisions and proposes delivery units. It does not reopen Slice 80,
authorize the whole program. Owner subsequently placed the maintenance POC in
Slice 81; its evidence now feeds approved CH1 Slices 82 and 83 before unchanged
Slice 90 scope. Other production units remain proposed.
The [production design](../design/current-history-search.md) remains a candidate.
CH labels are planning units: CH2 sanitation evidence is complete in
[Slice 81](0.8.28/features/slice-81/README.md); CH1 is assigned to Slices 82/83.
CH3–CH8 remain unassigned.

## Decision record

| # | Owner direction | Recorded boundary |
| ---: | --- | --- |
| 1 | B: secure deletion and retained-term cleanup across registered FTS families | Two-phase CH1 plan approved and allocated to Slices 82/83 on 2026-11-03; exact production design reviews remain required. Slice 81 demonstrated family-specific physical witnesses; production qualification remains required. |
| 2 | B: separate erasure, current isolation and temporal milestones | Accepted delivery direction; release placement remains open. |
| 3 | B: incremental ordinary maintenance plus bulk refresh; POC approved | Focused comparison authorized; mechanism selection and production implementation not approved. Owner wrote “PIC”; interpreted as POC in context. |
| 4 | Test B first, then compare C for arbitrary temporal subsets; retain A as reference/bounded candidate | Ordered probes: certified native reuse, then persistent selected statistics/partitions. No winning backend yet. |
| 5 | B: explicit temporal claim operations with existing contracts preserved | Accepted direction; exact successor ADRs/interfaces and schema need review. No automatic reinterpretation of existing flags or G11. |
| 6 | Lean B: atomic commit boundaries and fragment-aware evidence | Provisional preference. Evaluate minimal reuse; no identity/commit subsystem approved. |
| 7 | Carefully evaluate C: view-specific ranking/result units | Unresolved. Astra-medium planning, design and review required; no blanket claim-ranking or graph rewrite approved. |
| 8 | B: bounded admission/queues, explicit run-to-completion initially | Accepted direction; numeric limits, fairness and shutdown behavior need validation. No hard active-execution or close deadline promised. |
| 9 | B: separate bounded historical performance envelope | Accepted direction; historical SLOs/caps undecided. Default ACs unchanged. |

Direction approval differs from approval of a finished implementation plan.
No production change or new campaign is executed by this record. Decision 3's
approved POC completed in Slice 81 with scoped erasure/availability evidence
for the revised production CH1 proposal. Decision 4
establishes probe order without authorizing an unrestricted campaign.

## Temporal model in plain language

Keep two questions independent: **when did this apply in the world (V)?** and
**what had the database recorded by committed boundary K?** A claim has an
immutable payload revision and source. Interval records, called fragments,
state where that revision is accepted in V/K. Intervals are start-inclusive and
end-exclusive. A partial correction replaces the affected interval while keeping
unchanged outside intervals linked to their original revision.

Example: a source first says Alice holds a role from January 1. On March 10 it
reports Bob takes over on April 1: a future world change. On April 5 it corrects
the takeover to March 25. Corrected history for March 28 now selects Bob; prior
belief for March 28 at the earlier committed knowledge boundary selects Alice.
Current facts use now/latest knowledge. Archive enumerates revisions and status;
it is not a coherent graph snapshot. Disclosure still requires authority now;
erasure and source revocation cannot be bypassed with an old K.

A world change changes the asserted period of applicability. A correction changes
the recorded account of an affected period. Both use the same interval mechanism,
but cannot be inferred from recency alone. Applications supply intent and bounds.
The engine records source assertions; it does not decide objective truth or choose
a winner between conflicting independent sources.

Why it fits: future/late changes, partial corrections and prior belief can preserve
provenance without reviving retracted errors. The node POC demonstrated these
invariants against real engine constraints. Why it might not: two times, fragment
maintenance and coherent graph selection add write/storage/API complexity.
Archive-only consumers may not need it. It does not solve source conflict, and
integrated performance, arbitrary atomic batches, full evidence and temporal graph
correctness remain unqualified. Opt-in operations and staged integration tie costs
to actual needs.

## CH1 — two-phase production erasure proposal

The [revised CH1 plan](ch1-production-erasure-plan.md) incorporates the completed
[Slice 81 findings](../experiments/erasure-availability/RESULTS.md) and replaces
the earlier eager-first sketch. It proposes two production phases:

1. **Physical correctness and native cost:** profile/fix native erasure cost,
   integrate durable incremental sanitation, cover retained ghosts and original
   telemetry/WAL obligations, retaining conservative admission.
2. **Resumable work and availability:** qualify native denial/publication,
   reingestion/projection safety, restart/idle scheduling, finite admission,
   unrelated progress and pinned-reader finalization while preserving truthful
   public success meanings.

HITL approved the Astra-reviewed plan on 2026-11-03 and allocated Phase 1 to
[Slice 82](0.8.28/features/slice-82/plan.md) and Phase 2 to
[Slice 83](0.8.28/features/slice-83/plan.md). The 0.8.28 ladder is 82 → 83 → 90;
phase implementation has not started. At the end of
Slice 83, HITL will decide the physical-erasure guarantee. A specific API that
callers could invoke to ensure physical cleanup is under consideration then;
no API, numerical SLA or unconditional deadline is selected by this plan.
The linked plan owns requirements, acceptance, design tasks, TDD, focused
measurements and the end-of-83 decision package. Temporal features and ordinary
expiry remain separate.

## CH2 and CH3 — Bounded mechanism evidence

**CH2: sanitation mechanism evidence complete in Slice 81.** The narrowed
A/B investigation covered post-native FTS sanitation, durable progress and
physical witnesses; it supports incremental sanitation as CH1's candidate.
Native admission/availability, production routing and SLA remain unqualified.
The original ordinary-delta/10,000-record expiry comparison was deferred: current
membership expiry must preserve authorized history and is not source erasure.
That separate work needs placement and a scoped protocol; no repeat sanitation
grid or implicit expiry allocation follows from the CH1 revision. See
[Slice 81 results](../experiments/erasure-availability/RESULTS.md) for every
partial, incomplete and deferred disposition.

**CH3: historical backend POC in two stages.** First test native reuse (B) only
when the physical corpus exactly equals the authorized requested statistical
scope. Check validity, lifecycle, source authority, projection/tokenizer identity
and all property names; counts or an include-superseded flag cannot certify this.
Source revocation invalidates or refuses reuse. Report where no maintained corpus
exists. Then compare persistent query-term statistics/postings or justified finite
partitions (C) for arbitrary V/K against private native FTS (A).

Compare selected identities, compiler semantics, score/order, phrase document
frequency, stemming collisions, literal fallback, zero-token rows, all-name property
statistics and ties. Include write/erase/frontier costs and mixed default/history
contention. Query-term scoping may still have expensive selected-membership work;
finite partitions do not cover arbitrary temporal cuts. Use targeted entropy and
duplicate cases. No numerical cap/SLO follows from previous experiment stop budgets.

## CH4 — Minimal temporal contract and ranking design

Resolve decisions 6/7 before temporal code. Astra-medium reviews requirements,
design and test oracles. No broad identity/ranking redesign is implied.

**Decision 6: recommend minimal B, pending owner acceptance.** Reuse immutable
revision/source identities, single writer, atomic batches, receipt boundaries,
frozen contexts and stateless evidence sidecars. Existing actuation receipts expose
`resulting_write_boundary`: a candidate K, not a proved durable committed-cut ledger.
Prove mutation-only/retraction advancement, rollback/replay, reopen/erasure nonreuse
and rejection of interior-batch cuts. Add only missing durable boundary/membership
information. No wall-clock conversion service, distributed clock, universal event
log or retained-snapshot service is justified by current needs.

A temporal evidence selector binds the selected fragment/interval, intrinsic
revision/source and authenticated view. Reuse opaque references and current
checks. A revision alone cannot distinguish disjoint fragments of identical bytes.
No current-owner substitution, raw-ID authority or durable evidence table. Exact
selector shape, format admission and source-specific claim mapping need review.
These are internal distinctions, not five automatic new public ID classes.

**Decision 7: prefer minimal necessary distinctions; keep open.** Existing body
search deduplicates bytes, property search preserves revision identity, and graph
expansion groups logical targets with deterministic winning origins. Preserve
legacy contracts. Evaluate claim/provenance identity separately from units counted
by ranking statistics, caps and results. A claim ledger does not automatically
require a new public claim-level result or scorer.

Use equal bytes from two sources, disjoint fragments of one revision, many parallel
historical edges to one target, and a distinct target displaced by duplicate seeds.
Assess existing collectors plus opt-in provenance sidecars first. Temporal views
must explicitly define corpus, ranking unit, duplicate/tie rule and winning origin.
If temporal multiplicity breaks cap/prefix correctness, propose seed grouping before
limits with deterministic origins. Existing current-head uniqueness makes today's
seed row limit safe; temporal multiplicity needs a successor, not an assumed rewrite.
Do not collapse contradictory sources or aggregate raw BM25 across independent
corpora. Archive enumeration is distinct from ranked include-superseded search.
Reject broad entity/claim ranking changes without a concrete acceptance example.
Defer cross-time journeys, universal entity resolution and source trust arbitration.

Output exact successor ADR/interface deltas, minimal identity/batch mapping, scope
exclusions, RED oracles and acceptance proposal. Owner review of 6/7 precedes code.

## Delivery sequence and dependencies

| Unit | Deliverable | Dependencies and stop boundary |
| --- | --- | --- |
| CH1 | Two-phase physical erasure/cost, then native availability | Approved plan allocated to 82/83; implementation not started. End of 83: HITL guarantee/API decision. No temporal dependency. |
| CH2 | Slice 81 sanitation evidence complete; ordinary expiry deferred | Use scoped A/B findings for CH1; no integrated SLA or authorized-history expiry pass. |
| CH3 | B-first, then C historical comparison | Stable scope oracles; B precedes C. CH4 defines exact future contracts; fixture probes can precede it. |
| CH4 | Minimal temporal/evidence/ranking contract/tests | Existing evidence; resolve 6/7 with Astra-medium. No repeated basic POC required. |
| CH5 | Production default-current isolation | CH1 fixed, CH2 viable; ordinary identity semantics. Integrate public ranking, property/graph seeds, certificate/frontier and repair. No full temporal dependency. |
| CH6 | Opt-in temporal nodes, atomic cuts and exact evidence | CH4 accepted, CH5 ready. Compose ledger/current projection: A at V10, B from V20, B at V30; source denial, frozen reads and erase. Refuse unsupported temporal edge/graph requests. |
| CH7 | Source-specific temporal edges/coherent graph | CH6, reviewed G11 successor and decision 7. Same V/K for nodes/edges, exact winning origin/provenance. |
| CH8 | Selected-history production routing/bounded envelope | CH3 selection, CH4 contracts, CH6 node semantics; CH7 before graph claims. Integrate admission/fairness, tested caps, no-spill policy and historical SLOs. |

Owner placement now puts CH2 maintenance evidence in Slice 81 before CH1
production approval/repair. Slice 81 is complete; the linked two-phase CH1 repair is approved and allocated to Slices 82/83;
CH3–CH8 remain separately placed work, not additions to this release by implication. Resolve a specific CH4 contract first if CH3 needs it; never invent it
inside the benchmark. CH5 can land before temporal functionality. CH4 decisions
must precede any opt-in temporal population routed by CH5. CH8 node/edge/property
routing and graph qualification remain separate claims.

Additional slices are warranted: these units have independent acceptance/failure
boundaries. CH1–CH4 should be bounded follow-up slices; size CH5–CH8 from reviewed
designs and split further only for concrete boundaries. Place releases separately;
the completed Slice 81 POC and approved CH1 Slices 82/83 precede Slice 90.
Do not insert other units or move the 0.8.29 handover by implication.
Physical erasure remains an adoption blocker until fixed/qualified.

## Testing and resource acceptance

Each code unit gets behavior-specific RED staged/committed before GREEN, focused
blast-radius tests, Astra code review and independent verification. POC evidence
is separate from public acceptance. Current production integration runs unchanged
AC-076/T012 and AC-081a/b/c on actual Engine paths; affected AC-072/073/075 remain
binding. Ledger/native collector passes cannot qualify graph/fusion/evidence.
Do not rewrite failing tests to make candidates pass.

Bound admission before snapshots/corpus allocation, with explicit queue/resource
refusal and deterministic drop. Test default reader progress under history load;
concurrency/bytes/queue/time values remain unapproved. Admitted work initially
runs to completion: receiver drop does not cancel it; close/erase can wait or report
incomplete WAL completion. Queue deadlines cannot bound active work or shutdown.
No silent statistical-corpus truncation or successful partial ranking on failure.

Historical qualification includes entropy, duplicates, mixed load, construction,
hydration/drop, contention, WAL retry and reopen. Propose numbers after CH3 and
obtain acceptance before production qualification. Default limits stay unchanged.

Quality-roadmap consultation (2026-11-02): apply real-database differential,
boundary/property-state-machine, selected commit/repair fault and resource-cleanup
tests. New evidence: physical term retention, secure refresh cost and uncomposed
fragment/current selection. Defer broad VFS fuzzing, exhaustive OOM, MC/DC and new
test frameworks; bounded composition oracles address these findings. Documentation
uses the Markdown gate. Code phases use the required final production gate once,
with focused repairs rather than repeated full runs.

## Remaining owner decisions

- At the end of Slice 83, decide the physical-erasure guarantee and consider an
  explicit cleanup API using actual CH1 engineering evidence. The two-phase plan
  and Slices 82/83 allocation are approved; the guarantee/API choice remains open.
- Accept/adjust minimal decision 6 after CH4's proof/design; keep 7 open until its
  examples and Astra-medium review justify scope.
- Place follow-ups in the release ladder without moving other work by implication;
  approve detailed production plans when reviewable.
- Approve measured historical SLOs/resource values after mechanism comparison.

## Review and record status

Independent Astra-medium review approved this planning record on 2026-11-02 with
no required corrections: nine owner choices, minimal commit/evidence reuse,
existing graph grouping, erasure approval boundary and eight-unit dependencies
were checked against current contracts. This is planning approval, not the later
CH4 detailed design/code review or owner approval to implement CH1.

On 2026-11-03 Astra approved the revised two-phase CH1 proposal and CH2 evidence
reconciliation for HITL plan review, with no blocking findings. The
[concrete plan](ch1-production-erasure-plan.md) records the review and proposed
82/83 allocation. HITL subsequently approved both scope and allocation on
2026-11-03, retaining the guarantee/API decision at the end of 83.

## Availability research follow-up

The owner's 2026-11-03 clarification and subagent's primary-source research are
[recorded durably](../notes/dbms-erasure-availability.md). This changes the CH1/CH2
proposal and test questions, not shipped semantics or the earlier acceptance
results. Physical erasure remains an adoption blocker. Research is authorized;
the revised two-phase CH1 plan is now approved for Slices 82/83, with phase
design reviews still required before code.

## Slice 81 placement

Owner 2026-11-03 assigned the maintenance/erasure-availability POC to
[Slice 81](0.8.28/features/slice-81/README.md). Its package owns experiment
requirements/AC, design and test controls. This revises the earlier CH1-before-CH2
proposal: obtain mechanism/availability evidence before approving the production
repair. Existing Slice 90 remains after 81; historical and temporal work is not
allocated here. Slice 81 is complete with scoped results; CH1 Slices 82/83 now
own the approved production follow-up. Its findings do not close the production
erasure defect. The guarantee/API decision remains at the end of 83.

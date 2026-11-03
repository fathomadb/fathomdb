---
title: Current and historical search — decisions and staged follow-up
status: PROPOSED
date: 2026-11-02
target_release: unassigned
---

# Current and historical search — staged follow-up

Owner decisions follow the [reviewed results](../experiments/current-history-search/followups/RESULTS.md).
This records decisions and proposes delivery units. It does not reopen Slice 80,
allocate release slice numbers, change Slice 90 or authorize the whole program.
The [production design](../design/current-history-search.md) remains a candidate.
The eight CH labels below are planning labels, not assigned release slices.

## Decision record

| # | Owner direction | Recorded boundary |
| ---: | --- | --- |
| 1 | B: secure deletion and retained-term cleanup across registered FTS families | Direction selected; concrete approach below requires owner approval before production implementation. Node defect confirmed; each other family needs its own witness. |
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
approved POC is queued behind agreement on the erasure guarantees. Decision 4
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

## CH1 — Erasure approach; revision required before approval

Owner clarification (2026-11-03): full physical erasure may be lazy, but deletion
must be guaranteed under a defined SLA. Keep logical exclusion distinct from
accepted durable cleanup and verified physical completion. No numerical deadline,
SLA clock, worker mechanism or new API has been approved. The
[DBMS availability research](../notes/dbms-erasure-availability.md) informs the
revised proposal; the earlier eager policy below remains a comparison candidate.

Denial needs an explicit committed boundary for newly admitted reads and a
separate policy for existing pinned reads. Pending bytes must not taint authorized
corpus statistics, caps, graph or historical/evidence routing: publish safe
membership or refuse/await an affected scope. Output filtering alone is insufficient;
large closure/publication is not assumed constant-time.

A background worker alone is insufficient: the current physical-closure contract
blocks unrelated writes while any physical closure is owed. Scoped fencing,
publication/re-ingestion rules, automatic resume and acceptance-versus-completion
semantics need explicit successor design. Bounded finalization may still require
brief global reader/writer admission control for WAL truncation. An unbounded
reader or engine downtime cannot be hidden inside an unconditional deadline.
Decision 8's run-to-completion policy must be reconciled with this constraint;
no cancellation or global-fence change is approved here.

Repair the confirmed shipping defect independently of temporal features:

1. Inventory all registered FTS families and creation/recreation paths. Compare
   eager FTS5 per-table secure-delete with durable SLA-driven incremental cleanup
   and qualified replacement/rebuild. Retain core SQLite secure deletion and prove
   all registered-copy, telemetry and WAL obligations before physical completion.
   Do not preselect eager deletion on every ordinary projection update merely
   because it was the earlier reference policy.
2. Explicitly sanitize retained terms in admitted current-format stores. Candidate:
   transactionally rebuild/replace affected lexical projections from surviving
   authoritative content under secure deletion. Preserve each family's existing
   retained-history, lifecycle, source-policy, tokenizer and ranking membership.
   Qualify the exact command against each real FTS table mode. A flag, optimize,
   ordinary rebuild or WAL truncation alone is not proof of cleanup.
3. Discharge existing physical-erasure obligations. Failed repair, rollback,
   interrupted checkpoint or pinned WAL must not produce a clean-completion claim.
   Reopen/retry must preserve and finish the obligation without resurrection.
   Avoid rebuilding on every query or ordinary erase once initialization is safe.
   Existing recovery tooling does not establish retained FTS-term cleanup.
4. Preserve admission: refused formats remain refused before mutation. Cleaning
   residual terms in an admitted format is not cross-version migration. Unsupported
   old-file/offline repair is outside this proposal.

RED-first proof: retain the ordinary-node failing witness, then test actually
populated node, v2, edge and property families independently. Unique nonstemming
markers must be physically present before deletion. After complete erasure require
content/MATCH/live-shadow absence and applicable freed-page, main-file and WAL
observations, plus survivors. Test already-deleted terms, rollback, interrupted
cleanup/reopen, pinned-reader incomplete then retry, rebuild and no reintroduction.
An empty family supplies no positive removal witness. Reuse the registered purge
inventory and recovery/telemetry obligations rather than a second erasure system.

Expect extra posting deletion/update, property replacement, supersession-related
maintenance and one-time sanitation cost. The candidate's 4.6-second full refresh
is a warning, not measured shipping-erasure overhead. Measure ordinary writes,
deletes, supersession, physical erase, repair and surviving read parity separately.
Astra reviews design/code; independent verification reruns real witnesses. This
adds no promise about process/swap, filesystem snapshots or external backups.

SQLite distinguishes FTS5 per-table deletion from core freed-page deletion and
notes update/delete cost and format implications; see the
[official contract](https://www.sqlite.org/fts5.html#the_secure_delete_configuration_option).

## CH2 and CH3 — Bounded mechanism evidence

**CH2: secure bulk maintenance POC.** Compare incremental ordinary deltas plus
transactional replacement/rebuild or supported bulk clearing against the secure
reference. Use one ordinary mutation and a 10,000-record simultaneous expiry/repair
burst. Count construction, publication, queue time and WAL completion. Preserve
pinned snapshots, closure, rollback, visibility, certificate coherence and erasure.
A faster unsafe/stale success loses. With deferred physical cleanup, also measure
writer-lock hold time per unit, unrelated read/write progress, durable restart,
oldest debt/deadline and finalization pause. Page budgets are not time bounds.
Membership expiry/supersession is not source erasure of retained history.
Freeze controls before measurement; no broad new grid. Output a mechanism/cost tradeoff or reject the candidates. POC approved;
CH1 need not finish first if its agreed security guarantees are reproduced in isolation.

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
| CH1 | Physical FTS erasure, including retained terms | Concrete approach approval first; independently useful. Do not wait for temporal design. |
| CH2 | Secure ordinary/bulk maintenance comparison | Agreed CH1 guarantees; POC approved. Select no production mechanism before results. |
| CH3 | B-first, then C historical comparison | Stable scope oracles; B precedes C. CH4 defines exact future contracts; fixture probes can precede it. |
| CH4 | Minimal temporal/evidence/ranking contract/tests | Existing evidence; resolve 6/7 with Astra-medium. No repeated basic POC required. |
| CH5 | Production default-current isolation | CH1 fixed, CH2 viable; ordinary identity semantics. Integrate public ranking, property/graph seeds, certificate/frontier and repair. No full temporal dependency. |
| CH6 | Opt-in temporal nodes, atomic cuts and exact evidence | CH4 accepted, CH5 ready. Compose ledger/current projection: A at V10, B from V20, B at V30; source denial, frozen reads and erase. Refuse unsupported temporal edge/graph requests. |
| CH7 | Source-specific temporal edges/coherent graph | CH6, reviewed G11 successor and decision 7. Same V/K for nodes/edges, exact winning origin/provenance. |
| CH8 | Selected-history production routing/bounded envelope | CH3 selection, CH4 contracts, CH6 node semantics; CH7 before graph claims. Integrate admission/fairness, tested caps, no-spill policy and historical SLOs. |

Recommended serial order: CH1 approval/repair, CH2, CH3, CH4, CH5, CH6,
CH7, CH8. Resolve a specific CH4 contract first if CH3 needs it; never invent it
inside the benchmark. CH5 can land before temporal functionality. CH4 decisions
must precede any opt-in temporal population routed by CH5. CH8 node/edge/property
routing and graph qualification remain separate claims.

Additional slices are warranted: these units have independent acceptance/failure
boundaries. CH1–CH4 should be bounded follow-up slices; size CH5–CH8 from reviewed
designs and split further only for concrete boundaries. Place releases separately;
do not silently insert eight slices before Slice 90 or move the 0.8.29 handover.
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

- Revise and approve CH1's approach for availability and deferred guaranteed
  deletion before implementation; define physical scope, SLA clock/deadline,
  operating conditions, reader policy and deadline escalation after evidence.
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

## Availability research follow-up

The owner's 2026-11-03 clarification and subagent's primary-source research are
[recorded durably](../notes/dbms-erasure-availability.md). This changes the CH1/CH2
proposal and test questions, not shipped semantics or the earlier acceptance
results. Physical erasure remains an adoption blocker. Research is authorized;
production erasure implementation still awaits the revised reviewed proposal.

---
title: FathomDB work for Memex 0.6.0 — value/risk prioritization scaffold
status: DRAFT
date: 2026-09-12
baseline: FathomDB 0.8.25 and Memex 0.6.0 release designs
---

# FathomDB work for Memex 0.6.0

## Purpose

This is a portfolio scaffold, not a release commitment. It reconciles:

- the pre-0.8.25 FathomDB review of 24 Memex needs;
- the published 0.8.25 capability reconciliation performed in Memex;
- the accepted Memex/FathomDB responsibility split;
- the MMS, MACS, and breaking-cutover designs; and
- this branch's Slice 10 documentation, Slice 20 graph-evidence, and Slice 30
  edge-actuation plans.

Memex has many legitimate requests. They should not become one broad FathomDB
release. Work is ordered below by consumer value and unblock power, then by
stability risk, then by effort. A smaller read-only mechanism ranks above a
broader write, lifecycle, or retention mechanism unless the broader mechanism
is an immediate product blocker.

## Ranking method

Use ordinal judgments rather than a false numeric score:

- **Value:** cutover blocker, enabled-product value, operational assurance, or
  speculative/advanced value.
- **Stability risk:** effect on canonical writes, transaction/replay semantics,
  lifecycle/erasure closure, privacy disclosure, persisted formats, and public
  compatibility. This factor outranks effort.
- **Effort:** small, medium, or large, including cross-SDK/package verification.
- **Evidence threshold:** reproduced defect, currently gated Memex flow,
  concrete workaround that violates the responsibility contract, or only a
  future aspiration.

A candidate moves into implementation only when all are true:

1. An enabled or imminent Memex flow is named.
2. The current public surface cannot meet it safely.
3. The proposal is the smallest FathomDB-owned mechanism that removes the gap.
4. It does not move Memex semantic policy into FathomDB.
5. Failure, disclosure, replay, and upgrade behavior can be tested through a
   published artifact.

## Work already supplied by 0.8.25

Do not re-plan these as new FathomDB features:

- immutable canonical and derived artifact-revision identity;
- exact one-source provenance, locator, source version, and canonical hash;
- bounded atomic canonical-node, derived-node, dependency, and lifecycle
  actuation;
- direct one-source dependency lookup and closure;
- frozen reproduce-or-fail reads and eligibility before ranking;
- evidence-bearing frozen search and exact same-attempt evidence resolution;
- canonical and current-operational-state frozen pagination;
- projection generation/readiness and mutation correlation;
- bounded one-hop dependency tracing; and
- bounded deterministic constrained graph expansion.

Memex still has substantial adoption work for these mechanisms. Adoption is
not a reason for FathomDB to add another API.

## Priority 0 — finish and repair the released contract

These are the strongest candidates because they restore or explain an already
promised surface without expanding the data model.

### P0.1 Fix the frozen-explanation correlation defect

| Dimension | Assessment |
| --- | --- |
| Memex value | High: restores query-level and per-hit structural observability on the required frozen evidence path. |
| Stability risk | Low if fixed in the shared completion path and guarded by installed-artifact regressions. |
| Effort | Small. |
| Evidence | Reproduced against the published 0.8.25 wheel; both frozen explanation paths fail at `/correlationId`. |
| Recommendation | Fix first and publish only after wheel/npm/native consumer probes pass. |

The correction must finalize the same non-empty correlation identity on
`search_frozen(explain=True)` and
`search_with_evidence(include_explanation=True)` without changing ranking,
eligibility, evidence references, or the explanation-disabled path. Ordinary
non-frozen search is not an acceptable substitute.

### P0.2 Complete the frozen/evidence public guide

| Dimension | Assessment |
| --- | --- |
| Memex value | High integration value; prevents unsafe double-search and retry behavior. |
| Stability risk | Very low. |
| Effort | Small. |
| Evidence | Public post-tag docs inventory methods but still lack the full comparison, recipe, authority, and drift guidance. |
| Recommendation | Execute collaboration Slice 10 alongside the defect fix. |

The first fixed-version statement must be based on an installed artifact, not
source-tree tests alone.

### P0.3 Retain a Memex-shaped installed-artifact conformance witness

| Dimension | Assessment |
| --- | --- |
| Memex value | High stability leverage across releases. |
| Stability risk | Very low; test/example only. |
| Effort | Small to medium. |
| Evidence | The Memex reconciliation needed fifteen consumer spikes to discover a packaged contract defect missed by release tests. |
| Recommendation | Add a black-box witness for the supported one-source flow; keep Memex policy assertions in Memex. |

The witness should install the built wheel/package into a clean environment and
exercise actuation/replay/restart, frozen evidence search with explanation,
exact resolution, pagination, dependency lookup, and projection readiness. It
must not become a second semantic test suite or import Memex code.

## Priority 1 — strongest additive read-side capabilities

Read-only additions are favored because they unlock safe use without changing
canonical mutation, closure, or erasure behavior.

### P1.1 Resolve graph-target evidence by immutable artifact revision

| Dimension | Assessment |
| --- | --- |
| Memex value | High: enables constrained graph targets to enter the evidence-gated MMS path. |
| Stability risk | Low to medium: authorization and non-disclosure are sensitive, but the operation is read-only and reuses existing evidence state. |
| Effort | Medium across Rust, Python, TypeScript, wire, docs, and package probes. |
| Evidence | `GraphTargetV1` lacks artifact revision/evidence identity; body re-search is probabilistic and prohibited. |
| Recommendation | Execute collaboration Slice 20 before broader graph features. |

Prefer immutable artifact-revision lookup under an equivalent frozen context.
Expose target and terminal-edge revision identity separately. Do not implement
a logical-ID search filter merely to emulate a point evidence lookup, and do
not claim terminal-edge evidence proves a full path.

### P1.2 Provide a distributable read-only integrity inspection route

| Dimension | Assessment |
| --- | --- |
| Memex value | High operational assurance for import and cutover; prevents raw-SQL inspection. |
| Stability risk | Low to medium if quiescent, read-only, and operator-scoped. |
| Effort | Medium, primarily packaging and contract work. |
| Evidence | Integrity machinery and a Rust CLI exist, but the PyPI wheel installs no CLI and the Python SDK intentionally exposes no doctor/recovery verb. |
| Recommendation | First package/document the existing operator check; add an in-process SDK method only if that route cannot satisfy the cutover. |

The lowest-risk solution is not automatically “add doctor to Python.” Evaluate
a separately installed FathomDB CLI or narrow operator artifact with explicit
quiescence, version matching, machine-readable results, and no repair side
effects. Preserve the governed SDK recovery denylist unless Memex demonstrates
that an external operator route is insufficient.

## Priority 2 — high-value write-path work with stronger gates

These capabilities unblock product behavior, but touch transaction, replay,
receipt, projection, lifecycle, and compatibility invariants. They should not
be bundled with unrelated scope.

### P2.1 Add atomic derived-edge actuation

| Dimension | Assessment |
| --- | --- |
| Memex value | Very high if MMS graph authoring is enabled; otherwise the feature remains intentionally gated. |
| Stability risk | Medium to high because it changes the closed actuation grammar and canonical write transaction. |
| Effort | Medium to large. |
| Evidence | Memex otherwise needs non-atomic `actuate` plus `write`, violating its graph-authoring invariant. |
| Recommendation | Complete collaboration Slice 30's decision work, then implement the smallest versioned operation if graph authoring is an initial or near-term requirement. |

Reuse `ProvenancedEdgeV1` and the existing edge storage/projection path. The
working shape is `put_derived_edge`, not unqualified `put_edge`: provenance and
immutable revision identity are mandatory. Per HITL `seq-283`, change
`ActuationBatchV1` and its closed/exhaustive operation grammar in place; do not
introduce a parallel V2 surface during pre-1.0 development.

### P2.2 Improve mutation receipts only as part of actuation evolution

| Dimension | Assessment |
| --- | --- |
| Memex value | Medium: improves auditability, but Memex can safely wrap the existing compact receipt. |
| Stability risk | Medium because receipts are persisted/replayed and may retain sensitive identities. |
| Effort | Medium. |
| Evidence | 0.8.25 lacks a full before/after consequence manifest, but this does not block the reduced MMS profile. |
| Recommendation | Fold only proven fields into the actuation successor; do not launch an independent omnibus receipt redesign. |

Strong candidates are edge revision IDs, before/after lifecycle states already
known in the transaction, dependency-generation changes, and exact projection
work correlation. Do not duplicate Memex semantic verdicts or promise
consequences FathomDB did not compute.

## Priority 3 — valuable advanced memory, but correctness-heavy

These should enter design only after Memex proves that its enabled product
needs them. They carry substantially more lifecycle and erasure risk than the
items above.

### P3.1 Bounded multi-source provenance

| Dimension | Assessment |
| --- | --- |
| Memex value | High for corroborated claims, conflicts, summaries, and decisions. |
| Stability risk | High: liveness, erasure, supersession, and partial-source loss become coupled. |
| Effort | Large. |
| Evidence | The target MMS model needs it, but the initial honest 0.8.25 profile can reject multi-source materialization. |
| Recommendation | Design after the read/edge priorities; implement only for a named enabled flow. |

Keep the first form bounded and immutable. Do not combine source-set storage,
arbitrary dependency DAGs, recursive closure, and a general liveness language
in one slice.

### P3.2 Source-set liveness: `all_required` and `any_surviving`

| Dimension | Assessment |
| --- | --- |
| Memex value | High once multi-source materialization is enabled. |
| Stability risk | Very high because every source transition and erasure can change derived visibility. |
| Effort | Large. |
| Evidence | Required by the destination contract, not the initial reduced profile. |
| Recommendation | Follow, never precede, the bounded multi-source identity/dependency substrate. |

Specify every state transition, interruption point, resumption rule, and
zero-orphan proof before implementation. The grammar stays closed to the two
demonstrated policies; FathomDB enforces the caller's choice but never chooses
semantic liveness.

### P3.3 Bounded derived-to-derived dependency and recursive closure

| Dimension | Assessment |
| --- | --- |
| Memex value | Medium to high for layered claims, assessments, summaries, and plans. |
| Stability risk | Very high: cycles, amplification, fencing, and restart correctness dominate. |
| Effort | Large. |
| Evidence | Destination need; initial MMS can reject it. |
| Recommendation | Separate one-hop derived dependencies from recursive closure and require concrete depth/fanout bounds. |

Do not infer that “bounded” registration makes recursive lifecycle safe.
Cycle detection, admitted boundaries, work accounting, resumable closure, and
privacy-safe receipts require their own decision and stress evidence.

## Priority 4 — demand-triggered graph and observability expansion

Consider these only after real one-page/terminal-edge limitations appear in
Memex traces.

### P4.1 Full ordered graph-path evidence

Useful for explaining multi-hop support, but materially broader than exact
target and terminal-edge evidence. It requires bounded path identity,
same-snapshot authorization for every edge, deterministic path selection, and
careful non-disclosure. Build only if Memex must expose or audit the path, not
merely because traversal has one internally.

### P4.2 Deterministic graph continuation

The current 50-target/10,000-work-unit result is complete-or-fail. Add a cursor
only after Memex demonstrates a valid bounded query that cannot fit and cannot
be narrowed. A continuation must bind the frozen context, complete request,
ordering, and graph/projection generation; it is not ordinary ranked paging.

### P4.3 More complete exclusion/not-selected explanation

This has diagnostic value but meaningful privacy and performance risk. Start
with aggregate reason counts or caller-supplied candidate inspection before
returning per-record exclusion detail. Never disclose the existence or content
of ineligible records.

### P4.4 Richer indexed graph predicates

Add only predicates proven necessary after mapping Memex owner, workspace,
authority, retention, kind, and source class onto the existing declared
projections and frozen eligibility grammar. Reuse one grammar and indexed
lowering; do not create a graph-only filter language.

## Priority 5 — defer until measured demand overcomes stability cost

### P5.1 Persisted evidence replay

Memex already records durable artifact/source/version/locator/hash facts and
does not require old evidence bytes to remain readable. Persisted replay adds
retention, erasure, privacy, key-rotation, and storage obligations. Defer unless
a concrete audit workflow cannot use the existing Memex receipt.

### P5.2 Long-lived snapshot leases

The reproduce-or-fail frozen context is sufficient for the designed per-turn
attempt. Leases retain history and complicate compaction, erasure, resource
ownership, expiry, and crash recovery. Do not build them for convenience.

### P5.3 Deterministic candidate-selection profiles

Temporal, diversity, associative, routing, and global profiles remain
experiments. Prior availability of FTS/vector/RRF/CE/graph components does not
prove product value. Memex owns when a profile applies; FathomDB should add a
profile only after preregistered quality, latency, resource, and fixed-reader
gates show value.

### P5.4 General repair/job orchestration

Start with the read-only integrity route in P1.2. Public resumable repair jobs
expand the write and recovery surface substantially. Add a specific repair
only when an observed failure cannot be handled safely by the existing
operator recovery contract.

### P5.5 Exhaustive scale and platform expansion

Continue feature-local contention, package, Windows, and performance gates.
Do not claim million-record or managed-service parity from 50,000-record
evidence, but also do not block personal-memory work on an unrequested scale.
Expand only against a registered Memex workload and supported deployment.

## Explicitly do not build in FathomDB

The reviewed Memex designs assign these to Memex/MACS/MMS:

- memory admission, consent, owner/purpose/retention/disclosure policy;
- claim extraction, entity resolution, ontology, contradiction, truth,
  corroboration, consolidation verdict, and adoption;
- query intent, decomposition, reasoning, global synthesis, context packing,
  answer verification, citation choice, and abstention;
- personalization, goals/plans, prompt policy, model/tool selection, action
  authorization, provider/device/egress/spend, and HITL policy;
- Memex semantic and use receipts around FathomDB's structural receipts;
- migration disposition for ambiguous legacy memories; and
- the Memex adapter, version pin/lock, capability gates, and complete-system
  benchmarks.

Also reject these proposed shortcuts:

- body re-search or a logical-ID filter as a substitute for exact evidence;
- raw SQLite, private indexes, or a Memex shadow dependency/liveness store;
- synthetic one-source flattening of multi-source claims;
- non-atomic `actuate` plus `write` graph authoring;
- source dependencies overloaded as semantic graph edges;
- relation reification as nodes solely to avoid edge actuation; and
- non-frozen explanatory search substituted for frozen evidence retrieval.

## Suggested execution order

```text
repair published frozen explanation + finish docs + package witness
  -> immutable-revision graph evidence
  -> distributable read-only integrity inspection
  -> derived-edge actuation decision and versioned implementation
  -> narrowly improve the same actuation receipt
  -> observe Memex usage
  -> bounded multi-source identity/dependency
  -> explicit source-set liveness
  -> bounded derived dependency, then recursive closure if still required
  -> demand-triggered graph path/continuation/exclusion work
  -> retained leases, replay, profiles, and broad repair only with new evidence
```

The arrows express safety sequencing, not a promise to implement every later
item. Each arrow is a re-entry gate: if Memex can ship safely without the next
capability, stopping is a valid and preferred outcome.

## Coverage of the prior 24-need review

This table prevents a lower-ranked request from disappearing merely because it
does not deserve near-term implementation.

| Prior need | Current disposition |
| --- | --- |
| 1 durable identity | Shipped core; Memex adopts it. Slice 20 extends its use to graph evidence. |
| 2 canonical provenance | Shipped core; no new FathomDB feature. |
| 3 atomic semantic actuation | Shipped node/dependency/lifecycle core; P2.1 considers the proven edge gap. |
| 4 dependency registration | Shipped one-source core; P3.1 and P3.3 cover advanced forms. |
| 5 lifecycle closure | Shipped direct core; P3.2/P3.3 cover source-set and recursive consequences. |
| 6 erasure | Shipped direct core; extend only with the dependency forms that later ship. |
| 7 validity/frozen reads | Shipped reproduce-or-fail core; P5.2 defers leases. |
| 8 eligibility before ranking | Shipped closed grammar; P4.4 is demand-triggered grammar expansion. |
| 9 governed pagination | Shipped canonical/current-state pages; P4.2 covers graph continuation. |
| 10 source-complete evidence | Shipped search core; P1.1 adds exact graph-target use and P5.1 defers replay. |
| 11 eligibility-bound evidence | Shipped search core; P1.1 must preserve the same disclosure boundary. |
| 12 retrieval/exclusion explanation | P0.1 repairs the defect; P4.1/P4.3 defer rich path and exclusion detail. |
| 13 projection management | Shipped core; new writes must pass feature-local readiness tests. |
| 14 multi-source provenance | P3.1, followed by P3.2 only when required. |
| 15 constrained graph expansion | Shipped bounded core; P1.1 enables evidence, P4 covers richer graph reads. |
| 16 candidate-selection profiles | P5.3; experiment-gated. |
| 17 atomic consolidation application | Shipped supported core plus P2.1 for edge-bearing verdicts; multi-source remains P3. |
| 18 complete mutation receipts | P2.2, coupled narrowly to actuation evolution. |
| 19 operational tracing | Shipped bounded core; P0.1 repairs frozen correlation and P4 defers richer trace. |
| 20 integrity/maintenance | P1.2 for inspection; P5.4 for later repair orchestration. |
| 21 concurrent read/write | Standing acceptance gate for every new contract, not an independent feature program. |
| 22 predictable performance | Standing feature-local gate; P5.5 defers speculative scale expansion. |
| 23 cross-SDK/wire parity | Standing gate plus P0.3 installed-artifact witness. |
| 24 evaluation support | Keep data-plane receipts in FathomDB and semantic/end-to-end evaluation in Memex. |

## Promotion checklist for any candidate

Before moving an item from this scaffold into a release plan, record:

- the exact Memex feature and call path it enables;
- current typed failure and why capability gating is insufficient;
- the smallest public request, response, error, and version change;
- whether schema or persisted receipt formats change;
- authorization and non-disclosure precedence;
- lifecycle, supersession, dependency, erasure, and restart consequences;
- compatibility across Rust, Python, TypeScript, and wire decoders;
- RED, property, fault-injection, concurrency, and package tests;
- p50/p95/p99, memory, and mutation-to-ready budgets where applicable;
- the Memex consumer adoption test and rollback/degradation behavior; and
- explicit non-goals and a reason to stop rather than broaden the slice.

## Source map

The principal Memex inputs are the accepted roles-and-responsibilities record,
the MMS/MACS/cutover designs, and the completed 0.8.25 release reconciliation
under the supplied Memex 0.6.0 design worktree. The FathomDB-side baselines are
`dev/plans/memex-0.6.0-needs-in-fathomdb-0.8.25.md`,
`dev/plans/0.8.26-draft-scope.md`, the accepted 0.8.25 evidence/graph ADRs, and
the three collaboration slice plans in this directory.

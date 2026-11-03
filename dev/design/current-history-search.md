---
title: Current and historical search — production design after POC
date: 2026-11-02
status: PROPOSED
target_release: unassigned
blast_radius: canonical identity and temporal assertions; lexical projections; writer and readers; graph; frozen reads; evidence; erasure; Rust/Python/TypeScript contracts
---

# Current and historical search

This is the production design candidate following the
[qualified POC](../experiments/current-history-search/results.md).
The design is revised after the original POC and the
[follow-up experiments](../experiments/current-history-search/followups/RESULTS.md).
P1/P2 provide scoped evidence; P3 is PARTIAL. Production implementation has not started; performance
acceptance is UNQUALIFIED. The [additional POC proposal](../experiments/current-history-search/next-poc-plan.md)
identifies the evidence needed before selecting production schema and routing.
This document describes proposed behavior; [retrieval.md](retrieval.md) continues
to describe shipped behavior. No release slice or accepted ADR is changed here.

## Owner directions and implementation boundary

The [decision record and staged follow-up](../plans/current-history-followup-sequence.md)
records the owner's choices after the experiments. Erasure repair, current isolation
and temporal functionality are separate milestones. The registered-family erasure
approach awaits concrete approval before production implementation. The secure
bulk-refresh POC is approved; historical probes test certified native reuse first,
then selected statistics over persistent postings/partitions, with private FTS as
reference and a possible bounded path. No production backend is selected yet.

Explicit opt-in temporal operations, bounded admission with run-to-completion and
a separate historical envelope are agreed directions, with numerical limits and
release placement open. Atomic knowledge cuts/fragment evidence are a provisional
preference; view-specific result/ranking units remain under evaluation. The minimal
proposal reuses the writer/receipt boundary and stateless evidence where proven,
and preserves existing body/property deduplication and graph target grouping.
No general commit service, snapshot service or claim-ranking rewrite is approved.
Astra-medium contract review precedes temporal implementation. Existing source
policy, flags, G11 and public evidence contracts require explicit successors where
changed; this proposal does not reinterpret them.

## Changes considered and applied

| Change | Evidence after follow-ups | Production direction | Outstanding evidence |
| --- | --- | --- | --- |
| Eliminate history scans from normal current search | Initial selector scaled with history; P2 native node collector now preserves exact score/order and VM work across H0/4/16 with sub-2-ms healthy-read p99 | Current-index-first routing with bounded metadata certification; no history selection, vocabulary scan or corpus copy on certified default reads; body dedup may hydrate more than K | Integrated node/edge/public-ranking/graph-seed qualification; P1 fragment composition |
| Define validity-boundary refresh and fallback | P2 snapshot/coalescing/admission witnesses pass, but secure-delete full boundary refresh takes about 4.6 s | Same-snapshot certificate validation and writer-owned publication; incremental ordinary-write maintenance plus a separate bounded secure bulk-refresh/repair strategy | Simultaneous-expiry/bulk-repair comparison preserving erasure, rollback, pinned snapshots and WAL; no hard bound on executing publication yet |
| Reconsider historical backend | Initial private materialization beat a naive scorer; P3 full-corpus stress is costly and campaign PARTIAL | Private native materialization is a reference and explicitly bounded candidate, not the universal selected backend; first test certified native full-history routing, then term-scoped persistent statistics or finite partitions for arbitrary V/K | Exact scope/authority/all-name property certification; grammar parity and complete cost comparison; no validated production cap or historical SLO |
| Implement temporal semantics explicitly | Isolated graph oracle passed; P1 real-schema node ledger and P3 receipt-cut fixture selectors pass within stated restrictions | Separate claim identity, assertion revision and temporal fragments; preserve source policy; additive ledger remains a candidate | Successor contracts for competing claims, head/fragment mapping, arbitrary atomic batches and evidence carrier; P1/P2 composition and real temporal graph remain unqualified |
| Make existing acceptance criteria binding | Follow-ups provide scoped experiments, not integrated production acceptance | AC-076 and AC-081a/b/c qualify actual integrated engine paths; vector/stress/fidelity gates remain independently binding in their blast radius | Prescribed unchanged acceptance runs after candidate integration |

## Required meanings and statistical scope

| Read meaning | Valid-time selection | Knowledge selection | Result meaning |
| --- | --- | --- | --- |
| Current facts | Request's resolved now | Latest committed knowledge | Currently accepted applicable assertions |
| Corrected history | Explicit V | Latest committed knowledge | Best currently recorded account of V, including corrections |
| Prior belief | Explicit V | Explicit committed knowledge boundary K | Assertions recorded as accepted at V/K, subject to disclosure permitted now |
| Archive | Explicit version enumeration scope | May span revisions | Assertions and their status/provenance, not one graph snapshot |

A world change closes/opens application-supplied valid intervals. A correction
revises the account of the affected interval. Ingestion time does not determine
when the world changed. Structural revision preserves the stated validity;
retraction cannot revive an earlier error. The engine does not infer truth,
contradiction or source precedence from natural-language text or write recency.
Independent sources can retain conflicting claims with distinct provenance.

For identical admitted current facts, query and scope, changing excluded
superseded history must not change scores, order, candidate caps, graph seeds
or public result prefixes. Actual changes to admitted current facts may change
ranking. Node, edge and property statistical families remain separate. Property
name and ordinary content filters constrain output before limits; they do not
silently redefine the family's statistical corpus. True authorization is part
of scope and must not be treated as an ordinary content filter.

## Canonical temporal model and compatibility

The POC format is not a production schema proposal: it permits multiple
knowledge-current fragments per claim, whereas the accepted
[identity substrate](../adr/ADR-0.8.0-canonical-identity-substrate.md) enforces
one nonsuperseded governed row per `logical_id`. Preserve that invariant and
all existing anonymous `h:` identities; no backfill populates their logical IDs.
Do not reinterpret `superseded_at`, valid-time columns or canonical cursors.

The candidate mapping is an additive, opt-in temporal assertion ledger linked
to governed node/edge owners. Separate stable subject/relationship identity,
source-specific claim identity, immutable assertion revision, fragment identity
and governing commit boundary. Ledger fragments may express multiple disjoint
valid intervals at latest knowledge without adding multiple canonical heads.
FTS/vector references must identify the selected fragment and its intrinsic
source revision rather than substituting the current owner's body.
P1 established a scoped node mapping. Exact production DDL, ownership and public
carriers remain outputs of the staged contract/design work, not settled by
renaming POC tables. Temporal query support must not be claimed before this
mapping and write transaction are implemented.

A write closes overlapping assertion knowledge intervals and creates replacement
and unchanged outside fragments atomically. Half-open V/K boundaries, fragment
nonoverlap, immutable payload/provenance, partial corrections and nested
retractions are mandatory. Knowledge K is an authenticated committed order;
wall-clock-to-commit lookup, if desired, needs a separate explicit contract.
No implicit full bitemporal history is fabricated for pre-feature records.
Anonymous existing content stays outside opt-in claim versioning.

The accepted [G11 invalidation scope](../adr/ADR-0.8.1-graph-substrate-g11-migration.md)
supersedes body-bearing edges by endpoint/relation tuple. Source-specific
assertions must not silently bypass or redefine that rule. P1 demonstrated the
existing tuple-wide behavior; an explicit successor is still required before
independent competing claims use the new production operation.
Changes to corrected historical selection, governed heads and public read/write
shapes likewise require the corresponding ADR/interface successor.

Current disclosure authority remains independent of past assertion knowledge.
The POC's shared per-claim admission table must not replace shipped per-version
lifecycle or change `include_inactive` / `include_superseded` semantics silently.
Specify the application-policy scope separately; preserve the existing flag
matrix until an explicit successor defines the new operations. Ordinary source
closure behavior and stricter evidence-source authority must retain their
respective contracts. Historical knowledge never restores revoked authorization.

## Current projection and certificate

Maintain current node-body and edge-body FTS projections, and align property
projection membership with the same declared scope. Retain historical storage
for explicit selected views. Use the production query compiler, configured
tokenizer/stop-word policy and deterministic public tie ordering; the POC's
ASCII OR parser and claim-ID tie break are not production replacements.

Canonical/ledger writes, current lexical changes, source/lifecycle closure and
publication metadata commit together on the existing single writer. Maintain
membership deltas for affected records and dependencies. Rebuild may scan retained
history; ordinary publication and current reads must not re-scan it wholesale.
Projection enumeration for purge/source erasure includes every new family.
Boot/rebuild publishes usable state only after its certificate is coherent.
Dense projection work remains owned by existing bounded asynchronous machinery.

A certificate binds database and projection identity, projection/visibility and
dependency/policy generations, tokenizer/registry identity, statistical-scope
identity and a validity interval [published_at,next_boundary). It describes
actual published membership, not merely the canonical high-water cursor.
Only requests with matching scope/generations and effective time inside that
interval may use it. Backward time, changed policy, historical knowledge,
relaxation flags or a different authorization scope cannot borrow it implicitly.
Ordinary output filters preserve their documented baseline behavior.

Validation uses bounded singleton metadata reads in the same reader snapshot as
ranking. It does not enumerate selected IDs, scan canonical/projection history,
scan vocabulary or load every body. Native ranking operates directly on the
certified current FTS family; canonical hydration is bounded by the collector's
existing candidate/tie contract. Complete ties and public prefix stability
remain correct; an optimization cannot truncate an unexamined tie group.

Publication tracks an indexed frontier of relevant future validity boundaries,
including incoming/outgoing records and authoritative source/dependency bounds.
Future records outside today's index must still invalidate the certificate at
their start. Dependency propagation must follow existing closure barriers; a
certificate cannot publish around unfinished closure. Clock passage needs no
daemon: the next request detects an expired certificate. P2 must prove frontier
completeness; generation checks alone cannot detect a clock-only change.

## Request routing, refresh and failure

1. Resolve V once for the request. Control-plane admission may request the
   writer to ensure the default scope is published for V before reader dispatch.
2. The reader acquires one snapshot and validates the certificate in it. New
   writes may occur between admission and dispatch; only this validation decides.
3. If certified, rank/hydrate from that snapshot without reader writes or a
   history-dependent prepass.
4. If incompatible, release the read snapshot before waiting for writer refresh.
   The writer processes due membership changes and publishes atomically. Reacquire
   and revalidate; races cannot turn a stale certificate into an accepted one.
5. Handoff/coalescing and retry are bounded by a request resource/deadline policy
   validated by P2. No unbounded spin, per-reader refresh transaction or refresh
   while holding a read snapshot is allowed. Concurrent requests share refresh
   work for an equivalent scope rather than each rebuilding the corpus.

Refresh coalescing is interval-aware: a published certificate must cover each
request's original V. Explicitly test V1 before a boundary and V2 after it with
V2's refresh completing first. Never re-resolve V1 to make its request fit V2's
certificate, or let an older queued refresh publish over newer state/policy.
An older request obtains a compatible snapshot within its resource policy or
explicitly fails; it cannot borrow an incompatible certificate. Queued refresh
versus source denial/erase and boot repair/rebuild publication obey the same
generation and visibility rules.

For a strict default request, do not silently substitute an all-history scan,
naive custom scorer or private corpus build when certification fails. Refresh
must complete correctly within the supported request policy, or return an
explicit typed inability to complete. Exact public error spelling and numeric
resource limits are proposal outputs, not invented here. Such failures cannot
be counted as successful latency samples. Boundary bursts and contention costs
are included in supplemental performance measurements, not hidden as warmup.

Explicit historical/relaxed scopes use a selected-corpus backend under one held
main snapshot. Materialization remains a candidate, not an unconditional default
or a selected production backend. It must implement the complete supported query
compiler/tokenization semantics and use common statistics for a combined scope;
raw BM25 scores from separate indexes are not comparable. The existing custom
all-vocabulary scan is a correctness reference, not the production fallback.

Private corpora are per call, explicitly memory-only including SQLite temporary
storage, never pooled/cached, and dropped on success/error before response
completion. P3 must select measured corpus-byte, active-call and queued-work
limits and an explicit limit outcome without partial ranked success. No silent
corpus truncation, file spill, dropped version or post-top-K deduplication is a
resource policy. Receiver drop, cancellation, close and retained memory follow
one documented run-to-completion or cooperative cancellation choice. Do not
promise cancellation merely because a response receiver is dropped.

## Graph, frozen reads, evidence and erasure

Resolve nodes, edges, properties and provenance under one V/K scope and present
disclosure envelope. Both edge endpoints must be eligible before an edge seeds
or participates in traversal. Group semantic targets before their bound;
revision multiplication cannot consume current result/seed slots. Archive union
is refused as a snapshot traversal. Cross-time journeys remain a distinct scope.

Lexical, dense and graph candidate arms must agree on temporal fragment identity
and eligibility before candidate caps. A partial historical dense projection
cannot silently claim a complete arm; preserve explicit completeness/fallback
semantics. Fusion, reranking, explain and evidence must refer to the selected
revision, not a later hydration of its owner. Existing ranking rules remain
unless a separately reviewed contract changes them.

Frozen/paged contexts authenticate V/K, statistical-scope/certificate generation,
canonical and projection authority and disclosure policy. A historical selection
is not a frozen lease; preserve state-drift behavior. Artifact relaxation does
not relax evidence-source authorization. Reuse existing source-closure barriers
and at-rest erase implementation, extending its shared target inventory. Erased
fragments and FTS/vector entries cannot return through historical lookup,
reopen, rebuild, evidence or future materialization.

A held pre-write read may finish its coherent older snapshot. Erasure can report
`ErasureIncomplete` while WAL is pinned; retry and close behavior remain explicit.
Already returned bytes, process-memory zeroization and OS swap are outside the
current erasure guarantee. Longer historical calls and writer refresh may affect
queueing, close and erasure contention; P2/P3 must measure and report them.

## Implementation order and acceptance

P1/P2/P3 in the linked proposal precede main production implementation. Do not add
more semantic features or repeat the proved basic temporal POC. Once their
open decisions are resolved: propose the precise successor ADR/interface/schema,
implement canonical temporal writes with RED/GREEN, maintain/publish indexes,
wire every real entry point, and qualify the integrated candidate. No release
placement is inferred from this design revision.

| Qualification | Required verdict | Current status |
| --- | --- | --- |
| Three functional aspects | Engine tests for default score/limit isolation, update/correction/prior belief, coherent graph; source/erasure/frozen/binding coverage | Isolated POC PASS; engine integration NOT IMPLEMENTED |
| AC-076, using T-012 | Prescribed 10,000-row fixture, >=1,000 measured samples, p50 <=20 ms and p99 <=150 ms on actual Engine path | UNQUALIFIED |
| AC-081a | Unchanged 1,600-search sequential fixture <=500 ms; warning >=200 ms | NOT TESTED for candidate |
| AC-081b | Unchanged eight-reader/1,600-search fixture <=100 ms; warning >=80 ms | NOT TESTED for candidate |
| AC-081c | A distinct engine reader completes while its peer holds a live snapshot | NOT TESTED for candidate; POC WAL consistency is not this test |
| AC-072/073/075 | Existing vector latency, real-corpus stress and vector fidelity gates remain independently applicable to affected integrated paths | Not established by lexical POC |
| Supplemental current/history costs | Fixed current size/history ratios, clock/source boundary bursts, historical backend resources, writes/rebuild/erase/close | P2 fresh-fixture SCOUT executed: healthy reads isolate, secure bulk refresh about 4.6 s; P3 resource campaign PARTIAL; no new public threshold declared |

Thresholds/fixtures come from [acceptance.md](../acceptance.md) and the
[absolute-read ADR](../adr/ADR-0.8.25-absolute-read-performance-successor.md).
Do not infer a pass from 512-node/15-sample p50/p95 measurements, kernel-only
attribution, standalone surrogate queries or extrapolation. Report end-to-end
p50/p99 and failures. Existing acceptance fixtures/thresholds are unchanged;
supplemental history and clock tests extend evidence rather than rewrite them.

## Follow-up qualification boundary

The [follow-up results](../experiments/current-history-search/followups/RESULTS.md)
separate the three functional aspects from P1/P2/P3 probes, record all revised
campaign outcomes, and supersede any assumption that these probes are still unrun.
Healthy current-node reads isolate exact score/order and work from excluded history;
refresh, full public ranking/graph paths and integrated AC-076/081 remain unqualified.
P1 and P2 are not composed: production must prove partial/future-fragment selection
with old A current at V10 and new B selected at V30 before adopting the routing.

Universal private historical rebuilding is not selected. First probe a certified
native full-history fast path for exactly matching admitted statistical scopes;
then compare term-scoped persistent statistics or finite partitions for arbitrary
V/K against the private native reference. Authority, validity, all-name property
statistics and exact projection membership must be certified, not just counts.
Private MEMORY storage does not certify main-reader temp behavior; body dedup can
hydrate more than K; repetitive-payload RSS is not a general memory bound. There
is no accepted historical SLO or validated production cap from campaign budgets.

The ordinary-Engine node-FTS residual-term finding is a production adoption blocker.
Logical row removal plus WAL truncation is insufficient. Remediation must address
registered-table secure-delete policy and already retained terms, then qualify
rollback/WAL retry, visibility and unrelated-row preservation, cost and compatibility.
No production erasure rewrite is part of these experiments. Candidate-only P2
secure deletion qualifies fresh fixtures; it does not repair the shipping finding.

The final fresh P2 run retains sub-2-ms healthy-read p99 for its scoped native
node workload, but secure-delete-enabled full boundary reconciliation takes about
4.6 seconds versus 93–106 ms in the earlier candidate. This writer cost blocks
adopting the conservative full-refresh strategy as a low-latency design. Incremental
maintenance must be designed and qualified; the current result is healthy-read
isolation evidence, not production writer or boundary acceptance.

## Next evidence: secure bulk refresh

Incremental maintenance can avoid full reconciliation for an ordinary single
write, but does not eliminate 10,000 removals at a simultaneous expiry boundary.
The observed secure-delete cost therefore needs a separate bounded bulk-refresh
and repair probe before production strategy selection. Compare transactional
fresh-projection rebuild/swap or a valid bulk-clear mechanism with the current
reference while preserving certificate publication, pinned snapshots, erasure
ownership, rollback and WAL completion semantics. Measure the complete burst,
including construction and publication; do not hide it in warmup or widen request
budgets to make it qualify. No such optimization is implemented or qualified here.

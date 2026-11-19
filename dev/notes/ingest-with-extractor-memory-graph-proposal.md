---
title: Extractor ingestion identity and provenance investigation
status: PROPOSED
date: 2026-11-19
---

# Extractor ingestion identity and provenance investigation

Memex's concern is supported by current source. `ingest_with_extractor` is a
legacy graph-import convenience, not an exact, source-version-bound memory
assertion ingestion path. The smallest safe solution is to ingest source-local
entity occurrences and fact assertions using existing complete provenance and
bounded atomic actuation, leaving cross-source entity resolution to the caller.
A shared entity with multiple independent supporting sources is a separate,
larger design and requires a new owner decision.

## Evidence and limits

Inspected `release/0.8.30` at `5f4edd9831ddbb5ee12ebe594a3f6cd17fcc46b8`
(`plan: commission 0.8.30 Slice 30 correctness implementation`). This is source
and contract analysis; no new runtime reproduction or product tests ran.

The owner subsequently identified the consumer note in `coreyt/memex` at
`0785a803`: `dev/fathomdb/0.8.29-graph-arm-seeding-and-containment.md`.
Its status is **DRAFT for owner discussion — NOT FILED**. Read directly from
that commit, it requests graph-arm seeding, eligibility containment and
explanations. It does not directly specify the extraction identity/provenance
successor proposed here. These extractor findings stand independently; the
consumer note is assessed separately in the [graph seeding and containment review](graph-seeding-containment-consumer-proposal.md).

## Verified behavior

1. **Kind and identity have no caller scope.** Entity `kind` is copied from the
   model's `type`; its identity is SHA-256 of lowercased `kind:name`.
   `ExtractDocument` accepts only source ID and body. Two tenants or applications
   using `person/Alice` in one store therefore share one entity. Different kinds
   do produce different node IDs, but there is no configured kind mapping,
   namespace, or cross-document identity policy. Colons in the current derived
   kind reject, so blindly prefixing `memex:person` is not a working solution.
   Witnesses: `src/rust/crates/fathomdb-engine/src/ingest.rs:6`, `:160`;
   `src/rust/crates/fathomdb-engine/src/identity.rs:428`.
2. **First-source provenance is deliberate.** Batch dedup keeps the first node;
   an already active node is skipped, so later mentions add no source relation.
   The node carries the first source ID. The current caller-grounding check
   correctly prevents arbitrary model-supplied source IDs, but records only one
   selected source. Witness: `ingest.rs:96`, `:187`, `:204`, `:427`.
3. **Endpoint resolution is weaker than node identity.** The response index keys
   only lowercased names/aliases. A canonical name wins over an alias; same-name
   entries of different types are first-wins. Thus two distinct nodes can be
   written while an edge silently selects the wrong typed occurrence.
   Witness: `ingest.rs:229`–`:273`, `:366`–`:383`.
4. **Facts also lose independent source support.** Edge identity uses endpoints
   and relation, excluding source. Moreover, body-bearing writes invalidate
   prior active edges with the same `(from_id,to_id,kind)` even if their logical
   IDs differ. Source-scoping only an edge ID does not solve this.
   Witnesses: `ingest.rs:380`; `write_commit.rs:808`–`:820`.
5. **A source bucket is not complete provenance.** This method constructs plain
   `PreparedWrite::Node/Edge`; these register `legacy` artifact revisions with
   `migrated_incomplete` completeness, without exact source links. It does not
   register source dependencies. Consequently the later exact-source evidence
   and registered dependency closure machinery cannot infer these relations.
   Witnesses: `ingest.rs:174`, `:390`; `write_commit.rs:161`–`:190`.
6. **The operation has separate write commits.** Nodes commit before timestamp,
   confidence and source-selector validation of edges; each provider batch is
   processed independently. A later edge or provider-batch refusal can leave
   earlier nodes/batches committed. Existing write-boundary tests cover provider
   handshake/request-ID failures before writes, not a transaction around the
   whole ingestion call. This is a limitation, not evidence that `Engine::write`
   violates its own atomicity contract. Witnesses: `ingest.rs:78`, `:224`,
   `:307`, `:410`; `tests/write_boundary_atomicity.rs:538`, `:562`.

An example: A first mentions Alice, then B creates `Bob -> Alice` under B's
provenance. Erasing B leaves A's Alice, consistent with stored ownership but not
a complete mention history. Erasing A deletes Alice; B's differently owned edge
can remain stored, and traversal cannot reach Alice because it joins eligible
canonical nodes. Current source erasure deletes by source bucket and registered
dependencies, not every unrelated incident edge. `purge(logical_id)` instead
has a distinct all-version endpoint-cascade contract. Neither reassigning
Alice's `source_id` to B nor collecting a JSON list repairs these semantics.
Witnesses: `erasure.rs:756`, `:1114`, `:1374`–`:1401`, `:1466`;
`graph_expand/traversal.rs:145`–`:164`. Re-ingesting may recreate an endpoint;
that is not automatic support-aware repair. Plain extracted artifacts are not
registered dependencies of a document revision, so source supersession cannot
be assumed to hide them either.

**Authority:** TC-32 was explicitly accepted as-is on 2026-07-20, with no behavior
change and no unconditional erasure-completeness claim
(`dev/plans/plan-0.8.20.md:892`; `ingest.rs:427`). A new requirement may supersede
that decision; investigation does not itself do so. Global active identity
remains `logical_id` alone. Scope the derived IDs, not storage uniqueness keys.

## Practical designs

| Design | Benefits | Costs and constraints |
| --- | --- | --- |
| Source-local occurrences/assertions | Uses today's one-source provenance and dependency closure; independently erasable sources; no new shared-support storage | Cross-source grouping/resolution is caller policy; graph consumers must deliberately traverse occurrences or application-owned entity links |
| Shared canonical entities plus per-source support assertions | Preserves one application entity and independent support from A/B | Requires entity admission ownership, support enumeration, last-support removal and any-surviving/all-required rules; endpoint and fact invalidation changes; potentially new schema and erasure proofs |

Recommend the first design for a bounded FathomDB improvement. A kind prefix
alone cannot repair provenance, and replacing first-source with last-source
only changes which document's deletion breaks the graph. A true shared-entity
solution should build on explicit caller-owned canonical IDs and individual
source assertions; it must not turn a canonical row's scalar `source_id` into an
implicit many-source claim. Memex owns semantic identity, ontology, merge and
truth policy; FathomDB owns exact binding, atomic mutation and lifecycle proofs.

## Proposed bounded solution

Draft a successor extraction contract, then update the existing method and all
three SDKs together; do not silently change v1 identities or add compatibility
shims. Proposed shape:

- Caller supplies an opaque identity scope, an explicit allowed mapping from
  extracted entity/relation types to application kinds, and exact already-stored
  canonical source-revision references. The engine reads and verifies the source
  body/version, rather than treating an arbitrary body plus a source bucket as
  exact evidence. An identity scope avoids collisions; it is not an ACL or a
  tenant authorization boundary.
- A v2 provider result identifies entities by response-local opaque references;
  edges name those references and the selected document slot. Reject duplicate,
  missing, foreign-document or ambiguous references. Names/aliases become
  display/search data, never endpoint addresses. Unknown types reject against
  the caller's mapping instead of creating arbitrary application kinds.
- Derive occurrence IDs from a domain-separated, unambiguously encoded tuple of
  scope, exact source revision, mapped type and local entity reference. Names
  are not global identity. Different revisions are different occurrences;
  replay of the same accepted extraction is stable. Facts use source-local
  endpoints, mapped relation and a stable local fact reference, so independent
  sources do not collide with G11's endpoint/relation invalidation rule. In the
  bounded profile, reject multiple body-bearing assertions for the same
  endpoint/relation axis within one source revision before mutation; distinct
  fact IDs alone cannot preserve them. A richer assertion model is separate scope.
- Construct `ProvenancedNodeV1`/`ProvenancedEdgeV1` with exact one-source derived
  provenance. Use validated whole-body locators initially; byte-span support
  needs its own valid UTF-8 and trace proof, including the correction proposed
  for [issue #264](dependency-trace-partial-span-issue-264.md). Register direct dependencies for
  every occurrence/assertion. These link to the canonical document revision,
  not derived occurrences, respecting today's direct single-source bound.
- Reuse `Engine::actuate`: it already supports derived-node/edge puts and source
  dependency registration in one 1–128-operation transaction, with prospective
  active-endpoint validation and replay receipts (`dev/interfaces/rust.md:943`).
  Extraction runs outside the writer lock; source eligibility is revalidated
  inside actuation. Bound each document's entire result, including registrations,
  to that transaction and reject an oversized result before mutation. Do not
  split one accepted graph silently across writes. Multi-document progress must
  report the exact committed document receipts if a later document fails.
- Reuse existing operation-ID/digest replay and immutable artifact-revision
  rules. An identical accepted plan replays without duplicate rows; a different
  model output under the same operation identity refuses. Cold LLM generation
  is not a determinism guarantee. Preserve native closing/security/erasure
  admission precedence, then validate request shape, source binding, provider
  result/endpoint/type/time fields before mutation; assert precedence in tests.

This should need protocol, method-input, receipt and interface/design changes,
plus SDK/harness/conformance updates. The recommended storage representation
fits existing source links and dependencies; confirm actuation can consume the
complete bounded prepared result before claiming no schema change is needed.
No new multi-source table, migration or global graph rewrite is proposed.
A successor ADR must explicitly address the extraction identity contract and
TC-32; historical data cannot recover missing support merely by changing IDs.
Require explicit re-extraction into the new caller namespace when adopting it.

## Acceptance and delivery scope

Real-store tests should cover: same name under different types and identity
scopes; same-name homonyms within one response; identical entities and facts in
A/B; erasing either source independently; source supersession/deletion and
registered closure; retained historical views; same-result replay and changed
result refusal; missing/ambiguous/cross-document endpoints; malformed type,
provenance, time and locator with zero partial writes; concurrent source loss
between extraction and actuation; bounded multi-document partial progress;
SDK error/receipt parity and fresh-process reopen. Check raw rows, projections
and source links where search would conceal an orphan. Preserve existing
recovery-denylist and numerical controls. A pending runtime reset must not
create half a graph or reuse a stale source binding.

The investigation does not expand commissioned Slice 30. Its existing
correctness hunt could consume a separately approved, narrowly demonstrated
endpoint/refusal defect. The proposed extraction contract is new scope and an
explicit successor to accepted identity/provenance behavior; it needs an owner
placement decision and independent design review before implementation.

Until then, Memex can run its extractor under application control, assign its
own scoped IDs/kinds, and submit bounded source-local complete-provenance puts
plus dependency registrations via existing atomic actuation. Keep any shared
entity resolver/application admission separate from document support. Do not
use the current import method for a source-complete memory graph, and do not
claim that all references to a document are captured by a first-source row.

Owner choices: adopt source-local occurrences now or design shared canonical
entity support; choose the caller kind/identity policy and replay/partial-progress
contract; authorize a breaking extraction-protocol/API successor and its release
placement. The located Memex note concerns graph retrieval; align this extraction successor
with the consumer's concrete identity and source-lifecycle needs before adoption.

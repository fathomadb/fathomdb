---
title: 0.8.25 independent design review — Slices 35–55
status: COMPLETE
review_cycle: 0
reviewed_on: 2026-09-01
verdict: CHANGES_REQUIRED
---

# Independent design review — Slices 35–55

## Scope and verdict

Reviewed the Slice 35, 40, 45, 50, and 55 designs against their owning plans,
R25/AC25 packages, N25 and Memex needs, A25-01/02/03/05/06/07, approved
architecture v2, predecessor dispositions, Slice 15 wire rules, and dependency
order.

**Verdict: CHANGES_REQUIRED.** Nine P1 and six P2 implementation-shaping
findings remain. The documents correctly keep semantic policy outside
FathomDB, preserve historical designs/receipts, use `operational_state` as the
governed surface, and block READY on Slice 7. No P3 findings are recorded.

## Findings

### DR35-01 — P1 — Eligibility equivalence conflicts with evidence resolution

**Location:** `slice-35/design.md`, Public and wire contract, lines 54–58 and
Execution invariants, lines 91–93; `slice-50/design.md`, Codec, persistence, and
flow, lines 62–66.

Slice 35 rejects any changed eligibility and binds one eligibility digest,
while Slice 50 accepts an “identical or narrower” context. No canonical
normalization, subset test, or intersection semantics exists. Implementations
could disagree on authorization and allow a handle under a context that Slice
35 would reject.

**Correction:** make v1 require the exact originating frozen `ReadContext`
(while always rechecking current lifecycle/erasure/access fences), or fully
define a canonical predicate-normalization and subset algorithm. The safer
decision-complete v1 is exact context; defer narrower contexts. Align both
designs, failures, and tests.

### DR35-02 — P2 — Snapshot lifetime and expiry semantics are unspecified

**Location:** `slice-35/design.md`, Public contract lines 50–58 and Persistence
lines 66–80.

`expires_at` and pruning are required but no clock, default/max lifetime,
caller control, renewal rule, or expiration/pruning ordering is defined. Later
cursors and evidence receipts depend on this lifetime.

**Correction:** define one persisted UTC expiry policy, maximum/default TTL,
whether callers may request a shorter TTL, no renewal for an existing token,
expiry precedence, bounded pruning cadence, and exact typed behavior before and
after lease-row pruning. Carry the same lifetime into Slices 45 and 50.

### DR35-03 — P2 — Pre-Slice-40 projection bindings have no stable format

**Location:** `slice-35/design.md`, Persistence lines 73–80.

Slice 35 must pass restart/rebuild snapshot tests before Slice 40 exists, but
the design says Slice 40 later “replaces” effective projection identity without
defining what Slice 35 stores or how pre-generation leases migrate. That leaves
token compatibility and drift classification to implementation.

**Correction:** define `ProjectionBindingV1` now using the strongest existing
durable projection identity and boundary. Specify that Slice 40 atomically maps
an unexpired binding to one generation only when equivalence is proven;
otherwise consumption returns `SnapshotDrifted` or
`ProjectionGenerationUnavailable`. Never silently rebind.

### DR40-01 — P1 — Generation activation can falsely coexist with non-ready work

**Location:** `slice-40/design.md`, contract lines 31–42 and Persistence lines
58–68.

Generation state and work readiness are separate, but the activation predicate
is absent. It is unclear whether a `building` generation with degraded,
blocked, deferred, or a ready-prefix gap may become `active`, and how a migrated
ambiguous generation can be both serving and “never ready.” This is directly on
AC25-40's false-readiness boundary.

**Correction:** define generation-level readiness and activation exactly. A new
generation may activate only when every applicable work item through its fixed
source boundary is `ready`, with no pending/degraded/blocked/deferred gap.
Represent legacy serving state separately from readiness; an ambiguous migrated
generation may remain the compatibility serving generation but must report
degraded and cannot satisfy a frozen-generation readiness claim.

### DR40-02 — P2 — `ready_through` applicability and advancement are ambiguous

**Location:** `slice-40/design.md`, contract lines 35–42 and Persistence lines
52–64.

A scalar contiguous boundary is defined over work rows keyed by work kind, but
the design does not state which mutations are applicable, whether no-op work
gets a row, or how multiple work kinds advance one boundary.

**Correction:** define an applicability manifest per generation. Every
applicable `(mutation boundary, work kind)` receives a durable terminal or
non-terminal row; inapplicable mutations receive an explicit deterministic
skip marker or are excluded by a persisted rule. Advance `ready_through` only
when all applicable kinds for every prior boundary are ready.

### DR40-03 — P1 — Erasure omits retired and failed generations

**Location:** `slice-40/design.md`, Invariants/lifecycle lines 83–86.

The design fences until exposing `active/building` generations are clean, but
retired and failed generations may still retain indexed source bytes. Erasure
requires removal from every derived artifact, not only readable generations.

**Correction:** erasure must scrub active, building, retired, and failed
generation storage plus work payloads before completion. Retired-generation
reclamation may wait for leases during ordinary rebuild, but erasure must
invalidate affected leases and scrub immediately. Add raw-table canaries for
all states.

### DR45-01 — P1 — A cursor is used without a mandatory current authority context

**Location:** `slice-45/design.md`, Public contract lines 33–47 and Invariants
lines 70–79.

The type requires context, but prose says continuation supplies only cursor and
limit. An authenticated cursor identifies a prior envelope but cannot authorize
a current read. The behavior on changed current access/lifecycle is therefore
unresolved.

**Correction:** require current `ReadContextV1` on every continuation, not only
the first page. Under DR35-01 v1 it must exactly match the frozen context, and
current lifecycle/erasure/access fences are rechecked before emitting each
page. Define non-disclosing error precedence for mismatch versus newly hidden
records.

### DR45-02 — P1 — `read_graph_page` has no governed graph request or item shape

**Location:** `slice-45/design.md`, contract lines 33–47 and flow lines 60–64.

`read_graph_page`, `GraphPathKeyAsc`, and generic `PageV1<T>` do not identify
whether the operation pages nodes, edges, neighbors, or expanded paths, nor the
seed/direction/view inputs or stable key tuple. Slice 60 separately owns
combined expansion, so this ambiguity affects both scope and implementation.

**Correction:** scope Slice 45 graph pagination to an explicitly named existing
governed graph read (for example direct adjacency), define its request and
versioned item type, seed/direction constraints, stable edge/node ordering, and
deduplication. State that combined search expansion/path continuation belongs
only to Slice 60.

### DR45-03 — P2 — Public response records omit Slice 15 wire requirements

**Location:** `slice-45/design.md`, Public contract lines 33–41. Related:
`slice-40/design.md` lines 39–42 and `slice-55/design.md` lines 35–47.

`PageV1`, `OperationalStateRecordV1`, `MutationProjectionWorkV1`, and nearly all
Slice 55 response records omit `schema_version` and do not state unknown-field
behavior, despite A25-05 and Slice 15 requiring it on every new public or
persisted type.

**Correction:** add `schema_version: 1` to every public object (opaque scalar
tokens retain an internal version), and explicitly inherit Slice 15 request,
response, persisted-payload, unknown-field, unknown-variant, u64 wire, error,
SDK, Windows, and registry rules. Pin canonical fixtures.

### DR50-01 — P1 — The opt-in evidence carrier is undefined

**Location:** `slice-50/design.md`, Public contract lines 31–47.

The design says opt-in search returns a “sibling reference” without defining a
response carrier, ordering/key relationship, missing-handle representation, or
how it composes with the accepted explanation sidecar. Implementations could
grow `SearchHit`, return an unkeyed parallel list, or misassociate evidence.

**Correction:** define one versioned evidence sidecar on the opt-in search
response, with entries keyed by immutable hit identity and result index (or a
strict equal-length parallel vector), and require a typed whole-request failure
if any requested handle cannot be created. Default `SearchResult` and
`SearchHit` remain byte/allocation-identical.

### DR50-02 — P1 — The retrieval receipt required by `EvidenceRef` has no design

**Location:** `slice-50/design.md`, Codec/persistence lines 51–60.

The handle contains a retrieval-event ID and resolution promises exact ranking
contribution, but the “bounded privacy-safe retrieval receipt” has no schema,
storage, transaction point, lifetime, erasure behavior, or failure/recovery
semantics. Existing telemetry does not durably guarantee this contract.

**Correction:** define a versioned Engine-owned evidence receipt keyed by the
retrieval-event ID, written atomically before handles escape. Pin per-hit
projection/arm/rank/contribution fields, snapshot/generation binding, maximum
size, lease-equal retention, restart behavior, and erasure cleanup. It must
contain IDs/scores only, not query/source bytes. Missing or incomplete receipt
prevents handle creation.

### DR50-03 — P1 — Non-disclosure error precedence is contradictory

**Location:** `slice-50/design.md`, Public contract lines 42–47 and flow lines
62–66.

The API exposes distinct `Invisible`, `Erased`, `Stale`, and `Mismatched`
outcomes while also promising that unauthorized callers cannot learn whether
an identity exists. The order and authorization threshold for returning the
specific states is undefined.

**Correction:** define an explicit validation/error precedence. Authentication,
database/context mismatch, and current authorization/invisibility collapse to
one non-disclosing outcome. Only after the caller is authorized for the bound
identity may integrity/state outcomes such as erased, corrupt, stale generation,
or missing receipt be distinguished. Pin indistinguishable payloads/timing-safe
bounded paths in negative tests.

### DR50-04 — P2 — “Bounded summaries” can silently violate source completeness

**Location:** `slice-50/design.md`, flow lines 62–66 and invariants lines 74–78.

Dependency and contribution data are described as bounded summaries, but no
cap, truncation marker, continuation, or refusal is defined. A multi-source
record can therefore look source-complete while omitting provenance.

**Correction:** return complete dependency/contribution data up to declared
caps and include explicit `complete`/continuation metadata, or refuse resolution
as `EvidenceUnavailable` when completeness cannot be represented. Never return
an unmarked partial source set.

### DR55-01 — P1 — Trace continuation cannot reproduce a bounded graph walk

**Location:** `slice-55/design.md`, contract lines 31–48, flow lines 61–63, and
invariants lines 82–90.

Depth and page size do not bound a high-branching traversal across pages, and a
Slice 45 keyset cursor alone does not preserve BFS frontier/visited state for a
cyclic dependency graph. “Explicit/resumable” is not implementable from the
defined fields.

**Correction:** define total node/edge/work caps and one deterministic
continuation algorithm. Either persist an opaque trace lease containing
frontier/visited state under the frozen snapshot or deterministically recompute
from the root and resume after a fully specified traversal key. The cursor must
bind root, direction, depth/caps, context, generations, and ordering. Exhaustion
returns explicit truncation/limit, never an incomplete trace labeled complete.

### DR55-02 — P2 — Integrity jobs and repair actions are not closed or bounded

**Location:** `slice-55/design.md`, contract lines 44–57, flow lines 72–78, and
invariants lines 89–90.

`check_integrity` has no request/page/progress contract, and `RepairPlan.actions`
is unconstrained. “Rebuild or reconcile indexes” may be larger than one writer
transaction and could be interpreted to rewrite caller authority.

**Correction:** define versioned bounded integrity job/page/status types and a
closed repair-action enum limited to Engine-owned state: rebuild a named
projection generation, regenerate a reverse dependency index from authoritative
sets, or resume an existing closure operation. Small metadata repairs may be
atomic; rebuild actions enqueue governed generation work and return its receipt.
No action may create/delete dependencies or change canonical/caller lifecycle
state.

## Boundary and history checks

- **Semantic boundary: PASS.** The reviewed designs leave intent, truth,
  entailment, synthesis, and semantic repair outside FathomDB.
- **Historical preservation: PASS.** Predecessors are explicitly reused,
  superseded, or treated as historical/evidence; no accepted design or measured
  receipt was silently rewritten.
- **Dependency sequencing: CHANGES REQUIRED.** Slice 35→40 binding migration,
  35/45/50 authority context, and 45/55 continuation must be reconciled before
  any affected design becomes READY.
- **Readiness: BLOCKED.** Besides the findings above, all reviewed designs
  correctly remain blocked on Slice 7 architecture activation.

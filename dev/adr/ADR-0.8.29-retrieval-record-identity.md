---
title: Retrieval record identity for fusion and provenance
status: accepted; implementation pending
target_release: 0.8.29
---

# Retrieval record identity

## Context and authority

Slice9 Batch3 approves identity census and an ADR **before** exact text/snippet
implementation. Owner 2026-11-14 selects the successor below (seq-374); source implementation and qualification remain pending. Current source at frozen parent
`2f6913c1d59ac8ff3a30de835b57290a8635998c` deliberately merges equal bodies.
[Source/consumer census](../plans/0.8.29/features/slice-50/census.md) and
[requirements](../plans/0.8.29/features/slice-50/requirements.md) document the
actual delta. The source/consumer census covers the approved census scope without a private
Memex snapshot. Optional runtime population characterization supplies frequency/
benefit evidence, not migration or correctness proof. Actual30 baseline/source
prerequisites remain; no measured benefit or source entrance follows from this ADR.

Accepted constraints retained:

- [Canonical identity](ADR-0.8.0-canonical-identity-substrate.md): logical_id alone
  within each canonical table, retained revisions, no kind-based identity forks.
- [Embedder-owned identity](ADR-0.6.0-vector-identity-embedder-owned.md): no identity
  strings in vector configs. Provider identity is unrelated to fusion identity.
- IdSpace is permanent/write-time stable: l: entity, h: anonymous content, p:
  synthetic passage; anonymous surrogates remain cancelled. Stored artifact/source
  revision ownership is distinct from IdSpace, cursor, erasure source and view.
- Existing format37/visibility/span substrate only; owner seq-368 allocates no
  database migration. No pre-1 compatibility shim or optional legacy-fusion knob.

## Alternatives considered and selected decision

| Option | Result and consequence | Recommendation |
| --- | --- | --- |
| Keep body identity | Equal text across revisions, sources and node/edge rows merges; representative provenance can hide other records. Lightweight exact retrieval must still discover body equivalence. | Viable only if HITL explicitly wants text-unit presentation; current behavior, no successor inferred. |
| Logical/public-ID identity | Versions of one l: merge; h: twins still merge; node/edge logical-string collisions need namespace policy. Changes remain necessary for mixed governed/anonymous cases. | Not recommended for historical record retrieval; entity-centric semantics would need explicit representative/version selection. |
| Snapshot-local record-version identity | Canonical(Node/Edge,cursor); explicit ownerless node FTS uses (table,rowid). Same record across arms agrees; distinct records retain provenance even with equal body or public ID. | Selected by owner 2026-11-14. Does not redefine canonical lifecycle/public ID. |

Under the selected snapshot-local record-version option: original arm positions are not renumbered after
repeat-credit suppression; each record contributes once per arm at first rank.
Preserve K30 and weights1/3/1, score accumulation order, vector-input tie preference
and stable insertion ties. Keep direct-text candidate policy and node-then-edge
text concatenation. Existing nested graph fusion is unchanged; this ADR selects
identity, not a ranking experiment.

Graph traversal still visits logical entities/resolves the latest eligible node
as designed. Selected provenance is existing representative-first ownership
(vector input, then text, then graph); graph-only representatives retain the first
winning edge/entity seed and separate evidence origin. A body twin never lends
source/span/coverage. Owner selected this provenance policy with record-version identity.

Ownerless FTS is an explicit case, not permission to infer canonical ownership
from public IdSpace or branch. Keep ownerless content identity and absent source
truthful. Conflicting/malformed ownership needs an explicit testable disposition
using current error/reference behavior; never silently body-merge or mutate
storage to repair it. The source carrier must retain the real FTS locator.

## Public reference and consumer choice

[Repeated-ref decision](../plans/0.8.29/features/slice-50/feedback-decision.md)
separates existing historical SDK duplicates,60's newly reachable adapter flags,
and additional50 equal-body exposure. Owner selected bounded Unique/Ambiguous
feedback state and atomic typed refusal for named ambiguous labels only on
otherwise-recordable calls, preserving existing not_recorded precedence. The
correction is planned for .29; richer per-result occurrence labels are a .30
review follow-up, not a deferral of the .29 fix. No new public field is selected.
Logical fetch remains latest-eligible under its requested view and does not
promise exact searched revision retrieval. A feedback fix does not change it.

## Selected context-packing disposition

The existing SDK context_item_from_hit copies IdSpace's prefixed value and
returns parts decoded from the stored body (`standalone.rs:304–317`). Existing
pack_context rejects repeated IDs across hits/context before rendering
(`context_pack.rs:351–373`), mapped to SDK InvalidArgument. Thus equal anonymous
stored-body twins newly retained by record fusion share h: and helper→pack
changes from one packable item to duplicate-ID refusal. Different-body historical
versions already can share l: and refuse today. Do not promise packability from
record-unique fusion when public IDs are not occurrence-unique.

Owner explicitly preserves these helper/pack/codec contracts and acknowledges
that composed consequence. Existing caller-owned unique
ContextItem labels plus a label→selected-record map can represent both items;
packing labels are not IdSpace, artifact revisions or stable engine IDs. If the
owner requires automatic helper→pack acceptance of every hit list, a separate
helper-label or pack-contract change must be proposed and ruled. No override,
automatic relabeling, duplicate acceptance or new ID ABI is inferred here.

O3-09 role-tagged cases include equal stored tagged bodies on distinct records
and equal plain text with different tags. Assert chosen records, exact stored
bytes and original-body snippet coordinates; preserve codecs. Stage identity
assertions before the identity fix; unchanged codec/pack assertions are controls.

## Implementation consequences

- Carry selected keys through fusion, explanation ranks/scores, graph exclusion,
  weighting/evidence/snippets and final same-build SDK/MCP parity. Preserve actual
  graph path and frozen authority; do not deduplicate by body in one downstream map.
- Existing doc-hidden exported fusion helpers currently receive SearchHit only;
  it does not contain an ownerless FTS rowid or unambiguous artifact class. The
  implementation design must reconcile that helper contract with the internal
  keyed carrier before code. Do not guess keys from branch/body or introduce a
  compatibility adapter. Any public signature/type delta requires its interface
  update and review; this decision does not silently select an added API.
- Amend maintained body-dedup language in `dev/interfaces/rust.md`,
  `dev/interfaces/rust-sdk.md`, `dev/interfaces/python.md`,
  `dev/interfaces/typescript.md`, affected historical-view/retrieval designs,
  engine docstrings and intentional tests in the same authorized product change.
  MCP changes touch its requirements/interface only for the selected boundary.
  Preserve the old intent as history; do not regenerate golden identities.
- Complete reference first, then exact lightweight rank/hydration route and
  independent prefix specialization. No schema, new vectors, chunking, provider
  fallback, new score/coverage fields or performance thresholds are implied.

## Decision status

ACCEPTED by owner 2026-11-14 (seq-374 identity/packing; seq-375 feedback): snapshot-local
record/version identity, representative-first provenance, unchanged helper/pack
contracts and atomic ambiguous named-ref feedback refusal. These are selected
planned .29 changes, not shipped behavior. Source-entry baseline, independent
design/code review, genuine RED/GREEN, exact-route proof and qualification remain
required. No migration, private-snapshot dependency,30/Block1 clearance or
runtime population/benefit claim follows.

---
title: Retrieval record identity for fusion and provenance
status: proposed; HITL decision required
target_release: 0.8.29
---

# Proposed retrieval record identity

## Context and authority

Slice9 Batch3 approves identity census and an ADR **before** exact text/snippet
implementation; it does not select a successor. Current source at frozen parent
`3fa16620ac30cbe3ed66f5bfe542f2f7a62b8982` deliberately merges equal bodies.
[Source/consumer census](../plans/0.8.29/features/slice-50/census.md) and
[requirements](../plans/0.8.29/features/slice-50/requirements.md) document the
actual delta. Accepted30 P is a real provider-free direct-text baseline (16cells,
20000successful timed calls); whole30/G/D/M/F and the runtime consumer snapshot/
method remain open. No hybrid frequency/benefit or source entrance follows.
Prior bounded Phase A and feedback-presentation PASS remain historical. Initial
final PASS was superseded by the P2 cursor-alias proposal finding. Same-reviewer
DESIGN PASS covers exact v4 patch
`0f7ac8312786f8db9e116e175ffdbf1af7aa0b64cc9f861430a272f5b09ffd67`.
A new different-reviewer FINAL DOC/OUTPUT PASS covers corrected 13-file parent
candidate SHA256
`7a9be30c9974f36fa3ccd6bb1bef7347d1ee1c045616963c44b2b4a0387add49`,
with no remaining P1/P2 and actual parent scoped checks. The
[review log](../plans/0.8.29/features/slice-50/review-log.md) records subjects and
limits. V5 changes review/check records only; it has no invented separate PASS.

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

## Alternatives requiring owner selection

| Option | Result and consequence | Recommendation |
| --- | --- | --- |
| Keep body identity | Equal text across revisions, sources and node/edge rows merges; representative provenance can hide other records. Lightweight exact retrieval must still discover body equivalence. | Viable only if HITL explicitly wants text-unit presentation; current behavior, no successor inferred. |
| Logical/public-ID identity | Versions of one l: merge; h: twins still merge; node/edge logical-string collisions need namespace policy. Changes remain necessary for mixed governed/anonymous cases. | Not recommended for historical record retrieval; entity-centric semantics would need explicit representative/version selection. |
| Snapshot-local record-version identity | Canonical(Node/Edge,cursor); explicit ownerless node FTS uses (table,rowid). Same record across arms agrees; distinct records retain provenance even with equal body or public ID. | Recommended for review, conditional on HITL and census disposition. Does not redefine canonical lifecycle/public ID. |

Under the recommended option: original arm positions are not renumbered after
repeat-credit suppression; each record contributes once per arm at first rank.
Preserve K30 and weights1/3/1, score accumulation order, vector-input tie preference
and stable insertion ties. Keep direct-text candidate policy and node-then-edge
text concatenation. Existing nested graph fusion is unchanged; this ADR selects
identity, not a ranking experiment.

Graph traversal still visits logical entities/resolves the latest eligible node
as designed. Recommended provenance is existing representative-first ownership
(vector input, then text, then graph); graph-only representatives retain the first
winning edge/entity seed and separate evidence origin. Concretely, vector/text
representatives retain their own canonical source; graph-only edge seeds and
neighbors retain the first winning edge source/confidence, while entity-FTS
seeds retain the selected node source. This preserves current branch semantics;
it does not redefine all GraphArm source_id values as canonical-node ownership.
A body twin never lends source/span/coverage. This recommendation is part of the pending ruling.

Ownerless FTS is an explicit case, not permission to infer canonical ownership
from public IdSpace or branch. Keep ownerless content identity and absent source
truthful. Conflicting/malformed ownership needs an explicit testable disposition
using current error/reference behavior; never silently body-merge or mutate
storage to repair it. The source carrier must retain the real FTS locator.

## Public reference and consumer choice

[Repeated-ref decision](../plans/0.8.29/features/slice-50/feedback-decision.md)
separates existing historical SDK duplicates, conditional60/F11 search-flag
exposure and additional50 equal-body exposure. Integrated60 still rejects search
include_superseded/include_inactive; its FetchArgs support is a separate seam.
Recommend a private SafeCursor/Ambiguous target for visible refs: require one
ref occurrence in the returned page and one cursor occurrence across the
complete captured query result, before offset/budget shaping. Repeated visible
refs or captured cursor aliases refuse
otherwise-recordable named labels atomically, after existing visible-membership/
not_recorded guards. Same cursor does not prove same selected record; hidden
ref strings stay nonmembers when absent from the returned page, while their
records still contribute to cursor-alias detection. A singleton visible ref
also present off page at a different cursor remains safe; only page-local ref
multiplicity and complete-run cursor multiplicity govern refusal.
This conservative adapter guard needs no new engine/public carrier. Existing
cursor-only telemetry result_ids/arm_of remains limited; it is not upgraded by
this proposal. A response-local feedback_ref alone cannot disambiguate cursor
labels; any such ABI or telemetry identity redesign stays unselected. Neither
policy is selected here.
Logical fetch remains latest-eligible under its requested view and does not
promise exact searched revision retrieval. A feedback fix does not change it.

## Context-packing consequence requiring explicit disposition

The existing SDK context_item_from_hit copies IdSpace's prefixed value and
returns parts decoded from the stored body (`standalone.rs:304–317`). Existing
pack_context rejects repeated IDs across hits/context before rendering
(`context_pack.rs:351–373`), mapped to SDK InvalidArgument. Thus equal anonymous
stored-body twins newly retained by record fusion share h: and helper→pack
changes from one packable item to duplicate-ID refusal. Different-body historical
versions already can share l: and refuse today. Do not promise packability from
record-unique fusion when public IDs are not occurrence-unique.

Propose preserving these helper/pack/codec contracts and explicitly acknowledging
that composed consequence in the identity ruling. Existing caller-owned unique
ContextItem labels plus a label→selected-record map can represent both items;
packing labels are not IdSpace, artifact revisions or stable engine IDs. If the
owner requires automatic helper→pack acceptance of every hit list, a separate
helper-label or pack-contract change must be proposed and ruled. No override,
automatic relabeling, duplicate acceptance or new ID ABI is inferred here.

O3-09 role-tagged cases include equal stored tagged bodies on distinct records
and equal plain text with different tags. Assert chosen records, exact stored
bytes and original-body snippet coordinates; preserve codecs. Stage identity
assertions before the identity fix; unchanged codec/pack assertions are controls.

## Consequences if accepted

- Carry selected keys through fusion, explanation ranks/scores, graph exclusion,
  weighting/evidence/snippets and final same-build SDK/MCP parity. Preserve actual
  graph path and frozen authority; do not deduplicate by body in one downstream map.
- Existing doc-hidden exported fusion helpers currently receive SearchHit only;
  it does not contain ownerless FTS rowid or unambiguous artifact class. Propose
  making fuse_rrf/fuse_three_arms crate-private over an internal keyed carrier
  and removing the two lib.rs public exports. Relocate pure arithmetic tests
  without changing their independent formula oracle; integration REDs use
  existing Engine search and must fail on selected records, not missing exports.
  [Design](../plans/0.8.29/features/slice-50/design.md) specifies the carrier and
  locator/positional-sidecar flow. This public removal remains UNSELECTED:
  acceptance must expressly cover it and same-change interface notes. No
  body-keyed compatibility shim or exported physical-locator ABI is proposed.
  If public removal is rejected, review an exact public signature/carrier
  alternative before code; never infer keys from branch/body/public ID.
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

PROPOSED. Identity, graph representative/provenance and ambiguous-label policy
await HITL; missing authorized runtime census remains visible. The recorded
bounded documentation/census verdicts supply no ruling; parent still owns
recording actual human decisions. No semantic50 question has been asked or ruled. No acceptance,
policy waiver, production code or30/Block1 clearance is claimed. The bounded
[pending decision packet](../plans/0.8.29/features/slice-50/status.md) also makes
the helper→pack consequence and proposed helper visibility explicit. No accepted
ADR/index ruling or interface has been changed.

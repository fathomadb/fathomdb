---
title: Slice 50 proposed design
status: PROPOSED
target_release: 0.8.29
slice: 50
---

# Slice 50 proposed design

[Source census](census.md) is the evidence boundary. This design is reviewable
before30 runs, but production identity/exact-route work retains actual30 and
HITL prerequisites. No accepted successor or optimization benefit is claimed.

## Architecture delta and decisions

Create an internal rank/ownership carrier, update fusion/explanation/graph
exclusion uses, and preserve existing public IdSpace, eligibility, storage,
provider and snippet APIs. The [ADR proposal](../../../../adr/ADR-0.8.29-retrieval-record-identity.md)
presents entity/body/record-version alternatives. Recommend record-version
retrieval, conditional on HITL; canonical logical identity remains unchanged.

Proposed keys: Canonical(Node|Edge,cursor) for a resolved canonical row;
Ownerless(NodesFts,rowid) for an explicitly ownerless node FTS row. Keep the
actual FTS locator and public IdSpace separately. A graph-reached node uses the
same canonical node key as its vector/text occurrence. Do not key by branch:
a vector edge currently carries TextEdge; graph nodes have a different branch
from their text counterpart. Do not expose physical cursors as stable public IDs.

Ownership classification is built at the existing query/hydration seam on the
same snapshot. A lightweight LEFT JOIN can retain canonical-owner presence
without selecting body content. The all-nodes-directly-eligible proof does not
prove every FTS row has an owner; ownerless rows still need their FTS rowid.
Duplicate physical occurrences of one canonical record get one credit per arm
at first positional rank, without compressing following ranks. Two incompatible
locators/bodies for one owner are an invariant failure: keep complete-reference
routing/error behavior rather than invent body-based salvage. If correct
classification cannot be proved with existing37, stop that route for review;
no schema fix or new corruption vocabulary is authorized.

The identity ruling must also choose same-record graph provenance: recommend
existing representative-first ownership (vector input, then text, then graph),
with the existing first-winning graph path retained separately for a graph-only
representative/evidence. Never replace a node's own source with a body twin's
edge source. Preserve current nested graph fusion, graph logical visited set,
confidence mapping, recency→importance→CE ordering and their candidate domains.
This is an identity change, not a new graph-ranking experiment.

Public refs can repeat even today in historical SDK results. Follow the
[feedback decision](feedback-decision.md): recommend refusing labels ambiguous
across distinct cursors without changing logical fetch. The alternative can be
an MCP-only feedback_ref; no engine ABI is presumed. Parent obtains the actual
ruling and coordinates its smallest affected adapter boundary with60/80.

## Context packing and role-tagged stored bodies

Preserve context_item_from_hit's prefixed public ID and stored-body decoding,
and pack_context's upfront uniqueness check across hits/context. The code-backed
consumer trace is in [census](census.md). Equal anonymous stored-body twins would
now produce two helper items with one h: ID and typed packing refusal; historical
same-l: revisions already demonstrate that ID is not an occurrence label. Include
the newly exposed success→refusal in the identity ruling. Do not quietly weaken
pack validation, deduplicate/drop records or mint IDs to hide that consequence.

A caller needing both occurrences can already construct ContextItems with unique
caller labels and keep its own label→selected-record map. Copy parts/metadata and
use the same labels for full_ids; do not describe those packing labels as engine
record identity. If automatic helper→pack acceptance of all records is required,
propose that public helper/pack change separately for owner choice. No ID override,
new field or ABI is selected by this design; preserved validation is the proposed
default disposition pending the identity ruling's explicit consequence review.

O3-09 includes two fixture families: identical **stored tagged bytes** on distinct
records/sources, and equal decoded plain text with different role tags. Compare
record tuples, untouched stored bytes, decoded roles and exact body-relative
snippet windows/spans versus snippet-relative highlights. Search/snippets operate
on stored bytes, including marker and role-code offsets, not decoded part text.
Keep all codecs and malformed-body/character-count contracts unchanged. The
first family's selected-record count supplies an identity RED; differing tags,
codec round trips and existing pack rejection are regression controls, not new
RED claims. Stage the relevant semantic fixture assertions before identity GREEN.

## Complete ranking reference first

Reference preparation retains all current eligible candidates and body hydration
but uses the selected key consistently. Share the selected key, first-per-arm
credit, representative and final comparator with the optimized route. Also
retain independent manual oracles: shared code is not independent correctness.

Node text ordering is bm25 ascending then cursor; edge ordering is its existing
separate corpus ordering and its positional ranks follow all node candidates.
The direct-text fixed node candidate window remains100/current constant;
hybrid full-node rank does not acquire a prefix cap. Original positions include
repeated occurrences, even when later occurrences receive no credit. Vector
fanout/physical head ownership stays as shipped; no record-grouped KNN or
candidate-selection experiment is smuggled into the fusion key change.

The graph route keeps its current two-stage fusion; the reference includes
recency, importance/confidence and CE exactly as today. Filters validate in the
reader snapshot and predicates precede every existing cap. Frozen authentication,
branch nonce, time/view and dependency closure are never bypassed. Explanations
stay positionally bound; arm ranks/fused scores become selected-key maps.

## Exact lightweight text route

First production unit, after accepted identity: keep the current narrow hybrid
admission and make **one complete lightweight rank census**. Read numeric bm25,
artifact/owner key, cursor and FTS locator for every eligible node/edge match;
sort with the same per-corpus comparator and concatenate in the same order.
No approximate top200 or early stopping. Vector hits already carry their
bodies and keys; add every lexical overlap at its full positional rank.

Compute scores with exactly the reference's arithmetic and accumulation order.
Top-L selection preserves score bits, vector-input tie preference and insertion
order. Hydrate final text-only winners by their retained FTS locator/owner in
the same transaction; derive public IDs and source from the exact selected row.
Retain their FTS map for snippets. A body mismatch or statement failure cannot
be treated as an empty candidate. Errors propagate under current contract;
unsupported controls use reference before partial results escape.

This removes body-sized O(M) allocation/hydration for M lexical matches while
retaining O(M) lightweight work. It does **not** promise O(L) query time or
bounded SQLite bm25 internal I/O. Edge bodies may initially retain complete
reference handling if a mixed lightweight route is not independently proved.
If that restriction is selected, witness the route and include an edge-presence
fallback test; never silently drop edge candidates.

A later top-L heap or stopping bound is optional implementation design, not
required to claim the initial exact route. To stop before all ranks are known,
prove the maximum possible contribution for every unscanned vector overlap,
node/edge rank offset, representative/tie and post-fusion stage. Strictly beating
the score bound is necessary where a tie can win. No unproved optimization is
admitted because its first benchmark looks good.

No admission widening is required. Filters, relaxed/historical views, graph,
explanations, weights, rerank and source dependencies can keep the complete
reference; tests prove correct routing/result parity. Text mode stays provider-
free; no query embedding is used to make that route eligible.

## Bounded record-start specialization

Retain the complete selector as reference. Specialize only a body over the
clamped max_bytes where raw spans are empty initially; nonempty-but-invalid
spans, long normalized spans that cannot fit and exact-term selection may use
full fallback. Thus no full-body normalization is required just to decide the
first specialization. All valid options and Unicode still preserve behavior
through either specialization or fallback.

A conservative candidate fast case is an ASCII letter/digit/space prefix through
`target+1`, target=max_bytes−len(" …"). Check only that bounded slice, not the
whole suffix. In this restricted alphabet, word runs join letters/digits and
space runs; internal run transitions identify real word boundaries. Ignore the
artificial end boundary of the sliced prefix. Choose the greatest true boundary
at/before target if within32 bytes; otherwise target itself is a grapheme
boundary. Trim trailing ASCII spaces with the same retain-first-cluster floor.
Emit the exact Start/window/empty-span/empty-highlight/cut result. head_bytes and
max_windows do not affect that no-span fallback. Any unsupported prefix code
point (including CR/LF, punctuation or Unicode) uses the complete selector.

This is a proposed minimal specialization, not a claim that arbitrary Unicode
has fixed lookahead. A long Extend/Format/ZWJ sequence, regional-indicator state,
long word or whitespace run can defeat naive `take(k)` segmentation; do not
truncate the string and accept a fabricated end boundary. Expand the supported
alphabet/context only after a pinned-rule proof and counterexamples. Unicode
outside the bounded inspected prefix cannot change earlier ASCII transitions;
the lookahead byte prevents treating a cut token as a completed word. Confirm
this proof against actual pinned tables before implementation approval.

## Research and verification limits

Cargo.lock pins unicode-segmentation1.13.2 (checksum in lock). Its installed
`src/word.rs` iterator/state-machine was inspected: ALetter/Numeric, WSegSpace,
Format/Extend and Regional states explain why arbitrary prefix slicing is not
an established bound. Full selector/query source and the cited tests were read;
no dependency upgrade or fetched package is needed for this proposal. Future
review must verify the proposed restricted alphabet against pinned word and
grapheme tables and SQLite bm25/rank query semantics. Reference URLs for that
future check: <https://www.sqlite.org/fts5.html> and
<https://www.unicode.org/reports/tr29/>; no web research or current external
version claim is presented here.

Use per-behavior runtime REDs and work/allocator witnesses in [test map](test-map.md).
Latency claims require actual30 inputs and isolated untraced repeats; no timing
assertion replaces exact work. Current performance targets, unchanged full-gate
rules and no-migration constraints remain. Stop affected work on unruled identity,
missing authorized census, unproved bound, divergent score/provenance/Unicode,
source-dependency bypass or required storage change. Continue unrelated approved
work; do not create new release gates or infer policy waivers.

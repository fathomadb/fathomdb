---
title: Slice 50 proposed design
status: DESIGN_REVIEWED_IMPLEMENTATION_PENDING
target_release: 0.8.29
slice: 50
---

# Slice 50 proposed design

[Source census](census.md) is the evidence boundary. This design is reviewable
before remaining30 runs, but production identity/exact-route work retains
actual baseline and reviewed source-entry prerequisites. Owner seq-374 accepts the identity/packing successor and seq-375 the feedback
correction; implementation and optimization benefit are not claimed.

## Accepted exact source-entry amendment

Independent Astra-medium review passed November14 on documents through327672507;
parent activation replaces the whole30 source dependency with: closed10; qualified Linux20 source/gate metadata
`45167677bee0d00b59194897e63ad05be40d6e63`; accepted P measured source
`2fd772558e46d0b1645dce2ffea730d0bdd3305c` and its
[accepted results](../slice-30/results-p.md); accepted G normal assessment
`29de06806cc22ab6b8e67f970133f856f3c42f78`,
`gd-allocation/g-frozen-g-assessment-v5`, as recorded in
[execution](../slice-30/execution.md#accepted-normal-fixed84-assessment-at29de);
selected seq-374/375 contracts; and reconciled source/consumer census.

G supplies11 qualified read outcomes and73 refused outcomes, not wholeG/30
closure. Closed-copy tooling `0fa61810b` is source qualification, not later
runtime measurement closure. This amendment excludes M/D/F and Windows from50's
source entrance only; it does not waive their own obligations or claim their
measurements. Check exact integrated source/native/features/input equivalence at
entry; retain original failures, refusals, score/provenance/Unicode proofs and
same-input untraced comparisons. No private Memex snapshot is required. Do not
start a cache/optimization without meaningful applicable evidence, exact-route
proof and genuine per-behavior RED/GREEN. This accepted narrowing opens source
entry only. Actual current-input fit, owned setup, implementation and final
qualification remain; no optimization or runtime result is claimed.

## Architecture delta and decisions

Create an internal rank/ownership carrier, update fusion/explanation/graph
exclusion uses, and preserve existing public IdSpace, eligibility, storage,
provider and snippet APIs. The [accepted ADR](../../../../adr/ADR-0.8.29-retrieval-record-identity.md)
selects snapshot-local record-version retrieval and representative-first
provenance; canonical logical identity remains unchanged.

Selected keys: Canonical(Node|Edge,cursor) for a resolved canonical row;
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

Owner seq-374 selects same-record graph provenance as
existing representative-first ownership (vector input, then text, then graph),
with the existing first-winning graph path retained separately for a graph-only
representative/evidence. Never replace a node's own source with a body twin's
edge source. Preserve current nested graph fusion, graph logical visited set,
confidence mapping, recency→importance→CE ordering and their candidate domains.
This is an identity change, not a new graph-ranking experiment.

Public refs can repeat even today in historical SDK results. Owner seq-375
selects the [feedback correction](feedback-decision.md): atomic refusal of named
labels ambiguous across distinct cursors only for otherwise-recordable calls,
with existing not_recorded precedence and zero partial label writes. Implement
this in .29 before exposing repeated-reference MCP views; logical fetch stays
unchanged. .30 reviews precise per-result feedback through richer occurrence
labels. No new public field/ABI is selected. Parent coordinates the adapter seam
with60/80 and reruns both-format parity on the integrated identity build.

## Integrated implementation seams (November14 source entry)

Reconciled against integrated source `e1cb1bd5311f68da8be98bbe133b9cecd5ab56d4`.
The existing `fusion.rs` exports bare-SearchHit `fuse_rrf`/`fuse_three_arms`;
`search.rs` still uses body-keyed explanations and graph exclusions, and
`snippet_query.rs::FtsRows` is cursor-keyed. None supplies the selected ownership
carrier. The current MCP feedback implementation remains at `tools.rs::feedback`:
label resolution precedes acquisition, and response refs retain only one cursor.
The integrated60/70 changes alter presentation/degradation, not these identity
seams. No runtime population or performance result follows from this source read.

The implementation proposal changes the two doc-hidden exported fusion helpers
instead of inferring ownership from SearchHit or retaining a bare-hit adapter:

- Add doc-hidden `FusionOwnership::{CanonicalNode, CanonicalEdge,
  OwnerlessNodeFts { rowid: i64 }}` and `FusionHit { hit: SearchHit,
  ownership: FusionOwnership }` in `fusion.rs`, exported by `lib.rs`.
  Canonical record keys derive their cursor from the enclosed hit; ownerless
  keys use the explicit FTS rowid. The carrier does not alter SearchHit or its
  serialization, public IdSpace, provider identity or stable-reference ABI.
- `fuse_rrf(Vec<FusionHit>, Vec<FusionHit>)` and `fuse_three_arms` with three
  such arms return `Result<Vec<FusionHit>, EngineError>`. Callers must supply
  ownership explicitly. Conflicting bodies for one key refuse through
  existing `EngineError::InvalidArgument`; no new error kind or body salvage.
  Kind/source are representative-owned arm fields: vector edges intentionally
  use kind=edge_fact while text edges keep the stored kind, and graph-only
  nodes retain winning-edge provenance. Do not reject those existing differences.
  Stage the accepted record-identity successors of intentional body-dedup tests
  before GREEN: same-record repeats reuse their cursor; distinct-body twins
  retain separate cursors; the independent property reference credits records.
  Preserve manual RRF/tie values and unrelated tokenizer/injection assertions.
  Only helper construction/signatures migrate mechanically with GREEN, without
  changing the staged selected-record oracles. API migration is not a RED. Update `dev/interfaces/rust.md` with this public signature
  delta in the same implementation commit; no old-signature compatibility shim.
- Production creates the carrier at canonical vector hydration, node-FTS
  owner-presence LEFT JOIN, canonical edge-FTS join, and graph node resolution.
  Preserve actual node/edge FTS rowids alongside keys. Ownerless is selected by
  absent canonical-node owner, never by absent logical_id/source_id, Text branch
  or matching body. Validate joined owner body/kind against the FTS row when
  hydrated; conflicts propagate through the current SQLite/Engine failure path.
  A canonical-owner presence flag is selected even on deferred-identity routes.
- Keep the carrier through both actual graph fusion stages, recency, importance,
  CE and final selection. Factor the existing reweight/rerank arithmetic over a
  private hit-access trait implemented by SearchHit and FusionHit; unchanged
  standalone public reweight/rerank entry points delegate that same arithmetic.
  CE clones/sorts complete carriers so rank position remains attached to the key;
  no body/cursor reconstruction map and no second ranking algorithm.
- Explanations, graph exclusion and FTS locator maps use the selected key.
  Canonical-only weight/graph-origin lookups are gated by explicit ownership;
  an ownerless FTS cursor cannot borrow an edge/node weight or graph origin.
  `SearchOriginCapture::finish` receives final carriers and passes explicit
  ownership positionally into `evidence::build_search_result`; that builder no
  longer derives artifact class from branch. Preserve existing ownerless
  explanation failure (missing canonical lifecycle row) and evidence-unavailable
  behavior rather than invent an Unknown lifecycle or lend a colliding edge's
  EdgeValid state. Strip the carrier at the existing SearchResult boundary after
  key-bound explanation/snippet/evidence selection. Preserve positional sidecars.

The first staged fixture uses the unchanged Engine search API against a real
SQLite store, so it can fail semantically on the baseline before these helper
symbols exist. It pins equal-body governed, anonymous/governed, historical,
node/edge and stored tagged twins with exact cursors/source/score/coordinates.
It is staged preparation only until a leased runtime run observes its failure.
The helper API migration is part of identity GREEN, with independent manual
rank/repeat/tie controls staged before that change. Explanation and graph cases
must also be staged before shared identity GREEN if the carrier fixes them
atomically; already-fixed cases are controls, not later claimed REDs.

Feedback is a separate behavior unit: bounded Unique(cursor)/Ambiguous insertion
when constructing each response, then complete named-set ambiguity validation
after successful acquisition and final epoch check, before cursor resolution or
bridge/write. Test its real repeated-ref case independently of equal-body fusion
using existing different-body node/edge refs. Existing hooks may pause the
acquisition seam for precedence tests; no model or fusion change is needed.

Lightweight hydration and snippet prefix specialization remain separate RED/GREEN
units after identity/reference. Before either optimization, derive its allocation
bound from actual candidate constants, pair the complete control on the same
input, and witness allocation/work independently of latency. No admission widening,
early-stop algorithm, measurements or source baseline clearance is inferred here.

## Context packing and role-tagged stored bodies

Preserve context_item_from_hit's prefixed public ID and stored-body decoding,
and pack_context's upfront uniqueness check across hits/context. The code-backed
consumer trace is in [census](census.md). Equal anonymous stored-body twins would
now produce two helper items with one h: ID and typed packing refusal; historical
same-l: revisions already demonstrate that ID is not an occurrence label. Owner seq-374 explicitly accepts
that newly exposed success→refusal. Do not quietly weaken
pack validation, deduplicate/drop records or mint IDs to hide that consequence.

A caller needing both occurrences can already construct ContextItems with unique
caller labels and keep its own label→selected-record map. Copy parts/metadata and
use the same labels for full_ids; do not describe those packing labels as engine
record identity. If automatic helper→pack acceptance of all records is required,
propose that public helper/pack change separately for owner choice. No ID override,
new field or ABI is selected by this design; preserved validation is the selected
disposition under seq-374.

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

## Current paired measurement support

The remaining R6 work needs a small measurement-support change, not another
ranking optimization. Current `slice30_measure` already runs real P1/P2/G-R/G-W
schedules. Its G ordinary build uses `embed-cuda`; diagnostics add
`slice30-observer`. Neither build enables the existing full-sort test selector.
Enabling general `test-hooks` is unsuitable for untraced latency: the SQL profile
callback records test statements even without a witness path, and rank-stream
queries execute test EXPLAIN plans. Full-sort also disables the older deferred
identity/streaming routes, confounding attribution to50.

Add one private nondefault engine feature, `slice50-measurement`, forwarded by
the SDK. Under that feature alone,
`FATHOMDB_SLICE50_EAGER_TEXT_FOR_MEASUREMENT=1` suppresses only
`defer_full_hydration`. The retained `else if defer_text_identity` branch then
materializes every lexical body and resolves final identity as before50, while
retaining current accepted record fusion. Unset means the current winner-only
route. Both modes use the exact same binary/source/lock/provider/model and all
other admission guards; direct-text rank streaming is untouched. No general
`test-hooks`, extra SQL, observer collection or witness output enters ordinary
timing. Default builds contain no new selector or collector. Preserve general
full-sort correctness controls unchanged.

Extend the existing measurement program/runner rather than create another
harness. Bind the pair to a current owned `slice30_measure` release build with
`embed-cuda,slice50-measurement`; its separate diagnostic build adds
`slice30-observer`. Existing G freezes/FEATURES and their original binaries stay
attributed to their original candidates. New pair scope records the current
candidate and exact selected existing cell IDs/configs before dispatch; it
cannot relabel old G native artifacts or claim original G completions. Reuse
existing scheduling, bounded process/resource controls, CLOSED-copy preparation,
raw streams and metric math. Pair ordinary modes in reversed order on repeat1;
each write member gets a fresh identical CLOSED copy, never the other member's
mutated store. Preserve actual30 population, query order,500 warmups/1000 timed
calls, adapters, routes, write cohorts/probes and repetition semantics. Keep
all applicable natural/read-write obligations explicit; a smaller selected
prefix or diagnostic cannot silently discharge unstarted cells. Exact table
feasibility and remaining shared allocation must be established before runtime;
no new allowance or cheaper workload follows from this support change.

Qualified inputs already exist: the registered100k LME corpus/500 queries and
accepted G CLOSED format37 store with100000 physical vectors/spans and pending0.
Reuse its exact immutable package as data custody, with fresh owned copies and
current-candidate open/readiness/device qualification; it is not borrowed native
code. Copy the existing pinned BGE snapshot into the pair's owned cache. Keep
current fork pins/driver/CUDA12.6 and reserve a3090 for hybrid/default work. No
model download, regenerated vectors or old-source/provider comparison supplies
a50 delta. Native/tool builds and natural measurement wait for the parent's
exclusive native/heavy window; no concurrent compilation or campaign is allowed.

Separately collected diagnostics count actual lightweight lexical rows and
actual body materialization events/UTF-8 bytes at the row-to-owned-body seams,
including vector candidates, eager lexical candidates and selected text winners.
Count repeated materializations as repeated events; do not deduplicate by ID or
infer bodies from a final hit limit. Distinguish borrowed SQLite text validation
from owned Rust body materialization; these counters make no SQLite I/O or total
allocation claim. Record actual selected route and relevant limits, and sample
child RSS/peak RSS separately with cadence/unsampled-peak limits explicit.
Diagnostics must be opt-in and engine/operation owned, using the existing bounded
observer; missing/lost counters remain unavailable. Emit selected hit cursor,
source, score bits and existing result metadata outside API timers so pairs can
check ordered equality without reconstructing identity from bodies/public IDs.

The unchanged actual30 natural cells have snippets off. Prefix work/latency is
therefore not applicable in those cells; retain completed prefix correctness and
allocation proof without inventing a snippet workload or claimed natural prefix
benefit. Preserve each repeat's p50/p99, all errors/refusals/outliers and paired
query uncertainty. No target, significance, speedup or whole30/Block1 completion
claim follows from successful tooling or a partial pair.

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
rules and no-migration constraints remain. Stop affected work on missing actual baseline/source prerequisites,
unproved bound, divergent score/provenance/Unicode,
source-dependency bypass or required storage change. Continue unrelated approved
work; do not create new release gates or infer policy waivers.

Private Memex runtime census measures population frequency/benefit only. It is
not required for the selected no-migration format37 change; existing source/
consumer inspection plus real synthetic SQLite fixtures, independent oracles and
properties provide correctness evidence. Reconcile those against actual source
at entry; do not substitute population counts for baseline/performance evidence.

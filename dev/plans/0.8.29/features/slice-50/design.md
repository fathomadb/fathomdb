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

Accepted `83edcf2c7`, integrated at `e887e3fa6`, supplies the private
`slice50-measurement` selector, exact operation-owned body counts/UTF-8 bytes,
post-timer record fields and low-level dispatcher mode/RSS handling. The selector
suppresses only `defer_full_hydration`, selecting the retained eager-body/
deferred-identity branch with current record fusion. Both modes use the same
binary; direct-text streaming and all other controls stay unchanged. General
`test-hooks` is excluded: its test profiling/EXPLAIN work is unsuitable for
untraced latency, and full-sort would disable older optimizations as well.

The explicit private public path is implemented by `g_pair.py`,
`g_pair_freeze.py`, `g_pair_runtime.py` and `g_pair_report.py` under the existing
`g_runner.py` entry. Source controls cover canonical admission, current-build
refusal, actual retained P/G streams, exact pair credit and cumulative limits;
this does not claim the natural calibration or primary measurement complete.
The original default `g_runner` build/freeze supports only its
original ordinary/diagnostic feature lists and canonical84-cell admission;
old-source native equivalence cannot qualify current pairtools. P `control.py`
freeze still declares features=[] for its original scope; private paired P uses
its admitted immutable template copy and actual operation observer. The private
controller retains the existing guardian/attempt/native driver chain; no bare
`_cell` call, old ordinary executable or second harness substitutes.

### Public paired entry and current input binding

Add explicit private `pair-build`, `pair-freeze`, `pair-run`, `pair-calibrate`
and `pair-report` commands to the existing `g_runner.py`. They handle both P/G
families through existing drivers/schedulers/controllers; normal P/G commands,
FEATURES, specs, original freezes and admission remain unchanged. A P entry may
delegate this explicit scope to that same owner, never duplicate scheduling.

Public invocations (implemented; natural execution still needs the parent's
selected profile and reserved runtime window):

```bash
.venv/bin/python "$G_DIR/g_runner.py" pair-build --out "$OWNED/artifacts"
.venv/bin/python "$G_DIR/g_runner.py" pair-freeze \
  --options selected-pair.json --out "$OWNED/pair-frozen"
AGENT_LONG=1 .venv/bin/python "$G_DIR/g_runner.py" pair-run "$OWNED/pair-frozen"
.venv/bin/python "$G_DIR/g_runner.py" pair-report "$OWNED/pair-frozen"
```

`pair-build` builds current owned release `slice30_measure` artifacts with
`embed-cuda,slice50-measurement` and, separately, those features plus
`slice30-observer`. It records actual current source/compiler/lock/features and
binary paths before use; winner/eager select the same ordinary executable.
The diagnostic artifact has the same compiler inputs/provider, with only the
observer feature added. No general test hooks, borrowed target/module, old-build
retag or invented native equivalence is allowed. Natural execution still needs
current source qualification, the parent's native window and a reserved3090.

The executable chain is explicit. Build output carries `source_sha`,
`compiler_inputs`, lock identity and each ordinary/diagnostic binary's path and
features. Freeze consumes that actual output plus data-only donor references and
produces private canonical member configs: `family`, origin cell, purpose,
repeat, `slice50_pair_mode`, `observe`, admitted store/corpus/query paths, n,
adapters/routes, warmup/samples and applicable write fields. It does not patch
an old G config or accept caller-selected counts. The public controller binds
that exact member/store/deadline to live owned admission before preparation.
After validated copy, the existing `g_driver.run_program_step([binary], config,
fresh_output, absolute_deadline, ...)` writes its native config and invokes
`[binary, native_config_path, fresh_output]`; remaining allowance only declines.
The existing dispatcher sets the private selector for the selected mode. Native
paired P must consume its admitted copy/provider-none/observe fields; native G
uses its actual existing G-R/G-W calls and operation observer. Report checks the
native counts/open/close/device/work facts, exact member result fields and full
pair comparison, plus actual charging/actor cleanup, before any member credit
or selected copy retirement. Source review must trace this entire field/argv
chain; accepted helper functions alone cannot establish executable completeness.

`pair-freeze --options selected-pair.json --out OWNED/pair-frozen` binds a distinct
paired scope, exact canonical members below, actual artifacts and qualified
input identities. Original G CLOSED10k/100k stores are **data-only donors**:
10000/100000 vectors/spans, pending0, format37. Their old timings/native/provider
proof never qualify a current member. Copy existing registered LME/500 questions
and pinned BGE bytes into owned paths; no download, input regeneration or extra
fixture. Reuse existing input IDs and normal copy validation, not a hash campaign.

P needs current-provider-none templates for P1(10k/100k) and P2(100k plus its ten
initial note markers). Reuse the original1024-write seed loop and marker writes
once in charged preparation, then close/validate. Both members copy the same
immutable template; never donate an already-mutated P2 output. Paired native P
accepts its admitted fresh copy, skips reseeding truthfully, and reports actual
preparation/open/close separately. It starts the existing engine/SDK operation
observer only for diagnostics. Actual P provider-none/open facts and zero vector
rows are qualified as such, never labelled G negative controls or GPU work.
Default P seeding and its ordinary outputs/features remain unchanged.

Every read/write member gets a fresh owned matching copy. P1/G-R still close/
reopen between their original adapter passes. Current G opens must qualify the
actual model/device/readiness; hybrid requires own-PID memory plus selected3090
utilization. Direct-text query compute is N/A; do not add a GPU pulse. Current
limits remain10 returned hits,100 direct-text node candidates,192 binary KNN
candidates and the actual compiled vector fusion pool (currently10). Record
resolved limits and actual selected routes with work diagnostics.

### Exact primary and diagnostic coverage

The primary table contains68 original search cells and136 ordinary members:

| Original IDs/configurations | Cells | Preserved workload |
| --- | ---: | --- |
| `p1-n{10000,100000}-r{0,1}` |4| P1 direct text/provider none; engine→SDK on r0, reversed on r1. |
| `p2-b{1,10}-s{1,10,100}-r{0,1}` |12| P2 SDK/provider none, natural doc filter and ten separate note markers. |
| `g-r-n{10000,100000}-r{0,1}` |4| Default provider, both adapters; text→hybrid on r0, reversed adapter/route order on r1. Embed-only passes stay outside50. |
| `g-w-b{1,10}-s{1,10,100}-{engine,sdk}-{text,hybrid}-r{0,1}` |48| Default provider, one adapter/route per fresh copy, original natural-row replacement/probe/readiness schedule. |

Each ordinary member keeps500 warmups/1000 timed calls, all500 questions twice
in original order, original first-call policy and write cohorts/probes. Preserve
original within-repeat cell order; winner→eager on r0, eager→winner on r1. Private
IDs retain origin cell, purpose, repeat and mode. The24 G-W embed-only cells,
G0/G1/G-O correctness/embedding observations, M/D/F/Windows and whole30 closure
remain independent. No primary member is credited from another scope or prefix.
Only unfiltered P1/100k/provider-none reports tracked AC08380/300ms distances;
these remain descriptive, not new pass gates. G hybrid never supplies that target.

R6 separately needs work/RSS for provider-none/default reads and writes, both
implemented adapters, text/hybrid routing and both write cohort shapes. Select
these12 diagnostic configurations, each in both modes (24 members), on100k:

| Diagnostic origins, repeat0 only | Configurations | Reason and limit |
| --- | ---: | --- |
| P1 `p1-n100000-r0`, both original adapters |1| Provider-none unfiltered read;10k data/control are also observed in the separate calibration below. |
| P2 `p2-b10-s1-r0`, `p2-b1-s100-r0`, SDK |2| Original dense10-row and sparse1-row marker schedules, including filtered natural calls and probes. |
| G-R `g-r-n100000-r0`, both adapters and text/hybrid |1| Default-provider read, vector/FTS materialization, exact eager/winner route and adapter transport. |
| G-W b10/s1 and b1/s100 × engine/SDK × text/hybrid, r0 |8| Both governed cohort shapes, dense/sparse schedules, current natural replacements, probes/readiness and both route/adapter paths. |

These diagnostics keep the entire selected origin's500 warmups/1000 calls and
write schedule, not a50-call prefix presented as a full write/RSS witness. They
measure named cases only; no numeric RSS/work estimate is invented for s10,
other repeat/order or unobserved input/state. Those primary latency obligations
still run. This selects actual mechanism/cost extremes without repeating all68
cells under the observer. Existing50-query G-O observations remain their own
limited evidence and cannot replace these P/write facts. Report diagnostic
counts/RSS by exact origin; never merge their timing into untraced percentiles.

Keep exact ordered hit fields/score bits, projection cursor, fallback/degradation/
errors and sidecars in pair comparison. Validate each ordinary pair completely
before credit; a mismatched/error/partial member leaves that pair incomplete.
Use existing nearest-rank p50/p99 and paired query-cluster intervals, preserving
repeats, errors and outliers. Work counts are actual owned conversions, with
repeated vector/FTS materialization counted repeatedly; borrowed validation is
excluded. Missing/error/lost diagnostics remain unavailable, never zero or
SQLite-I/O/total-allocation evidence. Child RSS/HWM is sampled separately with
cadence/unsampled-peak limits. Snippets stay off in all natural cells: prefix
performance is N/A; completed prefix source/allocation proof stays valid.

### Admission, bounded calibration and retained copies

Private paired manifests use their own exact canonical table/admission/report
scope inside the existing entries. Validate current source/build/features,
input custody, purpose/mode/populations and live owned controller/deadline before
copy/start. Default G84/P16 admission is not weakened. A schema/ID/feature change
cannot slip into an old primary or qualify a reused original outcome. Public
report keeps every ordinary/diagnostic member complete/failed/refused/not-run and
the actual coverage; lower-stage completion never clears primary holds.

`pair-run` uses a selected explicit shared profile and finite next canonical
pairs under positive phase, pair, retained-growth and common-work stop allowances.
These are stop budgets, not measured costs or a forecast that all remaining cells
fit. Admit against the actual original G/M/D/common/calibration charges, all
retained data and explicit other-work reservations; do not reset or double-charge
history. Per-member cumulative900s limits, owned containment/resource floors,
input/native/source eligibility and strict cleanup remain. Unknown future costs
stay unavailable. No complete-future-table observation prerequisite is retained:
it was circular before first primary execution.

Prospective continuation adds `pair-run ROOT --selection FILE` for an existing
primary campaign. The explicit `slice50-pair-selection/v1` document binds `root`
and `manifest_uuid`, an approved `profile` on the same original ledger, positive
`primary_limits` (`pairs`, `phase_ns`, `pair_ns`, `new_retained_bytes`, `common_ns`,
`common_bytes`) and boolean `retire_complete_copies`; selected G work may additionally
bind qualified `g_donors`, `model_cache`, `gpu_uuid` and `gpu_ordinal` without
requiring those inputs for P. Under the existing exclusive ledger lock and outer
guardian, validate exact campaign/native/input eligibility, fresh actual cumulative
charges/retained bytes/reservations and finite selected fit; any active or uncertain
G/M/D/S50 owner refuses adoption. Retain the immutable original manifest/profile,
append each accepted selection to the existing invocation history, and report the
actual selection/controller revision with its guardian; never rewrite old outcomes,
renew cumulative member usage, replay credited successes or turn an old cap/failure
into qualification. Python-only controller revisions may continue the original
native source/build/features/lock binding only with identical compiler inputs and
artifacts, retaining both source identities rather than retagging the binary. The
prospective common technical ceiling is48h TOTAL/192GiB TOTAL across the unchanged
original shared ledger, including all prior study/common/failure charges; actual
adoption values require explicit selection after current work is strictly terminal
and capacity is checked, not automatic expansion. Keep the900s cumulative member/
1800s pair maxima,200GiB owned-data ceiling, disk/memory/swap floors, GPU exclusion
and study/lower-dependency qualification; M/D contracts remain independently bounded.
Unknown future cost stays unavailable, without an all-future-fit or calibration
prerequisite; all68 ordinary/12 diagnostic pairs remain required. Selected retirement
reuses final report/original-guardian credit plus required diagnostics and charged
local cleanup; retain immutable inputs/templates/donors, raw results, failures and
copies still needed for qualification. This amendment proposes the public adoption
path only; it does not adopt a profile or modify the active allocation.

Each invocation selects only the next required canonical pair(s), preserving
original member/adapter/route/order/repeat/populations and all136 ordinary plus24
diagnostic obligations. Freeze/prepare only the selected required P templates;
later templates are created once under charged common preparation when first
needed. A finite prefix never claims full-matrix fit or completion. Stop/growth
failure retains raw partials, failed copies and charges. Later explicit invocations
continue from actual qualified pair credit; failures do not renew member budgets.

The first real P1-10k-r0 ordinary pair is required measurement, without a separate
calibration replay. The former four-member10k calibration entry remains optional
historical support, not a bootstrap prerequisite. Its extra10k diagnostic pair is
outside the required24 and is never required to unlock primary work. If eligible
full ordinary10k G members already exist, explicit references may credit the
identical required members after strict input/source/build/config/result/final-
guardian matching on the same allocation. Keep their actual origin, raw events and
existing charges; never retag old results or charge/run those members twice.

Original P/G attempts retain their stores under their existing contracts. This
proposal explicitly amends **only new paired scope**, with a selected
`retire_complete_copies` option: after complete pair comparison, required
case-diagnostic completion, actual closes and strict native actor/cell-boundary
quiescence, require finalized healthy report and original execution guardians
for every required member before retiring only that pair's disposable working
DB/WAL/SHM copies. Local retirement follows final guardian publication, reuses the shared ledger
lock and charges its actual common work within the selected remaining limits.
Retain raw calls/warmups/probes/writes/results, actual input/build/provider identity,
pre/post counts/close facts and the immutable template/donor. Failed/incomplete/
refused/uncertain pairs retain their copies. Never retire original G/P data,
immutable donors/templates or another task's files. Reuse actual current retained
bytes for admission after retirement; do not discount raw output or assume this
makes the unknown full table fit. This is an explicit prospective retention
amendment for review, not a silent relaxation or authorization to delete now.

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

---
title: Slice 50 source and consumer census
status: SOURCE_ONLY
target_release: 0.8.29
slice: 50
---

# Slice 50 source and consumer census

Current source boundary is `3fa16620ac30cbe3ed66f5bfe542f2f7a62b8982`.
Prior full-file inspection at2f6913c1d59ac8ff3a30de835b57290a8635998c is
retained where exact Git blobs are unchanged; current relevant search/fusion,
span/selector, helper/packing and MCP search/feedback seams were reinspected.
Task source-attribution receipt records exact blob/content hashes and inspection
limits. No runtime census or tests. Paths below are repository-relative unless explicitly Memex.
The canonical memory index was read at
`/home/coreyt/.claude/projects/-home-coreyt-projects-fathomdb/memory/MEMORY.md`.
Relevant verified files include `g0-identity-scope-logical-id-alone.md`,
`opp12-record-lifecycle-protocol.md`, `verify-design-against-code-not-just-architecture.md`,
`slice-prompt-verify-test-claims.md`, `tdd-red-must-fail-per-finding.md`,
`acceptance-md-locked-no-feature-acs.md`, `full-gate-run-frequency.md`,
`0.8.25-slice35-eligibility-frozen-read-lessons.md`,
`0.8.25-slice45-pagination-lessons.md`,
`0.8.25-slice50-compact-evidence-lessons.md`,
`long-record-embedding-findings.md` and `dont-be-distracted-by-ceremony.md`.

## Identity layers and their actual uses

| Existing layer | Observed implementation | Consequence for50 |
| --- | --- | --- |
| Canonical entity | Accepted canonical-substrate ADR Decision5: active uniqueness is logical_id alone within each canonical table. Revision rows survive supersession. | Keep lifecycle/storage key; kind is not identity. Node/edge namespaces must remain distinct in retrieval. |
| Public IdSpace | `identity.rs` IdSpace/derive_stable_id and `search_types.rs:122–149`: logical IDs are l:, anonymous bodies hash to h:, synthetic rerank passages use p:. write_cursor is positional/internal. | Multiple revisions can share l:, equal anonymous bodies share h:. Neither public ID nor body uniquely names a record revision. Never retro-mint a surrogate ID. |
| Artifact revision/source | `_fathomdb_artifact_revisions` and source links are existing format37 provenance; `identity.rs` has legacy revision derivation; normal SearchHit carries source_id, not a revision-ID field. | Do not manufacture a revision from body or treat the source key as revision identity. A cursor/class internal key needs no new schema. |
| Vector ownership | `_fathomdb_vector_rows`/`_fathomdb_vector_spans`; `vector_spans.rs` validates physical head rowid=write_cursor, window0, byte_start0, body_length and optional UTF-8 byte_end. `projection_commit.rs:283` persists that tuple. | Keep record ownership distinct from provider identity and physical vector ID. Preserve unknown byte_end. No overflow vectors or coverage reporting added here. |
| Visibility authority | `search.rs:1873` onward uses one attributed reader transaction; attributes validate on it; dependency fallback and frozen time govern retrieval. Accepted format37 ADR keeps generation plus branch nonce and coalesced publication. | Eligibility/nonce is an envelope, not an identity key. Do not cache across snapshot, branch, view or time. |

## Fusion and downstream consumers

- Full `fusion.rs` inspected: RRF K30, vector1/text3/graph1; body hash buckets
  still compare full strings on collisions. Each arm credits first body occurrence
  once at its original positional rank. Output ties use vector-input membership,
  then insertion order. The actual accumulation order is vector, text, graph;
  a source comment claiming graph before text does not match that code.
- `search.rs:2669–2682,2840–2844`: explanation ranks and fused scores use body
  maps. Importance/confidence and PerHitExplain.id use cursor. A key-only fusion
  patch would leave body-based rank/source contamination behind.
- `search.rs:1381–1382,1634` and neighbor exclusion use seed bodies. BFS visited
  and entity resolution use logical IDs; historical resolution orders matching
  node versions by descending cursor. Replacing body exclusion does not justify
  changing traversal into a revision graph. Two-stage graph ranking currently
  fuses the already-fused two-arm list as the vector input of another fusion;
  do not silently replace it with one three-arm formula.
- Vector hydration at `search.rs:2010–2180` tries nodes, then edges; vector edge
  hits carry TextEdge, so public branch alone cannot recover artifact class or
  vector-input membership. Preserve membership separately in an internal key/
  rank carrier. Representative-owned source and captured graph-origin source
  are different facts; never transfer a body twin's provenance.
- `snippet_query.rs` uses cursor→(Nodes/Edges,rowid), not body. `snippets_for_hits`
  checks the selected hit body against stripped highlight text; marker collision
  or mismatch yields no spans. A wrong representative can therefore silently
  lose an anchor even if equal-body fusion appeared harmless.
- Existing evidence is positionally parallel to results, tied to source/frozen
  authority. Identity changes must keep the selected record and graph path in
  those carriers; do not use an equal body's source hash or stored vector span.
- MCP emits IdSpace refs and collapses repeated refs in response feedback state;
  logical fetch resolves latest eligible revision. The source-backed distinction,
  existing exposure, consumers and options are in [feedback decision](feedback-decision.md).
  No occurrence-token/ref ABI is selected.

## SDK context-packing consumer

`fathomdb-sdk/src/standalone.rs:304–317` implements context_item_from_hit by
copying `hit.id.to_prefixed()` into ContextItem.id and decoding the **stored**
body into parts. ContextHitOptions has group/position/order_key/header, no ID
override (`256–263`). Python `_sdk_context_pack.py:249–278` and TypeScript
`context-pack.ts:257–270` derive the same prefixed ID. This is an existing public
helper contract, not an injective record-version identifier.

Engine `context_pack.rs:212–218,351–373` validates before rendering and rejects
repeated nonempty item IDs across **both** hits and context; the SDK maps that
ContextPackError to InvalidArgument (`standalone.rs:265–282`). This check applies
even if an item would later be omitted by selection/budget. Existing SDK
`tests/slice41_context_pack.rs` was read in full: helper IDs equal l:round:1,
tagged/untagged parts and malformed-body refusal are pinned; shared pack_errors
include duplicate_id_across_lists with context[0].id. A helper result alone is
not proof that a list of those results is valid pack input.

Consequence: current body fusion returns one of two identical anonymous bodies;
helper→pack can succeed. Proposed record fusion returns both, with the same h:
ID, so mapping both through the unchanged helper makes packing refuse. Distinct-
body historical revisions already expose repeated l: IDs today. Merely selecting
record identity does not make that composed API accept every search hit list.

Proposed disposition is to preserve helper ID derivation, duplicate-ID rejection
and codecs, and include this new success→typed-refusal exposure explicitly in the
pending identity ruling. Do not infer a helper/pack behavior change or auto-merge/
relabel items. A caller already can construct ContextItems with its own unique
labels and retain a separate label→record/occurrence map (including matching
full_ids); those labels are packing provenance keys, not new IdSpace or stable
record IDs. Requiring automatic acceptance of every hit would need a separately
proposed helper-label or pack-contract change and owner ruling. This proposal
chooses no new field/ID ABI. Preserve both choices honestly in consumer guidance.

## Text paths: existing versus net-new

- `search.rs:2312–2330,2384–2420`: hybrid_deferred_identity is a narrow route
  for vector-present, no filter/perf cap/dependencies/relaxed flags/reweight/CE/
  graph/explain/full-sort override, plus all-nodes-directly-eligible proof.
  It SELECTs **body,kind,cursor,bm25,rowid for every match**, allocates SearchHits,
  sorts `(bm25,cursor)`, and hydrates final text representatives' ID/source only
  after truncation. It does not defer body hydration today.
- Direct text sets a fixed `MAX_SEARCH_RESULT_LIMIT` node candidate input
  (`search.rs:174,2233`), unlike hybrid no-vector fallback. Its rank stream
  completes boundary ties, and filtered SQL already applies eligibility before
  bounds. Do not convert this to caller-limit candidate selection or hybrid.
- Node FTS and edge FTS are different corpora. Nodes are ranked by bm25/cursor;
  edge candidates are independently ranked and **appended** to text_results
  (`search.rs:2540–2653`). They are not globally sorted by comparable raw bm25.
  Exact optimization must preserve this concatenated positional text rank.
- Node FTS rowid=write_cursor is already written by `index_projector.rs:366`.
  Edge FTS inserts omit rowid (`475`); snippet hydration must retain its actual
  FTS rowid. A node-only invariant must not be applied to edges.
- `slice90_filtered_body_fts.rs` supplies real provider-free closed-filter tests:
  metadata sentinel/time, missing metadata, node-scoped AND attributes,
  historical flags, typed undeclared-filter refusal and limit-prefix stability.
  This is shipped engine/SDK behavior; integrated60 now forwards the closed
  direct-text filter. Its relaxed-view search flags remain separately held by F11.

## Snippets: existing versus net-new

Full `snippet.rs` and `snippet_query.rs` inspected. Short bodies with empty raw
spans already return Whole without boundary tables. Long no-span bodies still
build whole-body grapheme, word and sentence tables before Start selection.
Normalisation ignores invalid char-boundary/out-of-body/empty spans, widens valid
ones to graphemes and merges overlaps. Start uses a word boundary within32 bytes
before the budget edge, otherwise a grapheme boundary, trims trailing whitespace
while retaining its floor, and adds " …" inside the byte budget. Head/max_windows
change match selection but not the no-span prefix. `windows`/`spans` are original
body offsets; highlights are offsets into joined snippet text.

Search-local span queries are unchanged: one combined pool and one all-or-nothing
per-term pool, each max_point_queries; at most twice that cap. Same state/query/
options/**run window** is the determinism boundary, not arbitrary continuation
window sizes. Preserve existing stored span bytes and all hint/default settings.

## Consumer and data-access limits

Read-only consumer **source only** at `/home/coreyt/projects/memex`, commit
`9cbe455888a50b1a69cc15cb9c0b35399416496d` (inspected paths clean at that commit):

| Actual callsite | Observed downstream behavior | Inference limit |
| --- | --- | --- |
| `src/memex/fathom_store.py:4299–4393`, `WorldModelRepository.search` | SDK hybrid search; decodes stored body to properties, filters retired/family hits, threads score/CE/source and logical IdSpace.value; returns in result order up to limit. | No occurrence cursor survives this adapter; JSON source property and engine source remain distinct. |
| Same file `:4489–4558`, `search_expand` and `:3610–3640`, WorldModelSearchHit | Opt-in graph search preserves logical ID and source; dataclass has no revision locator/packing label. | Graph source is currently a path provenance value; do not relabel it as canonical-node ownership. |
| `src/memex/memory/retrieval_wm_search.py:180–275` | Knowledge caller takes first item_id/CE occurrence, then loads by knowledge_id and assigns position score. | This is deliberate consumer entity aggregation, not proof that engine record revisions should fuse. |
| Same file `:277–391`, `:394–486` | Action retrieval deduplicates IDs in order; typed family retrieval filters properties, then by-ID loads and position-scores rows. | Search-selected revision is not promised by later by-ID loads. More engine hits need not survive consumer aggregation. |
| `eval/fathomdb/run_track_b_retrieval.py:260–300` | Production search_documents result IDs; separate opt-in graph/rerank trace call. | A diagnostic trace call is not the measured production query's record/census proof. |

A bounded source search of Memex `src`, `dev/fathomdb`, `eval/fathomdb` found
no context_item_from_hit/pack_context or response_id/feedback_ref consumer.
This says nothing about every deployed integration or other paths. No Memex
source was modified, imported or executed. Historical backup documentation/path
metadata in the prior census never established consistency or current format;
this phase did not repeat filesystem data inspection. No live DB, snapshot,
SQLite header, private data, models, configuration or secrets were opened.

The owner already approved50 identity census/ADR scope; no renewed generic
census approval is needed. No exact appropriate consumer snapshot/access protocol
was identified in the relevant inspected docs/source. Missing: an existing safe
snapshot path/identity, its provenance/consistency confirmation (closed copy with
no live writer/WAL dependency), and an access method preventing source sidecars/
writes. A concrete existing consistent snapshot within that approved scope may
later be used; creating one from live data first needs a concrete safe method. Do not open a readable live DB or use
`immutable=1` to pretend a live WAL database is frozen; do not invoke Engine.open
or safe_export against production, copy DB/model bytes, or dump config secrets.

After identifying the appropriate consistent snapshot/access method, bind its
identity/query set; run aggregate-only
counts of equal-body groups across artifact class/cursor/logical/revision/source/
visibility and per-arm overlap. Compare rows at fixed views and record population
selection. Publish no bodies/raw IDs. Until then, runtime census is HELD and all
frequency/benefit claims are unknown. The old LME twin counts in memory are
historical study evidence, not fresh consumer measurements or30 substitution.

## Current-source ownership hazards and consumer surface

`bfs_graph_arm_candidates` is fully traced at search.rs:1355–1827. Entity-FTS
seeds use the selected node source; edge-FTS endpoint seeds and reached neighbors
carry the winning edge source/confidence. The current SearchHit contract in
search_types.rs:100–120 documents that branch distinction. The record key names
the reached node; the provenance carrier names how it was reached. Keeping
representative-first means a same-record vector/text representative keeps its
own source, while a graph-only representative retains its existing winning path
source. Replacing every GraphArm source with the canonical node's own source
would be a separate semantic change, not an identity fix. Accepted exact-graph
artifact evidence also keeps selected target and terminal edge distinct.

`snippet_query.rs` currently keys FtsRows by cursor; that is not enough to keep
two explicit ownerless rows with the same cursor separate. The proposed internal
carrier must retain key + actual (table,rowid) through final positional snippet
assembly, independent of public/explanation cursor. Such injection fixtures are
real SQLite controls, not permission to read/repair a live store. SearchHit's
branch cannot classify vector-hydrated edges (they are TextEdge), nor distinguish
ownerless rows. Never recover keys from public ID, decoded text or branch.

Only engine lib.rs exports the two doc-hidden fusion helpers. Actual external-
module callable consumers are `tests/pr_g9_rrf_fusion.rs` and
`tests/slice40_fts_ranking.rs`; the IR-C experiment has a local formula and other
matches are comments. Proposed crate-private helper disposition is in
[design](design.md); absence of further matches in this checkout/inspected Memex
is not evidence about external crates. No helper signature was changed.

## Final-review P2: RecordKey is not the telemetry cursor domain

At the same3fa16620a source boundary, filter.rs `body_fts_rank_sql_view` selects
stored FTS write_cursor and independent rowid. The existing slice71 ownerless
fixture omits rowid when inserting cursor999999; this is not a rowid=cursor
invariant for ownerless FTS. Two ownerless locators or an ownerless node-FTS row
and canonical edge can alias one cursor. telemetry.rs `capture_telemetry` masks
stable IDs absent from both canonical tables but still captures cursor result_ids;
`capture_telemetry_with_sink` keys arm_of on cursor, and `record_feedback` accepts
cursor arrays. There is no proof that cursor equality implies selected-record
equality, even when refs match. See [feedback decision](feedback-decision.md)
for the conservative complete-result proof and visible-member boundary. Current
SearchRun::from_result retains the complete captured result before page shaping,
so the adapter can count complete-run cursors and visible-page refs without
a new engine/public ABI.
This source proof supplies no runtime reproduction, prevalence estimate or
production-defect claim and authorizes no telemetry-schema change.

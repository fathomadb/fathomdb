---
title: Slice 50 source and consumer census
status: SOURCE_ONLY
target_release: 0.8.29
slice: 50
---

# Slice 50 source and consumer census

Inspected frozen parent `2f6913c1d59ac8ff3a30de835b57290a8635998c`; no runtime
census or tests. Paths below are repository-relative unless explicitly Memex.
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
  This is shipped engine/SDK behavior;60's remaining MCP forwarding is separate.

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

Read-only Memex source/docs at `/home/coreyt/projects/memex` were searched.
`fathom_store.py:4300–4374` adapts SDK results from decoded body, kind, score,
ce_score, source_id and logical IdSpace.value; source_id inside JSON properties
is distinct from engine erasure provenance. Existing identity-recall audit is
historical source analysis, not a current duplicate census. In Memex,
`FathomStore.open` (`src/memex/fathom_store.py`)
runs writable migrations, registration and recovery; `_AdminAdapter.safe_export` currently raises NotImplementedError.
`src/memex/backup.py`'s `backup_all` opens the store and rotates/deletes backups;
`backup_dir` creates its destination directory.
Its checkpoint description is not a supported acquisition contract on this build.
No helper was invoked. R-I4's old spike supplies no current snapshot/access method.

Historical Phase A metadata only: `/home/coreyt/.memex/backups` existed with24
`fathom*.db` names. The latest filename timestamp candidate is
`fathom-startup-20260630T142206.db` (696320 bytes, not a symlink), with both
`-wal` and `-shm` present. The last8 filename-ordered candidates inspected also
have both sidecars. This does not establish live-writer state, consistency,
format37, source provenance or relevance; these old candidates are not a qualified
current consumer census snapshot. No DB content/SQLite header was read.

The owner already approved50 identity census/ADR scope; no renewed generic
census approval is needed. The independently reviewed method below is proposed;
the exact artifact, custody and capture provenance remain missing.
A qualified supplied artifact needs these inputs, not renewed census approval; this correction grants no new data access or acquisition authorization.

1. **Establish closed custody before any SQLite open.** Obtain the owner's exact
   artifact path, originating Memex population, capture method/time and confirmation that
   its main file contains all committed data, with no writer or WAL dependency.
   Digest, schema, lock availability and absent sidecars alone prove neither historical completeness nor representativeness.
2. **Prefer that existing qualified artifact.** Otherwise, propose a later explicitly
   authorized byte-preserving copy of an independently closed, main-file-complete,
   WAL-independent source. Exact source/destination paths and owner authorization
   precede copying; retain closed custody and verify byte identity. WAL-dependent
   sources are ineligible. Do not raw-copy live files, checkpoint or delete sidecars.
3. **Inspect format/layout read-only after step1.** Use a percent-encoded URI with
   `mode=ro&immutable=1`, READ_ONLY|URI, `query_only=ON` and one read transaction.
   Read `PRAGMA user_version` and require37; verify canonical/provenance/visibility/span
   layout and no unresolved WAL/journal dependency; hold on disagreement, without
   migration. Immutability follows custody; it does not freeze live files.
   [`inspect_data_plane_integrity`](../../../../../src/rust/crates/fathomdb-engine/src/operator/data_plane.rs)
   supplies the boundary pattern, not a census executor; it requires a pre-existing
   acquirable lock. Do not create one or invoke Engine.open.
4. **Bind repeatable aggregate queries.** Retain artifact digest/size, capture
   provenance, source/tool revision, SQLite build, exact aggregate SQL and fixed view/time/flags/population selection in this record. Preserve
   node/edge, physical locator, logical/revision/source and anonymous/ownerless
   distinctions; compare exact stored bytes internally and publish aggregates only.
   Generation/nonce binds the visibility envelope, not completeness or record identity;
   retain stored hints/spans and role-tagged byte coordinates.
5. **Report only observed scope.** SQL counts establish stored membership/eligibility,
   not actual physical/vector retrieval overlap unless separately executed under its
   authorized resource window. Execution stays held pending inputs; no calibration verdict, threshold or acceptance change.

Native [`Engine::safe_export`](../../../../../src/rust/crates/fathomdb-engine/src/operator.rs)
uses VACUUM INTO; CLI [`run_doctor_verb_inner`](../../../../../src/rust/crates/fathomdb-cli/src/lib.rs)
first opens a writable Engine with possible reconciliation/maintenance effects.
Export reconstructs the DB, with no demonstrated preservation contract for every census physical locator.
It is not the default fallback; an alternative needs
concrete justification/authorization. Closed, main-file-complete input needs no export.
No schema change or config-secret access follows from this method.

After qualifying the exact artifact under this method, run aggregate-only
counts of equal-body groups across artifact class/cursor/logical/revision/source/
visibility and stored arm membership. Compare rows at fixed views and record population
selection. Publish no bodies/raw IDs. Until then, runtime census is HELD and all
frequency/benefit claims are unknown. The old LME twin counts in memory are
historical study evidence, not fresh consumer measurements or30 substitution.

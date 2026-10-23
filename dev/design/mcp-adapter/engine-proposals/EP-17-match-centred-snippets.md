---
title: EP-17 — Match-centred snippets on search results
date: 2026-10-18
status: PROPOSED
desc: Draft engine proposal. An opt-in snippet sidecar on search and search_text_only, with match spans from FTS5 highlight() on the live index fetched only for hits that need them, plus a pure SDK selector, so each hit carries a byte-budgeted, grapheme-safe window centred on the words that matched.
inputs:
  - dev/design/agent-access/match-centred-snippets.md
  - src/rust/crates/fathomdb-engine/src/{search,search_types,filter,index_projector,fusion}.rs
  - src/rust/crates/fathomdb-schema/src/lib.rs
  - src/rust/crates/fathomdb-mcp/src/{shape,tools}.rs
---

# EP-17 — Match-centred snippets on search results

Status: Draft. Priority P1, because it removes an adapter workaround: prefix
snippets plus an instruction telling agents to fetch. Size M. Not an accepted
decision. The full analysis is in
[match-centred-snippets.md](../../agent-access/match-centred-snippets.md).

**Engine built (agent-access Block 1, S5a and S5b).** The pure selector
`snippet_for` and the search sidecar are in `fathomdb-engine` and
`fathomdb-sdk`, documented in `dev/interfaces/rust.md` (§§ "Match-centred
snippet selector", "Match-centred snippets on search") and
`dev/interfaces/rust-sdk.md`. The engine entry points are
`search_reranked_view_with_snippets`, `search_text_only_view_with_snippets`
and `search_frozen_with_snippets`. The SDK options gain `snippet`, including
`FrozenSearchOptions`. As built:

- `CompiledQuery.terms` carries the OR operands for the per-term pass.
- The per-term queries draw on a second per-search pool of
  `max_point_queries`, all or nothing per hit (Block 1 close, 2026-10-21;
  match-centred-snippets.md § 6.5). A search issues at most
  2 × `max_point_queries` point queries.

Still open: the MCP adapter (S5c), Python and TypeScript parity (S5d), and
the M1 and M2 measurements.

## Problem

The adapter can build snippets only as body prefixes
(`fathomdb-mcp/src/shape.rs:15-20`, `fathomdb-mcp/src/tools.rs:701-716`).

- `SearchHit.body` is already the full body
  (`fathomdb-engine/src/search_types.rs:95`), so the adapter has the text.
- What it lacks is where the match is. `SearchHit` and `SearchResult` carry no
  offsets (`search_types.rs:76-99`, `search_types.rs:137-150`).
- The adapter may not link rusqlite (MR-070.2), so it cannot ask FTS5.

On records longer than the cap, the match falls outside the snippet, and
agents answer from the wrong text.

## Proposed surface

- `SearchOptions.snippet: Option<SnippetOptions>`, and the same field on
  `TextSearchOptions`.
  - `SnippetOptions { max_bytes: u32 (40..=4000), max_windows: u8 (1..=3,
    default 1), head_bytes: u32 (0..=max_bytes/2), highlights: bool (default
    false), max_point_queries: u8 (default 20, set by M1) }`.
  - Values out of range are `InvalidArgument`.
- `SearchResult.snippets: Option<Vec<SnippetV1>>`.
  - It is a sidecar, the same length and order as `results`.
  - It is `None` when the option is off, following the explain sidecar's
    zero-cost rule R-OBS-2
    (`dev/design/0.8.8-explain-and-telemetry-adr.md:46`).
- `SnippetV1 { text, anchor: Whole | Match | Start, windows: Vec<(u32,u32)>,
  spans: Vec<(u32,u32)>, highlights: Vec<(u32,u32)>, cut: bool }`.
  - `windows` and `spans` are byte ranges into the body.
  - `highlights` are byte ranges into `text`, filled only on request.
  - `text` never contains inserted markup. Its only additions are the
    `" … "` joins and the leading and trailing ellipses.
- A pure selector: `snippet_for(body: &str, spans: &[(u32,u32)], options:
  &SnippetOptions) -> SnippetV1`.
  - The engine uses it, and the SDK exports it.
  - It needs no database. A caller that must fit a smaller budget re-runs it
    with a smaller `max_bytes`, deterministically and without losing the match.

## Mechanism

1. **Carry the FTS rowid.** The text candidate SELECTs also select
   `search_index.rowid` (`fathomdb-engine/src/filter.rs:697`,
   `search.rs:1855`, `search.rs:1881`, `search.rs:1971`, `search.rs:1997`).
   The edge SELECT does the same for `search_index_edges`
   (`filter.rs:708-715`).
   - The rowid goes into a search-local map from `write_cursor` to
     `(table, rowid)`. It is not stored on `SearchHit`.
   - The rowid is not known to equal `write_cursor`, because the projector
     inserts without one (`index_projector.rs:361`).
2. **Fetch spans only where needed.** After `results.truncate(final_limit)` and
   the identity pass (`search.rs:2265-2286`), in the same reader transaction,
   walk the final hits in rank order. Run a point query for a hit only if all
   of the following hold:
   - the hit is in the map;
   - `body.len() > max_bytes`, or `highlights` is true;
   - fewer than `max_point_queries` point queries have run so far.

   The point query is
   `SELECT highlight(<table>, 0, ?open, ?close) FROM <table> WHERE <table> MATCH
   ?match AND rowid = ?rowid`.
   - `?match` is the same compiled expression the arm used
     (`fathomdb-query/src/lib.rs:71-85`), so tokenizer and query parity hold
     by construction.
   - FTS5 seeks on `rowid = ?` with MATCH, as verified in the source at review
     (`fts5BestIndexMethod` `bSeenEq`; `fts5FilterMethod` → `sqlite3Fts5ExprFirst`
     → `sqlite3Fts5IterNextFrom`). So a point query costs one seek per OR term.
   - The markers are the first pair absent from the body, from U+E000/E001,
     U+F8FE/F8FF and U+FDD0/FDD1. A body that contains all three pairs is not
     highlighted.
3. **Select windows** with `snippet_for`, as specified in
   match-centred-snippets.md § 6:
   - a byte budget;
   - span ends widened to grapheme boundaries;
   - UAX #29 word and sentence snapping that never cuts a counted span;
   - FTS5-style distinct-term-first scoring and centring;
   - window merging;
   - exact reproducibility within a build.
4. **Anchor rule.** `anchor = Match` if and only if the map holds the hit's
   `write_cursor` and highlighting produced spans. `branch` is not consulted,
   because fusion keeps the vector hit's struct when both arms matched
   (`fathomdb-engine/src/fusion.rs:128-137`). Hits not in the map, those over
   the cap and those using the marker fallback get the prefix, with
   `anchor = Start`.
   - The default embedder sees the first 512 tokens
     (`fathomdb-embedder/src/candle_bge.rs:60-66`), about 1.5–2 KB, so the
     prefix lies inside the region the vector arm scored.
   - Residual gap: identical bodies with different `write_cursor`s dedup in
     fusion (`fusion.rs:101-105`). A body-hash key on the map would close it.

## Invariants and consistency

- With `snippet: None`, every existing `SearchResult` field equals today's,
  `snippets` is `None`, and nothing per-hit is allocated.
- Snippet options are not eligibility. They do not affect ranking, the frozen
  context digest or `ADR-0.8.25-eligibility-and-frozen-reads.md`.
- Every `windows` range lies on grapheme boundaries of the body, every counted
  span lies inside a window, and `text.len() ≤ max_bytes`.
- Identical state, arguments and build give byte-identical snippets. An
  upgrade of `unicode-segmentation` or the bundled SQLite may change them.
- The work runs only on reader connections, which are `query_only`
  (`fathomdb-engine/src/open.rs:1358-1360`). `query_only` rejects TEMP writes
  connection-wide (`vdbe.c` `OP_Transaction`, verified at review), so the
  design uses read-only statements only.

## Cost and performance

- **Memory: none new.** The FTS tables use FTS5's defaults: `detail=full`, plus
  a content copy of the body (`fathomdb-schema/src/lib.rs:338-345`,
  `lib.rs:425-430`). Positions and text are already stored.
- **Compute:** combined span queries run only for hits that are over budget
  or that need highlights, at most `max_point_queries` per search. The
  per-term pass of the exact-identity upgrade draws on a second pool of the
  same size, so a search issues at most 2 × `max_point_queries` point
  queries, whatever `k` is.
  - About 92% of LoCoMo turns fit in 300 characters, so most hits need none.
  - Each point query costs `k` doclist seeks, one cached content-row read and
    one tokenizer pass over the body. This is estimated at tens of
    microseconds for a body of a few KiB, but not measured.
- The MCP adapter can request 100 hits (`fathomdb-mcp/src/shape.rs:79-81`), so
  M1 measures limit 10 and limit 100 × `k` ∈ {1, 5, 10}, with a decision rule
  for each limit.
- The hybrid text candidate set has no `LIMIT` (`search.rs:1740-1752`). Doing
  the same work inside the candidate query would cost one pass per match, not
  one per final hit.

## Alternatives considered

- Re-tokenizing in Rust is rejected as the span source. Its parity with
  SQLite's `porter` and unicode61 tables is at risk, and the adapter cannot
  know the tokenizer.
- Stored byte offsets (a term-vector projection) are rejected: a schema bump,
  write amplification, and about twice the field's index size on
  Elasticsearch's evidence.
- FTS5 `snippet()` is rejected as the output format. It caps the window at 64
  tokens, returns one fragment and puts its markup inline.
- Exact term identity: the first upgrade is one `highlight()` per OR term, only
  on over-budget hits with `k > 1` whose first window misses spans. It needs no
  `unsafe`. Its queries come from a second per-search pool of
  `max_point_queries`; a hit the pool cannot cover in full keeps its
  approximate snippet.
  - A custom FTS5 auxiliary function returning `xInst` triples is a later
    option. It runs the same point query, so it improves precision, not cost.
- A per-search **time** cap is rejected in favour of the hit cap, because a
  time cap would make the output depend on load.

## Interaction with ADRs and interfaces

- New fields on `SearchOptions`, `TextSearchOptions` and `SearchResult`, and the
  new `snippet_for` function, across Rust, Python and TypeScript:
  `dev/interfaces/rust.md`, `rust-sdk.md`, `python.md`, `typescript.md` and the
  parity map. This needs an ADR or an interface update in the same PR.
  - `SearchResult` is `#[non_exhaustive]` (`search_types.rs:137`), so adding a
    field breaks no external code.
  - The two option structs have public fields
    (`fathomdb-sdk/src/options.rs:57-90`). External struct literals without
    `..Default::default()` break.
- MCP: `snippet_chars` becomes `snippet_bytes`, a new `snippet_anchor` hit
  field is added, and the MR-040 wording changes from characters to bytes
  (`dev/design/mcp-adapter/requirements.md:359-366`, `dev/interfaces/mcp.md:45`,
  `mcp.md:127`).
- No schema change and no migration.

## Test plan sketch

Real-database and pure RED tests T-1 to T-20 in match-centred-snippets.md § 8.
They include:

- a deep match in a 4 KiB record;
- porter and diacritic parity, including NFD;
- a `proptest` property over arbitrary Unicode;
- lexical anchoring decided by the map, not by `branch`;
- off-by-default field equality;
- reader-pool safety;
- `head_bytes` on single-line records;
- edge hits;
- frozen-read digest stability;
- window scoring and snapping;
- the rule-0 skip and the point-query cap;
- binding parity.

## Adapter change once it lands

- The adapter passes `SnippetOptions` from config and uses `snippets[i]`.
- It adds `snippet_anchor` and takes `snippet_cut` from `snippets[i].cut`.
- In `cut_bodies`, it re-runs `snippet_for` with a smaller `max_bytes` instead
  of cutting a prefix.
- It rewrites the instruction at `fathomdb-mcp/src/edge.rs:38-39`, and the
  hints design gets the matching edits (match-centred-snippets.md § 7.3).

## Open questions for the owner

1. Should `search_projected_text` (`property_search_index`) get snippets in
   the same change?
   - *Reviewer's view:* none recorded.
2. Ship exact term identity (per-term `highlight()`) in the first change, or
   after M2?
   - *Reviewer's view:* per-term `highlight()` first, when identity is wanted;
     the auxiliary function later.
3. Default `max_windows`?
   - *Reviewer's view:* ship 1. Raise it to 2 only if M2 shows a gain.

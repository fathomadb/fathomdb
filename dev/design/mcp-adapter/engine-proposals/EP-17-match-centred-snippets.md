---
title: EP-17 — Match-centred snippets on search results
date: 2026-10-18
status: PROPOSED
desc: Draft engine proposal. An opt-in snippet sidecar on search and search_text_only, built after final truncation from FTS5 highlight() on the live index, so each hit carries a byte-budgeted, grapheme-safe window centred on the words that matched.
inputs:
  - dev/design/agent-access/match-centred-snippets.md
  - src/rust/crates/fathomdb-engine/src/{search,search_types,filter,index_projector}.rs
  - src/rust/crates/fathomdb-schema/src/lib.rs
  - src/rust/crates/fathomdb-mcp/src/{shape,tools}.rs
---

# EP-17 — Match-centred snippets on search results

Status: Draft. Priority P1, because it removes an adapter workaround: prefix
snippets plus an instruction telling agents to fetch. Size M. Not an accepted
decision. The full analysis is in
[match-centred-snippets.md](../../agent-access/match-centred-snippets.md).

## Problem

The adapter can build snippets only as body prefixes
(`fathomdb-mcp/src/shape.rs:15-20`, `fathomdb-mcp/src/tools.rs:701-716`). The
SDK returns no match positions:

- `SearchHit` has no offsets (`fathomdb-engine/src/search_types.rs:76-99`).
- `SearchResult` has no snippet data (`search_types.rs:138-150`).

The adapter may not link rusqlite (MR-070.2), so it cannot ask FTS5. On records
longer than the cap, the match falls outside the snippet, and agents answer
from the wrong text.

## Proposed surface

- `SearchOptions.snippet: Option<SnippetOptions>`, and the same field on
  `TextSearchOptions`.
  - `SnippetOptions { max_bytes: u32 (40..=4000), max_windows: u8 (1..=3),
    head_bytes: u32 (0..=max_bytes/2) }`.
  - Values out of range are `InvalidArgument`.
- `SearchResult.snippets: Option<Vec<SnippetV1>>`.
  - It is a sidecar, the same length and order as `results`.
  - It is `None` when the option is off, following the explain sidecar's
    zero-cost rule R-OBS-2
    (`dev/design/0.8.8-explain-and-telemetry-adr.md:46`).
- `SnippetV1 { text, anchor: Whole | Match | Start, windows: Vec<(u32,u32)>,
  highlights: Vec<(u32,u32)>, cut: bool }`.
  - `windows` are byte ranges into the body.
  - `highlights` are byte ranges into `text`.
  - `text` never contains inserted markup. Its only additions are the
    `" … "` joins and the leading and trailing ellipses.

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
2. **Highlight the final hits.** After `results.truncate(final_limit)` and the
   identity pass (`search.rs:2265-2286`), in the same reader transaction, run
   one point query for each final hit that has a map entry:
   `SELECT highlight(<table>, 0, ?open, ?close) FROM <table> WHERE <table> MATCH
   ?match AND rowid = ?rowid`.
   - `?match` is the same compiled expression the arm used
     (`fathomdb-query/src/lib.rs:71-85`), so tokenizer and query parity hold
     by construction.
   - The markers are U+E000 and U+E001. A body containing either marker skips
     highlighting.
3. **Derive spans, then select windows.** Pure Rust, specified in
   match-centred-snippets.md § 6:
   - byte budget;
   - UAX #29 word and sentence snapping, so every edge is a grapheme boundary;
   - FTS5-style distinct-term-first scoring and centring;
   - window merging;
   - exact reproducibility.
4. **No anchor.** Vector-only, graph-arm and marker-fallback hits get the
   grapheme-safe prefix, with `anchor = Start`. The default embedder sees only
   the first 512 tokens (`fathomdb-embedder/src/candle_bge.rs:60-66`), so the
   prefix is the text the vector arm scored.

## Invariants and consistency

- With `snippet: None`, results are byte-identical to today, and nothing
  per-hit is allocated.
- Snippet options are not eligibility. They do not affect ranking, the frozen
  context digest or `ADR-0.8.25-eligibility-and-frozen-reads.md`.
- Every `windows` range lies on grapheme boundaries of the body, and
  `text.len() ≤ max_bytes`.
- Identical state and arguments give byte-identical snippets.
- The work runs only on reader connections, which are `query_only`
  (`fathomdb-engine/src/open.rs:1358-1360`). It writes nothing, not even to
  temp tables.

## Cost and performance

- **Memory: none new.** The FTS tables use FTS5's defaults: `detail=full`, plus
  a content copy of the body (`fathomdb-schema/src/lib.rs:338-345`,
  `lib.rs:425-430`). Positions and text are already stored.
- **Compute per final hit:** one rowid-constrained MATCH, one cached content-row
  read and one tokenizer pass over the body. This is estimated at tens of
  microseconds for a body of a few KiB, but not measured.
  - The hybrid text candidate set has no `LIMIT` (`search.rs:1740-1752`).
    Doing the same work inside the candidate query would cost one pass per
    match, not one per final hit.
- Measurement M1 in the design doc decides whether MCP turns snippets on by
  default.

## Alternatives considered

- Re-tokenizing in Rust is rejected. Its parity with SQLite's `porter` and
  unicode61 tables is at risk, and the adapter cannot know the tokenizer.
- Stored byte offsets (a term-vector projection) are rejected: a schema bump,
  write amplification, and about twice the field's index size on
  Elasticsearch's evidence.
- FTS5 `snippet()` is rejected as the output format. It caps the window at 64
  tokens, returns one fragment and puts its markup inline.
- A custom FTS5 auxiliary function returning `xInst` triples is the upgrade
  path if lowercased-surface term identity is too coarse. It needs `unsafe`
  `fts5_api` access.

## Interaction with ADRs and interfaces

- New fields on `SearchOptions`, `TextSearchOptions` and `SearchResult`, across
  Rust, Python and TypeScript: `dev/interfaces/rust.md`, `rust-sdk.md`,
  `python.md`, `typescript.md` and the parity map. This needs an ADR or an
  interface update in the same PR.
- MCP: `snippet_chars` becomes `snippet_bytes`, a new `snippet_anchor` hit
  field is added, and the MR-040 wording changes from characters to bytes
  (`dev/design/mcp-adapter/requirements.md:359-366`, `dev/interfaces/mcp.md:45`,
  `mcp.md:127`).
- No schema change and no migration.

## Test plan sketch

Real-database RED tests T-1 to T-13 in match-centred-snippets.md § 8. They
include:

- a deep match in a 4 KiB record;
- porter and diacritic parity;
- a `proptest` property over arbitrary Unicode;
- the vector-only prefix;
- off-by-default byte identity;
- reader-pool safety;
- binding parity.

## Adapter change once it lands

The adapter passes `SnippetOptions` from config and uses `snippets[i]`. It adds
`snippet_anchor`, trims `cut_bodies` from both ends, and rewrites the
instruction at `fathomdb-mcp/src/edge.rs:38-39`.

## Open questions for the owner

1. Ship variant (a′), the custom auxiliary function, from the start, or the
   `highlight()` marker parse first?
2. Should `search_projected_text` (`property_search_index`) get snippets in
   the same change?
3. Default `max_windows`: 1 (simplest) or 2 (covers two distinct terms far
   apart)?

---
title: Match-centred snippets (query-biased snippet generation)
date: 2026-10-18
status: PROPOSED
desc: Work item 3. Terminology and prior art for query-dependent snippets, how FathomDB's text and vector arms could anchor a snippet, the compute-versus-memory options, precise UTF-8 text handling, and a recommended engine-side design (EP-17) with adapter changes and a RED-test plan.
inputs:
  - dev/design/agent-access/README.md
  - dev/experiments/mcp-agent-loop/README.md
  - dev/design/mcp-adapter/requirements.md
  - dev/interfaces/mcp.md
  - dev/design/0.8.8-explain-and-telemetry-adr.md
  - dev/adr/ADR-0.8.25-eligibility-and-frozen-reads.md
  - src/rust/crates/fathomdb-mcp/src/{shape,tools,config,edge,output}.rs
  - src/rust/crates/fathomdb-engine/src/{search,search_api,search_types,filter,index_projector,open,fusion}.rs
  - src/rust/crates/fathomdb-sdk/src/options.rs
  - src/rust/crates/fathomdb-schema/src/lib.rs
  - src/rust/crates/fathomdb-query/src/lib.rs
  - src/rust/crates/fathomdb-embedder/src/candle_bge.rs
---

# Match-centred snippets

Paths are relative to `src/rust/crates/` unless given in full. Every claim
about FathomDB cites code at `path:line`. Claims about other systems cite the
primary documentation or source. Where something could not be verified, the
text says so.

## 1. Problem

The MCP adapter builds every snippet as a prefix of the record body:

- `snippet(body, cap)` returns the first `cap` characters, cut on a character
  boundary (`fathomdb-mcp/src/shape.rs:15-20`).
- The search tool calls it once per hit with `snippet_chars`
  (`fathomdb-mcp/src/tools.rs:701-716`). The default is 300, and the allowed
  range is 40 to 2000 (`fathomdb-mcp/src/config.rs:112`, `config.rs:254-255`).
- When the response is over budget, `cut_bodies` shortens the longest snippet,
  again as a prefix (`shape.rs:174-198`).
- The server instructions tell agents that "Snippets are the first N
  characters of each record, not the passage that matched"
  (`fathomdb-mcp/src/edge.rs:38-39`).

On LoCoMo turns this mostly worked, because 300 characters covers about 92% of
turns. Two things make it fail on longer records:

- The match often lies past the prefix. Agents rarely fetch, even when
  `snippet_cut` is true, so they answer from text that does not contain the
  answer (`dev/experiments/mcp-agent-loop/README.md`, run 2, and the work list).
- The evaluated stores put the session date at the start of each turn, on the
  same single line as the text: `(unknown date) [<session date>] <speaker>:
  <text>` (`dev/experiments/mcp-agent-loop/README.md:46`, `README.md:96`,
  `README.md:304-305`). A prefix spends part of its budget on that header. A
  centred window may drop it, and temporal questions need it. § 6.6 handles
  this.

The goal is a snippet centred on the passage that matched. It should handle
text precisely, and its compute and memory cost should be explicit.

## 2. Terminology: what this is called

**Verdict: "rewrite engine" is the wrong name.** The feature is
**query-biased (query-dependent) snippet generation**, also called **dynamic
summaries**. Search libraries call the mechanism **hit highlighting**, and the
classic display form is **keyword-in-context (KWIC)**. It is extractive: it
selects spans of the stored text and does not rewrite anything.

- Tombros and Sanderson introduced *query-biased summaries* ("Advantages of
  query biased summaries in information retrieval", SIGIR 1998, pp. 2–10,
  DOI [10.1145/290941.290947](https://doi.org/10.1145/290941.290947)). In their
  user study, query-biased summaries replaced title plus leading sentences, and
  relevance judgements became both faster and more accurate.
- *Introduction to Information Retrieval* § 8.7 separates *static* summaries,
  "which are always the same regardless of the query", from *dynamic (or
  query-dependent)* ones, and names the common form keyword-in-context
  (<https://nlp.stanford.edu/IR-book/html/htmledition/results-snippets-1.html>).
  It also notes that a positional index alone cannot reconstruct the
  surrounding text. Engines therefore cache the document text, which FathomDB's
  FTS table already does (§ 3.1).
- Lucene and Elasticsearch call it *highlighting*. Its units are *fragments*
  (classic Highlighter) or *passages* (UnifiedHighlighter: `Passage`,
  `PassageScorer`, `PassageFormatter`).

Why "rewrite" is wrong, and where it does appear:

- A *rewrite engine* is a URL rewriter or, more generally, a term-rewriting
  system (<https://en.wikipedia.org/wiki/Rewrite_engine>).
- In Lucene, `Query.rewrite` turns a query into primitive queries. For example,
  a `PrefixQuery` becomes a `BooleanQuery` of `TermQuery`s
  (<https://lucene.apache.org/core/9_11_0/core/org/apache/lucene/search/Query.html>).
- Highlighters call rewrite as a sub-step, to learn which concrete terms a
  multi-term query matched. So rewriting is one input to highlighting, not its
  name.
- FathomDB has no multi-term queries. `compile_text_query` emits only quoted
  literal tokens (`fathomdb-query/src/lib.rs:71-85`), so FathomDB does not need
  even that sub-step.

## 3. FathomDB today

### 3.1 The text arm's FTS5 table already stores positions and text

- The node text arm queries `search_index`, which migration step 11 creates
  as `fts5(body, kind UNINDEXED, write_cursor UNINDEXED, tokenize = 'porter
  unicode61 remove_diacritics 2')` (`fathomdb-schema/src/lib.rs:338-345`).
  Edges use `search_index_edges`, which has the same column set and tokenizer
  (`lib.rs:425-430`). `search_index_v2` has the columns `kind, body, status`
  (`lib.rs:497-503`).
- None of these tables sets `detail=` or `content=`. They therefore use FTS5's
  defaults: `detail=full`, which stores token positions, and a normal content
  table that keeps its own copy of `body`. The memory cost that query-time
  highlighting needs is **already paid**. FTS5 documents the size cost of
  `detail=full` as 743 MiB, against 340 MiB for `detail=column` and 134 MiB for
  `detail=none`, on a 1636 MiB email corpus (<https://sqlite.org/fts5.html>).
- Rows are inserted without an explicit rowid
  (`fathomdb-engine/src/index_projector.rs:361`). The FTS rowid is therefore
  not known to equal `write_cursor`. A comment at `index_projector.rs:366`
  claims it does, but the insert does not enforce it, and step 11 re-creates
  the table (`fathomdb-schema/src/lib.rs:337-345`). The design must not rely
  on that equality.
- The bundled SQLite is 3.53.2 (repository root:
  `third_party/libsqlite3-sys-0.38.1/sqlite3/sqlite3.h:149`). It has the full
  `Fts5ExtensionApi`, including `xInstToken` (`sqlite3.h:13982`).
- A rowid-constrained MATCH seeks; it does not scan every match. This was
  verified in the SQLite source at review:
  - `fts5BestIndexMethod` records a rowid equality constraint (`bSeenEq`).
  - `fts5FilterMethod` passes it as `iFirstRowid`/`iLastRowid` to
    `sqlite3Fts5ExprFirst`, which advances each term's iterator with
    `sqlite3Fts5IterNextFrom`.

  The cost is one doclist seek per OR term. It grows with the number of query
  terms `k`, not with the number of matching rows.

### 3.2 What a match is

- `compile_text_query` lowercases the query, splits it on non-alphanumerics,
  drops tokens shorter than 3 characters and a fixed stopword list, and
  OR-joins the survivors as quoted FTS5 strings
  (`fathomdb-query/src/lib.rs:30-56`, `lib.rs:71-85`). When nothing survives,
  it falls back to OR-ed whitespace tokens (`lib.rs:74-79`).
- FTS5 then tokenizes each quoted string with the table's tokenizer: porter
  stemming over unicode61 with diacritic folding. A "match" is therefore a body
  token whose stem equals a query token's stem. The highlighter must use this
  definition, or it will mark the wrong words.
- The engine already reuses FTS5 as its tokenizer. `fts5_tokenize` writes into
  a temp FTS5 table with the same tokenizer and reads tokens back through
  `fts5vocab(..., 'instance')` (`fathomdb-engine/src/search.rs:565-596`). It
  returns terms and token positions, not byte offsets. It runs on the writer
  connection (`fathomdb-engine/src/search_api.rs:1262-1264`), not on a reader.
- Reader connections set `query_only = ON`
  (`fathomdb-engine/src/open.rs:1358-1360`). That rejects writes to the TEMP
  schema too, across the whole connection. This was verified in the SQLite
  source at review: `OP_Transaction` in `vdbe.c` returns `SQLITE_READONLY` for
  any write transaction under `query_only`, before it looks at the database
  index `p1`. **A temp-table approach therefore cannot run on the reader
  pool**, and the design uses only read-only statements.

### 3.3 Where hits are built, and when

- In hybrid search the text candidate query has no `LIMIT`. `limit_clause`
  is empty unless the direct text-only bound or a perf experiment sets it
  (`fathomdb-engine/src/search.rs:1740-1752`). The SQL is `body_fts_rank_sql`
  (`fathomdb-engine/src/filter.rs:690-706`). So any per-row snippet work done
  in the candidate query would run on every matching row, not on the hits
  returned.
- After fusion and rerank, the engine truncates to the final limit
  (`search.rs:2265`). It then resolves identity per final hit with one point
  query each, in the same reader transaction (`search.rs:2267-2286`). That is
  the precedent and the place for snippet work: post-truncation, per final
  hit, on the same snapshot.
- `SearchHit` carries `id`, `write_cursor`, `kind`, `body`, `score`,
  `branch`, `source_id` and `ce_score` (`fathomdb-engine/src/search_types.rs:76-99`).
  `body` is the full record body (`search_types.rs:95`), so the adapter
  already holds all the text a snippet needs. **What only the engine can
  supply is the match spans.**
- `SearchResult` carries `projection_cursor`, `soft_fallback`, `results` and an
  optional `explanation` sidecar (`search_types.rs:138-150`). It holds no match
  offsets. It is `#[non_exhaustive]` (`search_types.rs:137`). The SDK
  re-exports both types (`fathomdb-sdk/src/lib.rs:105`). Its options have no
  snippet setting (`fathomdb-sdk/src/options.rs:57-90`).
- Fusion keys on body (`fathomdb-engine/src/fusion.rs:101-105`). When the text
  arm matched a body the vector arm also returned, the fused hit keeps the
  vector hit's struct, and so its `branch` and `write_cursor`, because vector
  hits are accumulated first (`fusion.rs:128-137`). **`branch` therefore does
  not say whether a hit matched lexically.**
- The MCP adapter can ask for up to 100 ranked hits. `page_window` caps the run
  limit at `ranked_cap` (`fathomdb-mcp/src/shape.rs:79-81`), and `MAX_RANKED`
  is 100 (`fathomdb-mcp/src/tools.rs:36`). A continuation re-runs the search
  with `offset + limit` hits (`tools.rs:637`). So per-search snippet cost must
  be bounded at 100 hits, not only at 10.

### 3.4 The vector arm has no lexical anchor

- A hit with no row under the arm's MATCH contains none of the query's tokens
  under the store's tokenizer. Such a hit can only have come from the vector or
  graph arm.
- The default embedder truncates every input to 512 tokens
  (`fathomdb-embedder/src/candle_bge.rs:60-66`, `candle_bge.rs:558-568`), and
  the engine embeds the whole body with no chunking. 512 wordpieces is
  roughly 1.5–2 KB of English text. **The vector similarity of a long record
  is a similarity of its first ~512 wordpieces.** A snippet-sized prefix
  therefore lies inside the region the arm scored. The prefix is not the
  scored text itself, but it is an honest fallback, `anchor = Start`, rather
  than an arbitrary one.

### 3.5 Contract constraints

- MR-070.2: the adapter depends only on `fathomdb-sdk`, and never on
  `rusqlite` (`dev/design/mcp-adapter/requirements.md:533-534`). The adapter
  therefore cannot call FTS5.
- MR-040 caps snippets at 300 characters
  (`dev/design/mcp-adapter/requirements.md:359-366`). `dev/interfaces/mcp.md:45`
  and `mcp.md:127` fix `snippet_chars` and the hit fields.
- R-OBS-2 requires "no per-hit struct growth" for opt-in result data
  (`dev/design/0.8.8-explain-and-telemetry-adr.md:46`). The explain sidecar
  follows it.
- `ADR-0.8.25-eligibility-and-frozen-reads.md` forbids post-cap filtering.
  Snippets filter nothing; they decorate final hits. They also add nothing to
  the frozen context digest. `search_frozen` (`fathomdb-engine/src/search_api.rs:53`)
  must return the same results and digest with or without snippets (T-16).

## 4. Prior art

| System | Offsets come from | Compute per hit | Index or memory cost | Window selection |
| --- | --- | --- | --- | --- |
| SQLite FTS5 `snippet()` / `highlight()` | Stored token positions (`xInst`), mapped to bytes by re-tokenizing the stored text | Read the row text, plus one tokenizer pass per column considered | `detail=full` 743 MiB vs 134 MiB for `none` (docs' corpus); needs a content or external-content table | Up to 64 tokens; distinct phrases dominate (1000 for a first sighting, 1 for a repeat); sentence starts +100, column start +120; re-centred on the matched span |
| Lucene/ES unified | Postings with offsets, else term vectors, else re-analysis | Re-analysis bounded (Lucene `DEFAULT_MAX_LENGTH` 10 000 chars; ES `max_analyzed_offset` 1 000 000) | Postings offsets: "less disk space" than term vectors (not quantified) | Sentence `BreakIterator`; each passage scored as a mini-document with BM25 (k1 1.2, b 0.75) times a start-position prior (pivot 87) |
| ES plain | In-memory re-index of the text | Highest on large text | None | Count of unique query terms |
| ES fvh | Term vectors with positions and offsets | Low on large fields | `with_positions_offsets` "will double the size of a field's index" (term-vector docs) | Total matching terms times boost |
| Tantivy `SnippetGenerator` | Re-tokenizing the stored field | One tokenizer pass | Field must be stored | Greedy fragments of at most 150 bytes; score is the sum of `1/(1+df)` over matched terms; empty when nothing matches |

Sources:

- FTS5 documentation: <https://sqlite.org/fts5.html>.
- `ext/fts5/fts5_aux.c`, read from the GitHub mirror
  (<https://raw.githubusercontent.com/sqlite/sqlite/master/ext/fts5/fts5_aux.c>):
  - `fts5SnippetScore` scores a window and re-centres it:
    `iAdj = iFirst - (nToken - (iLast-iFirst))/2`.
  - `fts5SnippetFunction` reads the column text (`xColumnText`) and re-tokenizes
    it with a sentence-finder callback.
  - `fts5HighlightFunction` merges overlapping instances (`CInstIter`), maps
    token positions to byte offsets during a re-tokenization, and skips
    colocated (synonym) tokens.
  - Auxiliary functions run only inside a full-text query.
  - The instance APIs warn that they "can be quite slow" under
    `detail=column` or `detail=none`.
- Elasticsearch highlighting (highlighter types, offset strategies, defaults
  `fragment_size` 100, `number_of_fragments` 5, `boundary_scanner` `sentence`
  for unified):
  <https://www.elastic.co/guide/en/elasticsearch/reference/current/highlighting.html>.
- Elasticsearch term vectors, the source of "Setting `with_positions_offsets`
  will double the size of a field's index":
  <https://www.elastic.co/guide/en/elasticsearch/reference/current/term-vector.html>.
- Lucene `UnifiedHighlighter` javadoc (offset sources `POSTINGS`,
  `TERM_VECTORS`, `ANALYSIS`):
  <https://lucene.apache.org/core/9_11_0/highlighter/org/apache/lucene/search/uhighlight/UnifiedHighlighter.html>.
  `DEFAULT_MAX_LENGTH = 10000` is from the class source, not the javadoc:
  <https://github.com/apache/lucene/blob/main/lucene/highlighter/src/java/org/apache/lucene/search/uhighlight/UnifiedHighlighter.java>.
  `PassageScorer`:
  <https://lucene.apache.org/core/9_11_0/highlighter/org/apache/lucene/search/uhighlight/PassageScorer.html>.
- Tantivy `src/snippet/mod.rs`:
  <https://raw.githubusercontent.com/quickwit-oss/tantivy/main/src/snippet/mod.rs>.
  Its limit counts bytes, although the doc comment says characters.

On cost:

- Turpin et al., "Fast Generation of Result Snippets in Web Search" (SIGIR 2007,
  DOI 10.1145/1277741.1277766), found that "finding documents on secondary
  storage dominates the total cost of generating snippets". Processing a
  document already in RAM is small by comparison.
- No primary source gives per-hit microseconds for re-analysis against stored
  offsets. LUCENE-7438, which introduced the unified highlighter, names
  performance as its motive but publishes no results
  (<https://issues.apache.org/jira/browse/LUCENE-7438>). The only numbers found
  are anecdotal and include search time
  (<https://peterbe.com/plog/benchmark-comparison-of-elasticsearch-highlighters>).

Lesson for FathomDB: every system above finds byte offsets by re-tokenizing
text it already holds. Stored offsets (term vectors) buy speed on very large
fields at about twice the field's index size. FathomDB records are small,
LoCoMo turns being typically under 300 characters, and their text is already in
the FTS content table. So re-tokenization is the right default.

## 5. Options, with the compute-versus-memory trade-off

Costs marked *est.* are not measured. § 9 specifies the measurements.

### (a) FTS5 `highlight()` on the live index, only where needed (recommended)

For each final hit that has a row under the arm's MATCH, **and** whose body is
longer than `max_bytes` or whose caller asked for highlights (§ 6.3, rule 0),
run one point query on the same reader transaction:

```sql
SELECT highlight(search_index, 0, :open, :close)
FROM search_index
WHERE search_index MATCH :match AND rowid = :fts_rowid
```

Then derive byte spans in Rust by parsing out the markers (§ 6.1).

- **Offsets:** stored positions, plus FTS5 re-tokenizing the stored body with
  the table's own tokenizer and the same MATCH expression. Tokenizer and query
  parity hold by construction.
- **Compute per point query, est.:**
  - one rowid seek per OR term, `k` seeks in all (verified, § 3.1);
  - one content-row read, whose page was read moments earlier when the
    candidate was fetched, so it is likely still in the page cache;
  - one porter+unicode61 pass over the body. For bodies up to a few KiB that
    should be tens of microseconds.
- **How many point queries:** only for hits longer than the budget, unless
  highlights are requested. About 92% of LoCoMo turns fit in 300 characters
  (§ 1), so on the evaluated stores this skips about nine point queries in ten.
  The worst case is 100 hits, all long, with `k` up to the query's content
  token count (§ 3.3). § 7.2 bounds it with a deterministic per-search cap.
- **Memory:** no new index bytes. It needs the FTS rowid of each final hit.
  The candidate SELECTs (`filter.rs:697`, `search.rs:1855`, `search.rs:1881`,
  and the fallback forms at `search.rs:1971` and `search.rs:1997`)
  already read the row and would also select `search_index.rowid`. That rowid
  goes into a search-local map keyed by `write_cursor`, not into `SearchHit`,
  following R-OBS-2.
- **Gaps:**
  - A hit missing from the map has no lexical anchor and gets the prefix
    (§ 3.4).
  - The residual dedup gap: two records with identical bodies and different
    `write_cursor`s collapse in fusion (`fusion.rs:101-105`). If the surviving
    struct is the vector arm's, its `write_cursor` may not be in the map even
    though its body matched, so it gets `Start`. A second map keyed by body
    hash, mirroring fusion's own keying, would close this. The case is rare,
    and the identical text makes the prefix no worse than today.
  - Distinct-term identity is approximate (§ 6.3, rule 3).

Variant (a′): register a FathomDB FTS5 auxiliary function that returns
`(phrase index, start byte, end byte)` triples through `xInst` and
`xTokenize`.

- It runs the **same** point query, so it does not reduce cost. It changes only
  the precision of term identity, and it removes the marker parse.
- It needs the `fts5_api` pointer, obtained with `SELECT fts5(?1)` and
  `sqlite3_bind_pointer`, which means `unsafe` FFI through `libsqlite3-sys`. I
  did not check whether rusqlite 0.40 exposes this safely.
- It is a later option for exact identity (§ 10, Q4), behind the cheaper
  per-term upgrade in § 6.3, rule 3.

Variant (a″), FTS5 `snippet()` itself, is rejected as the output format:

- Its window is a token count capped at 64, not a byte budget.
- It returns one fragment.
- Its ellipsis and markers are inline text.
- It does not cut on grapheme boundaries.

Its scoring is still a good model, and § 6.3 borrows it.

### (b) Re-tokenize the body in Rust

- **Compute:** likely the fastest per hit, since there is no SQL round trip.
- **Memory:** none.
- **Parity risk: high.** A Rust tokenizer must reproduce three things exactly:
  - SQLite's `porter` stemmer, the original Porter algorithm, which is not the
    Snowball "English" (Porter2) stemmer most Rust crates ship;
  - unicode61's Unicode 6.1 separator and case-folding tables;
  - `remove_diacritics 2`.
- Any mismatch fails silently. A stemmed variant would match in MATCH but not
  be found, or the reverse, so the snippet would centre on the wrong text.
- It also cannot live in the adapter. The adapter does not know the store's
  tokenizer (MR-070.2, § 3.5), and the tokenizer is schema state
  (`fathomdb-schema/src/lib.rs:344`).
- **Rejected** as the source of spans. The *window selection* that follows the
  spans is pure Rust and needs no tokenizer; § 7.2 exposes it as a function.

### (c) Store byte offsets

- FTS5 stores no byte offsets at any `detail` level, only token positions.
  `detail=full` is already the most it stores (§ 3.1).
- Storing offsets would mean a new projection table: term to
  `(write_cursor, start, end)`, the equivalent of term vectors. That is a
  schema bump, write amplification on every projection, and on Elasticsearch's
  evidence about twice the field's index size.
- It saves only a tokenizer pass over a body that is already in cache.
  Turpin et al. show that the expensive part, reading the text, is already paid.
- **Rejected,** unless M1 shows the tokenizer pass dominating on very long
  records. Even then, raise the long-record question first (§ 10, Q5).

### (d) Sentence and passage segmentation and scoring

- **Lexical (adopted inside (a)):** once (a) gives match spans, scoring
  windows that snap to sentence starts, as Lucene's `BreakIterator` and FTS5's
  sentence bonus do, is pure Rust over a short body. It costs microseconds and
  no memory. § 6.3 specifies it.
- **Semantic, for hits with no lexical anchor (deferred, measured first):**
  - Split the body into sentences (UAX #29), embed each, and centre on the
    sentence nearest the query vector.
  - Cost: one embedder batch per hit. For 10 hits of ~10 sentences, that is
    ~100 short embeddings per search on the reader path. On CPU this is likely
    tens to hundreds of milliseconds (*est.*), comparable to or larger than the
    search itself. It also needs the embedder on the read path, which text mode
    never uses.
  - Memory: none persistent. Storing per-sentence vectors would be chunked
    embeddings, a different and larger feature.
  - Because the embedding only saw the first 512 tokens (§ 3.4), the best
    sentence by embedding is not the region that made the hit rank. It would be
    a new relevance judgement, not an explanation of the existing one.
  - **Deferred** to experiment M3.

## 6. Text handling

### 6.1 Getting spans from `highlight()`

- **Markers, chosen per call.** The engine picks the first pair from this
  ordered list whose two code points are both absent from the body:
  1. U+E000 / U+E001 (private use);
  2. U+F8FE / U+F8FF (private use);
  3. U+FDD0 / U+FDD1 (noncharacters).

  Private-use code points occur in real agent memory: Nerd Fonts and Powerline
  glyphs appear in terminal and log text. That is why there is a list rather
  than one pair. The markers are bound as parameters. If all three pairs occur
  in the body, highlighting is skipped and the hit gets the prefix with
  `anchor = Start`. This is deterministic and very rare.
- **Parse once, left to right.** Each marker's byte position, minus the bytes
  of the markers before it, gives a span in the original body.
- **Check that stripping the markers reproduces the body byte for byte.** If
  it does not, fall back to the prefix as above. This guards against tokenizer
  surprises.
- FTS5 already merges overlapping instances (`CInstIter`), so spans come out
  sorted and disjoint.
- **Spans are code-point boundaries, not always grapheme boundaries.** They
  are byte offsets that unicode61 produced at code-point boundaries, so they
  are valid UTF-8 slice points. But unicode61 treats combining marks as
  separators. In an NFD body, `cafe` + U+0301 therefore gives a token that ends
  before the accent, inside the grapheme cluster `é`. The selector widens every
  span end to the next grapheme boundary before it scores or emits highlights
  (T-3 has an NFD case).

### 6.2 Budget in bytes

- The engine budget is `max_bytes` of UTF-8 output, ellipses included. The
  adapter's estimator is bytes ÷ 4 (`fathomdb-mcp/src/shape.rs:22-26`). A
  character cap charges CJK text three times as much as ASCII; a byte cap
  charges what the budget charges.
- The MCP setting `snippet_chars` becomes `snippet_bytes`. Pre-1.0, there is
  no alias (AGENTS.md § 10). The default is an owner question (§ 10, Q1).

### 6.3 Choosing windows (deterministic)

The selector is a pure function: `snippet_for(body, spans, options) ->
SnippetV1` (§ 7.2). Its inputs are the body, its spans `[s, e)`, `max_bytes`,
`max_windows` (default 1) and `head_bytes` (default 0, § 6.6).

0. **When spans are needed.** The engine fetches spans only when
   `body.len() > max_bytes` or `highlights` is true. Otherwise rule 1 applies
   with no point query, and `spans` is empty.
1. **Whole body.** If `body.len() ≤ max_bytes`, return the body, with every
   span if spans were fetched, and `cut = false`. Every record that fits today
   returns the same text as today.
2. **No spans.** Return the prefix, cut back to a UAX #29 word boundary (or a
   grapheme boundary, as in rule 6) within `max_bytes − len(" …")`, plus a
   trailing `" …"`. Set `anchor = Start`.
3. **Span identity.** By default, a span's identity is its text after Rust's
   Unicode lowercasing. This is an approximation: `running` and `runs` share a
   porter stem but count as two terms.

   **Exact identity, first upgrade (no `unsafe`).** Run `highlight()` once per
   OR term, with a MATCH of that one term, and label each span with its term.
   Do this only when all of the following hold:
   - `body.len() > max_bytes`;
   - the query has `k > 1` terms;
   - the first window, chosen by approximate identity, does not cover every
     span.

   It costs `k` extra point queries for exactly the hits where identity
   changes the choice, drawn from a per-search pool (§ 6.5). Variant (a′) is
   a later option.

   *As built (S5b):* the third condition is checked as "some span lies
   outside every window of the approximate snippet". It is not checked
   against `windows[0]` alone, for two reasons:
   - With a record head (`head_bytes > 0`, the MCP default), `windows[0]` is
     the head, so a `windows[0]` check would run the per-term pass on almost
     every long hit.
   - When every span is already shown (in the head, the match window, or a
     rule 7 window), exact identity cannot show more terms.

   With `max_windows = 1` and no head, the two readings are the same.
4. **First window.** For each span start, consider the window of width
   `W = max_bytes − ellipsis allowance` that begins there. Score it, in this
   lexicographic order:
   1. the number of distinct identities fully inside;
   2. the number of spans inside;
   3. a sentence-start bonus, when a UAX #29 sentence start lies within 64
      bytes before the first span;
   4. the earlier start.

   This mirrors FTS5's 1000-to-1 distinct-term weighting and its sentence
   bonus. It does not use floating point, so it is exactly reproducible.
5. **Centre.** Shift the window so the covered spans sit in the middle:
   `start = first_s − (W − (last_e − first_s)) / 2`, clamped to the body. This
   is FTS5's `iAdj` rule, applied in bytes.
6. **Snap.**
   - The start moves back to the sentence start found in step 4.3 if the
     budget still covers the last span. Otherwise it moves to the nearest
     UAX #29 word boundary at or before the first counted span.
   - The end moves back to a word boundary, but never before the end of the
     last counted span.
   - **Every span counted in step 4 must still lie inside the window after
     snapping.** UAX #29 words and unicode61 tokens differ: `don't` is one
     UAX #29 word but two unicode61 tokens, and `e-mail` is the reverse case.
     So a token can start in the middle of a UAX #29 word. Where snapping to a
     word boundary would cut a counted span, the edge moves to the nearest
     grapheme boundary outside the span instead (T-18).
   - UAX #29 word boundaries never fall inside a grapheme cluster, so word
     edges are grapheme boundaries. A window never splits a combining
     sequence, an emoji ZWJ sequence or a flag pair.
   - If no word boundary falls within 32 bytes, the edge moves to the nearest
     grapheme boundary instead (`unicode-segmentation`, already in
     `Cargo.lock` at 1.13.2).
7. **Second window (only when `max_windows > 1`).** If the remaining budget is
   at least 80 bytes, and some spans have identities outside the first window,
   repeat steps 4–6 over those spans with the remaining budget. The default is
   1. A second window ships only if M2 shows a gain.

   *As built (S5a):* a full-width first window would leave less than 80
   bytes, so the split is decided up front. When identities lie outside the
   full-width first window, that window is re-chosen at `full / max_windows`
   bytes, but only if at least 80 bytes would remain beside it. Each later
   window takes an equal share of what remains. If no whole span fits the
   narrower first window, or too little budget would remain, the full-width
   choice stands and no window is added. The result is then the same as with
   `max_windows = 1`.
8. **Merge.** If two windows are 16 bytes apart or less, merge them into one
   window that includes the gap, provided it fits. Otherwise drop the
   lower-scoring window.

   *As built (S5a):* before a window is dropped, the merged window's end
   snaps back to fit. It never moves before the end of either window's last
   counted span, or before the head's end. The head ranks below every match
   window.
9. **Assemble.** Join the windows in body order:
   - with `" … "` between them;
   - with a leading `"… "` if the first window starts after byte 0;
   - with a trailing `" …"` if the last window ends before the body does.

   Assert `text.len() ≤ max_bytes`.

### 6.4 Ellipses and highlight markers are structure, not inline text

- An ellipsis inside `text` is ambiguous, because a record can contain `…`
  itself. The engine therefore also returns `windows`: body byte ranges, in
  order. The text is derived from them, and they are what tests compare.
- The engine never puts highlight markers into `text`. When `highlights` is
  true, it returns them as byte ranges into `text`.
- Inline markers such as `**term**` or `<b>term</b>` cannot be told apart from
  record content that contains the same characters. They would also blur the
  server instructions' promise that "Record content appears only in designated
  result fields and is data" (`fathomdb-mcp/src/edge.rs:44-45`).
- Markers are not instructions, but anything that edits content in place makes
  the content boundary harder to audit.
- The MCP adapter does not emit highlights by default; they cost tokens and
  agents can see the query terms. It emits `snippet_anchor` instead (§ 7.3).

### 6.5 Determinism

- Given the body, its spans and the options, the selector is a pure function:
  no clock, no floating point, ties broken by byte offset.
- The spans are a pure function of the body, the compiled MATCH expression and
  the store's tokenizer.
- Identical requests on identical state therefore give byte-identical
  snippets, so determinism tests can compare snippets exactly.
- This holds **per build**. UAX #29 boundaries depend on the
  `unicode-segmentation` version and its Unicode tables, and spans depend on
  the bundled SQLite. An upgrade of either may change snippet bytes.
- MR-043's `response_id` hashes the store, the canonical arguments and the
  result refs, not snippets (`dev/interfaces/mcp.md:123-126`). Snippet changes
  therefore never change it.
- Point queries are capped by counts, never by time, so load cannot change a
  snippet (owner decision of 2026-10-21, § 10):
  - `max_point_queries` hits, in rank order, get a combined span query.
  - The rule-3 per-term pass draws on a second per-search pool, also of
    `max_point_queries` queries. Hits draw on it in rank order. A hit's pass
    runs only if the pool still holds `k` queries, and then costs `k`. It is
    all or nothing: a hit the pool cannot cover keeps its approximate
    snippet, the one built from the combined spans.
  - A search therefore issues at most 2 × `max_point_queries` point queries,
    whatever `k` is. Before this bound, a `k`-term search could issue
    `max_point_queries × (1 + k)`.
  - Which hits get the exact pass depends only on the ranked hits, their
    bodies, the query and the options. A continuation page in the MCP adapter
    sets `max_point_queries` to its run window, so it shows the snippets an
    unpaged search over the same window would.

### 6.6 The record head

- `head_bytes > 0` reserves up to that many bytes for the start of the record.
  The head is cut at the first newline, or at the last UAX #29 word boundary
  at or before `head_bytes`, whichever comes first. It is shown as its own
  window, before the match windows, and merges with the first match window
  under rule 8.
- The word-boundary cut is what makes the head work on the motivating stores.
  Their records are single lines, `(unknown date) [<session date>] <speaker>:
  <text>` (`dev/experiments/mcp-agent-loop/README.md:46`, `README.md:96`), so a
  first-line rule would never fire.
- The default is 0, because the engine cannot know a store's conventions.
  Whether the MCP adapter should set it is an owner question (§ 10, Q2).

## 7. Recommended design

### 7.1 Where it lives

The design lives in the **engine, exposed through the SDK, with the adapter as
a thin consumer.** The reasons:

- Only the engine can run `highlight()` on the live index, on the same
  snapshot (§ 3.3). The adapter may not link rusqlite (MR-070.2).
- The rest is a pure function the SDK exports. The adapter can therefore
  re-run selection under its own budget (§ 7.3), and SDK users can reuse it.
- Python and TypeScript SDK users get the same snippets as MCP agents.
- Engine proposal: [EP-17](../mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md).

### 7.2 Engine and SDK surface (EP-17)

- `SearchOptions.snippet: Option<SnippetOptions>` and the same field on
  `TextSearchOptions`.
  - `SnippetOptions { max_bytes: u32 (40..=4000), max_windows: u8 (1..=3,
    default 1), head_bytes: u32 (0..=max_bytes / 2), highlights: bool
    (default false), max_point_queries: u8 (default 20) }`.
  - Values out of range are `InvalidArgument`. `SnippetOptions::validate()`
    is the rejecting check, and search entry points call it. `snippet_for`
    returns no `Result`; it clamps out-of-range values into range.
  - `max_point_queries` is the per-search snippet budget: the number of hits
    that get a combined span query. Hits are processed in rank order. Once
    the cap is reached, the remaining long hits get the prefix with
    `anchor = Start`. A hit cap is used rather than a time cap, because a
    time cap would make the output depend on machine load and break § 6.5.
    M1 sets the default.
  - The rule-3 per-term pass draws on a second per-search pool of
    `max_point_queries` queries, all or nothing per hit (§ 6.5). A search
    issues at most 2 × `max_point_queries` point queries.
- `SearchResult.snippets: Option<Vec<SnippetV1>>`.
  - It is a sidecar parallel to `results`, the same length and in the same
    order. `None` when the option is off.
  - With the option off, every existing field of `SearchResult` is equal to
    today's, and no per-hit allocation happens (R-OBS-2).
- `SnippetV1 { text: String, anchor: SnippetAnchor, windows: Vec<(u32, u32)>,
  spans: Vec<(u32, u32)>, highlights: Vec<(u32, u32)>, cut: bool }`.
  - `SnippetAnchor` is `Whole`, `Match` or `Start`.
  - `spans` are the body-relative match spans the engine fetched. They are
    empty when rule 0 skipped the point query.
  - `highlights` is filled only when `SnippetOptions.highlights` is true.
- **Pure selector:** `fathomdb_sdk::snippet_for(body: &str, spans: &[(u32,
  u32)], options: &SnippetOptions) -> SnippetV1`. It is the same function the
  engine calls, and it needs no database.
- **Anchor rule.** `anchor = Match` if and only if the search-local
  `(table, rowid)` map contains the hit's `write_cursor` and highlighting
  produced at least one span. `branch` is not consulted (§ 3.3).
  - Node text rows use `search_index`.
  - Edge rows use `search_index_edges` (`fathomdb-engine/src/filter.rs:708-715`).
  - Hits not in the map, including vector-only and graph-arm hits, get `Start`.
- The snippet computation runs after `results.truncate(final_limit)` and the
  identity pass (`search.rs:2265-2286`), in the same reader transaction, with
  read-only statements only (§ 3.2).
- Frozen reads: snippet options are presentation. They are not eligibility,
  and they are not in the context digest.
- `search_projected_text` is left out at first; its hits come from
  `property_search_index` (`fathomdb-schema/src/lib.rs:930`).

### 7.3 Adapter changes (after EP-17 lands)

- The search tool sets `snippet: Some(SnippetOptions { max_bytes:
  snippet_bytes, max_windows: 1, head_bytes, highlights: false, .. })` and
  uses `snippets[i].text`. It deletes `shape::snippet` as the snippet source.
- New hit field `snippet_anchor`: `"whole" | "match" | "start"`. `snippet_cut`
  keeps its meaning, true when any of the body is left out, and its source
  becomes `snippets[i].cut`. `dev/interfaces/mcp.md:127` gets updated.
- **Budget trimming re-runs selection.** `cut_bodies` (`shape.rs:174-198`)
  calls `snippet_for(body, spans, options)` with a smaller `max_bytes`, instead
  of cutting a prefix. The result is deterministic and keeps the match.
  - A hit whose body fitted, so rule 0 fetched no spans, has none. If budget
    trimming must cut it, it is trimmed as a prefix, with `snippet_anchor =
    "start"`.
  - This case needs a long page under a small budget. The adapter can avoid it
    by setting `highlights: true`, at the point-query cost M1 measures.

  *Correction (S5c):* this bullet rested on a wrong premise. The search
  budget loop never cuts snippets: it drops whole hits from the end of the
  page (`shape_search_page`, `kept -= 1`). `cut_bodies` serves only `fetch`
  and `expand`, which cut full record bodies with no query and no spans. So
  no snippet is trimmed after selection, and nothing re-runs `snippet_for`.
  The as-built adapter takes `snippets[i]` as returned. MR-041.1 holds
  because the loop drops hits, not because snippets shrink.
- The server instruction at `edge.rs:38-39` changes. Snippets are centred on
  the matched words when `snippet_anchor` is `match`, and are the record start
  when it is `start`. `fetch` stays the way to read the rest.
- Config: `snippet_chars` becomes `snippet_bytes`
  (`config.rs:91`, `config.rs:150`, `config.rs:192`, `config.rs:254-255`).
  MR-040's "300 characters" becomes a byte cap, which needs a requirements
  edit (`requirements.md:364`).
- **The hints design needs edits** (`dev/design/agent-access/hints-sidecar.md`
  on branch `exp/hints-design`, read at `7c0642d4e`; the line numbers the
  reviewer cited, 206, 265-273 and 357, belong to a different revision):
  - `hints-sidecar.md:222` says snippets are adapter presentation cut at
    `snippet_chars`. The source of `snippet_cut` becomes `snippets[i].cut`, and
    the cap becomes `snippet_bytes`.
  - `hints-sidecar.md:301` plans from the `snippet_cut` flags at `tools.rs:709`.
    It should read the same flag from `snippets[i].cut`.
  - `hints-sidecar.md:419` gives the example hint text "snippets are prefixes".
    It should be keyed on `snippet_anchor`, for example "snippet centred on the
    match" for `match` and "snippet is the record start" for `start`.
  - `hints-sidecar.md:647-648` states that snippet hints stay valid. That
    remains true, because `body_cut` planning still works on `cut` flags, and
    re-running `snippet_for` keeps those flags deterministic.

### 7.4 Public-surface impact

- **Rust:** new option fields, a new sidecar and the `snippet_for` function in
  `fathomdb-engine` and `fathomdb-sdk`, recorded in `dev/interfaces/rust.md`
  and `dev/interfaces/rust-sdk.md`.
  - `SearchResult` is `#[non_exhaustive]` (`search_types.rs:137`), so adding
    `snippets` breaks no external code.
  - `SearchOptions` and `TextSearchOptions` are plain structs with public
    fields (`fathomdb-sdk/src/options.rs:57-90`). External struct literals
    that do not use `..Default::default()` stop compiling. Pre-1.0 this is
    allowed.
- **Python and TypeScript:** the same options, sidecar and selector
  (`dev/interfaces/python.md`, `dev/interfaces/typescript.md`), plus the
  parity map entries.
- **MCP:** `snippet_bytes` and `snippet_anchor` (`dev/interfaces/mcp.md`), the
  MR-040 wording, and the instructions text.
- **No schema change and no migration.**

## 8. RED-test plan (later, under TDD; each its own failing commit)

Each engine test runs on a real database (AGENTS.md § 10). "Pure" tests call
`snippet_for` directly.

| # | Test | Layer |
| --- | --- | --- |
| T-1 | A 4 KiB record whose only query term is at byte 3000. `snippet.anchor == Match`, the text contains the term, and `text.len() ≤ max_bytes`. Today's prefix would miss it. | engine |
| T-2 | Porter parity: query `running` against a body containing `runs` far into the record. The window covers `runs`. | engine |
| T-3 | Diacritics: query `cafe` against `Café` beyond the prefix, in both NFC and NFD forms. The highlight covers `Café`'s bytes exactly, including the combining accent in the NFD form (§ 6.1). | engine |
| T-4 | Property (`proptest`) over arbitrary Unicode bodies and span sets. Windows are sorted, disjoint and on grapheme boundaries, and `text` is their ellipsis join. Every highlight lies inside `text`. Length is at most `max_bytes`. The whole body is returned unchanged when it fits. | pure |
| T-5 | Grapheme safety: emoji ZWJ sequences, flags and combining marks at every window edge. No edge splits a cluster. | pure |
| T-6 | No lexical anchor: a hybrid hit found only by the vector arm, whose body lexically matches none of the query terms, gives `anchor == Start` and the prefix. A hit whose fused struct has `branch == Vector` but whose body does match gives `anchor == Match` (§ 3.3). | engine |
| T-7 | Off by default: with `snippet: None`, every field of `SearchResult` is equal to a pinned baseline, field by field, and `snippets.is_none()`. No sidecar is allocated. | engine |
| T-8 | Determinism: the same search twice gives byte-identical `snippets`. | SDK |
| T-9 | Reader-pool safety: snippet work runs on a `query_only` reader with no writer lock, and issues no write statement. `query_only` rejects TEMP writes connection-wide (verified, § 3.2), so any temp-table write would fail this test. | engine |
| T-10 | Marker selection: a body containing U+E000 uses U+F8FE/U+F8FF and still gives `anchor == Match`. A body containing all three pairs gives `anchor == Start`, with no garbled text. | engine |
| T-11 | Two windows, with `max_windows = 2`: two distinct terms 2 KiB apart give two windows joined by `" … "`. Terms 10 bytes apart give one merged window. | pure |
| T-12 | Python and TypeScript parity for the options, the sidecar and `snippet_for`. | bindings |
| T-13 | MCP: `snippet_anchor` and centred text appear. `cut_bodies` re-runs `snippet_for` and keeps the matched term when it trims. The budget property MR-041.1 still holds. | adapter |
| T-14 | `head_bytes` on a single-line record `(unknown date) [8 May 2023] Caroline: …` with the match at byte 1500. The head window ends on a word boundary at or before `head_bytes`, and the match window follows. | engine |
| T-15 | Edge hits: an edge body matched through `search_index_edges` gives `anchor == Match`, with spans from that table. | engine |
| T-16 | Frozen reads: `search_frozen` with `snippet: Some(..)` returns the same results and the same context digest as with `snippet: None`. | engine |
| T-17 | Window scoring: one window holding two distinct terms beats one holding the same term three times. A sentence start within 64 bytes before the first span moves the window start to it. | pure |
| T-18 | Snapping keeps counted spans: bodies with `don't`, `e-mail` and similar forms at window edges. Every span counted in rule 4 is still inside the window after rule 6. | pure |
| T-19 | Rule 0: a body that fits, with `highlights: false`, issues no point query, and `spans` is empty. | engine |
| T-20 | Point-query cap: 30 long hits with `max_point_queries = 20`. Hits 21–30 in rank order get `anchor == Start`, with the same result on repeat. | engine |

## 9. Measurements that settle what is estimated

**M1, cost.** Settles whether MCP turns snippets on by default, and the default
for `max_point_queries`.

- Stores:
  - the LoCoMo conversation 0 store;
  - a long-record store built from the same turns, concatenated into records
    of 1, 4, 16 and 64 KiB.
- Queries: LoCoMo questions, in text and hybrid modes, chosen so that the
  compiled MATCH has `k` ∈ {1, 5, 10} OR terms.
- Grid: limit ∈ {10, 100} × `k` ∈ {1, 5, 10} × both stores × `highlights` ∈
  {false, true}.
- Measure:
  - search latency p50 and p95 with snippets off and on;
  - the number of point queries per search;
  - per-point-query microseconds by body size and `k`.
- **Decision rule at limit 10:** default snippets on in MCP if the added p95 is
  at most the larger of 1 ms and 5% of search p95.
- **Decision rule at limit 100:** the same bound, but at 2 ms. If it fails, set
  `max_point_queries` to the largest value that passes at `k = 10` on the
  long-record store, and anchor the rest at `Start`.
- If even limit 10 fails, the fallback is a lower `max_point_queries`. The
  rule-0 skip is already in. Variant (a′) is **not** a cost fallback, because
  it runs the same point query.

**M2, quality.** Settles adoption and the defaults for `max_bytes` and
`max_windows`.

- For each gold-evidence record returned, test whether the gold evidence text
  lies inside the snippet. Compare prefix against centred, at 300 and 600
  bytes, with `max_windows` 1 and 2, on both stores.
- **Decision rule:** adopt if coverage on the long-record store rises, and the
  short-record store is unchanged. It should be unchanged by construction,
  because of rule 1. Raise `max_windows` to 2 only if it adds at least 3
  points of coverage at the chosen `max_bytes`. Answer-level effects belong to
  work items 4 and 5.

**M3, hits with no lexical anchor.** Settles whether option (d)'s semantic
variant is worth an experiment.

- Measure the share of final hybrid hits with `anchor == Start`, and their
  evidence coverage under M2.
- Run the sentence-embedding experiment only if that share is above 20% and
  their coverage is below the `Match` hits' coverage by more than 10 points.

## 10. Open questions for the owner

**Owner decisions, 2026-10-18.**

1. `snippet_bytes`: keep 300 until M2 decides.
2. Record head: MCP keeps a head, snapped to a word boundary at or before
   `head_bytes`.
3. Highlights: off by default, opt-in argument.
4. Term identity: the reviewer's exact answer (one `highlight()` per OR term,
   only under the stated conditions); the auxiliary function stays a later
   option.
5. Chunked embedding: added to `dev/ROADMAP.md` under "Requested". Until it
   ships, vector-only hits keep the record-start fallback.

**2026-10-21 owner (Block 1 close):** bound the per-term pass to a second
pool of `max_point_queries`; at most 2 × point queries. `max_point_queries`
keeps its meaning and range, and the MCP setting is unchanged. A hit whose
per-term pass the pool cannot cover keeps its approximate snippet (§ 6.5).
The receipt is `dev/experiments/agent-access-snippet-cost/`.

The questions as originally put, with the Fable-medium reviewer's view:

1. **Default `snippet_bytes`.** Keep 300, which is comparable with past runs,
   or raise it to 600 once windows are centred? M2 gives the evidence.
   - *Reviewer's view:* none recorded beyond letting M2 decide.
2. **Record head.** Should MCP set `head_bytes` (for example, 48) so the
   session-date header survives centring, or should stores be expected to put
   dates in metadata?
   - *Reviewer's view:* if a head is kept, it must snap to a word boundary at
     or before `head_bytes`. A first-line rule never fires on the single-line
     LoCoMo records (§ 6.6).
3. **Highlights in MCP.** Emit highlight offsets to agents at all? The
   proposal is no by default, with an opt-in argument.
   - *Reviewer's view:* default off, as `SnippetOptions.highlights = false`.
     Turning them on also costs a point query on every hit that fits (rule 0).
4. **Exact term identity.** Is the lowercased-surface approximation (rule 3)
   acceptable?
   - *Reviewer's view:* the cheap exact answer comes first: one `highlight()`
     per OR term, only when the body is over budget, `k > 1`, and the first
     window misses some spans. It needs no `unsafe`. The auxiliary function
     (a′) is a later option and changes precision only, not cost.
5. **Long records generally.** The vector arm sees only the first 512 tokens
   (§ 3.4). Is chunked embedding on the roadmap? If so, the vector arm would
   gain a natural anchor, the matched chunk, and M3 becomes moot.
   - *Reviewer's view:* none recorded. The review only reworded § 3.4: the
     prefix lies inside the scored region but is not the scored text.

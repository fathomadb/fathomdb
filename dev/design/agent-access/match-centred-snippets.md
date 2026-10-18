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
  - src/rust/crates/fathomdb-engine/src/{search,search_types,filter,index_projector,open,fusion}.rs
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
- The evaluated stores put the session date at the start of each turn
  (`dev/experiments/mcp-agent-loop/README.md:304-305`). A prefix spends part of
  its budget on that header. A centred window may drop it, and temporal
  questions need it. § 6.6 handles this.

The goal is a snippet centred on the passage that matched. It should handle
text precisely, and its compute and memory cost should be explicit.

## 2. Terminology: what this is called

**Verdict: "rewrite engine" is the wrong name.** The feature is
**query-biased (query-dependent) snippet generation**, also called **dynamic
summaries**. Search libraries call the mechanism **hit highlighting**, and the
classic display form is **keyword-in-context (KWIC)**. It is extractive: it
selects spans of the stored text and does not rewrite anything.

- Tombros and Sanderson introduced *query-biased summaries* ("Advantages of
  query biased summaries in information retrieval", SIGIR 1998,
  <https://marksanderson.org/publications/my_papers/SIGIR98.pdf>). In their
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
  returns terms and token positions, not byte offsets. It also runs on the
  writer connection (`fathomdb-engine/src/search_api.rs:1262-1264`), not on a
  reader. Reader connections set `query_only = ON`
  (`fathomdb-engine/src/open.rs:1358-1360`). I believe `query_only` also
  rejects writes to the temp schema, so a temp-table approach cannot run on the
  reader pool. That is unverified; RED test T-9 settles it.

### 3.3 Where hits are built, and when

- In hybrid search the text candidate query has no `LIMIT`. `limit_clause`
  is empty unless the direct text-only bound or a perf experiment sets it
  (`fathomdb-engine/src/search.rs:1740-1752`). The SQL is `body_fts_rank_sql`
  (`fathomdb-engine/src/filter.rs:690-706`). So any per-row snippet work done
  in the candidate query would run on every matching row, not on the ten
  returned.
- After fusion and rerank, the engine truncates to the final limit
  (`search.rs:2265`). It then resolves identity per final hit with one point
  query each, in the same reader transaction (`search.rs:2267-2286`). That is
  the precedent and the place for snippet work: post-truncation, per final
  hit, on the same snapshot.
- `SearchHit` carries `id`, `write_cursor`, `kind`, `body`, `score`,
  `branch`, `source_id` and `ce_score` (`fathomdb-engine/src/search_types.rs:76-99`).
  `SearchResult` carries `projection_cursor`, `soft_fallback`, `results` and an
  optional `explanation` sidecar (`search_types.rs:138-150`). It holds no match
  offsets. The SDK re-exports both types (`fathomdb-sdk/src/lib.rs:105`), and
  its options have no snippet setting (`fathomdb-sdk/src/options.rs:57-90`).
- Hybrid fusion keys on body (`fathomdb-engine/src/fusion.rs:101-105`).

### 3.4 The vector arm has no lexical anchor

- A vector-only hit contains none of the query's tokens under the store's
  tokenizer, except when the text candidate set was capped (§ 3.3).
  Otherwise it would also be a text candidate.
- The default embedder truncates every input to 512 tokens
  (`fathomdb-embedder/src/candle_bge.rs:60-66`, `candle_bge.rs:558-568`), and
  the engine embeds the whole body with no chunking. **The vector similarity of
  a long record is a similarity of its first ~512 wordpieces.** For a
  vector-only hit, the prefix is the text the arm actually scored. That makes
  the prefix an honest default, not just a fallback.

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
  the frozen context digest.

## 4. Prior art

| System | Offsets come from | Compute per hit | Index or memory cost | Window selection |
| --- | --- | --- | --- | --- |
| SQLite FTS5 `snippet()` / `highlight()` | Stored token positions (`xInst`), mapped to bytes by re-tokenizing the stored text | Read the row text, plus one tokenizer pass per column considered | `detail=full` 743 MiB vs 134 MiB for `none` (docs' corpus); needs a content or external-content table | Up to 64 tokens; distinct phrases dominate (1000 for a first sighting, 1 for a repeat); sentence starts +100, column start +120; re-centred on the matched span |
| Lucene/ES unified | Postings with offsets, else term vectors, else re-analysis | Re-analysis bounded (Lucene 10 000 chars; ES `max_analyzed_offset` 1 000 000) | Postings offsets: "less disk space" than term vectors (not quantified) | Sentence `BreakIterator`; each passage scored as a mini-document with BM25 (k1 1.2, b 0.75) times a start-position prior (pivot 87) |
| ES plain | In-memory re-index of the text | Highest on large text | None | Count of unique query terms |
| ES fvh | Term vectors with positions and offsets | Low on large fields | "will double the size of a field's index" | Total matching terms times boost |
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
- Elasticsearch highlighting:
  <https://www.elastic.co/guide/en/elasticsearch/reference/current/highlighting.html>.
  Term-vector size: <https://www.elastic.co/guide/en/elasticsearch/reference/current/term-vector.html>.
  Defaults: `fragment_size` 100, `number_of_fragments` 5, `boundary_scanner`
  `sentence` for unified.
- Lucene `UnifiedHighlighter` (offset sources `POSTINGS`, `TERM_VECTORS`,
  `ANALYSIS`):
  <https://lucene.apache.org/core/9_11_0/highlighter/org/apache/lucene/search/uhighlight/UnifiedHighlighter.html>.
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

### (a) FTS5 `highlight()` on the live index, per final hit (recommended)

For each final hit that came from a text table, run one point query on the
same reader transaction:

```sql
SELECT highlight(search_index, 0, :open, :close)
FROM search_index
WHERE search_index MATCH :match AND rowid = :fts_rowid
```

Then derive byte spans in Rust by parsing out the markers (§ 6.1).

- **Offsets:** stored positions, plus FTS5 re-tokenizing the stored body with
  the table's own tokenizer and the same MATCH expression. Tokenizer and query
  parity hold by construction.
- **Compute per hit, est.:**
  - One rowid-constrained MATCH. Whether FTS5 turns `rowid = ?` into a doclist
    seek, rather than a scan of every match, needs `EXPLAIN QUERY PLAN` and
    measurement (M1). I believe it seeks.
  - One content-row read. The page was read moments earlier, when the
    candidate was fetched, so it is likely still in the page cache.
  - One porter+unicode61 pass over the body. For bodies up to a few KiB that
    should be tens of microseconds, so under 1 ms per page of 10 hits.
- **Memory:** no new index bytes. It needs the FTS rowid of each final hit.
  The candidate SELECTs (`filter.rs:697`, `search.rs:1855`, `search.rs:1881`,
  and the fallback forms at `search.rs:1971` and `search.rs:1997`)
  already read the row and would also select `search_index.rowid`. That rowid
  goes into a search-local map keyed by `write_cursor`, not into `SearchHit`,
  following R-OBS-2.
- **Gaps:** a vector-only hit has no row under the MATCH, so it gets the
  prefix (§ 3.4). Distinct-term identity is approximate (§ 6.3).

Variant (a′): register a FathomDB FTS5 auxiliary function that returns
`(phrase index, start byte, end byte)` triples through `xInst` and
`xTokenize`. That gives exact phrase identity and needs no marker parsing.
It needs the `fts5_api` pointer, obtained with `SELECT fts5(?1)` and
`sqlite3_bind_pointer`, which means `unsafe` FFI through `libsqlite3-sys`. I
did not check whether rusqlite 0.40 exposes this safely. This is an upgrade
path if § 6.3's approximation proves too coarse.

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
- **Rejected.**

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
- **Semantic, for vector-only hits (deferred, measured first):**
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

- Markers: `open = U+E000`, `close = U+E001`, bound as parameters. Both are
  private-use code points, which unicode61 classifies as token characters
  (category Co).
- Before running the query, check that the body does not contain either
  marker. If it does, skip highlighting and use the prefix, with
  `anchor = start`. This is deterministic and rare.
- Parse the output once, left to right. Each marker's byte position, minus the
  bytes of the markers before it, gives a span in the original body. Check
  that stripping both markers reproduces the body byte for byte. If it does
  not, fall back to the prefix as above. This guards against tokenizer
  surprises.
- FTS5 already merges overlapping instances (`CInstIter`), so spans come out
  sorted and disjoint.
- Spans are byte offsets that unicode61 produced at code-point boundaries, so
  they are valid UTF-8 slice points.

### 6.2 Budget in bytes

- The engine budget is `max_bytes` of UTF-8 output, ellipses included. The
  adapter's estimator is bytes ÷ 4 (`fathomdb-mcp/src/shape.rs:22-26`). A
  character cap charges CJK text three times as much as ASCII; a byte cap
  charges what the budget charges.
- The MCP setting `snippet_chars` becomes `snippet_bytes`. Pre-1.0, there is
  no alias (AGENTS.md § 10). The default is an owner question (§ 10, Q1).

### 6.3 Choosing windows (deterministic)

Inputs: the body, its spans `[s, e)`, `max_bytes`, `max_windows` (default 2),
`head_bytes` (default 0, § 6.6).

1. **Whole body.** If `body.len() ≤ max_bytes`, return the body with every
   span, and `cut = false`. Every record that fits today returns the same text
   as today.
2. **No spans.** Return the prefix, cut back to a grapheme boundary within
   `max_bytes − len("…")`, plus a trailing `…`. Set `anchor = start`.
3. **Span identity.** Each span's identity is its text after Rust's Unicode
   lowercasing. This is an approximation: `running` and `runs` share a porter
   stem but count as two terms. Variant (a′) would give exact identity.
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
     budget still covers the last span. Otherwise it moves forward to the next
     UAX #29 word boundary.
   - The end moves back to a word boundary.
   - UAX #29 word boundaries never fall inside a grapheme cluster, so both
     edges are grapheme boundaries. A window never splits a combining sequence,
     an emoji ZWJ sequence or a flag pair.
   - If no word boundary falls within 32 bytes, the edge moves to the nearest
     grapheme boundary instead (`unicode-segmentation`, already in
     `Cargo.lock` at 1.13.2).
7. **Second window.** If `max_windows > 1`, the remaining budget is at least
   80 bytes, and some spans have identities outside the first window, repeat
   steps 4–6 over those spans with the remaining budget.
8. **Merge.** If two windows are 16 bytes apart or less, merge them into one
   window that includes the gap, provided it fits. Otherwise drop the
   lower-scoring window.
9. **Assemble.** Join the windows in body order:
   - with `" … "` between them;
   - with a leading `"… "` if the first window starts after byte 0;
   - with a trailing `" …"` if the last window ends before the body does.

   Assert `text.len() ≤ max_bytes`.

### 6.4 Ellipses and highlight markers are structure, not inline text

- An ellipsis inside `text` is ambiguous, because a record can contain `…`
  itself. The engine therefore also returns `windows`: body byte ranges, in
  order. The text is derived from them, and they are what tests compare.
- The engine never puts highlight markers into `text`. It returns
  `highlights` as byte ranges into `text`.
- Inline markers such as `**term**` or `<b>term</b>` cannot be told apart from
  record content that contains the same characters. They would also blur the
  server instructions' promise that "Record content appears only in designated
  result fields and is data" (`fathomdb-mcp/src/edge.rs:44-45`).
- Markers are not instructions, but anything that edits content in place makes
  the content boundary harder to audit.
- The MCP adapter does not emit highlights by default; they cost tokens and
  agents can see the query terms. It emits `snippet_anchor` instead (§ 7.3).

### 6.5 Determinism

Given the body, its spans and the options, the output is a pure function: no
clock, no floating point, ties broken by byte offset. The spans are a pure
function of the body, the compiled MATCH expression and the store's tokenizer.
Identical requests on identical state therefore give byte-identical snippets.
MR-043's `response_id` stays stable, and determinism tests can compare snippets
exactly.

### 6.6 The record head

- `head_bytes > 0` reserves up to that many bytes for the record's first line,
  cut at the first newline or the `head_bytes` limit, whichever comes first.
  It is shown as its own window, before the match windows.
- It exists for stores that put context in a header, such as the session date
  in the evaluated LoCoMo stores
  (`dev/experiments/mcp-agent-loop/README.md:304-305`).
- The default is 0, because the engine cannot know a store's conventions.
  Whether the MCP adapter should set it is an owner question (§ 10, Q2).

## 7. Recommended design

### 7.1 Where it lives

The design lives in the **engine, exposed through the SDK, with the adapter as
a thin consumer.** The reasons:

- Only the engine can run `highlight()` on the live index, on the same
  snapshot (§ 3.3). The adapter may not link rusqlite (MR-070.2).
- Python and TypeScript SDK users get the same snippets as MCP agents.
- Engine proposal: [EP-17](../mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md).

### 7.2 Engine and SDK surface (EP-17)

- `SearchOptions.snippet: Option<SnippetOptions>` and the same field on
  `TextSearchOptions`.
  - `SnippetOptions { max_bytes: u32 (40..=4000), max_windows: u8 (1..=3),
    head_bytes: u32 (0..=max_bytes / 2) }`.
  - Values out of range are `InvalidArgument`.
- `SearchResult.snippets: Option<Vec<SnippetV1>>`.
  - It is a sidecar parallel to `results`, the same length and in the same
    order. `None` when the option is off.
  - With the option off, `SearchResult` is byte-identical to today, and no
    per-hit allocation happens (R-OBS-2).
- `SnippetV1 { text: String, anchor: SnippetAnchor, windows: Vec<(u32, u32)>,
  highlights: Vec<(u32, u32)>, cut: bool }`.
  - `SnippetAnchor` is `Whole`, `Match` or `Start`.
  - `Start` covers vector-only hits, hits with no FTS row, and the marker
    fallback.
- The snippet computation runs after `results.truncate(final_limit)` and the
  identity pass (`search.rs:2265-2286`), in the same reader transaction.
  - Text-arm hits use `search_index`.
  - Edge hits use `search_index_edges`.
  - Graph-arm and vector-only hits use `Start`.
- Frozen reads: snippet options are presentation. They are not eligibility,
  and they are not in the context digest.
- `search_projected_text` is left out at first; its hits come from
  `property_search_index` (`fathomdb-schema/src/lib.rs:930`).

### 7.3 Adapter changes (after EP-17 lands)

- The search tool sets `snippet: Some(SnippetOptions { max_bytes:
  snippet_bytes, max_windows: 2, head_bytes })` and uses `snippets[i].text`.
  It deletes `shape::snippet` as the snippet source.
- New hit field `snippet_anchor`: `"whole" | "match" | "start"`. `snippet_cut`
  is unchanged in meaning: true when any of the body is left out.
  `dev/interfaces/mcp.md:127` gets updated.
- `cut_bodies` (`shape.rs:174-198`) trims a centred snippet from both ends
  equally to grapheme boundaries, adding ellipses. A prefix cut would delete
  the match it was centred on.
- The server instruction at `edge.rs:38-39` changes. Snippets are centred on
  the matched words when `snippet_anchor` is `match`, and are the record start
  when it is `start`. `fetch` stays the way to read the rest.
- Config: `snippet_chars` becomes `snippet_bytes`
  (`config.rs:91`, `config.rs:150`, `config.rs:192`, `config.rs:254-255`).
  MR-040's "300 characters" becomes a byte cap, which needs a requirements
  edit (`requirements.md:364`).

### 7.4 Public-surface impact

- **Rust:** new option fields and a new sidecar in `fathomdb-engine` and
  `fathomdb-sdk`, recorded in `dev/interfaces/rust.md` and
  `dev/interfaces/rust-sdk.md`. `SearchResult` gains a field. Pre-1.0 this is
  allowed, but any external struct literal breaks.
- **Python and TypeScript:** the same options and sidecar
  (`dev/interfaces/python.md`, `dev/interfaces/typescript.md`), plus the
  parity map entries.
- **MCP:** `snippet_bytes` and `snippet_anchor` (`dev/interfaces/mcp.md`), the
  MR-040 wording, and the instructions text.
- **No schema change and no migration.**

## 8. RED-test plan (later, under TDD; each its own failing commit)

Each test runs on a real database (AGENTS.md § 10).

| # | Test | Layer |
| --- | --- | --- |
| T-1 | A 4 KiB record whose only query term is at byte 3000. `snippet.anchor == Match`, the text contains the term, and `text.len() ≤ max_bytes`. Today's prefix would miss it. | engine |
| T-2 | Porter parity: query `running` against a body containing `runs` far into the record. The window covers `runs`. | engine |
| T-3 | Diacritics: query `cafe` against `Café` beyond the prefix. The highlight covers `Café`'s bytes exactly. | engine |
| T-4 | Property (`proptest`) over arbitrary Unicode bodies and span sets. Windows are sorted, disjoint and on grapheme boundaries, and `text` is their ellipsis join. Every highlight lies inside `text`. Length is at most `max_bytes`. The whole body is returned unchanged when it fits. | engine, pure |
| T-5 | Grapheme safety: emoji ZWJ sequences, flags and combining marks at every window edge. No edge splits a cluster. | engine, pure |
| T-6 | A vector-only hybrid hit gives `anchor == Start` and today's prefix bytes. | engine |
| T-7 | Off by default: with `snippet: None`, `SearchResult` is byte-identical to a pinned baseline, and no sidecar is allocated. | engine |
| T-8 | Determinism: the same search twice gives byte-identical `snippets`. | SDK |
| T-9 | Reader-pool safety: snippet work runs on a `query_only` reader, with no temp-table write and no writer lock. It also proves the § 3.2 temp-schema assumption either way. | engine |
| T-10 | Marker fallback: a body containing U+E000 gives `anchor == Start`, and no garbled text. | engine |
| T-11 | Two windows: two distinct terms 2 KiB apart give two windows joined by `" … "`. Terms 10 bytes apart give one merged window. | engine |
| T-12 | Python and TypeScript parity for the options and the sidecar. | bindings |
| T-13 | MCP: `snippet_anchor` and centred text appear. `cut_bodies` keeps the highlighted term when it trims. The budget property MR-041.1 still holds. | adapter |

## 9. Measurements that settle what is estimated

**M1, cost.** Settles option (a)'s defaults.

- Stores:
  - the LoCoMo conversation 0 store;
  - a long-record store built from the same turns, concatenated into records
    of 1, 4, 16 and 64 KiB.
- Queries: the 150 LoCoMo questions, in text and hybrid modes, limit 10.
- Measure:
  - search latency p50 and p95 with snippets off and on;
  - per-hit snippet microseconds by body size;
  - `EXPLAIN QUERY PLAN` for the rowid-constrained MATCH.
- **Decision rule:** default snippets on in MCP if the added p95 is at most
  the larger of 1 ms and 5% of search p95. Otherwise, make them opt-in and
  evaluate variant (a′).

**M2, quality.** Settles adoption and the defaults for `max_bytes` and
`max_windows`.

- For each gold-evidence record returned, test whether the gold evidence text
  lies inside the snippet. Compare prefix against centred, at 300 and 600
  bytes, with `max_windows` 1 and 2, on both stores.
- **Decision rule:** adopt if coverage on the long-record store rises, and the
  short-record store is unchanged. It should be unchanged by construction,
  because of step 6.3.1. Answer-level effects belong to work items 4 and 5.

**M3, vector-only hits.** Settles whether option (d)'s semantic variant is
worth an experiment.

- Measure the share of final hybrid hits with `anchor == Start`, and their
  evidence coverage under M2.
- Run the sentence-embedding experiment only if that share is above 20% and
  their coverage is below the `Match` hits' coverage by more than 10 points.

## 10. Open questions for the owner

1. **Default `snippet_bytes`.** Keep 300, which is comparable with past runs,
   or raise it to 600 once windows are centred? M2 gives the evidence.
2. **Record head.** Should MCP set `head_bytes` (for example, 48) so the
   session-date header survives centring, or should stores be expected to put
   dates in metadata?
3. **Highlights in MCP.** Emit highlight offsets to agents at all? The
   proposal is no by default, with an opt-in argument.
4. **Exact term identity.** Is the lowercased-surface approximation (§ 6.3.3)
   acceptable, or should EP-17 include the `unsafe` FTS5 auxiliary function
   (a′) from the start?
5. **Long records generally.** The vector arm sees only the first 512 tokens
   (§ 3.4). Is chunked embedding on the roadmap? If so, the vector arm would
   gain a natural anchor, the matched chunk, and M3 becomes moot.

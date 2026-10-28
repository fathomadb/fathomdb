---
title: Compact text block for MCP results
date: 2026-10-24
status: ACTIVE
desc: Design for agent-access item 4 — a `result_format` server setting whose default sends every tool result as a compact text block without `structuredContent`; the per-tool text contract with quoting and framing rules, the field inventory, budget accounting, tests, and how the re-measure is read. Revised for the GPT-6 Astra review. Implementation is placed in 0.8.29.
inputs:
  - dev/design/agent-access/README.md
  - dev/design/agent-access/compact-text-block-astra-review.md
  - dev/experiments/agent-remeasure/REPORT.md
  - dev/experiments/agent-remeasure/compact_proxy.py
  - dev/interfaces/mcp.md
---

# Compact text block for MCP results

## 1. Decision and evidence

The owner decided on 2026-10-21 to design the compact text block (agent-access
item 4) and to place its implementation in 0.8.29. The measurement is in
[REPORT](../../experiments/agent-remeasure/REPORT.md) § "Item 4". On a
30-question sample with Haiku 5.5 and Sonnet 5.5, the compact rendering cut
search tokens per call by 33 % and cost per question by 28–30 %. It made no
measurable difference to accuracy or to how often agents fetched cut
snippets.

The [GPT-6 Astra review](compact-text-block-astra-review.md) of the first
draft returned PASS-WITH-FIXES. This revision applies every finding; § 8
maps each one to its fix.

## 2. The problem

- Every successful result carries `structuredContent`, with its schema
  declared as `outputSchema`, plus a text block that renders the same data
  (`dev/interfaces/mcp.md` § 3, `fathomdb-mcp/src/edge.rs`, the
  `output_schema` attributes).
- **Protocol (MCP 2025-06-18).** A tool that declares `outputSchema` must
  return `structuredContent` that conforms to it, and for backwards
  compatibility should also return the serialized JSON in a text block. A
  tool that declares neither is valid; its result is its content blocks.
- **Observed client behaviour, not a protocol guarantee.** The Claude Code
  version used in the 2026-10-19 runs shows the model `structuredContent`
  as JSON whenever it is present, and not the text block. Other clients may
  show the text block, or both. The design therefore does not depend on any
  one client: the setting in § 3.1 decides what the server sends.
- **The budget counts both copies.** `shape::with_budget` estimates
  `structured + text` bytes. A model sees one copy, so about half of every
  `max_tokens` budget is spent on bytes that model never reads, and the cut
  loop drops hits earlier than it needs to.

## 3. Design

### 3.1 Setting

A server config key, `result_format`, with two values:

- `text` (**default**): for all seven tools, `tools/list` declares no
  `outputSchema`, and results carry no `structuredContent`. The single text
  block in § 3.3 is the whole result.
- `structured`: `outputSchema` is declared and `structuredContent` is sent.
  The text block becomes the serialized JSON of `structuredContent`, as the
  protocol recommends; today's human-readable rendering is used only under
  `text`. The budget counts both copies, as today.

The setting is server-wide. A deployment that serves both a model client and
a programmatic client runs two stdio server processes with different
configs. The server does not guess from the client's name, and there is no
session negotiation until a deployment shows it needs one.

Tool errors follow the same setting. Under `text`, an error is the rendered
error text (`error.rs` `render`) with `isError: true` and no
`structuredContent`.

The default changes what existing clients receive, so the release notes for
0.8.29 must say so and show the `structured` config line.

### 3.2 Quoting rules (all tools)

These rules make every line parseable left to right, whatever a writer put
in a record:

- **Writer-controlled strings are JSON strings.** References, kinds, source
  ids, edge kinds and store names are written as JSON string literals
  (`shape::quote`, with surrounding quotes and escapes). A logical id may
  contain spaces, newlines and ` | ` (`tools.rs` `parse_ref` rejects only an
  empty id and NUL), so a bare reference could split a line or a field.
- **Server-made tokens are bare.** Continuations (`c<epoch>-<hash>`),
  response ids (`r<epoch>-<hash>`), numbers, booleans and enum words never
  contain spaces or quotes.
- **A field is `name=value` or a bare flag word** (`cut`). Values follow the
  two rules above, so a parser reads a JSON string whenever a value starts
  with `"`, and otherwise reads up to the next space.
- **Snippets** (`search`) and **expand bodies** come last on their line,
  after the field list, separated by ` | `. Newlines in them become spaces,
  so one hit is exactly one line. The separator is the first ` | ` after the
  field list, which a parser finds after reading the fields; a ` | ` inside a
  quoted field or inside the snippet does not confuse it. Snippets and
  expand bodies are for reading, so collapsing their newlines is allowed;
  references are not, so they are always quoted.
- **Fetch bodies** are verbatim and length-framed (§ 3.3).

### 3.3 Per-tool text

`<…>` is a value; `[…]` is present only when it applies.

**`search`**

```text
search store=<q> epoch=<n> validity_instant=<n> mode=<text|hybrid> hits=<n> completeness=<word> [kind=<q>] [continuation=<c>] [continuation_exact=false] response_id=<r> tokens=<used>/<max>[ cut]
degraded: <arm> <reason>
hint: <action> <fields> (<reason>: <template>)
<q ref> arm=<arm> score=<x.xxx> [ce=<x.xxx>] [anchor=<word>] [cut] [kind=<q>] [src=<q>] [hl=<a>-<b>,…] | <snippet>
[explanation: <compact JSON>]
```

- `<q>` marks a JSON string (§ 3.2).
- `degraded:` lines: one per degradation entry. `hint:` lines: one per hint,
  rendered as `hints.rs` `render` does today.
- `kind=` is in the header when every hit shares one kind; otherwise each
  hit line carries its own.
- Scores are rounded to three decimals. `anchor=` is omitted when the
  snippet is the whole record.
- `src=` is omitted when the hit has no source id.
- `continuation_exact=false` appears exactly when the structured output
  would carry it (`tools.rs`, set from the hosting mode).
- Per-hit explanations, when requested, follow their hit as an indented
  `explain:` line, as today.

**`expand`**

```text
expand store=<q> epoch=<n> validity_instant=<n> targets=<n> complete=<bool> [bound=<word>] tokens=<used>/<max>[ cut]
degraded: …
hint: …
<q ref> kind=<q> hop=<n> edge=<q> [cut] | <body>
```

**`fetch`**

```text
fetch store=<q> epoch=<n> validity_instant=<n> items=<n> tokens=<used>/<max>[ cut]
hint: …
record <q ref> kind=<q> bytes=<n> [cut]
<exactly n bytes of body, verbatim>
error <q ref> <class>/<code>
```

- There is one item per requested reference, in request order. Each item is
  either a `record` or an `error`. An item never disappears: a reference
  that fails to parse, is missing, or is cut out by the budget still has its
  line.
- A `record` header is followed by a newline, then exactly `bytes=` bytes
  of UTF-8 body, then a newline. The parser skips the body by its length, so
  a body that contains a line looking like `record "…"` cannot be mistaken
  for a header. `cut` means the body was truncated to fit the budget;
  `bytes=` counts what is sent.

**`orient`**

```text
orient store=<q> epoch=<n> validity_instant=<n> tokens=<used>/<max>
projection <q name> roles=<a,b> fts=<bool> vector=<bool> dense=<readiness>
embedding: state=<word> usable_embedder=<bool> pending=<n> [affected_kinds=<q>,<q>,…]
runtime_embedder: available=<bool> [reason=<word>]
[vector_unsupported_kinds=<q>,<q>,…]
dense_disabled: <bool> [reason=<q>]
```

Today's text block leaves out the projection status, the affected kinds and
the reason the vector side is disabled. Agents need these to tell why hybrid
search degrades, so the text carries them. The `kinds` and `counts`
placeholders (always `unavailable`) are dropped.

**`list_stores`, `use_store`, `feedback`**: their current text renderings,
with store names quoted (§ 3.2). They carry every field of their structured
output already.

### 3.4 Field inventory

Every field of every `*Output` type in `output.rs` is either kept in the text
or dropped for a stated reason. Implementation adds this table to
`dev/interfaces/mcp.md` and a test checks that it covers every serialized
field.

| Field | Under `text` | Why |
| --- | --- | --- |
| `store`, `epoch`, `validity_instant` | kept (header) | Epoch names the open generation a ref belongs to; it does not prove freshness. |
| `budget.used_tokens`, `max_tokens`, `cut` | kept (`tokens=`) | Agents act on `cut`. |
| `degradation` | kept (`degraded:` lines) | Explains a text-only answer. |
| `hints` | kept (`hint:` lines) | Suggested next calls. |
| `continuation`, `continuation_exact`, `response_id`, `completeness` | kept | Needed to page and to send feedback. |
| hit `ref`, `arm`, `score`, `ce_score`, `snippet_anchor`, `snippet_cut`, `kind`, `source_id`, `highlights`, `snippet` | kept | The measured line. |
| `explanation` | kept when requested | Opt-in diagnostics. |
| `state` (`{status: unavailable}`) | dropped | A constant placeholder until a freshness check exists (B2-1 restores it as a real field). |
| `verdict` (`unavailable`) | dropped | A constant placeholder until scores are calibrated (MR-031). |
| orient `kinds`, `counts` | dropped | Constant placeholders. |
| orient projection status, affected kinds, dense-disabled reason | kept (new to the text) | See `orient` above. |
| fetch per-item errors, `body_cut` | kept | One outcome per reference. |

A placeholder that later becomes a real value goes into the text in the same
change that makes it real.

### 3.5 Budget

Under `text`, `used_tokens` is ceil(bytes ÷ 4) of the final text block,
including the `tokens=` field itself. Because the field's width depends on
its own value, the renderer:

1. renders the page with `used` set to the last estimate (initially 0);
2. measures the text, sets `used` = ceil(bytes ÷ 4), and renders again;
3. repeats step 2 until the byte length stops changing.

While the page is fixed, the width of `used` can only grow, so the loop
stops after a few renders; the implementation caps it at four and a test
fails if the cap is reached. The cut loop compares the converged `used` with
`max_tokens`. If the page is over, the loop drops a hit (search, expand) or
shortens the last body (fetch) and restarts at step 1. The number in the
text is always the size of the text that is sent.

Under `structured`, accounting stays as today. A page under `text` holds
more hits for the same `max_tokens`; the default `max_tokens` is unchanged,
and `limit` still caps the hit count.

### 3.6 Server instructions

The server instructions (`edge.rs`) already describe match-centred snippets.
They gain one line per tool that names the text fields (`cut`, `anchor=`,
`continuation=`, `tokens=`), and a sentence saying that quoted values are
JSON strings to be passed back verbatim.

## 4. Public surface

- `dev/interfaces/mcp.md` § 3 changes:
  - `structuredContent` and `outputSchema` exist only under
    `result_format = structured`, where the text block is their serialized
    JSON;
  - § 3.2–3.3 become the text contract;
  - the field inventory of § 3.4 is added;
  - the config table gains `result_format`.
- The Rust allowlist is not touched: this is adapter config, not a `pub`
  type. If a `ResultFormat` enum becomes `pub` in `fathomdb-mcp`, it needs
  an allowlist entry, which requires owner sign-off.

## 5. Tests (RED first, one per behaviour)

1. `text_format_lists_tools_without_output_schema`
2. `text_format_results_carry_no_structured_content`
3. `structured_format_keeps_output_schema_and_sends_serialized_json_text`
4. `text_format_errors_are_text_with_is_error`
5. `refs_with_space_newline_quote_and_pipe_round_trip_in_every_tool`. Store
   records whose logical ids contain ` `, `\n`, `"` and ` | `, then parse
   the search, expand and fetch text and recover each reference verbatim.
6. `snippet_with_pipe_and_newline_stays_on_one_hit_line`
7. `fetch_body_that_looks_like_a_record_header_is_framed_by_length`
8. `fetch_returns_one_outcome_per_requested_ref_in_order`. Mix found,
   missing, malformed and budget-cut references.
9. `text_tokens_field_equals_final_text_size`. Include pages whose size
   crosses a digit boundary of `used` (9→10, 99→100, 999→1000 tokens), with
   hints and cut flags present.
10. `text_format_budget_counts_only_the_text_block`
11. `text_format_fits_more_hits_in_the_same_max_tokens`
12. `orient_text_carries_projection_status_affected_kinds_and_dense_reason`
13. `search_text_carries_continuation_exact_when_set`
14. `text_inventory_covers_every_serialized_output_field`
15. Property test: for arbitrary hits and references, the parser recovers
    every reference, cut flag and the continuation from the text.
16. `text_and_structured_results_carry_the_same_kept_fields`. Run every tool
    under both settings on the same store and request, parse the text, and
    compare it with `structuredContent` for each field § 3.4 keeps.
17. MCP conformance passes under both settings, not only the default.

The existing MCP suite reads results through `structuredContent`, so its
shared harness (`tests/common/mod.rs`) sets `result_format = structured`. It
keeps covering that path unchanged.

## 6. Reading the re-measure

The proxy re-rendered pages that were already cut to the JSON budget. The
built server under `text` cuts against the text alone, so a page can hold
more hits, and tokens per call need not fall by the proxy's 33 %. The two
effects are measured separately:

- **Rendering, same page.** A deterministic check renders fixed pages, taken
  from the item-4 run, in both formats, and reports the byte ratio. It is
  expected to be close to the proxy's (0.67 for search); a large gap means
  the server format differs from the measured one.
- **End to end.** Re-run the agent-remeasure item-4 arms against the built
  server, without the proxy, under `text` and `structured`. Report hits per
  page, tokens per call, tokens and cost per question, fetch rate on cut
  snippets, and strict accuracy. There is no token target here. An accuracy
  drop outside the paired interval is a stop.
- **Before the re-run:** fix the experiment prompt's sentence that says
  snippets show the record start (`dev/experiments/agent-remeasure/README.md`
  notes it). The server instructions are already correct.

## 7. Decisions (approved by the owner on 2026-10-27)

1. **Default `text`.** The measured agent benefit supports it. It does not
   establish compatibility across clients, which is why § 3.1 requires
   release notes and a documented `structured` config.
2. **Server-wide setting.** Separate stdio processes cover mixed
   deployments. No client-name guessing and no negotiation until a
   deployment needs it.
3. **Fields.** Keep `epoch` and `validity_instant`, drop the `state` and
   `verdict` placeholders, keep `continuation_exact` and the readiness and
   degradation information (§ 3.4).
4. **`fetch` is included**, with the length-framed contract of § 3.3. Its
   gain is unmeasured; the end-to-end re-run reports it.

## 8. Review findings and fixes

| Finding | Fix |
| --- | --- |
| P1 bare references break the grammar | § 3.2: writer-controlled strings are JSON strings; tests 5 and 15. |
| P2 operational fields lost | § 3.3 `orient`, § 3.4 inventory, `continuation_exact` kept; tests 12–14. |
| P2 budget needs a final-render rule | § 3.5 converging render; test 9. |
| P2 fetch contract incomplete | § 3.3 `fetch`: per-reference outcomes in order, length framing; tests 7 and 8. |
| P2 acceptance conflates rendering and pagination | § 6: same-page ratio and end-to-end run reported separately. |
| P3 protocol versus client behaviour | § 2 separates them; § 3.1 `structured` text is the serialized JSON. |
| P3 stale snippet sentence is in the prompt | § 3.6 and § 6: the server is correct; fix the experiment prompt before the re-run. |

## 9. Placement

Implementation is placed in 0.8.29
([ROADMAP](../../ROADMAP.md) § 0.8.29). It depends on Block 1, which closed
on 2026-10-23 at `b5f539880` on `feat/agent-access-block1`. The owner
approved § 7 on 2026-10-27; the build lands on `dev/0.8.29`.

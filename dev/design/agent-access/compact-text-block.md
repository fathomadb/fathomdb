---
title: Compact text block for MCP results
date: 2026-10-21
status: ACTIVE
desc: Design for agent-access item 4 — a `result_format` server setting whose default sends search and expand results as a compact text block without `structuredContent`, the line grammar, budget accounting, tests and the open questions for review. Implementation is placed in 0.8.28 as D28-14.
inputs:
  - dev/design/agent-access/README.md
  - dev/experiments/agent-remeasure/REPORT.md
  - dev/experiments/agent-remeasure/compact_proxy.py
  - dev/interfaces/mcp.md
---

# Compact text block for MCP results

## 1. Decision and evidence

The owner decided on 2026-10-21 to design the compact text block (agent-access
item 4) and to place its implementation in 0.8.28. The measurement is in
[REPORT](../../experiments/agent-remeasure/REPORT.md) § "Item 4". On a
30-question sample with Haiku 5.5 and Sonnet 5.5, the compact rendering cut
search tokens per call by 33 % and cost per question by 28–30 %. It made no
measurable difference to accuracy or to how often agents fetched cut
snippets.

## 2. The problem

- Every successful result carries `structuredContent`, with its schema
  declared as `outputSchema`, plus a text block that renders the same data
  (`dev/interfaces/mcp.md` § 3, `fathomdb-mcp/src/edge.rs:173-263`).
- Claude Code shows the model `structuredContent` as JSON whenever it is
  present, and not the text block. The text renderers in
  `fathomdb-mcp/src/tools.rs` (`render_search`, `render_expand`) are
  therefore never read by the main agent client.
- The budget counts both copies: `shape::with_budget` estimates
  `structured + text` bytes. A model sees one copy, so about half of every
  `max_tokens` budget is spent on bytes that model never reads. The cut loop
  drops hits earlier than it needs to.
- MCP 2025-06-18: a tool that declares `outputSchema` must return conforming
  `structuredContent`, and should also return the serialized JSON as text.
  The only way to make the text block what a model sees is to declare no
  `outputSchema` and send no `structuredContent`.

## 3. Design

### 3.1 Setting

A server config key, `result_format`, with two values:

- `text` (**default**): for all seven tools, `tools/list` declares no
  `outputSchema` and results carry no `structuredContent`. The text block
  is the whole result.
- `structured`: today's behaviour. `outputSchema` is declared and
  `structuredContent` is sent alongside the text block.

The setting applies to the whole server; there is no per-call or per-session
switch. Clients that parse results programmatically set `structured`.
Tool errors follow the same setting: under `text` an error is the rendered
error text with `isError: true` and no `structuredContent`.

### 3.2 Line grammar (`search`)

The measured proxy format becomes the server's format, with one change:
references are written bare (they never contain spaces), as the proxy did.

```text
search store=<name> mode=<text|hybrid> hits=<n> <completeness> [kind=<k>] [continuation=<c>] response_id=<id> tokens=<used>/<max>[ cut]
degraded: <arm> <reason>                      (one per degradation entry)
hint: <action> <fields> (<reason>[: <template>])   (one per hint, as hints.rs renders)
<ref> <arm> <score> [ce=<s>] [@<anchor>] [[cut]] [kind=<k>] src=<source_id> [hl=a-b,…] | <snippet>
```

- `kind=` goes in the header when every hit shares a kind; otherwise each
  hit line carries its own.
- Scores are rounded to 3 decimals.
- Newlines in a snippet become spaces. A ` | ` inside a snippet needs no
  escaping, because everything after the first ` | ` is the snippet.
- Dropped from the text: `epoch`, `validity_instant`, `state` and `verdict`.
  Under `text` they are not available at all.
- `explanation`, when requested, is one `explanation:` line of compact JSON.

### 3.3 Other tools

- **`expand`**: a header `expand store=… targets=<n> complete=<bool> bound=<b> tokens=…`,
  then `degraded:` and `hint:` lines, then
  `<ref> kind=<k> hop=<n> edge=<kind> [[cut]] | <body>` per target.
- **`fetch`**: one header line and one block per record, with the body
  verbatim and not collapsed to one line. Fetch was not measured; bodies
  dominate its size, so the gain is small. It is listed here so that every
  tool follows the same setting.
- **`orient`, `list_stores`, `use_store`, `feedback`**: their current text
  renderings, unchanged.

### 3.4 Budget

Under `text`, `used_tokens` is ceil(bytes ÷ 4) of the text block alone, and
the cut loop sizes against that figure. Under `structured`, it stays as it
is today (both copies). Because a page under `text` holds more hits for the
same `max_tokens`, the default `max_tokens` is left unchanged, and
`limit` still caps the hit count.

### 3.5 Server instructions

The instructions and tool descriptions describe the hit-line fields in a
single line, so a model can read `[cut]`, `@anchor` and `continuation=`
without a schema. They also correct the stale sentence that says snippets
show the start of the record; snippets have been match-centred since
Block 1.

## 4. Public surface

- `dev/interfaces/mcp.md` § 3 changes: `structuredContent` and
  `outputSchema` exist only under `result_format = structured`, and the
  grammar in § 3.2 becomes the text contract. The config table gains
  `result_format`.
- The Rust allowlist is not touched: this is adapter config, not a `pub`
  type. If a `ResultFormat` enum becomes `pub` in `fathomdb-mcp`, it needs
  an allowlist entry (39 → 40), which requires owner sign-off.

## 5. Tests (RED first, one per behaviour)

1. `text_format_lists_tools_without_output_schema`
2. `text_format_results_carry_no_structured_content`
3. `structured_format_keeps_output_schema_and_structured_content`
4. `text_format_search_hit_line_carries_every_ref_cut_flag_and_continuation`.
   This is a round-trip check: parse the lines and compare with the page.
5. `text_format_budget_counts_only_the_text_block`
6. `text_format_fits_more_hits_in_the_same_max_tokens`
7. `text_format_errors_are_text_with_is_error`
8. `snippet_with_pipe_and_newline_renders_on_one_line`
9. Property test: for arbitrary hits, every `ref` and the `continuation` can
   be recovered verbatim from the text.

Re-run the agent-remeasure item-4 arm against the built server, without the
proxy, and confirm the token cut matches the proxy figure.

## 6. Open questions for review

1. **Is `text` the right default?** The alternative is `structured` by
   default, with agents opting in to `text`. The measurement favours
   `text`, but a programmatic client that relies on `outputSchema` breaks
   silently.
2. **Is a server-wide setting enough?** A per-session choice at
   `initialize` (for example by client name) would let one server serve
   both kinds of client. It adds negotiation for one known client
   behaviour.
3. **Are the dropped fields safe to drop?** `epoch`, `validity_instant`
   and `verdict` are not shown under `text`. No agent in the runs used
   them; a future freshness check (B2-1) might need them.
4. **Should `fetch` be included?** It was not measured.

## 7. Placement

Implementation is placed in 0.8.28 as **D28-14**
([draft scope](../../plans/0.8.28-draft-scope.md)). It depends on Block 1
landing, and starts after the review in § 6 and owner approval of the
answers.

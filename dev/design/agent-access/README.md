---
title: Agent access — work list after the LoCoMo MCP evaluations
date: 2026-10-18
status: PROPOSED
desc: Enumerated work items raised by the MCP agent-loop and Claude-subagent LoCoMo runs — admission queue, hints sidecar, match-centred snippets, compact text block — with their gates.
inputs:
  - dev/experiments/mcp-agent-loop/README.md
  - dev/design/mcp-adapter/architecture.md
  - dev/design/mcp-adapter/requirements.md
  - src/rust/crates/fathomdb-mcp/src/registry.rs
  - src/rust/crates/fathomdb-engine/src/reader_pool.rs
---

# Agent access — work list

## Why this exists

The MCP evaluations on LoCoMo conversation 0 found three problems:

- Six concurrent Claude subagents got 29 `busy/admission` refusals out of
  about 125 calls.
- Agents ignored `snippet_cut: true` and rarely fetched. They answered from
  cut-off, prefix-only snippets, even when the record held the answer.
- Agents misread or tripped on other signals: `verdict: unavailable` on every
  search, repeated `list_stores` calls, and unknown arguments.

Each item below is a design deliverable with a review gate. Nothing here
changes code until its design passes review and the owner approves.

## Work items

1. **Admission queue between an agent and SQLite** →
   [queue-admission.md](queue-admission.md).
   1. Map every queue and limit on the path agent → MCP edge →
      `fathomdb-mcp` registry → `fathomdb-sdk` → `fathomdb-engine` (reader
      worker pool, embedder pool) → rusqlite → SQLite (WAL, busy handler,
      page cache): its type, capacity, ordering, behaviour when full, and
      timeout and cancellation behaviour.
   2. Record the defect: `registry.rs` refuses at the cap immediately, but
      `architecture.md` § "Threads, cancellation and admission" says excess
      calls wait up to the deadline.
   3. Define the queue type needed, and where it lives: bounded FIFO with
      deadlines, fairness across sessions or agents, accounting for abandoned
      work, load-shedding signal, `retry_after_ms`.
   4. Set the defaults and the ceiling: `admission_cap` 4 → 7 against the
      reader pool of 8, and whether the engine pool should become
      configurable. Specify a concurrency sweep (1, 2, 4 and 7 in flight ×
      text and hybrid; p50/p95 latency and throughput) with a decision rule;
      do not guess.
   5. Find 1–3 exemplar queue or admission managers on the web (structure,
      not popularity) and compare them with the chosen design.
   6. **Gate:** Fable-medium critical review of the queue choices and the
      exemplars.

2. **Hints sidecar for agents (MCP and SDK)** →
   [hints-sidecar.md](hints-sidecar.md).
   1. An adapter-written `hints` array, kept separate from record content: fetch
      cut refs, follow the continuation, rephrase on empty results, explain
      degradation, and say a default store is set.
   2. Design it as an SDK-level mechanism that MCP passes through, so agents
      that use the SDK directly get the same hints.
   3. Make it very fast (no extra queries, no allocation when off, a measured
      overhead budget) and configurable off, per call and per process.
   4. Cover the related error feedback: unknown arguments name the accepted
      ones, feedback refs not in the response, busy with `retry_after_ms`,
      and an honest label for the unavailable state verdict.
   5. **Gate:** Fable-medium design review.

3. **Snippets centred on the match** →
   [match-centred-snippets.md](match-centred-snippets.md).
   1. Survey how search engines build query-dependent snippets and
      highlights: SQLite FTS5 `snippet()`/`highlight()`, Lucene/Elasticsearch
      highlighters, Tantivy's snippet generator, and others. Verify the
      terminology. The working guess was "a rewrite engine"; check whether
      that is the right name.
   2. Choose where match offsets come from (FTS5 auxiliary functions,
      re-tokenising the body, stored offsets) and the compute-versus-memory
      trade-off for each, including the vector arm, where there is no lexical
      match.
   3. Handle the text precisely: UTF-8 and grapheme boundaries, tokenizer
      parity with FTS5, merging several matches into windows, ellipses, and
      a budget in bytes.
   4. Output an engine proposal draft (EP-17) plus the adapter changes.
   5. **Gate:** Fable-medium design review.

4. **Compact text block instead of JSON: value first, design later.**
   1. Before any design work, show that it helps. Replay the 30-question
      sample with the current JSON text block and with a compact one-line-per-hit
      rendering. Compare tokens per call, fetch rate on cut snippets, and
      accuracy with a stricter judge or hand grading.
   2. Do this only after items 2 and 3 land, because hints and centred
      snippets change what the text block carries.
   3. **Gate:** owner decision on the measured value.

5. **Re-measure without the test-set tuning.** After items 1–3, re-run
   Sonnet 5.5 and Haiku 5.5 subagents on a conversation the prompt was never
   tuned on (conv1), with all ten conversations in one store and one agent
   per question.

## Block 2 — agent feedback signals

Block 1 is items 1–5 above. Block 2 is the feedback table below. Block 2 is
**implemented only after Block 1 is complete and passing all tests**. The
Block 1 designs must still account for it now: the hints sidecar (item 2)
and the queue (item 1) define the shapes these signals use, so Block 2 adds
entries rather than redesigning.

| # | Situation | Today | Proposed | Designed in |
| --- | --- | --- | --- | --- |
| B2-1 | `verdict:"unavailable"`, `state:{status:"unavailable"}` on every search | Looks like a failure; Haiku flagged it | Omit it, or label it "freshness check not supported in this release" | item 2 |
| B2-2 | Zero or weak results | Empty list | Hint: rephrase or switch `mode` | item 2 |
| B2-3 | Hybrid fell back to text only | `degradation` label | The same, plus a plain-language hint | item 2 |
| B2-4 | Wrong argument (e.g. `limit` on `fetch`) | Error | Error names the accepted arguments | item 2 |
| B2-5 | `feedback` refs not in the response | Error | Error names the refs that aren't in that response | item 2 |
| B2-6 | Repeated `list_stores` calls (gpt-5.4-mini) | Agent can't tell a default store is set | Guide says calls without `store` use the default; `list_stores` says so too | item 2 |
| B2-7 | Busy | Refusal | Wait in the queue, then `retry_after_ms` | item 1 |

## Gates

- **HITL review before any code.** No code for Block 1 or Block 2 is
  written until the owner has reviewed the designs, after the Fable-medium
  reviews and their fixes.
- Block 2 code starts only after Block 1 is complete and every test passes.

## Order

Items 1, 2 and 3 are designed in parallel. Each gets a Fable-medium review,
then the review fixes, then HITL review. Item 4 waits for 2 and 3. Item 5
waits for the build of 1–3. Block 2 follows Block 1.

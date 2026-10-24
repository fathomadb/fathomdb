---
title: Agent access — work list after the LoCoMo MCP evaluations
date: 2026-10-18
status: ACTIVE
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
      work, load-shedding signal (busy, without `retry_after_ms`; see
      queue-admission.md).
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
      ones, feedback refs not in the response, busy (B2-7, item 1),
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
   6. **Built (Block 1, S5a–S5c).** The engine sidecar and selector are in
      place. MCP search hits carry `snippet_anchor`, `snippet_cut` comes from
      the engine, and the config key is `snippet_bytes` (300 bytes, record
      head `min(48, snippet_bytes / 2)`; `snippet_chars` is gone, with no
      alias). `highlights` is opt-in. The M1 and M2 measurements are still
      open.

4. **Compact text block instead of JSON: value first, design later.**
   1. Before any design work, show that it helps. Replay the 30-question
      sample with the current JSON text block and with a compact one-line-per-hit
      rendering. Compare tokens per call, fetch rate on cut snippets, and
      accuracy with a stricter judge or hand grading.
   2. Do this only after items 2 and 3 land, because hints and centred
      snippets change what the text block carries.
   3. **Gate:** owner decision on the measured value.
   4. **Measured 2026-10-19**
      ([REPORT](../../experiments/agent-remeasure/REPORT.md)). Compact cuts
      search tokens by a third and cost per question by about 30 %, with no
      measurable accuracy or fetch change.
   5. **Owner decision 2026-10-21:** design it; implementation placed in
      0.8.29. Design:
      [compact-text-block.md](compact-text-block.md), GPT-6 Astra review
      [PASS-WITH-FIXES](compact-text-block-astra-review.md); fixes applied
      2026-10-24. Awaiting owner approval of its § 7 before code.

5. **Re-measure without the test-set tuning.** After items 1–3, re-run
   Sonnet 5.5 and Haiku 5.5 subagents on a conversation the prompt was never
   tuned on (conv1), with all ten conversations in one store and one agent
   per question.
   1. **Measured 2026-10-19**
      ([REPORT](../../experiments/agent-remeasure/REPORT.md)). 81 conv-1
      questions: Sonnet 0.951 lenient, 0.840 strict; Haiku 0.926 and
      0.741. No busy results or errors.

6. **MCP server defects found during the evaluations.** Each is fixed
   test-first (its own RED commit) as part of the Block 1 build:
   1. **Progress notifications can arrive out of order.** Each one is sent
      from its own `tokio::spawn` (`fathomdb-mcp/src/edge.rs:107-108`), so
      two in a row can be delivered in either order. Send them in order
      from one task per call.
   2. **Exit code when stdin closes before `initialize`.** The server exits
      1. `dev/interfaces/mcp.md` § 1 says 0 for "normal shutdown (stdin
      closed)", which was written for a shutdown after a session. Decide the
      contract, then make `main.rs` match it.
   3. **A store with the vector side disabled reports `internal`.** Hybrid
      search on such a store falls through the catch-all in
      `fathomdb-mcp/src/error.rs:244` to `internal`. It should be a
      degradation (EP-03) or a named, actionable error.
   4. **The full gate has not run.** `./scripts/agent-verify.sh` was never
      run on `experiment/mcp`. Run it on the Block 1 build before claiming
      green.

## Block 2 — agent feedback signals

Block 1 is items 1–6 above. Block 2 is the feedback table below. Block 2 is
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
| B2-7 | Busy | Refusal | Wait in the queue, then busy (no `retry_after_ms`; see queue-admission.md) | item 1 |

## Gates

- **HITL review before any code.** No code for Block 1 or Block 2 is
  written until the owner has reviewed the designs, after the Fable-medium
  reviews and their fixes. **Block 1 designs approved by the owner on
  2026-10-18.** From the follow-up test (queue-sim REPORT § 12), the owner
  dropped the p90 start check and set `run_limit_ms` to 30 s on 2026-10-19.
- Block 2 code starts only after Block 1 is complete and every test passes.
- **Block 1 CLOSED 2026-10-23** on `feat/agent-access-block1` (not merged to
  `main`). The full gate is green at `32fdc4167` (196/196 suites). The close
  record and the open 0.8.29 todos are in
  `dev/plans/agent-access-block1-plan.md`. Block 2 may start; B2-8 still
  needs owner approval as a Block 2 row (todo seq 296).

## Order

Items 1, 2 and 3 are designed in parallel. Each gets a Fable-medium review,
then the review fixes, then HITL review. Item 4 waits for 2 and 3. Item 5
waits for the build of 1–3. Item 6 is fixed during that build. Block 2 follows Block 1.

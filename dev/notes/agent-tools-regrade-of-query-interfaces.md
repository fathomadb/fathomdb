---
title: Re-grading the §4 query-interface list against shipping agent tools (OpenClaw, Hermes, Letta, Mem0, Zep, Claude Code, Codex)
date: 2026-10-16
status: research note (non-canonical; see dev/notes/README.md)
purpose: Check the sixteen interface properties in §4 of dev/notes/llm-agent-query-interfaces.md against what agent harnesses and memory layers actually ship in October 2026. Report which hold, which are heightened, which need revision, and which were missing.
inputs:
  - https://docs.openclaw.ai/concepts/memory, /concepts/memory-builtin.md, /concepts/memory-search.md
  - https://hermes-agent.nousresearch.com/docs/user-guide/features/memory and https://github.com/NousResearch/hermes-agent
  - https://docs.letta.com/configuration/memory
  - https://docs.mem0.ai/core-concepts/memory-operations/search
  - https://help.getzep.com/graphiti/getting-started/overview and https://help.getzep.com/v4/llms.txt
  - https://www.anthropic.com/engineering/writing-tools-for-agents
  - https://modelcontextprotocol.io/specification/2025-11-25/server/tools
  - Claude Code tool behaviour observed first-hand in this session (2026-10-16)
  - dev/design/0.8.0-agent-memory-fit.md §8 and dev/research/personal-agent-database-market-2026-07-02.md (the repo's prior picture, treated as a baseline to check, not as fact)
---

# Re-grading the §4 query-interface list against shipping agent tools

> "§4.x" refers to `dev/notes/llm-agent-query-interfaces.md`. "H-n" refers to
> `dev/notes/infino-hypotheses-vs-agent-query-interfaces.md`. Vendor docs are
> reported as what they say; nothing was run. The repo's own June and July 2026
> analyses of these tools are out of date in several places, listed in §2, so
> every claim below is sourced to a current page or to first-hand observation.

## 1. What the tools actually do today

### OpenClaw (builtin memory engine, the only engine since QMD was removed)

- **Storage.** Plain Markdown is the memory of record: `MEMORY.md` (curated),
  `USER.md`, dated `memory/YYYY-MM-DD.md` working notes, `DREAMS.md`. "The
  model only remembers what gets saved to disk; there is no hidden state." The
  index is a per-agent SQLite file with FTS5 BM25 and optional sqlite-vec
  (fallback to in-process cosine). Chunks are 400 tokens with 80 overlap.
- **Per-chunk provenance kept outside the Markdown:** origin class (`owner`,
  `agent`, `untrusted`, `system`), session kind, observation time, optional
  supersession key, write-time importance. Stated reason: "recalled prose cannot
  rewrite its own trust classification."
- **Tools.** `memory_search(query, maxResults, corpus)` returns snippets, paths,
  source line references, a final `score`, and provenance metadata.
  `memory_get(path, line range)` hydrates. Session transcript hits need a
  different pair, `sessions_search` then `sessions_history`, and the docs warn
  that "memory-search line numbers are not session-history offsets."
- **Ranking.** `hybrid relevance × recency decay × importance multiplier`.
  Weighted merge of the two legs, 200 candidates per leg, keyword fill, 30-day
  half-life on dated files only, `minScore` applied before decay, MMR with
  lambda 0.7 over snippet-token Jaccard. No learned reranker, no query-time
  model call, by design ("local, fixed-rule ranking keeps the default
  dependency-free and predictable").
- **Failure posture.** A 30-second tool deadline returns the keyword matches
  with a partial-result warning naming the timeout. An explicitly configured
  provider that is unavailable "reports memory as unavailable instead of
  silently degrading." Dimension or config mismatch pauses search rather than
  serving mismatched data.
- **Context assembly.** `USER.md` and `MEMORY.md` are injected at session start
  under a bootstrap budget; an oversized `MEMORY.md` stays intact on disk and the
  injected copy is truncated. Trigger recall adds up to three compact entries
  before a reply, from promoted trusted entries only. A silent memory-flush turn
  runs before compaction. "Dreaming" promotes items into `MEMORY.md` behind
  score, recall-frequency and query-diversity gates.

### Hermes Agent (Nous Research)

- **Two always-loaded files with hard byte caps:** `MEMORY.md` at 2,200 chars
  and `USER.md` at 1,375 chars, injected as a frozen snapshot per session, which
  "preserves the LLM's prefix cache." A write that would exceed the cap returns
  an error; the agent must consolidate first. Duplicates are rejected. Entries
  are scanned for injection, exfiltration and invisible Unicode before
  acceptance.
- **Memory tool has no read verb** (`add`, `patch`, `remove` only) because
  content is injected. `replace` was removed because it wiped other facts in
  multi-fact entries.
- **`session_search`** over SQLite FTS5 returns "actual stored messages, no LLM
  summarization, no truncation," in discovery, scroll and browse shapes, about
  20 ms a query. The repo's earlier note that session recall used LLM
  summarisation is no longer true.
- **External providers** (RetainDB, ByteRover, Honcho, Mem0, Supermemory, and
  others) run alongside the built-in files and return full prefetched context
  up to a ceiling of ten times the output-spill limit, at least 100,000 chars.
  An opt-in spill mode replaces oversized recall with a preview plus a file
  path, which the docs admit "can hide the middle of the recall from the model."
- **Code-mode composition.** The README's headline mechanism: "Write Python
  scripts that call tools via RPC," which "collapses multi-step pipelines into
  zero-context-cost turns."
- **Background review** after a turn uses the main model from a warm prompt
  cache, deferred to idle time on local hardware.

### Letta (formerly MemGPT)

- Memory is **MemFS, a git-backed memory filesystem** the agent inspects and
  edits, shared across its conversations. `/doctor` audits placement,
  duplication and system-prompt token usage. Dreaming runs background subagents
  after a step count or on compaction. Core memory blocks are always in context;
  archival and recall tiers are searched on demand.

### Mem0

- `search(query, filters, top_k, threshold, rerank, explain)`. Scoping by
  user, agent and run lives inside `filters` as JSON logic. `explain=True`
  attaches `score_details`: semantic score, normalised BM25, entity boost, raw
  combined, **maximum possible score**, final, and the threshold used. Response
  shape is unchanged when off. Temporal reasoning runs internally for
  time-aware queries.

### Zep and Graphiti

- Hybrid semantic plus BM25 plus graph traversal "into a single ranked answer,"
  with "no LLM-in-the-loop reranking"; results can be reranked by distance from
  a named node. Facts carry `valid_at` and `invalid_at` for point-in-time
  queries; contradictions are handled by temporal edge invalidation. Data enters
  as episodes for provenance. Retrieval for agents is packaged as a prompt-ready
  **Context Block** shaped by context templates; the tool-building guide says
  "return compact results."

### Coding harnesses: Claude Code and Codex CLI

Observed first-hand in this session, so current rather than reported:

- `Grep` defaults to files-with-matches, caps content output at 250 lines with
  `offset`, offers context lines and a count mode. `Read` pages by `offset` and
  `limit` with a 2,000-line default. A Bash result over roughly 30 KB is written
  to a file and only a 2 KB preview enters context, with the path to read more.
  Tool schemas are deferred and loaded on demand by name. Subagents run
  searches in their own context and return a summary. `WebFetch` runs a small
  model over the page against a caller prompt and returns the answer, not the
  page.
- Anthropic's published tool-design guidance: consolidate tools; prefer
  `search_x` over `list_x` because agents "shouldn't spend context reading every
  record"; drop low-signal fields such as `uuid` and `mime_type`; map UUIDs to
  semantic names or a 0-indexed scheme, which "reduced hallucinations and
  improved retrieval precision"; offer a `response_format` of concise or
  detailed (concise used about one third of the tokens); combine pagination,
  range selection, filtering and truncation with sensible defaults; a default
  25,000-token cap on tool responses; when truncating, say how to get more;
  errors that say what to fix with an example.
- MCP 2025-11-25 adds `structuredContent` with an `outputSchema`, and resource
  links so a tool can return a URI to large data instead of inlining it.
- Codex CLI makes ripgrep a first-class tool, uses `apply_patch` diffs rather
  than whole-file rewrites, and intercepts the patch in the harness.

## 2. Where the repo's picture is out of date

`dev/design/0.8.0-agent-memory-fit.md` §8a and the July market report describe
these tools as of mid-2026. Differences that matter for interface design:

| Repo said | Current docs say |
| --- | --- |
| OpenClaw graph/world-model: "none (markdown)"; retrieval is hybrid plus MMR plus decay | Still no graph, but there is now per-chunk **trust class, supersession key and write-time importance** kept in SQLite apart from the Markdown, trigger recall, dreaming with promotion gates, partial-result warnings, and a visible-failure posture. The QMD engine is gone. |
| Hermes: "FTS5 + LLM summarization for cross-session recall" | `session_search` returns raw messages with **no LLM summarisation and no truncation**. Hard byte caps and a frozen per-session snapshot are the memory contract. Provider plugins and code-mode RPC are the extension points. |
| Letta: tiers (core, recall, archival) | Memory is a **git-backed filesystem** the agent edits, with `/doctor` token audits and sleep-time subagents. |
| Mem0: "search / store / get / list / forget" | `search` now carries `explain` with a full score decomposition including maximum possible score, and JSON-logic filters. |
| Market report: MCP server is the number-one gap for FathomDB | Still true, and MCP now standardises structured output and resource links, which bear directly on §4.2 and §4.11 below. |

## 3. Re-grade of the §4 list

### 3a. Hold, with fresh confirmation

| § | Property | Evidence |
| --- | --- | --- |
| 4.2 | Handles plus snippet, hydrate on demand | OpenClaw's `memory_search` then `memory_get` is exactly this. Claude Code `Grep` returns file paths by default. Anthropic: `search_` over `list_`. |
| 4.5 | Exact tokens alongside semantics | Every hybrid system cites the same rationale: OpenClaw names "IDs, error strings, and config keys"; Infino H5; Mem0 adds an entity boost. |
| 4.6 | One snapshot per plan | OpenClaw publishes rebuilt indexes atomically so searches read the last good index. Hermes freezes the injected memory per session. Zep and Graphiti answer point-in-time queries. |
| 4.7 | Provenance and validity on every hit | OpenClaw origin class and observation time; Graphiti episodes and `valid_at`/`invalid_at`; Mem0 timestamps. Confirmed, but the *handle* half of 4.7 needs revision (3c). |
| 4.12 | Bounded deterministic traversal | Zep reranks by node distance and runs no LLM in the loop. OpenClaw and Hermes avoid graphs entirely, which is its own data point: for personal memory, graph is optional. |
| 4.13 | Change-oriented queries | Letta's git-backed MemFS makes history and diffs native. OpenClaw's supersession key and dated files make "since" trivial. |

### 3b. Heightened in importance

- **§4.15 output designed for a per-token reader.** This is now the dominant
  design axis across every tool surveyed. Hard caps are everywhere: 2,200 chars,
  a bootstrap budget, 25,000 tokens, a concise mode at one third the size. Two
  sub-properties the original §4.15 did not name:
  - **Cache stability.** Hermes freezes the memory snapshot per session
    specifically to preserve the prefix cache, and its background review reuses
    a warm cache. Byte-stable, deterministically ordered output is now a cost
    property, not just a tidiness property. FathomDB's deterministic RRF and
    `graph.expand` already have this shape; any new verb should keep it.
  - **Say how to get more.** Every truncation path that works carries the
    continuation: a path and line range, an offset, a file to read. Claude
    Code's spilled Bash output and MCP resource links are the same move.
- **§4.8 errors that name the recovery.** Heightened and extended. Hermes
  returns an error on an over-cap write instead of silently dropping entries.
  OpenClaw fails visibly for an explicitly configured provider rather than
  degrading to keyword-only, and pauses on index mismatch rather than serving
  wrong data. The extension: **never degrade silently; label partial results.**
  OpenClaw's timeout path returns keyword hits with a warning that names the
  timeout and whether partial results exist. §4.3's "absent versus not ready"
  distinction has become a shipping contract.
- **§4.3 and §4.4 together: score decomposition is the shipping form of
  explain.** Mem0's `explain` returns the per-arm scores, the combined score,
  the **maximum possible score** and the threshold. That last field is the
  calibration signal §4.3 asked for: it lets the agent tell a confident hit from
  best-of-noise without a second query. OpenClaw returns a final `score` with
  `minScore` semantics documented. Negative explain is still absent everywhere.
- **§4.10 compound requests, now as code.** Hermes's "Python scripts that call
  tools via RPC" with "zero-context-cost turns" is a stronger mechanism than the
  compound envelope §4.10 proposed: the agent writes a short program, the
  harness runs it, only the final value enters context. Anthropic's guidance to
  consolidate multi-step operations into one tool is the static form. See §3d
  N1.

### 3c. Need revision

- **§4.7, the handle half.** The original text asked for a typed id on every
  result. Anthropic's measured finding is that opaque UUIDs increase
  hallucination and that semantic names or a 0-indexed scheme reduce it.
  OpenClaw's handles are human-legible paths and line ranges, and its own docs
  record the trap when two verbs use incompatible handle spaces ("memory-search
  line numbers are not session-history offsets"). Revised property: **handles
  must be short, legible, uniform across every verb, and stable within a
  snapshot.** Opaque identifiers belong in a `detailed` response for chaining,
  not in the default. FathomDB's `IdSpace` prefixes (`l:`, `h:`, `p:`) are
  half-way: legible kind, opaque value. The uniformity requirement is met; the
  legibility of the value is not.
- **§4.11 named working sets.** No surveyed tool keeps a server-side named
  result set. The shipping substitutes are: a subagent with its own context
  (Claude Code, Hermes, Letta dreaming), a spilled file with a preview and a
  path (Hermes, Claude Code), an MCP resource link, or a script that holds
  intermediate state in variables. Revised property: **spill to a resource the
  agent can page through**, which is §4.2's hydration applied to the result
  set itself. The Hermes caveat that a preview "can hide the middle" is the
  design constraint: the resource must support ranged reads, not just a head.
- **§4.16 constrained SQL escape hatch.** No agent-facing memory tool exposes
  SQL to the model. The escape hatch that shipped is **code execution over the
  SDK** (Hermes RPC scripts, Claude Code running Python or shell against local
  libraries). Infino's SQL-primary bet (H4) is the exception and is aimed at a
  lake, not at agent memory. Revised: the escape hatch for an embedded store is
  a scriptable SDK behind the same governed verbs, not a query language.
- **§4.1 self-describing catalogue.** Still right for stores with a real schema,
  but the memory tools show the degenerate case: when the schema is a handful of
  files, the catalogue *is* the always-loaded compact layer (`MEMORY.md`,
  core memory blocks). Revised framing: **progressive disclosure**, a small
  always-present orientation layer under a hard budget, with the full catalogue
  and the data behind on-demand verbs. Letta's `/doctor` and OpenClaw's
  `memory status` are the operator-side views of the same thing.
- **§4.9 estimate before execute and §4.14 aggregates.** No memory tool ships
  a dry-run or count verb; strict default limits and cheap thresholds do the
  job. Both remain correct for analytics-shaped stores (Infino H3, H4) and are
  downgraded for memory-shaped ones.

### 3d. Missing from §4, now discovered

- **N1. Scriptable composition.** Let the agent run a short program against the
  query surface so intermediate results never enter context (Hermes RPC;
  Anthropic's consolidation guidance; Claude Code subagents as the heavyweight
  form). For FathomDB this is nearly free: the Python and TypeScript SDKs are
  already the surface; what is missing is a sandboxed, read-only invocation
  path an agent harness can call. Supersedes the §4.10 envelope idea.
- **N2. Trust class on every record, carried through retrieval.** OpenClaw
  stores `owner` / `agent` / `untrusted` / `system` beside each chunk, outside
  the content, and injects only promoted trusted entries automatically. Hermes
  scans writes for injection and invisible Unicode. Retrieval results are
  untrusted input to the model; the interface must say *how* untrusted each hit
  is and let the caller filter on it. FathomDB has `source_id` provenance but no
  trust classification. This is the largest genuinely new gap.
- **N3. Budgeted summary view with a hard cap and an explicit overflow policy.**
  Both OpenClaw and Hermes keep a curated layer under a byte budget, and they
  chose opposite overflow rules (truncate the injected copy versus refuse the
  write). An interface should make the budget and the policy explicit rather
  than implicit. Relates to §4.2 and §4.15 but is a distinct verb: "give me the
  current state of X in at most N tokens."
- **N4. Recency and importance as ranking inputs, with write-time scoring.**
  OpenClaw's `relevance × recency decay × importance`, following Generative
  Agents, with importance scored once at write time so no model runs at query
  time. The repo recorded this as G12 in 2026-06 and FathomDB's recency seam is
  still shipped off. For agent memory this has moved from differentiator to
  default.
- **N5. Recall feedback into the store.** OpenClaw's dreaming promotes entries
  behind recall-frequency and query-diversity gates, which means the store
  records what was recalled and by which queries. Hermes's background review
  replays the conversation to decide what to save. The query interface is the
  natural place to accept "this hit was used" so the store can learn. FathomDB's
  real-gold telemetry capture is the adjacent primitive.
- **N6. Client-side loop closure, revisiting Infino H6.** Infino proposed
  putting small models inside the engine to judge and reformulate. The agent
  harnesses already do this on their side: `WebFetch` runs a model over the
  page, subagents search and summarise, OpenClaw's dreaming consolidates
  offline. The engine's job is therefore to emit the signals (§4.3, §4.4) and
  stay deterministic; the model in the loop belongs to the harness. H6 is
  confirmed as a need and relocated out of the store.
- **N7. Memory as files the agent can read, diff and edit.** Three of the four
  memory systems chose plain files as the memory of record (OpenClaw Markdown,
  Hermes Markdown, Letta git). The stated reasons are inspectability ("no
  hidden state"), prefix-cache stability, and the agent's existing fluency with
  file tools. An embedded store that wants to be the memory of record must offer
  an equally legible read of its curated layer, or position itself as the index
  beneath the files, which is where OpenClaw already puts SQLite.

## 4. Consequences for the FathomDB mapping in §5 of the first note

Candidates only; nothing here commissions work.

| Change | Status after this pass |
| --- | --- |
| §4.7 handles | Downgrade from "has" to "partial": `IdSpace` values are opaque; a legible alias or 0-indexed per-response scheme in a concise mode would meet the revised property. |
| §4.11 working sets | Reframe the gap: not a server-side set but a pageable spill resource; falls out of an MCP adapter that returns resource links. |
| §4.16 escape hatch | Reframe: a sandboxed read-only scripting path over the SDK, not SQL. |
| N2 trust class | New gap, high. A per-record origin class beside `source_id`, filterable in `Filter`, surfaced on every hit. |
| N4 recency and importance | Existing gap, raised. The shipped-off recency seam and absent importance field are now below the memory-tool floor. |
| N3 budgeted summary view | New candidate verb. |
| N5 recall feedback | New candidate, builds on real-gold telemetry. |
| MCP adapter | Unchanged as the top market gap; now also the vehicle for §4.2 concise mode, §4.15 truncation-with-continuation, and §4.11 resource links. |

## 5. Net assessment

Of the sixteen §4 properties, six hold with fresh confirmation, four are
heightened, five need revision in framing, and one (§4.9 with §4.14) is
downgraded for memory workloads. The revisions share one direction: shipping
agent tools favour legible, file-like, cache-stable, budgeted surfaces over
clever server-side state. Two properties the first note missed are now plainly
table stakes for agent memory: a trust class on every record, and recency and
importance in ranking. One mechanism the first note under-weighted, scriptable
composition, is the strongest answer to the round-trip problem in current use.
The engine-side substrate FathomDB holds (snapshots, provenance, supersession,
bounded traversal, deterministic fusion) remains the right foundation; the
re-grade moves the priority of what to build on it.

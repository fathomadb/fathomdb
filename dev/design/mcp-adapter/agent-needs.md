---
title: Agent Needs for a memory store
date: 2026-10-17
status: PROPOSED
desc: Outcome-level needs of an LLM agent that uses a memory store through a tool boundary. Input to requirements.md and architecture.md in this directory.
inputs:
  - dev/notes/llm-agent-query-interfaces.md
  - dev/notes/agent-tools-regrade-of-query-interfaces.md
  - dev/notes/infino-hypotheses-vs-agent-query-interfaces.md
  - dev/notes/agent-datastore-priorities-critical-review.md
  - dev/notes/mcp-rust-sdk-selection.md
---

# Agent Needs for a memory store

The persona is an **LLM agent** working through a tool boundary: it reads the
store only through tool calls, pays for every returned byte in context, keeps
no memory outside its context window, and acts on what it retrieves. The
application engineer who embeds FathomDB (`dev/needs.md` primary persona)
remains the owner of the data; the agent is a reader and, by permission, a
writer.

Conventions:

- Each need is a stable id `AN-###` and one falsifiable outcome statement:
  what the agent must be able to do or know, never how.
- Where a need depends on a measured quantity, the threshold is owned by
  `requirements.md`; the need names what is measured.
- "Observed" records the evidence from the 2026-10-16/17 working session or the
  notes listed above. Needs without a direct observation say so.
- Ids were renumbered once, in review cycle FIX-1, before anything outside
  this directory cited them. From FIX-1 on they are stable: a removed need
  keeps its id marked retired, and ids are never reused.

## Trust

**AN-001 — Know how far to trust each result.** For every result, the agent can
tell where it came from, what class of origin wrote it, and whether it was
written by a party allowed to assert that class — without reading the content
to find out.
*Observed:* OpenClaw keeps an origin class beside each chunk; the session
harness labels subagent output as carrying no user authority. FathomDB results
carry `source_id` but no origin class.

**AN-002 — Content cannot raise its own trust.** Nothing a writer puts in
content — an agent's, or an ingested external document's — can change the
origin class reported for it, and retrieved content is never presented to the
agent as instructions.
*Observed:* OpenClaw's stated reason for keeping trust outside the Markdown:
"recalled prose cannot rewrite its own trust classification."

**AN-003 — Know whether a fact is still current.** For every result, the agent
can tell whether it is still valid and whether something newer replaced it.
*Observed:* the session's memory loader warns that entries "reflect what was
true when written"; `AGENTS.md` records ten citations to memory files that
never existed. Both are stale-reference failures, and both were caught only by
re-checking against the source.

## Finding

**AN-004 — The answer is in the first few results.** On a designated
agent-memory evaluation corpus, the record that answers a question ranks within
the first k results at a measured rate, including when the query contains an
exact token (an id, an error string, a quoted phrase) that must match
literally.
*Observed:* the session located every target by exact names (branch, ADR,
symbol); `dev/design/fathomdb-graph-vs-mem0-zep-and-longmemeval-diagnosis.md`
records FathomDB's fusion trailing plain BM25 on conversational data.

**AN-005 — Orient cheaply.** Before its first real query, the agent can learn
what the store holds — kinds, rough sizes, time range, which indexes are ready —
at a cost small against its working context.
*Observed:* the session's `MEMORY.md` index, loaded at start under a size cap,
was what made the rest of the memory usable.

**AN-006 — Ask about a point in time.** The agent can ask what the store held
as true at a stated instant, not only now.
*Observed:* no direct observation; the engine is bitemporal (`ReadView`
validity axis) and Zep/Graphiti ship point-in-time queries.

## Stopping

**AN-007 — Know whether a result set is complete, and why not.** The agent can
tell whether it saw everything that matched, a truncated subset, or nothing;
and, when retrieval was partial — an index not ready, an embedder unavailable,
a deadline hit — which parts ran and which did not, in the same response.
*Observed:* every surveyed tool converged on labelled partial results; the
session repeatedly proved absence (no MCP files tracked, branch not merged) by a
second query. (Merges former AN-005 completeness and AN-007 degradation.)

**AN-008 — Know whether a hit is a confident match.** On a calibration corpus,
the agent can distinguish a strong match from the best of a weak field at a
measured error rate, without a second query.
*Observed:* Mem0's maximum-possible-score field is one shipping form.

## Context economy

**AN-009 — Spend context deliberately.** The agent receives no more content than
it asked for, can choose what to read in full, and, when a response was cut
short, knows exactly how to get the rest.
*Observed:* a 59 KB hook output in this session was spilled to a file with a
2 KB preview and a path; `Read` and `Grep` page by offset.

**AN-010 — References it can repeat correctly.** Every reference to a record that
the store returns can be passed back to any operation that takes a record, and
is short and legible enough to transcribe without error.
*Observed:* every reference in this session was a path, branch or `file:line`;
Anthropic's tool guidance measured fewer hallucinations with legible ids.

**AN-011 — Identical questions cost the same.** Asking the same question of the
same store state yields byte-identical output, so the harness's prompt cache is
not invalidated by incidental differences.
*Observed:* no direct observation in this session; Hermes freezes its memory
snapshot per session specifically to preserve the prefix cache.

**AN-012 — Bounded time per call.** Every call returns within a bounded time,
with whatever it has and a label, rather than blocking the agent's turn.
*Observed:* OpenClaw's 30-second tool deadline returns keyword matches with a
partial-result warning.

## Multi-step work

**AN-013 — Know when the world moved.** Across the calls that make up one plan,
the agent can tell whether consecutive calls saw the same store, including what
was searchable. When they did not, it learns, where knowable, whether records
changed or only indexing progressed. It is never given one response that mixes
two states.
*Observed:* the repo's memory records a shared checkout whose branch was stale
against the session's assumption; "verify state from git" became a rule.

**AN-014 — Few round trips.** The agent can complete a multi-part lookup — many
records by reference, or a lookup and its follow-up — in few calls.
*Observed:* the session issued independent lookups as parallel tool calls and
delegated multi-step research to subagents to keep intermediate output out of
context.

**AN-015 — Recover from a wrong identifier in one step.** When the agent names
a kind, field, or record that does not exist, it learns that, learns the
nearest valid alternatives, and can tell "invalid request" from "valid, no
match" from "valid, not ready".
*Observed:* `AGENTS.md` warns that a wrong subagent type "is a hard error that
cascades"; the guard is enumerating valid values.

**AN-016 — Ask what changed.** The agent can ask what changed since a point it
recorded earlier, instead of re-searching everything.
*Observed:* the session started from a "changes since last session" summary
and began by finding "a branch that was added today."

## Writing

**AN-017 — Remember safely.** When permitted, the agent can record a new fact,
replace an existing one, and retract one; its own origin is recorded without
its say; and recording the same fact twice does not create two.
*Observed:* the session's memory rules: check for an existing entry and update
it rather than duplicate; delete memories that turn out to be wrong.

## Reach and scope

**AN-018 — Reach the store the application uses.** The agent's harness can
attach, through the standard agent tool protocol and with no network service
or bespoke glue, to the same store the owning application reads and writes,
with nothing on the protocol channel except protocol messages.
*Observed:* the session's harness discovers tools by name over MCP; FathomDB's
consumers (Memex, OpenClaw, Hermes) own their stores; the July market report
names the missing MCP server as the largest integration gap. *Constraint:* the
sidecar lock lets only one process open a database file.

**AN-019 — Stay inside its scope.** An agent sees only the records the operator
scoped it to (for example one user's or one project's), and cannot widen that
scope through any request. Scope is distinct from origin class.
*Observed:* no direct observation in this session; ADR-0.8.25 anticipates
owner and scope mapped through declared exact projections.
*Tension, accepted:* an out-of-scope record must look absent, so for it the
agent cannot tell "valid, no match" apart from "outside my scope" (AN-015).
Hiding existence wins.

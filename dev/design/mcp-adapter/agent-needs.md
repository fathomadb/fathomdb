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

Conventions: each need is a stable id `AN-###` and one falsifiable outcome
statement. Needs say what the agent must be able to do or know, never how. The
"observed" line records the evidence from the 2026-10-16/17 working session
and the notes listed above.

## Trust

**AN-001 — Know how far to trust each result.** The agent can tell, for every
result, where it came from, what class of origin wrote it (owner, agent,
untrusted external, system), whether it is still valid, and whether something
newer replaced it — without reading the content to find out.
*Observed:* the session's own memory loader warns that entries "reflect what
was true when written"; `AGENTS.md` records ten citations to memory files that
never existed. The costliest agent failure is a confident wrong hit, not a
miss.

**AN-002 — Content cannot raise its own trust.** Nothing an agent or an
external source writes can make itself appear to come from a more trusted
origin, and retrieved content is never presented as instructions.
*Observed:* OpenClaw keeps trust class outside the content because "recalled
prose cannot rewrite its own trust classification"; the session harness labels
subagent output as carrying no user authority.

## Finding

**AN-003 — The right record is in the first few results.** For a question the
store can answer, the answering record appears in the top few results, including
when the agent's query contains an exact token (an id, an error string, a quoted
phrase) that must match literally.
*Observed:* the session located every target by exact names (branch, ADR,
symbol); the repo's own diagnosis records fusion trailing plain BM25 on
conversational data.

**AN-004 — Orient cheaply.** The agent can learn what the store holds — kinds,
rough sizes, time range, which indexes are ready — in one small response,
before its first real query.
*Observed:* the session's `MEMORY.md` index, loaded at start under a size
budget, was what made the rest of the memory usable.

## Stopping

**AN-005 — Know whether a result set is complete.** The agent can tell whether
it saw everything that matched, a truncated subset, or nothing; and whether
"nothing" means absent, not yet indexed, or filtered out.
*Observed:* every surveyed tool converged on labelled partial results; the
session repeatedly had to prove absence (no MCP files tracked, branch not
merged) by a second query.

**AN-006 — Know whether a hit is a confident match.** The agent can tell a
strong match from the best of a weak field without a second query.
*Observed:* Mem0's maximum-possible-score field is the shipping form of this.

**AN-007 — Never be misled by silent degradation.** When part of the retrieval
could not run (an index not ready, an embedder unavailable, a deadline hit),
the agent is told, in the same response, what ran and what did not.

## Context economy

**AN-008 — Spend context deliberately.** The agent receives short handles and
snippets first and fetches full content only for what it chooses, within a
budget it sets; any truncation tells it exactly how to get the rest.
*Observed:* a 59 KB hook output in this session was spilled to a file with a
2 KB preview and a path; `Read` and `Grep` page by offset.

**AN-009 — Handles it can repeat correctly.** Handles are short, legible, and
the same across every operation, so the agent can pass a handle from one result
into the next call without transcription errors.
*Observed:* every reference in this session was a path, branch or `file:line`;
Anthropic's tool guidance measured fewer hallucinations with legible ids.

**AN-010 — Identical questions cost the same.** Asking the same question of
unchanged data yields byte-identical output, so the harness's prompt cache is
not invalidated by incidental ordering or formatting differences.

## Multi-step work

**AN-011 — One consistent world across a plan.** A sequence of calls that make
up one plan either sees one state of the store or is told that the state
changed underneath it.
*Observed:* the repo's memory records a shared checkout whose branch was stale
against the session's assumption; "verify state from git" became a rule.

**AN-012 — Few round trips.** The agent can express a multi-part lookup in few
calls, and the intermediate results of a pipeline need not pass through its
context.

**AN-013 — Recover from a wrong identifier in one step.** When the agent names
a kind, field, or handle that does not exist, the error says so, names the
nearest valid candidates, and distinguishes "invalid request" from "valid, no
match" from "valid, not ready".
*Observed:* `AGENTS.md` warns that a wrong subagent type "is a hard error that
cascades"; the guard against guessing is enumerating valid values.

**AN-014 — Ask what changed.** The agent can ask what changed since a point it
recorded earlier, instead of re-searching everything.
*Observed:* the session started from a "changes since last session" summary
and began by finding "a branch that was added today."

## Writing

**AN-015 — Remember safely.** When permitted, the agent can record a new fact,
correct or supersede an existing one, and retract one, with its own origin
recorded automatically, and without creating a duplicate of something already
stored.
*Observed:* the session's memory rules: check for an existing entry and update
it rather than duplicate; delete memories that turn out to be wrong.

## Reach

**AN-016 — Zero-integration reach.** The agent's harness can attach the store
as a tool through the standard agent tool protocol, launched locally with no
network service, no bespoke glue code, and nothing on the protocol channel
except protocol messages.
*Observed:* the session's harness discovers and loads tools by name over MCP;
the July market report names the missing MCP server as FathomDB's largest
integration gap.

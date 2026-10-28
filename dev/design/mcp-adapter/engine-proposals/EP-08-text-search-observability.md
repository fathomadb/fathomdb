---
title: EP-08 — Telemetry, explain and lifecycle events on text-only search
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Give search_text_only the same observability as hybrid search, which means telemetry capture with a per-search query id, an opt-in explanation, and query counters and lifecycle events, so feedback and explain work on the lexical path.
---

# EP-08 — Telemetry, explain and lifecycle events on text-only search

Status: Draft. Priority P1. Not an accepted decision.

## Problem

- Hybrid search finishes through `finalize_search_observability`
  (`fathomdb-engine/src/search_api.rs:1219-1222`). That captures telemetry, and
  when `explain` is set it puts the query id into
  `explanation.correlation_id` under the sink lock (`telemetry.rs:184-206`).
- `search_text_only_view_with_limit` (`search_api.rs:1026-1085`) returns
  straight from the reader. It does not capture telemetry, emit
  `Started`/`Finished`/`Failed` lifecycle events, call `counters.record_query`,
  or run slow-query detection. The hybrid path does all four
  (`search_api.rs:1204-1235`).
- `TextSearchOptions` has only `view` and `limit`
  (`fathomdb-sdk/src/engine.rs:214-222`). There is no `explain` input.

The adapter's default mode is `text` (interface §2), because text mode is the
path that must clear the MR-021 quality floor. So on the default path:

- feedback always returns `not_recorded/text_mode`
  (`src/rust/crates/fathomdb-mcp/src/tools.rs:1159-1161`);
- `explain` returns `unavailable/text_mode` (`tools.rs:753-757`);
- an operator's subscriber and counters never see the adapter's searches.

## Proposed surface

- `TextSearchOptions { view, limit, explain: bool }`, with `explain` defaulting
  to `false`. That is an in-place SDK change in all three bindings.
- `search_text_only*` calls the same `finalize_search_observability` and the
  same event, counter and slow-detection wrapper as hybrid search.
- For `explain=true`, `Explanation` carries the text-only trace: `text_hits`,
  per-hit `text_rank` and arm, and the score the hit is returned with. The
  vector and graph fields are `None` or 0. The struct is `#[non_exhaustive]`, so
  no field is removed. How the text path computes its returned `score` was not
  checked for this proposal.
- Telemetry events record each hit's own `branch` in `arm_of`, which is the
  existing mapping (`telemetry.rs:218-235`).
- No new errors.

## Invariants and consistency

- With `explain=false` and telemetry off, `results` and `projection_cursor`
  bytes are unchanged. That is the R-OBS-2 zero-cost rule
  (`search_types.rs:143-150`).
- With telemetry on, each text search consumes one `seq`, so ids interleave
  with hybrid searches on the same sink. Uniqueness depends on EP-02, not on the
  path.
- Correlation ids come from the sink lock, as on the hybrid path, so concurrent
  searches cannot swap them.

## Interaction with ADRs

- `dev/design/0.8.8-explain-and-telemetry-adr.md` (RATIFIED) scopes capture to
  "each `search`". Text-only search was left out by implementation, not by a
  ratified decision as far as I could find. If the owner reads it as a
  decision, this needs an amendment.
- `ADR-0.8.25-governed-dependency-trace-and-integrity.md` added the
  content-free explained-search correlation. This extends it to one more entry
  point.
- `ADR-0.8.27-rust-sdk-parity.md`: the `TextSearchOptions` field lands in all
  three SDKs.

## Cost and performance

When telemetry is off and `explain=false`, the cost is one atomic load
(`telemetry.rs:173-178`). Lifecycle events and counters add the same per-call
cost hybrid search already pays. The text-query latency gates
(`ADR-0.7.0-text-query-latency-gates-revised.md`) must be re-measured, because
the text path now emits events.

## Test plan sketch

- Real engine with telemetry on: a text search appends one `event` with
  `q{nonce}-{seq}`, and `record_feedback` on that id succeeds.
- `explain=true` returns `correlation_id` equal to that query id, with
  `text_rank` 1..n in result order.
- A subscriber attached with `attach_subscriber` sees `Started` and `Finished`
  for a text search.
- Byte test: default text search output is identical before and after the
  change.

## Adapter change once it lands

In text mode, run with `explain` forced on while telemetry is enabled, as hybrid
does (`tools.rs:640-643`), and take the query id the same way. Remove the
`text_mode` reason from `feedback` and from `explanation`. MR-024.1 then holds
for both modes.

## Open questions for the owner

1. Should text-only search count toward the same `query` counter as hybrid, or
   get its own?
2. Is telemetry capture on the lexical path in scope for the 0.8.8 privacy
   review, which approved "ids, arms, and query length" only? It records the
   same fields.

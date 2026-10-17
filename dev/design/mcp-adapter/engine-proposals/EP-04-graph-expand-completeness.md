---
title: EP-04 — graph.expand truncation signal and kind-aware work accounting
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal. Make GraphExpandResultV1 say when result_limit cut eligible targets, and stop charging edges that edge_kinds or supersession would discard against max_work_units, so hub nodes stop failing GraphExpansionBoundExceeded.
---

# EP-04 — graph.expand truncation signal and kind-aware work accounting

Status: Draft. Priority P1. Not an accepted decision.

## Problem

This is designed behaviour, not a bug. The Slice 60 design
(`dev/plans/0.8.25/features/slice-60/design.md:548-575`) says `complete=true`
means "the exact top N of the complete bounded walk; it does not mean every
matching target fit". It also says work is "charged before all semantic filters
so a high-degree node cannot hide unbounded scanning". The code follows that
design, and the result is two gaps for an agent.

- **No truncation signal.** Once `result_limit` candidates exist at the terminal
  depth, later edges are skipped (`graph_expand/execution.rs:854-858`), better
  candidates replace worse ones (`execution.rs:895-920`), and the set is
  truncated (`execution.rs:940`). The result always sets `complete: true`
  (`execution.rs:1065`), and the codec rejects `false` as `GraphCorrupt`
  (`codec.rs:1226-1227`). A caller cannot tell "exactly N exist" from "more
  exist".
- **Filters do not reduce work.** The incident query selects every edge on the
  endpoint, of any kind, superseded or not
  (`execution.rs:484-491`). Each row costs one unit before the `edge_kinds`
  check and before the `superseded_at`/`t_invalid` check
  (`execution.rs:836-845`). If one frontier node's raw incident count is above
  the remaining budget, the call fails with `GraphExpansionBoundExceeded`
  (`execution.rs:784-801`). A hub with 10,001 edges fails even when the caller
  asked for one rare edge kind, and edge history counts against the budget
  forever.

Adapter workarounds removed:

- `bound: "result_limit"` is inferred from `targets.len() >= limit`
  (`src/rust/crates/fathomdb-mcp/src/tools.rs:1048-1055`). That is a false
  positive whenever exactly `limit` targets exist.
- The interface tells agents that `edge_kinds` "do not help" with
  `bound_exceeded` (`dev/interfaces/mcp.md` §4 `expand`).

## Proposed surface

Change `GraphExpandResultV1` in place (`ADR-0.8.26-...` forbids a V2):

- Add `truncated_by_result_limit: bool`. It is true when at least one eligible
  target was skipped or evicted because of `result_limit`. `complete` keeps its
  meaning: the walk finished within the work bound.
- Push `edge_kinds` and active-edge eligibility into the incident SQL:
  `... AND kind IN (...) AND superseded_at IS NULL`. Charge only the rows that
  query delivers. A row is still charged before node and target-kind filters,
  so the design's anti-hiding property holds for what is scanned.
- Index support: `canonical_edges(from_id, kind)` and `(to_id, kind)`, as
  additive `CREATE INDEX`. Today only `from_id` and `to_id` indexes exist
  (`fathomdb-schema/src/lib.rs:376-379`). Without them the kind predicate filters
  rows after the endpoint probe, and the scan is not bounded by delivered rows.
- Errors do not change. `GraphExpansionBoundExceeded` keeps its "no partial
  result" rule.

## Invariants and consistency

- Returned targets and their order are unchanged for any request with no
  `edge_kinds` and no superseded edges.
- With `edge_kinds` given, results are unchanged too. The skipped rows were
  already discarded at `execution.rs:838-840`. Only the work count, and so
  `bound_exceeded`, changes.
- `work_units` (already returned, `graph_expand/types.rs:180`) keeps meaning
  "rows delivered to the enumerator". Its value drops for kind-filtered
  requests.
- Frozen graph evidence encodes `max_work_units` in its request digest
  (`execution.rs:1114-1115`). The evidence semantics need re-review, because
  the same request now does different work.

## Interaction with ADRs

- `ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md`: in-place V1
  change, fresh databases only, so the new index needs no data migration.
- `ADR-0.8.26-exact-graph-artifact-evidence.md` and the Slice 60 design own
  graph semantics. The work-accounting change alters a stated rule, so it needs
  a successor or amending ADR. The truncation flag alone is additive.

## Cost and performance

Kind-filtered expansions on hubs get cheaper. The two indexes add write
amplification on every edge insert. The existing graph latency fixtures must be
re-measured under the paired-p95 rule of
`ADR-0.8.25-frozen-pagination-and-operational-state.md`.

## Test plan sketch

- Real database: a seed with exactly 20 eligible targets and `result_limit=20`
  reports `truncated_by_result_limit=false`. With 21, it reports `true`.
- A hub with 10,001 `mentions` edges and 3 `cites` edges, expanded with
  `edge_kinds=["cites"]`, succeeds with `work_units=3`.
- Superseding 10,001 edges on one node and then expanding over its active edges
  succeeds.
- Codec round-trip property test over the new field, in Rust, Python and
  TypeScript.

## Adapter change once it lands

`bound: "result_limit"` comes from the flag. Remove the length inference and the
"edge_kinds do not help" wording. Report `work_units` in the response so agents
can size the next call.

## Open questions for the owner

1. Accept moving `edge_kinds` and supersession before the charge, which amends
   the Slice 60 accounting rule? Or keep the rule and add only the flag?
2. Add the two composite indexes, or accept unbounded endpoint scans for
   kind-filtered requests?

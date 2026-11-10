---
title: "Slice 90: implement approved residual Block 2 behavior"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90: implement approved residual Block 2 behavior

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Current checkpoint scope

Conditional residual build scope approved in Slice9 Batch4: accepted80 AND Block1 follow-up exit are hard gates. Build only accepted residuals; preserve existing signals/storage.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Evidence, goal and boundary

Implement only missing behavior explicitly approved by Slice 80 on the current
format-37 integration tip. This plan is a proposal; it authorizes no code now.
The [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md)
and [release ladder](../../../plan-0.8.29.md) establish the residual scope.
Inspected [bounded hints design](../../../../design/agent-access/hints-sidecar.md),
[accepted hints ADR](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md),
[long-record guard](../../../../design/long-record-embedding.md) and
[compact output design](../../../../design/agent-access/compact-text-block.md).
Code evidence: MCP [tools](../../../../../src/rust/crates/fathomdb-mcp/src/tools.rs),
[output](../../../../../src/rust/crates/fathomdb-mcp/src/output.rs) and
[errors](../../../../../src/rust/crates/fathomdb-mcp/src/error.rs); engine
[span validation](../../../../../src/rust/crates/fathomdb-engine/src/vector_spans.rs),
[span publication](../../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs),
[snippets](../../../../../src/rust/crates/fathomdb-engine/src/snippet.rs) and
[provider contract](../../../../../src/rust/crates/fathomdb-embedder-api/src/lib.rs).
No product tests were run for this draft; inclusion is not qualification.

| Row | Observed maturity | Proposed implementation delta, subject to 80 |
| --- | --- | --- |
| B2-1 | Required unavailable state/verdict placeholders remain | Apply approved omission/optional semantics and orient disclosure in both formats |
| B2-2/3 | Empty-result/degradation hints built | Preserve and qualify; consume 70's accepted reason vocabulary |
| B2-4 | Generic serde parse error remains | Approved accepted-argument details and authoritative schema-derived names |
| B2-5 | Feedback rejects foreign refs generically | Approved bounded offending-ref details, preserving request-order semantics |
| B2-6 | Row default markers and default-store hint built | Only approved top-level/default text/instruction gaps still missing after 60 |
| B2-7 | Admission already waits before busy | Regression qualification; no replacement queue or retry-after field |
| B2-8 | Span table, cleanup/validation and default-unknown offset exist | Approved orient coverage and per-hit scored-extent reporting with binding parity |

Overflow chunking, ANN, new ranking thresholds and SDK hints-default changes
remain outside this slice. `weak_results` stays reserved. No storage rewrite,
legacy migration or broader EP-03 adoption follows from B2 allocation.

## Preparation and local draft contracts

Produce `requirements.md`, `design.md` and `test-map.md` beside this plan before
implementation; IDs below are release-local drafts, not canonical AC additions.

- N90-1: callers can distinguish unavailable capabilities from real failures.
- N90-2: callers can correct invalid arguments/refs and use the session default.
- N90-3: callers can assess represented vector content without fabricated coverage.
- R90-1 / A90-1: approved B2-1 fields are omitted/disclosed consistently in text
  and structured modes; schema and serialized bytes agree on actual presence.
- R90-2 / A90-2: approved B2-4/5 errors name accepted keys/offending refs with
  deterministic ordering, UTF-8-safe bounds and omission counts where specified;
  successful calls allocate no error details and errors contain no record body.
- R90-3 / A90-3: default-store field, text and instructions agree with session
  state through default changes, no-default sessions and hints-disabled calls.
- R90-4 / A90-4: coverage/scored extent obey 80's denominator, identity, view,
  snapshot and unknown rules; unknown is distinguishable from zero and complete.
- R90-5 / A90-5: existing hints, budgets, paging, queue and shutdown remain valid
  in both output modes; snippet opt-out retains its approved cost/shape contract.
Architecture reading includes SDK [exports](../../../../../src/rust/crates/fathomdb-sdk/src/lib.rs),
[Python native snippets](../../../../../src/rust/crates/fathomdb-py/src/snippet.rs),
[N-API snippets](../../../../../src/rust/crates/fathomdb-napi/src/snippet.rs),
[Python types](../../../../../src/python/fathomdb/types.py) and
[TypeScript types](../../../../../src/ts/src/search.ts). All need coordinated review.
Challenge the design against current native conversions, projection generations,
format-37 visibility publication and fusion identity; do not infer byte extent
from tokenizer limits, body length, snippet windows or a different record's span.
Planned primary research: inspect pinned serde/schemars error/schema APIs and
current provider tokenizer-offset implementations only where 80 leaves a concrete
implementation uncertainty. Record versions/source locations and bounded findings.
Fit review revises the delta against integrated 50/60/70 and 80's signed design;
return contradictions to 80 rather than inventing a new approved field layout.

## Dependencies, prerequisites and isolation

Hard gate: exact approved Slice 80 requirements/design revision AND the recorded
Block 1 follow-up exit after integrated selected 20–70. That exit includes required
full/feature/performance checks, both-format conformance and approved smoke/item-4
remeasure. It is an unnumbered milestone; final Slice 160 is downstream of 90.
Slice 10 must provide approved release setup and checkout-owned native tooling.
Missing prerequisites: 80's final wire/public field layouts, bounds/precedence,
coverage denominators, unknown aggregation, view/time behavior, projection/provider
identity and snapshot provenance; a bounded no-scan orient strategy; approved
scored-extent applicability for text-only, hybrid and vector hits. Historical
`scored_bytes` is design input, not permission to assume its final type or layout.
Record those decisions before coding; B2-8 and text-default approval stay settled.
Prefer implementation after 50/60/70 land to avoid redoing identity/rendering work.
Use one fresh implementer worktree after preflight, one writer per checkout;
serialize engine/interface/native/state writes with other active slice landings.

## Work, TDD and deliverables

1. Freeze a code-grounded built/residual matrix and approved contract revision;
   map each residual to owner needs, test names and affected public interfaces.
2. Implement approved B2-1/4/5/6 deltas as separate RED then GREEN units, retaining
   existing 2/3/7 behavior; record genuine failing assertion evidence at each RED.
3. Implement B2-8 reads/reporting under the approved snapshot/view authority;
   preserve null unknowns, known empty/full/truncated extents and UTF-8 boundaries.
   Join by approved record/vector/projection identity, including equal-body records;
   test reproject, rollback, visibility changes and stale generation rejection.
4. Carry approved public types through Rust SDK, PyO3/Python, N-API/TypeScript and
   MCP formats; amend affected interfaces/designs in the same reviewed change.
5. Add real-database RED cases for unknown/mixed-known coverage, empty bodies,
   hidden/inactive/superseded rows, frozen concurrent snapshots and provider changes.
   Property tests generate multibyte bodies/valid extents and assert byte bounds,
   unknown preservation and reference/serialization round trips; use human intent,
   never regenerated golden oracles. Add shutdown/queue cancellation regressions
   only for changed behavior; preserve existing admission/lifecycle tests as controls.
6. Deliver reviewed implementation, contract/test map and verification receipts
   identifying exact source/native artifacts, settings and any unresolved failures.

## Verification, review and stop points

Run focused lint/typecheck/tests per unit; validate actual checkout-owned Python
and Node native artifacts plus Rust SDK parity, not just wrapper mocks. Exercise
both MCP formats, errors, hint toggles, budget/paging and stdio/shutdown/queue paths.
Run the required full agent gate once on the final code phase; missing owned tools
is a preflight failure. Run scoped Markdown/interface checks for contract edits.
Obtain independent requirements/design challenge before code and adversarial code
review before landing; audit RED shas and require fixes at the reviewed revision.
Integrate from the current .29 tip, rerun affected seam checks, and leave composed
release qualification to 160. Stop for unresolved 80 prerequisites, changed public
semantics, unbounded orient scans or unknown-to-full inference; seek a recorded
bounded design ruling. No fabricated approvals, benchmark gains or pass receipts.

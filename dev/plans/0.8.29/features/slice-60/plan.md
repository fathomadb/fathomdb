---
title: FathomDB 0.8.29 Slice 60 — MCP surface
status: PROPOSED
target_release: 0.8.29
slice: 60
---

# Slice 60 — MCP surface

## Current checkpoint scope

Remaining formats/flags/text-filter scope approved in Slice9 Batch4. Preserve settled compact-default direction; implementation still needs reviewed contracts and both-format checks.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and inspected evidence

Agents need readable, bounded results and exact filtered lexical retrieval with
truthful view, readiness and continuation signals. This plan proposes later
implementation; drafting changes no product, environment, canonical contract or release state.
The [brief](../../planning-brief.md) and [reconciliation](../../reconciliation-plan.md)
confirm integrated Block 1 and .28 flags/filters on format 37; no branch remerge is needed.
The [compact design](../../../../design/agent-access/compact-text-block.md) §7
has owner approval (October 27): server-wide `text` default, explicit `structured`.
Runtime support is unbuilt: inspected MCP config/edge/output/shape/tools still
always declare output schemas/send structured content, reject text filters and
omit search existence flags. SDK `search_text_only` already forwards filter,
view, limit and snippet together; adapter exposure is the delta.
Also inspected [MCP interface](../../../../interfaces/mcp.md),
[SDK engine](../../../../../src/rust/crates/fathomdb-sdk/src/engine.rs),
[adapter tools](../../../../../src/rust/crates/fathomdb-mcp/src/tools.rs),
[compact review](../../../../design/agent-access/compact-text-block-astra-review.md),
[finish plan](../../../agent-access-block1-finish-plan.md) and
[failed-open ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md).
Historical proxy savings do not prove native paging/client compatibility/current accuracy; no product tests ran for this draft.

## Slice preparation: local needs, requirements and acceptance

Preserve [R29-60-FTS1–5 and AC29-60-FTS1–6](requirements.md) in full, including
snippet/filter composition, metadata availability and same-build SDK parity.
Extend that local record with these draft needs and criteria; create no canonical AC IDs.

| Need / requirement | Falsifiable local acceptance to add |
| --- | --- |
| N29-60-OUT / R29-60-OUT: compact output without losing actionable fields | AC29-60-OUT: all seven tools and errors honor both formats; text has one block and neither outputSchema nor structuredContent; structured declares its schema and sends conforming content plus serialized JSON text. |
| N29-60-REF / R29-60-REF: recover writer-controlled values exactly | AC29-60-REF: JSON-quoted refs/store/kind/source/edge values round-trip spaces, quotes, newlines, pipes and Unicode; search/expand remain one hit per line; fetch bodies use exact UTF-8 byte lengths and preserve one ordered success/error outcome per requested ref. |
| N29-60-BUD / R29-60-BUD: truthful final budgets and paging | AC29-60-BUD: final text tokens equal ceil(final bytes/4), including its own count, hints and cut flags; digit-width boundaries converge within the reviewed cap; each mode fits its wire-accounting rule and continues at offset plus kept hits. |
| N29-60-VIEW / R29-60-VIEW: explicit historical selection | AC29-60-VIEW: as_of and independent include_superseded/include_inactive flags, including both together, reach SDK ReadView in text/hybrid; default false preserves current selection; malformed flags fail typed validation and continuation overrides are refused. |
| N29-60-SIG / R29-60-SIG: retain readiness/degradation and honest placeholders | AC29-60-SIG: field inventory accounts for every serialized output field; kept fields agree across parsed text/structured results, including hosted continuation_exact=false, explanations, hints, projection status, affected kinds and runtime unavailability reasons. |

FTS acceptance retains explicit/config-selected text, all five flat filter fields,
AND attributes excluding edges, unknown/non-filterable attribute errors even with
no lexical matches, >100 stronger nonmatches before a selective limit-1 hit,
node/edge duplicate bodies, inclusive projected time and empty-status controls.
Provider-free lexical tests prove zero embedding/vector/CE/graph work; missing projected
status/time fails to match, including history, without JSON reinterpretation.
Apply selection before candidate caps, deduplication, ranking and limit inside the
engine; never filter MCP top-K or substitute hybrid/property FTS. No ranking,
search-mode default, weak-result threshold, storage change or new tool is proposed.

## Architecture reading, design draft and fit review

Read [adapter architecture](../../../../design/mcp-adapter/architecture.md) and
[design](../../../../design/mcp-adapter/design.md), then trace config -> discovery ->
canonical request/ReadView -> SDK -> shaping -> transport and typed errors.
Deliver `design.md` with existing-versus-new inventory, request/field tables,
per-tool grammar, budgeting and need -> requirement -> AC -> test mapping; reconcile
three-decimal compact scores with required FTS score parity before code.
Use one governed result model and explicit format renderers; keep eligibility in
SDK/engine. Retain SnippetOptions while adding TextSearchOptions.filter and flags.
Canonical identity/continuations retain every filter and effective view; format
is a server setting independent of retrieval mode. Do not invent stronger freshness.
On unchanged state, pages have no missing/duplicate/out-of-filter refs through
ranked cap 100. Preserve epoch/reopen/cross-store stale-handle and embedded
best-effort/state-advanced caveats; rejected overrides cannot change selection.
Reconcile compact test 12's old `dense_reason` name with seq-359: verification
refusal returns typed unavailable/open_failed and no Engine/orient dense-disabled
field. Deliberate None-provider access is distinct; do not recreate degraded-open engines.
Record B2-1 ownership explicitly: approved text omits unavailable state/verdict;
structured retains its present placeholders until reviewed Slice 80/90 changes.
Specify optional real state/verdict serialization and matching text presentation
with that B2 contract before making fields real; epoch alone proves no freshness.
Keep B2-2/3 hints and partial B2-6 signals; remaining reasons belong to 70.
Structured accounting counts both serialized JSON copies, including final budget overhead.
Challenges: dynamic tool schemas, final-count convergence, Unicode framing,
no-progress budgets, hint fallback and format-dependent page sizes.
Research before design approval: pinned rmcp schema/result APIs and official MCP
2025-06-18 tools/structured-content specification; distinguish protocol obligations
from observed client display. Inspect current conformance harness and prompt fixtures.
Fit review compares each compact-design claim against current code/provider policy
and revises stale tests/docs; do not repeat an already-settled default decision.
Apply [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
boundary/property and differential tests on real stores; defer broad fuzz/MC-DC
campaigns. Current wire changes justify focused protocol composition coverage.

## Dependencies, work breakdown and deliverables

Hard prerequisites: reviewed combined requirements/design/test mapping, recorded
execution authorization and approved Slice 10 preparation with checkout-owned tools.
Existing .28 engine flags/filter support is satisfied; verify actual APIs at handoff.
Slices 40/50 are not build prerequisites. Prefer sequential shared-interface landings;
final MCP/SDK parity must use the same integrated build and accepted fusion identity.
Serialize MCP interface/requirements edits with 70 and 80; isolated implementer
worktree, one writer per checkout. No GPU or paid run is required for lexical proofs.

1. Reconcile/extend `requirements.md`; deliver `design.md`, `test-plan.md` and
   independent design review, resolving provider/B2 discrepancies before code.
2. Implement validated result_format config/discovery and per-tool/error renderers
   incrementally; pin the legacy harness to explicit structured mode.
3. Add quoted/framed text, field inventory and converged budgeting; preserve fetch
   item count or existing typed refusal when even empty bodies cannot fit.
4. Expose flags and filtered direct text search; remove obsolete refusals and
   hybrid-only discovery/docs; retain filter/view identity, snippets and paging.
5. Update MCP interface/instructions/config examples and release-note migration
   to structured mode in the implementing change; audit any new public enum approval.
6. Deliver `status.md` with source/artifact identities, reviewed AC evidence and
   limitations; hand both-format conformance and remeasure inputs to follow-up exit.

## TDD, review, validation and stop points

Each changed behavior gets observed failing RED evidence before separate GREEN;
review RED shas, keep tests read-only during fixes and avoid all-tools big-bang GREEN.
Use real SQLite/MCP calls and same-store/fixed-time SDK controls. Property tests
cover arbitrary refs, UTF-8 framing/cuts and continuation recovery; ample-budget
field/ID/order/score parity is separate from constrained per-format paging checks.
Independent design review precedes implementation; codex review gates integration.
Run focused MCP config/schema/search/filter/view/paging/error/hints/stdio suites,
then both-format conformance and one final strict full gate for the code phase.
Planning integration uses Markdown/scoped validators; never call that workspace green.
Before approved item-4 remeasure, fix its record-start prompt. Report same-page
rendering separately from native end-to-end hits/tokens/cost/fetch/accuracy; no token target.
An accuracy drop outside the paired interval stops promotion. Missing owned tooling,
provider-policy contradictions, schema mismatch, budget nonconvergence or unresolved
B2 semantics stop the affected change for review. No fabricated approvals or passes.
Block 1 follow-up exit after integrated 20–70 owns required feature/performance,
conformance and approved smoke/remeasure; 90 waits for that exit plus 80 approval.

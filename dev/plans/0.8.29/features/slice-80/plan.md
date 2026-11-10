---
title: "Slice 80 — unified residual Block 2 design"
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 — unified residual Block 2 design

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Current checkpoint scope

Residual design scope approved in Slice9 Batch4. Preserve built hints/span storage; settle actual fields/coordinates and omit unavailable-only placeholders under accepted authority. No calibrated verdict/weak threshold inferred.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and boundaries

Produce one reviewable B2-1–8 requirements/design package for Slice 90, following the
[drafting brief](../../planning-brief.md) and [reconciliation](../../reconciliation-plan.md).
This slice designs residual behavior; it implements no product code or tests and changes no canonical contracts.
The current format-37 baseline includes published .28, Block 1 queue/hints and known-or-unknown head-span storage.

| Row | Source-grounded maturity and residual |
| --- | --- |
| B2-1 | `Envelope.state` and search `verdict` remain required unavailable placeholders; apply the accepted omission decision consistently in both formats and discovery. |
| B2-2/3 | Empty-result and degraded hints already exist in engine/MCP; preserve and map their tests, then consume Slice 70's settled missing reasons. No weak-result threshold is authorized. |
| B2-4/5 | MCP parsing/feedback still use generic errors; design typed accepted-argument and offending-ref details. |
| B2-6 | Per-store default flags and `default_store_set` hint exist; top-level default-store value, compact header and guide wording remain to reconcile. |
| B2-7 | Admission queue, typed busy reasons and no-retry-delay policy already cover substantial scope; map existing controls and identify only demonstrated residuals. |
| B2-8 | Approved format-35 storage survives in format 37: head-only spans, NULL unknown, transactional validation/upkeep. Orient coverage and scored-span presentation remain unbuilt. |

Keep accepted compact `text` default and B2-8 approval settled. No re-embedding, backfill,
projection-generation bump, overflow generation, new anchor, ranking change or inferred freshness/calibration.
Physical storage permits future windows; this release produces only head window 0.

## Inspected inputs and slice preparation

Read the [release allocation](../../../plan-0.8.29.md),
[accepted hints ADR](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md),
[hints design](../../../../design/agent-access/hints-sidecar.md),
[compact design](../../../../design/agent-access/compact-text-block.md),
[queue design](../../../../design/agent-access/queue-admission.md),
[guard rationale](../../../../design/long-record-embedding.md),
[format-35 ADR](../../../../adr/ADR-0.8.28-store-format-35.md),
[storage design](../../../0.8.28/features/slice-60/design.md),
[head-coverage contract](../../../../interfaces/rust.md#embedder-head-coverage-format-35),
[wire contract](../../../../interfaces/wire.md), and [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
Inspected MCP output/tools/error/hints, engine hints/snippet/search-result/span validation and projection span callsites;
SDK options/re-exports and Python/TypeScript interface snippet contracts establish the carrier gaps.
Relevant memory read: `agent-access-block1-design-state.md`, `long-record-embedding-findings.md`,
`verify-design-against-code-not-just-architecture.md`, `0.8.25-slice50-compact-evidence-lessons.md` in the main-repo memory store.
Prepare local `needs.md`, `requirements.md`, `acceptance.md`, `design.md`, `test-map.md` and `review.md`.
All IDs below are release-local proposals, not additions to canonical acceptance IDs.

## Draft needs, requirements and falsifiable acceptance

| Need / requirement | Completion signal for design approval |
| --- | --- |
| S80-N1 / S80-R1: agents receive informative fields | S80-A1: every B2 row has built/residual evidence; both-format/schema/guide examples omit unavailable state/verdict, list missing capabilities once in orient, and preserve real readiness fields. |
| S80-N2 / S80-R2: errors provide safe corrective detail | S80-A2: typed error schemas/examples fix codes, precedence, deterministic ordering, bounds, quoting and omission; successful requests incur no error-detail construction. |
| S80-N3 / S80-R3: coverage never implies unseen text was scored | S80-A3: exact populations/formulas, unknown/empty/error cases, view/snapshot authority, cache invalidation and range provenance are specified with hand-computed examples. |
| S80-N4 / S80-R4: SDK and MCP contracts agree | S80-A4: Rust engine/facade/SDK, PyO3/Python, N-API/TypeScript and both MCP formats have a field/default/nullability/error mapping, including frozen/text-only and standalone snippet behavior. |
| S80-N5 / S80-R5: implementation has a bounded entrance | S80-A5: independent review and owner approval resolve design findings; Slice 90 requires approved 80 AND the unnumbered Block 1 follow-up exit. |

## Design draft, challenges and research fit

Reconcile compact's text-only dropping of placeholders with the accepted hints ADR: make structured state/verdict optional and absent too.
Orient lists unavailable state/verdict/kinds/counts without inventing their implementation; real readiness/degradation retain their separate meaning.
Map accepted Stage 2 search hints carriers across `search`, `search_text_only`, `search_frozen`; retain SDK default off pending separately authorized Slice 150 decision.
Distinguish disabled/absent hints from enabled-empty arrays; engine subset stays pure and MCP adds only adapter facts, sharing final response budgets.
For B2-4 derive accepted keys from each tool schema, select unknown keys deterministically before deserialization, and preserve known-key type failures as bad arguments.
Specify `unknown_argument` class/code, complete accepted list, per-tool constant remedy and UTF-8-safe 64-byte argument truncation including marker.
For B2-5 preserve request order, fix duplicate handling and error precedence, cap returned refs at 50 and each at 128 UTF-8 bytes, and count omitted refs explicitly.
Do not truncate a ref into a different valid identifier: omit overlong refs with counts; JSON-quote caller strings in text and keep them out of message templates.
Carry the same typed details in structured JSON and compact errors without hints; specify budget behavior rather than silently dropping corrective detail.
Complete B2-6 top-level `default_store: string|null`, quoted compact header and default-use instructions; retain row flags and existing hint configuration.

Proposed coverage model for review: E is distinct vector-targeted canonical records eligible under orient's documented default read view at its fixed validity instant.
P is E with a validated committed physical head tuple; K is P with known end; U=P−K; M=E−P is pending/missing, never full or known zero coverage.
Each record counts once under Slice 50's settled identity; name node/edge, source-closure, lifecycle and validity inclusion rules explicitly.
Report E/P/K/U/M counts; truncated-known share is count(end<body_length)/K, unseen-known byte share is sum(body_length−end)/sum(body_length) over K.
Expose those denominators with the ratios; empty K or zero known body bytes makes the respective ratio unavailable, not NaN, zero or proof of completeness.
Unknown and missing populations remain visible; any all-population unseen-byte claim is unavailable while U or M is nonzero.
Define no-vector/not-applicable separately from pending, provider-unknown and corruption; SQL/validation failures retain typed failure semantics, not unknown success.
Bind aggregate data, projection membership and spans to one SQLite read snapshot, database/branch nonce, visibility generation, effective view and validity instant.
The open epoch alone is insufficient. Design cached summary construction/invalidation so orient adds no store-wide scan and cannot reuse another snapshot's facts.
Prove time-only eligibility transitions, external writes, rollback/coalescing, projection commit/rebuild, erasure and reopen; if proof/cost is unresolved, stop approval.

Define scored coverage as body-relative half-open UTF-8 bytes [0,end), separate from snippet windows, match spans and snippet-relative highlights.
Specify vector contribution versus text-only/graph/standalone selector, known-empty [0,0), provider-unknown and omitted-sidecar cases explicitly.
Preserve `scored_bytes: Option<u32>` accepted intent or propose a reviewed successor for range representation; settle bounds/conversion and absence distinctions before approval.
Bind coverage to the exact returned record/vector identity after fusion and paging; identical bodies cannot lend each other coverage. No extra per-hit hot-path struct growth.
MCP re-snippeting/budget cuts retain body-relative scored extent, never reinterpret it as displayed bytes; vector-only snippets retain Start.
Specify matching serialization, strict native decoder bounds and UTF-8 examples for Rust/Python/TypeScript, including frozen reproduce-or-fail behavior.
Planned primary research: pinned serde/schemars schema behavior, SQLite WAL snapshot/trigger guarantees and current tokenizer offset contract; record exact sources in `design.md`.
No external research is claimed complete here. Fit review favors bounded real-database snapshot/resource cases and Unicode properties; defer broad fuzzing and chunking experiments.
Revise historical generation-bump guidance to reuse existing truthful stored metadata; accepted storage removes any need to reproject just for reporting.

## Dependencies, work packages and review gates

Design may start before measurements finish; approval requires settled Slice 50 identity, Slice 60 final fields/formats/budgets and Slice 70 degradation contracts.
Newly discovered prerequisites: a no-scan snapshot-correct aggregate strategy, denominator policy, scored-range carrier/absence rules and error-detail budget/precedence decisions.
Approved preparation/environment from 10 governs later execution; missing contract inputs keep approval open, not all design drafting blocked.
Serialize future shared engine/MCP/native/interface edits with 50/60/70; design needs no GPU venue or dependency installation.

1. Produce code/test-backed B2 census and local needs→requirements→acceptance→planned-test map; retain built controls without declaring them newly implemented.
2. Draft unified architecture and all-binding/schema examples, resolve the explicit coverage/error/placeholder questions, and record differences from historical design.
3. Challenge snapshot/time caching, zero/unknown math, identity mixing, hostile refs, output budgets and native width/nullability with hand-computed cases.
4. Obtain independent adversarial requirements/design review, revise findings and present the concrete package for owner approval; record no invented verdict.
5. Deliver approved residual-only Slice 90 scope plus canonical ADR/interface amendment list; update release-local status through the parent-owned integration process.

## Validation and decision points

This draft executes no product tests. Parent validates Markdown and links after integration; focused planning checks are not a full-workspace green claim.
The test plan requires Slice 90 per-behavior staged/committed failing assertions before GREEN, with independent review of human-specified expected results.
Plan real-store known/unknown/empty/pending/corrupt spans, concurrent old/new snapshots, all supported views, lifecycle/time boundaries, rebuild/erase and rollback controls.
Use Unicode/range properties and cross-binding fixtures; assert identity-specific scored extents, zero/no-vector distinctions and unchanged provider-free text retrieval.
Cover both formats, all tools/errors, hints on/off, default-store switching, unknown arguments, feedback-ref limits, paging/budget boundaries and queue/resource regressions.
Retain accepted overhead/allocation controls and composed Block 1 follow-up evidence; later 90 runs required full verification with checkout-owned tools.
Stop for unresolved semantic choices or new persisted/public authority; propose an explicit reviewed amendment instead of silently broadening scope.
Owner approval concerns the residual design package only; compact default and B2-8 acceptance are not reopened, and weak_results remains reserved.

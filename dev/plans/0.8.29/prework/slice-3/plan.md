---
title: 0.8.29 Slice 3 — draft contract CRUD and slice allocation
status: PROPOSED
target_release: 0.8.29
slice: 3
---

# Slice 3 — draft contract CRUD and slice allocation

## Evidence, goal and authority

- Produce release-local proposals connecting user needs → requirements → acceptance → architecture → owning slice; this slice does not change product behavior or canonical contracts.
- Starting maturity: MCP/Block 1 are integrated; B2-2/B2-3 and part of B2-6 already exist. B2-7 needs a queue adequacy check. Format 37 and B2-8 span storage exist; reporting remains a proposed residual.
- Source inspection found body-keyed production fusion, pure shared hints, MCP default-store rows/hints, and MCP text-mode filter refusal. Source inclusion is not current-tip qualification.
- The [drafting brief](../../planning-brief.md) governs proposal-only prework and the proposed 120–160 allocation. The [reconciliation plan](../../reconciliation-plan.md) now reflects the same proposal-only Slice 0 boundary and approved Slice 10 activation.
- Preserve the [release plan](../../../plan-0.8.29.md)'s allocation history; map former 120 CLOSE → proposed 160, with 120–150 separately proposed. Do not silently promote old approvals.
- Canonical needs/requirements/AC/interface/ADR files, Git, ledgers and environments remain read-only. Draft “delete” means a proposed retirement record, never an executed deletion.

## Sources actually inspected

- [AGENTS.md](../../../../../AGENTS.md), the external memory index and verified entries `acceptance-md-locked-no-feature-acs.md`, `verify-design-against-code-not-just-architecture.md`, `agent-access-block1-design-state.md`, `long-record-embedding-findings.md`, `physical-erasure-done-defects-only.md`.
- [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md), [ADR index](../../../../adr/ADR-0.6.0-decision-index.md), [accepted hints ADR](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md), [visibility-coalescing ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md).
- [Block 1/B2 inventory](../../../../design/agent-access/README.md), [hints design](../../../../design/agent-access/hints-sidecar.md), [compact design](../../../../design/agent-access/compact-text-block.md), [existing filtered-FTS requirements](../../features/slice-60/requirements.md).
- [CUDA review procedure](../../../../design/cuda-version-support-review.md), [accepted CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md); fresh usage collection is incomplete, not repeated in this slice.
- Code-grounded excerpts: [MCP tools](../../../../../src/rust/crates/fathomdb-mcp/src/tools.rs), [MCP discovery](../../../../../src/rust/crates/fathomdb-mcp/src/edge.rs), [engine hints](../../../../../src/rust/crates/fathomdb-engine/src/hints.rs), [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs), [span validation](../../../../../src/rust/crates/fathomdb-engine/src/vector_spans.rs), [schema 37](../../../../../src/rust/crates/fathomdb-schema/src/lib.rs).

## Slice preparation: local needs, requirements and acceptance

- **N29-3-1:** A release reviewer needs to see which user problem is unsolved, already met or merely unqualified before authorizing more work.
- **N29-3-2:** Implementers need a single proposed owner and testable behavior for each residual without changing accepted semantics by inference.
- **R29-3-1:** Each draft row records local ID, audience/scenario, source authority, current behavior, proposed delta, CRUD action, maturity, owner, dependencies, acceptance oracle and unresolved decision.
- **R29-3-2:** Reuse existing release-local IDs such as R29-60-FTS1 and AC29-60-FTS2; new draft identifiers below are local proposals, never additions to canonical `dev/acceptance.md`.
- **R29-3-3:** Every selected requirement maps to a need and falsifiable acceptance; every acceptance maps to a test/measurement class and responsible slice. Mark uninspected tests as candidates, not coverage.
- **R29-3-4:** Every architecture proposal states existing versus net-new seams, accepted ADR constraints, alternatives, public surfaces, shared files and required decisions; unresolved conflicts cannot be labeled approved.
- **AC29-3-1:** A reviewer can trace every seeded row below in both directions and identify its source, current maturity, owning slice and independent failure condition without searching chat.
- **AC29-3-2:** No built signal is allocated for rebuild solely because its old branch is absent; every proposed update/delete has a specific contradiction or replacement and preserved provenance.
- **AC29-3-3:** The allocation check finds no ownerless mandatory row, unexplained multi-owner row, canonical AC invention, 90↔160 cycle or automatic policy/default change.
- **AC29-3-4:** Slices 4/5 return explicit alignment/verification findings for this draft; unresolved findings are passed to 9 with disposition choices, not hidden in a completion claim.

## Draft CRUD method and seeded contract rows

- **Create:** add a missing release-local behavior/architecture proposal with evidence; **Read/retain:** preserve an accepted or built requirement and assess qualification; **Update:** propose a bounded replacement with before/after semantics; **Delete:** propose retirement only after identifying all consumers and the replacement or explicit scope removal.
- Record actions in `crud-register.md`; maintain stable IDs and a supersedes link. Do not conflate deleting stale prose with withdrawing supported behavior. Slice 2 owns cruft decisions; 6 owns audience freshness evidence.
- Seed **D29-MCP1**, need: an agent can retrieve constrained lexical evidence without an embedder. **Read/retain** existing FTS requirements; **update** obsolete hybrid-only descriptions and refusal. Owner 60, contract review 3/4/5; existing engine support is prerequisite.
- Acceptance seed: explicit/config-selected text mode with a selective match behind >100 stronger nonmatches returns that match at limit 1; the same fixed-view SDK call agrees. Empty/malformed filters, unknown attributes, paging and both formats preserve typed semantics. Reuse AC29-60-FTS1–6.
- Seed **D29-MCP2**, need: an agent spends its response budget on usable output. **Create implementation delta** for approved compact text default/structured option, all tools/errors; owner 60. Do not reopen the default decision.
- Acceptance seed: text advertises no outputSchema and sends no structuredContent; structured advertises a conforming schema and includes serialized JSON text. Hostile refs are quoted, fetch bytes are framed, budgets and continuations remain valid; ample-budget reference order matches across formats.
- Seed **D29-BLK1**, need: a caller receives fair bounded admission and a usable completed result. **Read/retain** queue run-to-completion semantics; investigate residual enqueue/drain race in 20 and B2-7 adequacy in 80, with missing behavior only in 90.
- Acceptance seed: deterministic barrier cases cover admission before wait expiry, cancellation/drain at enqueue and release of real permits; compare to accepted 10 s wait/30 s run-limit behavior. Do not authorize reader interruption or a new retry_after_ms field.
- Seed **D29-B2**, need: an agent understands an empty/degraded result and missing capabilities. **Read/retain** B2-2/3 and partial B2-6; **update** only proven gaps in state/verdict, accepted-argument errors, bad feedback refs and default-store presentation. Design owner 80, implementation 90; degradation labels 70.
- Acceptance seed: built no_results/degraded hints survive both formats; unavailable-only state/verdict are omitted per hints ADR and orient reports absence; errors identify accepted arguments/offending refs. A nonempty result never gains an invented weak_results threshold.
- Seed **D29-SPAN**, need: an agent knows which returned bytes had vector evidence. **Create reporting delta**, retain existing span storage; owners 80/90 and parity 70. An unknown endpoint is unknown, never full coverage or zero truncation by inference.
- Acceptance seed: multibyte bodies and snippets crossing a known UTF-8 head boundary yield truthful scored byte extents in the specified coordinate system; unknown-provider spans remain unknown. Define denominator, view and nonvector-result behavior before tests; no overflow chunking.
- Seed **D29-SEARCH**, need: distinct records retain identity/provenance while retrieval scales. **Update** body-keyed fusion only through an accepted successor ADR in 50; **create conditional** format37 cache proposal in 40 after 30 measurements.
- Acceptance seed: two records with identical bodies/different provenance follow the approved identity rule, with one vector candidate per record; exact optimized/reference results agree. Cache preserves eligibility across time-only boundaries, old/new snapshots, rollback and external writes; write-interleaved costs decide adoption.
- Seed **D29-CUDA**, need: a consumer can select a supported artifact and close safely after a permitted GPU fault. **Read/retain** policy constraints; **create** reset lifetime proposal 110 and CUDA13 artifact contract 120; qualification 130, conditional fork work 140.
- Acceptance seed: installed-consumer evidence identifies actual driver/PTX/library/architecture and artifact bytes; reset/drop controls prove the declared bounded outcome and release safety. Library qualification alone cannot establish artifact delivery or context-reset survival.

## Architecture/design draft and challenging aspects

- Draft `architecture.md`: engine owns eligibility, exact ranking, span truth and pure hints; SDK/native layers preserve semantics; MCP owns admission, request identity, budgeting, rendering and adapter-only guidance. Proposal files do not become another runtime authority.
- Draft `requirements.md`, `needs.md` and `acceptance.md` carry the seed rows; `allocation.md` maps owners, public surfaces, source evidence, candidate tests and dependencies. Canonical changes are later implementation deliverables needing accepted design/interface updates.
- Challenge 1: B2-1 omission conflicts with compact design prose implying restoration; resolve using the accepted hints ADR, then have 80 specify both formats together. Existing hints are facts; calibrated verdicts remain separately gated.
- Challenge 2: exact text-route equivalence must use the approved identity successor, not preserve old body dedup accidentally. Census twins, edges and ownerless rows before choosing a key; preserve provenance in the decision record.
- Challenge 3: cache invalidation follows transaction-coalesced format37 visibility and snapshots. Distinguish remaining direct-node eligibility from native vector filtering and dependency-closure fallback; counters/schema changes need another decision.
- Challenge 4: span offsets, snippet offsets and rendered text offsets are different coordinates. Define conversion and unknown rules before reporting; role-tagged bytes and Unicode must not be normalized into false evidence.
- Challenge 5: CUDA library major, build toolkit, emitted PTX, driver floor, architecture and delivery route are separate contract dimensions. Keep R580/primary 12.6 fixed and K620 unused; venue availability remains unproven.

## Research approaches, fit review and proposed revision

- First compare accepted ADR/interface text to current structs/routes and relevant tests; record exact source locations and distinguish absent code, absent evidence and stale prose. Read candidate tests fully before claiming assertions.
- Research plan: use official MCP schema/result documentation for format contracts, SQLite snapshot/transaction documentation for cache assumptions, and NVIDIA/CUDA/fork source contracts for artifact/reset boundaries; record retrieval dates and questions. No new external factual claims are needed to finish this planning slice.
- Fit review with 4: challenge placement and API ownership against current code. Fit review with 5: reject vacuous parity, shared-predicate oracles and “no crash” substitutes for state/resource assertions.
- Apply quality-roadmap recommendations to real-store boundary/property cases, reference comparisons and bounded resource controls. Defer global coverage targets, general SQLite fuzzing and exhaustive CUDA matrices; format37 coalescing is the new reason for fresh snapshot controls.
- Proposed revision: preserve integrated MCP/Block1/B2 signals, retire stale descriptions only after replacements are approved, and allocate residuals on the new ladder. Slice 9 decides disagreements; no result of these reviews is claimed here.

## Dependencies, sequencing and new prerequisites

- Hard input: current brief, accepted ADRs and current source. Slices 3/4/5 iterate on drafts; 9 requires their outputs. No full product gate is prerequisite to drafting.
- 60 requires accepted surface reconciliation and integrated flags/filter support, not 40/50 implementation; rerun final parity after integrated fusion. 40 requires 30 evidence; 50 requires 30 plus accepted identity before optimized route. Preferred 40→50 limits attribution/shared-file conflicts.
- 80 may design early; approval needs settled field/identity/degradation contracts. 90 needs approved 80 AND unnumbered Block1 follow-up exit after integrated 20–70, including approved smoke/remeasure; final affected smoke remains in 160.
- Conditional fork edges: included 140 work qualifies in its fork before pin adoption; adopted fixes precede affected 110/120 integration and 130 final qualification. These labels do not impose numerical execution order.
- Newly exposed reviewer prerequisites: span coverage denominator/coordinate rules; B2-1 cross-format contradiction; default-store existing-versus-missing field census; current 100k/write-interleaved evidence before cache selection; fresh CUDA usage inputs and artifact-selection/distribution decision before expanded claims.
- Slice 0 supplies proposed tool/venue inventory, 1 proposed dependency pins, 2 proposed retirements, 6 stale-audience evidence and 7 validation constraints. Missing inputs become explicit 9 choices, never inferred approvals.
- Serialize eventual engine landings 40/50/70/100/110 and canonical interface/ADR/state writes. Only parent imports this scratch draft; no writer shares a checkout or changes tools to satisfy planning.

## Work breakdown, validation and decision points

1. Inventory source authority and built/residual/unknown rows; prepare the six named local deliverables with stable IDs and the seeded scenarios above.
2. Expand allocation to every selected 20–160 obligation and cross-cutting qualification; preserve former allocations and mark 120–150 proposed, 150 conditional and publication separately authorized.
3. Inspect candidate tests and attach behavior/negative control/real-store oracle plus missing-evidence labels; hand detailed adequacy decisions to 5, code-alignment decisions to 4.
4. Check links from the final destination, ID uniqueness, CRUD replacements, bidirectional traceability and dependency cycles; request independent read-only review, then expose unresolved findings to 9.
5. After parent imports planning Markdown, use scoped Markdown validators only. A plan-completeness pass is not product qualification; no source tests, full gate, dependency installation or environment mutation is needed here.

- Later implementation requires reviewed contracts, per-behavior observed RED staged/committed before GREEN, current-checkout tools, appropriate feature/full gates and independent code review; this slice neither runs nor waives them.
- Stop for 9 on conflicts with accepted contracts, ambiguous identity/coverage semantics, unapproved experiment/default changes or a proposed canonical deletion without a disposition. Approved text default and B2-8 storage are settled, not fresh decisions.
- Exit maturity: complete, reviewable proposal with explicit unresolved decisions. Slice 10 implements only recorded preparation decisions; feature owners implement their accepted deltas. Erasure remains complete except concrete defects and bounded 100 interruption qualification.

---
title: Inspector 0.2.0 Slice 5 — verification review design
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Minimal design for evidence-aware review of the existing verification matrix
---

# Verification adequacy review design

Current disposition: owner S9-010, applied in [Slice 10](../slice-10/status.md),
qualifies supported single head windows in [0.2.0 scope](../scope.md) and defers
within-source multiple-window positive. Earlier findings/draft proposals below
retain their historical meaning; cache-only CPU Engine serving remains mandatory
and unqualified in 40.

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

This design governs a future prose review/proposal, not a new test framework or product architecture. It consumes [scope](../scope.md), [plan](plan.md), canonical [NRA](../../../requirements.md), [AC](../../../acceptance.md), [HLD](../../../design/high-level-design.md), and the one [M01–M40 matrix](../../0.1/verification-plan.md). Inspected 2026-11-19 at `609dd85c41fb7980e05faa1132a248d376d8ec89` on the planning branch; unchanged Engine `6edc1155f`, current format 37, unpublished Inspector 0.1.0. No runtime checks or adequacy review executed today.

## Existing mechanisms versus proposed work

Existing A/S/Q/P/O/R/U/K ownership is retained. Source `src/inspector/src/worker.rs` accepts only record-foundation operations; new lexical/vector/graph/comparison routes remain feature work. `ReadConnection` in `session.rs` owns the bounded queued cursor set for its transaction, and session/admission checks reject dirty coalescing. `tests/integration/disclosure.rs` has real Autonomous absence and ExplicitMaintenance queued-content/export/freshness cases, not merely supplied-content policy tests. Existing browser scripts exercise the actual service and disposable Engine fixtures, including controlled late delivery and cache revalidation.

The matrix and canonical AC already describe product goals, required High investigation, independent oracles and most cross-cutting tests. Proposed review work connects each assertion to that specification and preserves its actual execution scope; no schema, API, test harness, telemetry or product-state machine is added by Slice 5. Local DRAFT NRA/design changes remain in [the plan's allocation table](plan.md#local-draft-needsrequirementsac-and-design-allocations) until the owning review/decision accepts them.

## Review structure and evidence semantics

Review in both directions: user outcome → requirement → observable AC → method/test → actual result, then test → assertion → intended violation → requirement. Use M rows as the join key. A requirement without AC, AC without a meaningful test and existing unexecuted test are different findings. A declared unavailable state has a pass only for that negative behavior; it never satisfies a mandatory positive except the specifically conditional absent-erasure case. Current legitimate queued-content support prevents applying the old atomic-only waiver universally.

Attach a short finding to an existing row or its linked owner status: row/criterion, actual source/assertion/revision, evidence class, missing or weak obligation, uncertainty/confidence, smallest observable check, prerequisite and proposed owner/disposition. Do not add a second M-register, broad spreadsheet, new ID system, execution diary or hash inventory. Needed raw results may remain in owned scratch later; reproducible builders/tools remain committed when implemented.

Source-inspected coverage is not executed. Preserve historical versus current recorded pass, ignored/feature-gated/unexecuted, partial, blocked and unavailable/not-applicable states. Record default versus fixture-control, debug versus release, built versus previously compiled artifact, real native call versus synthetic sleep, measured sample versus true maximum and API versus browser interaction. Port final ordinary results are 37 unit/real-DB plus 1 CLI/1 no-GUI/2 worker checks; four explicit native/scale/allocation cases remain unrun in that port. Astra's direct compiled-artifact rerun is not a new compilation. Missing checkout-owned `.venv` stopped full source verification.

## Critical-path checks to specify

Use the plan's M-row starting points, then cover every row. First follow supported attach to exact record, representation, native result, stored relationship, original context and two-observation comparison. At every boundary ask whether the UI reveals actual class/identity/revision/scope rather than a convenient text match, whether a read is later than the request, and whether unknown stays unknown. Finish with cancellation/close and the installed/human path; no layer can stand in for another.

- Record/representation identity: collisions, equal text, retained revisions, zero/one/many origins, exact operational collection/key/entry and derived record versus projection. Format-37 vector sidecars/spans require multiple windows per cursor with exact window number, byte range, profile, source revision and origin cardinality; search identity does not become a stored locator by assumption.
- Lifecycle/disclosure: class-correct node windows versus edge invalid-time/dependency/supersession, declaration/materialization/readiness/runtime/eligibility/result distinct. Exercise all eight flags, independent I/E/T overlaps and each representation/evidence/link/copy/export/cache/pin delivery. Verify transaction-scoped queued membership and fresh subsequent read after a supported test-owned state change; production Inspector creates/withdraws no queues and changes no maintenance policy.
- Independent oracles: intended real-DB fixture manifests before reads; small written lexical corpus/effective tokenizer/route and score semantics; f64 L2 from stored f32 with declared tolerance/tie groups, independent bit checks when relevant; exact directed adjacency/content for self/parallel/cyclic/disconnected edges. Shared FTS may establish conformance, not independent origin correctness. No duplicated adapter predicates, query-string guessed highlights, generated goldens or database mocks.
- Async/resources: one costly request plus eight waiting, excess busy; cancel acknowledgement ≤250 ms distinct from native quiescence/close ≤30 s, owned child/lock/handle/scratch release, no late update of selected/pinned evidence. Cover page/byte/chunk/work/observation limits, memory/model residency and graph links/depth/display budgets from requirements. Repeated navigation/eviction and active native/model close need genuine operations, not only paused IPC.
- Local/native/UI: exact origin/Host/session/credential/build rejection before read; inert HTML/scripts/URLs and no automatic fetch/log leaks. Actual keyboard and pointer workflows cover tree, lexical results, vector metric values, graph selection/expansion/collapse, Back and comparison pin/replace/unpin/eviction. Labels or API JSON alone cannot establish keyboard/focus/interaction.
- CPU/assets/artifact/humans: compatible cached CPU positive with no-network/no-cache-write evidence plus missing/partial/corrupt negatives and model-free usability. Default installed offline artifact, unchanged Engine/current format and declared Linux x86_64/Chromium pairing must exercise all required views and finite native close; help/checksum and no-GUI refusal are separate. Use five real unseen-transfer participants with ordinary docs and fixed interpretation rubric; include High investigation and report unfinished activity honestly. Diagnostic success ratio is not a new threshold.

## Architectural fit, feasibility and research questions

The review is feasible with existing matrix, deterministic builders and code inspection. Public callable Engine search/evidence/graph/dependency APIs may be reused unchanged; private `vector_storage.rs` documents layout, not a callable import. Inspector-owned exact stored reads require semantic tests. Fixture-only controls/test hooks are setup mechanisms, absent from the default artifact; a product route must never inherit their mutation privileges. Missing required public capabilities become explicit feasibility findings, not copied private rankers/provenance or waived positive tests.

Historical HLD says no sibling snippet API while adopted scope identifies current search/snippet/frozen APIs. Inspect current `search_api.rs`, types, feature visibility and implementation during execution before selecting route-specific expected evidence. Do not settle API availability from old prose or semver. Likewise revalidate `LoadedWeights`/CPU constructor ownership and asset lifetime rather than asserting the cache-only design has shipped.

If source leaves a concrete method ambiguity, investigate only its primary-source question during execution: SQLite FTS5 BM25/tokenizer/highlight semantics, sqlite-vec distance/bit conventions, Playwright keyboard/clipboard/network-event scope, and Linux process/handle/peak-memory measurement limits. Start with pinned source and official versioned docs; no web research is run for this draft, and no general library sweep or benchmark campaign follows. Record an unresolved question when a proof is unavailable.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): narrow boundary/property checks, selected first/persistent ownership faults, resource cleanup and a few useful test-strength mutations. The real queue-budget defect and multiple vector windows change risk priorities. Defer general SQLite fuzzing, global MC/DC, exhaustive allocation failures, VFS/power-loss campaigns and broad platform/relevance matrices; no percentage target or expensive campaign is a prerequisite to this report.

## Handoff and review

Slice 3 consumes concrete NRA gaps; 4 consumes route/design mismatches; 6 receives human/task/docs shortcomings; 7 receives build/preflight/packaging command mismatches. Slice 9 receives ranked proposals, confidence/effort and include/postpone rationale; conditional 10 may implement individually approved prework remedies. Features 20/30/40/50/60 own local positive/error/cancel/accessibility tests, 70 additional integration/installed/resources/security/human qualification. Required basics cannot be deferred wholesale to 70.

Future direct Sol-high review checks actual design deltas and substantive traceability/test-strength reasoning; Terra verifies document links, allocation, evidence states and applicable Markdown checks without running runtime campaigns. Capture one short execution status only when commissioned. Slice 5 completion is correctness of this bounded deliverable and scoped checks; all product qualification remains with its owners. The [future prompt](plan.md#future-execution-prompt) explicitly preserves these prework limits.

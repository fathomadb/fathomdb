---
title: Inspector 0.2.0 Slice 3 — high-level draft design
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Design of local product-contract proposals and delivery allocation
---

# Slice 3 — high-level draft design

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

## Boundary and baseline

This designs the future Slice 3 drafting deliverable, not a product implementation.
Consume [plan](plan.md), [adopted scope](../scope.md), [master](../plan.md),
[canonical HLD](../../../design/high-level-design.md), canonical
[requirements](../../../requirements.md)/[acceptance](../../../acceptance.md)
and [single matrix](../../0.1/verification-plan.md). Adoption is complete at
`609dd85c41`; INS-05.7–05.10/M40 are existing canonical scope. Keep every mandatory
Critical/High/Need/Should obligation. No canonical file is edited by Slice 3.

Existing: Inspector 0.1.0 record foundation, current compiled format 37,
session-owned Engine and read connections, common policy/observations/resources.
Net-new feature work: usable lexical, vector, directed graph, comparison and
installed all-view qualification. Old draft source findings are not runtime facts.
Reuse the selected report worktree sequentially; no independent environment
or checkout mutation is part of this planning artifact.

## Draft record structure and ownership

Use the plan's D3 records as the local change proposal. Each future expanded record
holds operation (`create`, `rename`, `update`, `delete`), exact destination/text,
reason and source facts, affected UN/INS/V01/X/F/T/M IDs, independent check intent,
architecture fit, one delivery owner, prerequisites and needs-HITL reason.
No record invents global AC IDs, changes canonical scope or grants approval.
No new need is presently justified; include evidence for no-needed-change.

Flow: canonical criteria + current source + earlier prework proposals → local
draft CRUD/fit/allocation → Slice 4 alignment and Slice 5 adequacy review →
Slice 9 deduplicated decisions → individually approved Slice 10 remedies and
feature-local contracts/tests. Slices 6/7 contribute their own audience-doc and
build/packaging evidence; Slice 3 does not repeat their audits or implement fixes.
Unique accountable M-row ownership is in the plan; all per-view obligations
still land in that feature, and final integration is 70 while comparison is 60.

## Architecture fit and concrete questions

Retain A/S/Q/P/O/R/U/K boundaries from HLD: guarded admission, named storage
reads, unchanged public query operations, independent disclosure, typed observations,
bounded ownership, native accessible views and paired local packaging. No parallel
ranker/tokenizer, inferred provenance or speculative universal adapter is proposed.

| Candidate / source-backed evidence | Question for execution and minimal approach | Owner / prerequisite |
| --- | --- | --- |
| Public `search_text_only_filtered_with_snippets`, `search_text_only_view_with_snippets`, `search_frozen_with_snippets`, `search_with_evidence`, resolvers in `fathomdb-engine/src/search_api.rs`; documented in `dev/interfaces/rust.md`. HLD still contains no-snippet claim. | Which supported route returns same-reader body spans/ordered sidecars and effective filters? Which property/BM25F routes lack spans? Prefer actual public returned evidence; qualify later S index reads separately, never invent shared snapshots. | 30; 4 checks exact callable/export/feature boundaries. |
| Format-35 ADR/current `vector_storage.rs` separate vector ID/cursor/span; compiled schema is 37. ADR's original producer is head-only. | Which current ordinary producers yield multiple windows, and how do body/source/profile/vector identities and UTF-8 offsets relate? Require real supported positive fixture, no synthetic storage proof or new chunker. Separate stored L2 population from Engine semantic/hybrid candidate and fused-score context. | 40; actual current routes and cached CPU compatibility. |
| `graph_api.rs`, frozen graph evidence/resolver and canonical edge storage. | Which public route supplies exact selected edge content/lifecycle? Where is bounded S adjacency/full revision read still required? Preserve parallel edges, direction, unknown sources and edge-only temporal semantics. | 50; current source/ADR and independent adjacency manifest. |
| Worker currently implements records/associations, not native queries or compare. Existing O/P/R can anchor references. | What minimal observation carrier preserves exact typed result identity/revision/origin cardinality and query context across new views? Recheck policy each delivery; queued scope cache lives only within its read transaction. | 20; carried port gaps and current clean visibility. |
| INS-05.7–05.10/M40 already require two bounded observations. | Pin actual observations without rerun; compare membership, available rank and each metric separately, unranked graph membership, ambiguous origins, caps/staleness/eviction. Fit inside ten observations/20 MiB, no separate pin budget or persistence. | 60; 20–50 real observation contracts. |

Public callable is not a guarantee of all Inspector semantics. `tc5_vector_stage`
is feature-gated benchmark access; test hooks are fixture-only; private search/
vector internals cannot become product imports. A missing required route must
first evaluate Inspector-local bounded reads with conformance; excluded Engine/
SDK/MCP/schema/ranker/API change remains a Slice 9 scope blocker, never silent work.

## Research, verification and handoff

Do not run web research as part of writing this draft. During future review,
if source leaves a real semantic question, consult primary SQLite FTS5
documentation for tokenizer/highlight/snippet meaning, sqlite-vec documentation
for actual selected metric/limits, and browser accessibility documentation for
keyboard fact equivalence. Source/API contracts remain authority for this build;
research does not approve upgrades or substitute for independent fixtures.

Adopt quality-roadmap real databases, authored lexical/adjacency/L2 oracles,
identity/UTF-8/value properties, targeted policy/cancel/resource faults and actual
browser/keyboard/human tasks. Defer global MC/DC/fuzz/allocation programs. The
new evidence is a working record foundation, current public snippets, format-37
visibility and legitimate queued erasure; none proves unimplemented native views.
Measurement tools added during later feature work stay in Inspector's existing
tracked harness; raw logs need not become additional receipts.

Future direct fresh Sol-high design review checks source fidelity, draft authority,
unique ownership and missing mandatory positives; Terra checks prose, links and
traceability. Scoped Markdown checks suffice for Slice 3; no artificial failing
test/runtime-green. Slice 4 receives source-route/architecture questions, Slice 5
receives exact matrix/oracle deltas, and Slice 9 receives one reconciled item per
finding including 6/7 feedback. Status is written only on actual execution.

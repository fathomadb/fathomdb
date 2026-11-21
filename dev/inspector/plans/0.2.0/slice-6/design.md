---
title: Inspector 0.2.0 Slice 6 — documentation evidence design
status: DRAFT
date: 2026-11-19
target_release: 0.2.0
---

# Slice 6 draft design

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

This is a review/report design, not a runtime architecture or authorization to edit
reviewed documents. [Scope](../scope.md), [plan](plan.md), current [HLD](../../../design/high-level-design.md)
and [master mapping](../plan.md) bound the work. Baseline inspected 2026-11-19:
adoption `609dd85c41`, Engine `6edc1155f`, format 37, unpublished Inspector 0.1.0.

## Evidence flow and report shape

Use one concise human-readable `findings.md`, created on execution, with an audience/
root coverage table, material evidence gaps and deduplicated finding rows/details.
No database, crawler, permanent evidence registry or generated acceptance register.
Each finding carries:

| Field | Meaning |
| --- | --- |
| Key / aliases | `doc:<repo-relative-path>#<heading>:<claim-topic>`; aliases preserve peer/path identity. |
| Audience / owner | User, library/API/SDK consumer, contributor; Inspector or named external subsystem. |
| Claim | Exact excerpt plus path/heading/line at inspected revision; tense, target version and asserted support level. |
| Witness | Source/interface/decision/run/transcript origin, revision/date, availability and limitations. |
| Classification | Wrong now, ambiguous/incomplete, intentionally historical, already corrected, unresolved evidence. |
| Proposal | Exact replacement text or explicit retain/redirect/no-action; impact, rationale and prerequisite evidence. |
| Allocation | Single lead slice/owner, peer aliases, external-authorization requirement and expected verification. |

Do not count intentionally historical or already-corrected items as required fixes.
A source witness establishes what code exposes at one revision; a run establishes
only the exercised build/environment/cases. An accepted design establishes intent,
not implementation. Preserve disagreement as unresolved; source cannot silently
supersede an accepted decision and an old accepted proposal cannot prove code exists.

## Baseline candidates to investigate, not final audit findings

- “Documentation only/no application” requires date qualification: actual worker,
  assets, CLI and package exist. Current public/source entry pages already state this;
  S16's documentation-only description of its own historical diff can remain correct.
- Old full-v0.1 promises must remain distinguishable from 0.2 adoption and current
  0.1.0 package behavior. Keep mandatory carried obligations, High cases and unresolved
  historical Slice 20 gates; do not turn adoption into release evidence.
- HLD lexical-table “no sibling snippet API” conflicts with visible current public
  `search_text_only_filtered_with_snippets`, view/frozen/reranked routes and EP-17
  interface prose. Proposed wording: “Current selected source exposes these public
  snippet routes; qualify their exact evidence/read semantics for Slice 30. Property
  FTS/BM25F expose no snippet option; Inspector has not yet implemented lexical views.”
  Check the whole route before choosing it; MCP byte limits and envelope are a separate
  consuming-developer contract, not evidence that Inspector uses MCP or has its UI.
- Test prose must retain port versus remediation boundaries: 39 then 41 ordinary
  Rust cases, ten port browser cases, four explicit native/scale/allocation cases unrun,
  installed/22.04 compatibility unqualified and full gate stopped at missing own `.venv`.
  Historical review's direct compiled-artifact execution is not a fresh Cargo build.
- Proposed operating wording: “Inspector controls inspect; its owned Engine may
  change files/projections during normal startup/background/shutdown. External writers
  must stop; initially nonempty WAL/journal is unsupported; preserve files/sidecars.”
  Retain existing D-01 flags/conditional narrow export and no erased-byte reconstruction.

## Architectural fit, challenges and allocation

No new UI, transport, storage reader, ranker or API is needed for this review. Source
inspection is feasible with existing `rg`, contracts and port/review records. Genuine
challenges are mixed historical/current prose, diverging proposal/source timelines,
partial transcript availability, and a passing test being mistaken for full support.
A missing positive witness is a gap; it is not evidence of impossible functionality.

Slice 2 owns cruft disposition; 3 owns proposed canonical need/requirement/AC edits;
4 owns capability/design feasibility; 7 owns build/CI/package evidence; 6 owns this
report's audience/wording. Reuse peer keys and link their evidence rather than reopen
reviews. Slice 9 decides actions; conditional 10 handles only approved early items;
20/30/40/50/60 carry feature documentation and 70 final installed/support wording.
Outside-Inspector docs/API defects are referrals pending separate authority.

Future web research is conditional, not a sweep: if a claim depends on Chromium
profile/security semantics, consult official Chromium documentation for the selected
pair; if it depends on FTS highlight/tokenizer behavior, consult SQLite's primary
FTS5 docs and the selected source version. Record question, version and relevance
first. Current source and authoritative local decisions take priority for project
behavior; no product comparison, library refresh or new benchmark campaign is needed.

Before review, refresh changed source/claims and peer allocations, then revise or
reject stale proposals. Direct Sol-high review checks substantive design changes;
Sol-high content review and Terra verification inspect witness accuracy, links,
coverage/gaps and scoped Markdown results. No artificial prose RED/GREEN or runtime
green claim. Preserve approved feature real-DB/independent-oracle/property/resource/
cancellation/browser/keyboard/human obligations without performing them here. The next
reviewer needs the final planning HEAD, accessible evidence inventory, peer keys,
resolved current slice mapping and explicit outstanding transcript/qualification gaps.

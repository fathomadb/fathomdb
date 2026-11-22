---
title: Inspector 0.2.0 documentation evidence findings
date: 2026-11-19
status: REVIEW
---

# Audience documentation review

Reviewed at `aef5b2eac19dbcf5850713081ccd427896909eea` after the plan/design/
filled prompt and 0–5 reports. Keep adopted scope and historical records;
adjust draft provenance and already-current source/public entry pages. No
canonical/public doc correction applied. Read-only discovery used rg across
root README, docs, dev, source READMEs/public Rust/Python/TS exports and examples;
search topics were Inspector/current release, snippet/Caller, schema, cache,
Engine effects and qualification. This is a bounded claim review, not proof
all repository prose/API docs are current.

| Audience / searched roots | Actual witnesses and limit |
| --- | --- |
| Users: docs/index.md, docs/inspector, src/inspector/README.md, MkDocs nav | Inspector entry/getting-started/reference/operations/contributing distinguish unpublished 0.1.0 record browser from adopted 0.2.0; source README records glibc 2.39 limitation and explicit native tests. General docs entry still calls it planned/unshipped, which is true of distribution; clarify only if useful, not a mandatory defect. |
| Consuming developers: dev/interfaces/rust.md EP-17, SDK Rust exports, Python __init__, TS index, MCP contract and manifests | Current public snippets exist and are exported; no assertion that Inspector consumes SDK/MCP UI. Direct public-callable, doc-hidden Rust BM25F and test-only provider injection remain distinct. No qualified broad SDK parity audit. |
| Contributors: dev/inspector canonical owners/HLD/plans/reviews; dev/DOC-INDEX.md; source README/tests/tool index | Current navigation and stale current-route guidance findings below; existing historical S15/16/20 statuses remain evidence. Build/CI root cause authority is 7. |

Authorized evidence consists of this task's owner commission, tracked adopted
scope/checkout ruling, port/review/status records and actual source/interfaces.
No full earlier chat transcript or hidden history store was supplied/inspected;
those are explicit evidence gaps. Durable records establish their stated rulings,
not unseen conversational intent. Historical executed checks are not rerun here.

## Deduplicated findings and exact proposals

| Claim key / alias | Existing excerpt at source revision and classification | Exact proposed remedy / owner / verification |
| --- | --- | --- |
| doc:dev/inspector/design/high-level-design.md#existing-mechanisms-and-new-work:snippets / D2-D6-HLD | Lexical table: “current source has no sibling snippet API”; wrong as current source guidance. Other paragraphs qualify historical S16 findings, but this table can guide new 30 incorrectly. Source/interface witness: current filtered/view/frozen snippet methods, EP-17 same-reader extraction; projected/BM25F have no snippet option. | Replace with “Current selected source exposes filtered/view/frozen/reranked snippet routes; qualify route-specific evidence in Slice 30. Projected text and BM25F expose no snippet option. Inspector lexical views remain unimplemented.” 10 factual correction if approved; 30 detailed contract. Verify public gate/body and separate observation context; no new capability promise. |
| doc:dev/inspector/design/high-level-design.md#query-adapter:provider / D2-D6-HLD, S4-CACHE | HLD helper/Caller composition conflicts with production Default/None and normal loader's fetch/write behavior. Wrong selected seam; standalone constructor existence is not Engine serving. | Replace selected route with “Cache-only CPU Engine serving remains an unresolved prerequisite on this source. Production Default/None and accepted admission policy apply; no test/evaluation Caller route is product support.” Preserve required positive and investigate supported contained composition under 40. 10 factual correction if approved; 4/40 feasibility owner. |
| doc:dev/inspector/design/README.md#inspector-design:allocation / D2-D6-NAV | “Feature-level high-level drafts are colocated ... plans/0.1”; incomplete current pointer, valid history. reviews README target 0.1 and old plan link remain historical but lack current navigation. | Add “Current 0.2.0 delivery drafts and ownership are in plans/0.2.0/plan.md; legacy 0.1 drafts/reviews retain their original source/evidence scope.” Keep old paths/content. 10 approved docs; verify links/anchors and no historical status rewrite. |
| doc:dev/inspector/needs.md#related-specifications:plan-link / D2-D6-NAV | Label “the v0.2.0 plan” targets plans/plan-0.1.md; wrong destination for current plan. requirements/acceptance current execution prose also names legacy 20–60/final 60. | Point current label to plans/0.2.0/plan.md; retain separately labeled legacy plan. Apply exact allocation text from Slice 3 draft, comparison 60/final qualification 70; retain historical statements. 10 approved docs; S3-MAP alias for matrix owner cells. |
| doc:dev/DOC-INDEX.md#inspector-product-area:current-navigation / D2-D6-NAV | Product rows describe canonical needs/requirements/acceptance/source as “Inspector v0.1” and omit current 0.2.0 ladder. Historical v0.1 draft section is intentionally historical; product entry rows are ambiguous. | Add current adopted-scope/0.2.0 plan row and qualify current canonical rows as 0.2.0 scope with unpublished 0.1.0 implementation; keep historical draft section and dates. Inspector index owner, 10 only if approved; no broader index cleanup. |

Each key is one semantic remedy with aliases to 2/3/4/7; 9 counts overlap effort
once. Route/helper factual corrections are separate from feature implementation.
No change in need/criterion, Engine API, pins, immutable-read promise or supported
multiple-window guarantee is proposed by wording.

## Keep/already-correct and evidence limits

Retain docs' owned Engine startup/background/shutdown effects, stopped external
writers/current schema/nonempty-sidecar refusal and preserve-files guidance.
D-01 independent gates, legitimate queued scope, exact narrow export and no
reconstruction are current; physical deletion is correct and unchanged.
No stale absolute immutable/no-Engine instruction was found in the sampled
current Inspector public guidance. Historical “documentation-only” S16 diff
statements and format-34 source tables remain correct for their dated context.

Port records distinguish 35 then 37 unit/DB tests plus CLI/no-GUI/two workers
(39 then 41 total), ten original browser checks, four explicit native/scale/
allocation cases unrun, compiled-artifact review versus fresh build, missing
checkout-owned .venv full-gate stop and no all-view/22.04 installed qualification.
Do not rewrite these as current release pass. Future support wording belongs to
70 after actual installed/native/browser/human qualification; 20–60 document
implemented feature behavior locally. S5-METHOD owns evidence-method clarity.

No new architecture, legal/retention rule, broad audit or current user-demand
inference needed. Source/link review and Markdown limits are in [the shared record](../slice-9/prework-verification.md).

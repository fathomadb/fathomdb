---
title: Inspector 0.2.0 Slice 9 — decision and handoff design
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: DRAFT
desc: Evidence-preserving consolidation and explicit human authority for conditional Slice 10
---

# Slice 9 design

## Baseline and design boundary

This is a document/session design, not new product architecture or an executed
human approval. It consumes [scope](../scope.md), [master allocation](../plan.md),
[shared workflow](../slice-workflow.md), actual 0–8 outputs and the
[HLD](../../../design/high-level-design.md). Baseline inspected 2026-11-19 is
adoption `609dd85c41` over Engine `6edc1155f`, packages 0.8.28/format 37.
Existing record browsing and historical port checks are not new feature passes.

No new need or product AC is inherent. All existing INS/V01/X/F/T/High and M40
obligations remain canonical. Local draft edits selected here are proposals
until ratification, then implemented only by the assigned owner. 9 cannot
commission outside-Inspector implementation or reduce a mandatory positive.

## Data and authority flow

```text
Actual 0–7 reports + reserved 8 + current source/contracts
  -> one de-duplicated S9 item table with all aliases/evidence limits
  -> interactive human item decisions, dependencies and unresolved questions
  -> selected conditional 10 plan + local contract/design edits
  -> independent draft/design review -> at most FIX-1 through FIX-4
  -> Terra correspondence/source-limit verification
  -> human ratification of exact revised plan and item set
  -> 10 only for that approved set; blocked dependencies stay explicit
```

The single table in [plan](plan.md) is the consolidation authority, not another
acceptance register. Keep source item IDs as aliases when merging a cause/remedy;
split remedies that require different ownership, acceptance or independent
approval. A rejected duplicate points to its surviving item; do not lose a
conflicting observation. Row order/risk is presentation, never consent.

Every item carries evidence/baseline, outcome, owner, understanding/risk/effort
with reasons, recommendation, affected canonical IDs, dependencies, verification
and actual human response reference. Understanding is distinct from risk: a
well-understood policy change may be high-risk; an unknown implementation is
not low-risk because its diff might be small. Do not total scores into approval.

## Session state and exceptional paths

Use `UNDECIDED -> proposed include/postpone/reject/defer-evidence -> human ruled`
as a documentation convention, not an automated app. Record actual decisions
only; the draft starts with no ruled items. Explicit human batches must name
the exact IDs/conditions. Clarify an ambiguous response instead of guessing.

Before ratification present a concise selection/dependency delta after review.
A changed item or added prerequisite returns to human decision; prior inclusion
does not authorize an expanded remedy. Record final plan revision, exact selected
IDs, unresolved blockers and actual ratification response. Missing response leaves
10 unauthorized. Stop after four unresolved FIX cycles; do not invent acceptance.

Unknown evidence yields a concrete question/owner; a bounded investigation can
be selected explicitly but does not authorize speculative implementation.
Rejected/postponed dependencies are not auto-included. A missing mandatory
positive blocks its feature even if review recommends postponement. An external
Engine/SDK/MCP/schema/ranker/API/pin dependency needs separate authority and a
named handoff; it never becomes an Inspector-owned 10 item by relabeling it.
A ratified empty set gives 10 a no-change result, with all unresolved mandatory
feature blockers retained. 9's completion certifies the decision workflow only.

## Source-backed architectural challenge and review

[open.rs](../../../../../src/rust/crates/fathomdb-engine/src/open.rs) has production
`EmbedderChoice::Default/None`; `TestEmbedderChoice::Caller` is cfg-contained.
The [accepted ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
prohibits treating test injection as production cache-only admission. Public
[search routes](../../../../../src/rust/crates/fathomdb-engine/src/search_api.rs)
include frozen/snippet/evidence operations, so stale HLD absence claims need
source reconciliation. These observations do not prove all cached CPU serving
is impossible or supported; 4/9/40 must resolve supported routes and runtime
ownership, preserving the required positive and out-of-scope Engine boundary.

During execution research only unresolved concrete questions against official
source/docs: supported public offline initialization, actual caller/cfg contract,
and artifact/tool compatibility. No web investigation is claimed now. Reconcile
changes since drafting before review; label public/private/test-only/missing
seams explicitly. Sol-high reviews source fidelity, authority and dependency
closure; Terra checks every source item and human ruling maps correctly to 10.
Prerequisite for the next reviewer is complete 0–8 evidence, one reconciled table,
actual decisions and the exact revised 10 draft, not a presumed approved bundle.

Use scoped Markdown/link/traceability checks without runtime certification or
artificial TDD. Real DB/oracle/property/fault/resource/browser/human qualification
belongs to approved code and feature owners; no global MC/DC/fuzzer campaign.
Create concise status only on execution; preserve the sequential release checkout
and unrelated work, cleaning only verified slice-owned extra isolation.

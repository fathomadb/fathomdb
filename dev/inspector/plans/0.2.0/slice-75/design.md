---
title: Inspector 0.2.0 Slice 75 — UX remediation design
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Preserve exact evidence and ownership while making native inspection readable
---

# Slice 75 — UX remediation design

This design is grounded in the [seven observed findings](../../../evaluations/0.2.0/expert-ux-2026-11-20/README.md)
and current `1ac4b49f3` source. The [plan](plan.md) owns acceptance and completion;
canonical identity, observation, policy and budget contracts remain unchanged.
The executor must inspect actual code and obtain Sol-high review before additions.

## Existing owners and bounded changes

- `assets/app.js` owns attachment/navigation, retained observations, labels,
  query controls, graph expansions and original-view restoration. The single
  `notice` element in `assets/index.html` currently mixes durable attachment and
  transient request feedback. Separate these owners and update durable state only
  from accepted current-session responses; stale asynchronous errors cannot win.
- `assets/comparison.mjs` derives typed native entries and a bounded multiset
  union/intersection without copying result payloads. Keep its native key semantics.
  Add a separate bounded exact-origin correspondence index over permitted known
  database/attachment/class/cursor/revision facts. Do not compare opaque handles
  alone, infer currentness from absent revisions, merge rows or convert scores.
  Unknown, multiple or refused origins remain explicit; any matching group or
  count is emitted only after current-policy validation and charged to retention.
- `assets/style.css`, `index.html` and renderers can establish a compact action
  header, flexible `min-width:0` content tracks, sensible wrapping and locally
  scrolling raw regions. Make stored content, query, results and origin actions
  primary. Raw context remains directly accessible through bounded expansion;
  collapsed does not mean unaccounted or retained after its owner is forgotten.
- `src/storage.rs` already has qualified bounded scalar previews, exact handles,
  classification and shared work/response budgets. Inspect actual response shapes
  before extending them. Human-readable labels must use admitted stored fields,
  preserve exact IDs and fall back honestly. Never run unbounded full-document JSON parsing
  or unbounded per-record hydration simply to obtain a title; any necessary named
  preview read shares the reader's classification/work/byte budgets.
- Catalog/projection declarations already expose actual names, roles and scoped
  availability. Build property choices from exact eligible declarations, not the
  literal `biography` default. Distinguish configured/effective/available facts;
  truncated/unknown metadata cannot authorize a query. Keep request-local feedback
  and old-result attribution separate when a replacement request fails.

## Graph and evidence ownership

Graph UI state should retain the requested expansion owner and a usable branch
collapse action even when its bounded response cannot include that node again.
Do not switch silently to the seed. Shared nodes/edges survive until all retained
expansion owners release them. Preserve original observation/Back/focus and count
all retained graph state, event handlers and correspondence indexes. Inspect the
actual graph update/render functions before settling keys: endpoint keys are
response-local; exact record identity is not the diagram preview label.

Keep a bounded stable SVG viewport and synchronized exact relationship list.
Make a hub readable with minimal local navigation/selection controls as needed;
no force-layout dependency or new graph framework. All controls need keyboard and
pointer parity. Partial bounds, examined links and retained objects remain visible;
geometry never acquires stored strength or vector-distance meaning.

Summaries are presentation of established facts: actual method, input/filter/view,
limit/population, own metric/direction, read time and scope. Later readiness or
eligibility uses its own later timestamp/source; keep unknowns explicit. Render
recognized machine tokens as concise labels with raw evidence available. Date
formatting preserves the original exact epoch; invalid/out-of-range values remain
unformatted rather than rounded. UI text and markup stay inert. No narrative
explanation of omission, shared snapshot or eligibility is synthesized.

## Verification design and release effect

Design review must settle the exact-origin key/proof, any new preview read,
property declaration source, durable/transient feedback state machine, bounded
expansion ownership and DOM cleanup before code. Tests must demonstrate each
original failure and independent expected identity/state, not reproduce the new
implementation's own formulas. Existing real-DB, property and installed harnesses
are reuse seams. The preserved expert database is a concrete integration subject,
not a replacement for small authored regression fixtures or all-policy negatives.

After source corrections, rebuild the Ubuntu22-compatible cached-CPU package and
verify actual installed default/narrow-window views with screenshots, exact copy,
keyboard/Back, cancellation/late invalidation and native close. Record the new
build identity separately from Slice70's historical package. Apply targeted
resource/policy checks to changed ownership and preview paths; unchanged native
rank/cache mechanics need no new algorithm campaign. Slice70 remains responsible
for the five real participant sessions on the updated candidate. Slice75 cannot
turn expert or Terra use into a human session.

## Reviewed implementation decisions

Sol-high approved the source-grounded design; code re-review at `5d1163eaa`
approved both graph corrections. Attachment state has a dedicated live region;
transient operations retain their own serial/navigation guard, including fetch
and decode failures. Canonical correspondence requires exactly one permitted
origin with complete database, attachment, class, positive decimal cursor and
revision descriptors. Exact physical identity is established by database/attachment/class/cursor; revision display may be bounded. The bounded index references original entries and preserves all
native keys, scores, multiplicities and independent contexts.

The reader admits a title/name preview only after payload classification. It
reads complete JSON bodies of at most4096bytes under the shared content budget,
returns at most120characters, and labels oversized/absent/budget-limited fallback.
No SQL JSON full-body extraction or unbounded per-card hydration is introduced.
Property choices use actual bounded names, CSV role facts and recorded tokenizer
strings (an empty string denotes the default); anchored query views separately
own their catalog observation and release it through existing eviction paths.

Raw evidence stays directly accessible in contained regions. Compact summaries
use each actual request and separately timed later checks. Exact BigInt epoch
formatting recognizes only declared seconds/milliseconds within the Date range;
raw scalar copy is unchanged. No omitted-candidate cause or shared snapshot is
inferred. Graph anchors are policy-validated and prioritized within the existing
shared response/work limits. Each explicit expansion owner starts at depth0;
`depth_origin` labels that scope. Collapsing the seed cannot remove adjacency
owned by a separately expanded hub. Geometry uses bounded stable presentation
slots, local scale/scroll controls and synchronized exact selection, with scope
before the relationship list. No geometry is evidence of stored strength.

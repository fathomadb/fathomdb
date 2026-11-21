---
title: Inspector 0.2.0 Slice 60 — observation comparison design
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: COMPLETE
desc: Two bounded actual observations with typed identity and separate metric contexts
---

# Slice 60 — observation comparison design

## Fit and current implementation

Inspected scope-adoption `609dd85c41fb7980e05faa1132a248d376d8ec89` on 2026-11-19; that historical draft baseline is superseded by the concrete current-source design below. Follow the [plan](plan.md), [scope](../scope.md), current [master](../plan.md) and [HLD](../../../design/high-level-design.md). No
new user need is proposed: INS-05.7–05.10/M40 already adopt this behavior; changes here refine its realization.

Existing browser `observations`, `observe`, `evict`, `invalidate`, `acceptContext` and `api` in
[app.js](../../../../../src/inspector/assets/app.js) supply retention, cache-version/epoch rejection and global context invalidation. The
[storage reader](../../../../../src/inspector/src/storage.rs) supplies opaque exact locators and per-read context with unknown write
boundary; native text/vector/graph/history routes now exist. Comparison state and per-method result adapters are the new Inspector work. Extend these owners rather than create another cache, Engine wrapper or persistent subsystem.

## Observation and identity model

Each side references one admitted registry observation, not a copy or a query recipe to rerun. Retain its original request/seed, effective
route/options/filters, limits, returned count, actual measured population/stages or explicit unknown, read
interval/generations/build/session and completion/truncation status. Later hydration/checks keep separate context; they cannot upgrade
historical evidence. Separate times are distinct observations even when generations agree. Never claim one snapshot merely from equal
generation fields or a pin operation.

Use a typed, lossless key: database/session, artifact class, namespace/value and revision evidence where known; preserve physical entry
identity for passages, vector windows, parallel graph edges and missing-revision exact locators. Bare strings, equal text, logical ID alone
and origin cardinality are not equality. Rows with unknown revision evidence cannot be merged with a known revision by assuming currentness.
Expose recorded same-logical/different-revision relationship without replacing either row. Distinct result entries stay distinct even if
they share one canonical origin. Show optional verified canonical correlation as links, not deduplication; zero origins remain unresolved,
one resolves exactly, many present choices. Never coerce a content/passage identity into canonical identity.

Comparison is a bounded union of the actual admitted rows, with A/B membership and each side's actual available rank/value. Label method,
metric, direction and population separately; no cross-family bars or synthesized delta of unlike scores. Graph sets have membership with
rank/value not applicable. Missing side means “not returned in this bounded observation”; it is neither rank zero nor proof of storage
absence, ineligibility or a causal explanation. Policy-restricted rows do not expose a hidden identity through a membership badge, aggregate
or label.

## State, resource and disclosure transitions

State is empty/one pin/two pins plus explicit refused/expired/evicted placeholders. Pin A/B acts on a completed or honestly partial actual
observation; replace is explicit and atomically swaps a reference; unpin releases it. Pins gain no extra retention entitlement: existing ten
observations/20 MiB include all pins, graph expansions, cached values and comparison projections. Derive comparison rows from existing
values and bound/paginate rendering; any retained derived bytes count. Do not emit a 400-row union through a 200-item/2 MiB response;
locally page the bounded union or use a capped shared-registry adapter within the same limits.

D01 revalidation precedes new comparison emission, origin navigation and copy. Retained pin data, derived indexes, history references and
DOM nodes must register with the existing invalidation/clear path. Classification/generation changes clear sensitive retained payloads as
today; pinning cannot freeze policy or retain a snapshot transaction that delays erasure. A safe original locator/context may survive only
if policy permits it. Changed generation is new observation, not a new database identity; comparing differing read times is allowed while
evidence remains admitted. Once invalidated, show unavailable evidence and require explicit fresh observation; do not relax invalidation to
preserve an old comparison.

The pin slot has a selection/version token in addition to existing request epoch and cache version. A late hydration/query cannot overwrite
replacement/unpin, reattach, cancel or invalidation. Clear comparison DOM and captured closures on those transitions; errors stay distinct
from complete empty. Existing one costly request/eight waiting and cancellation/shutdown bounds apply without another queue.

## Coordinated interaction and challenges

Two labeled side panels expose context; a keyboard-readable membership table/list preserves per-side facts and explicit origin actions.
Pin/replace/unpin, row select, origin choice and Back use native controls, visible focus and announced changes. Eviction announces the
affected side and returns focus predictably. Native text, vector and graph renderers retain their own encodings and equivalent numeric/text
facts; comparison does not replace any native view. Selection/Back restores the exact source observation and query scope, never silently
reloads current content.

Person workflow uses actual verified associations: complete biography/metadata, indexed text/match, stored vectors/window/profile, supported
semantic/hybrid and Find similar, directed relationships/edge body, readiness/lifecycle/provenance, then exact origins and original
observation. Derived canonical records remain separate from projections. No similarity link is a stored relationship.

Main risks are mismatched adapter identity, accidental copied payload retention, policy leaks through union membership and async replacement
races. Future design review must inspect actual predecessor envelopes/cfg/call sites and resolve these with independent fixture facts,
property invariants and targeted browser cases. The mandatory positive public cache-only Engine route remains prerequisite 9/40: production
Default/None does not admit helper Caller. Consume 40's qualified supported head-window/profile/metric/origin contract and actual
cache-only CPU Engine serving evidence. S9-010 defers the within-source multiple-window positive; exact metadata, revisions, no
fabricated windows and every other person/compare criterion remain. Missing cached Engine serving stays a blocker. No Engine change,
copied ranker, test-only positive or unsupported-pass in Slice 60.

## Research questions and reviewer handoff

No web research was performed for this draft. During execution investigate only questions remaining after code inspection: do chosen
DOM/ARIA table/list controls preserve focus and announced status under eviction (W3C ARIA APG/HTML primary sources), and do actual browser
memory measurements include detached DOM/closures (Chromium DevTools/CDP documentation)? Use the existing browser harness first; no library
sweep, generalized benchmarking or new UI framework is commissioned. Before Sol design review provide current native observation contracts,
supported route/admission evidence, bounded comparison state transitions, fixture expected identities/origins/metrics and D01 invalidation
ownership. Before Terra verification provide the running build, reproducible real-database fixtures and independent expected outcomes. Final
installed/support/docs/human qualification belongs to 70; Slice 60 must first finish its own errors, accessibility, tests and working
feature.

## Concrete current-source design — 2026-11-20

Current source at668bf73e implements all native routes. New browser-only
`comparison.mjs` supplies pure typed adapters and a50-row union/intersection
iterator; the existing authenticated static-asset server serves it. No new
worker operation, Engine call or response envelope is needed. Original envelopes
remain the only payload store. Native result identities retain class, IdSpace,
value and exact cursor; representative retrieval arm is per-side evidence, not identity; when native identity is unavailable the key is observation
and ordinal, never a fabricated canonical identity. BM25F cursor identities remain
its own representation. Vector row identities include profile/row/source cursor.
Canonical graph membership uses class/exact cursor/revision, never endpoint key.
Frozen graph result identity includes target plus terminal edge, retaining parallel
paths; recorded origins are separate exact links. Each key includes database and
attachment. Unknown revision does not alias known revision. No Number coercion.

The registry stores each actual request at admission. A pin slot contains only
observation ID, source view kind and a monotonic version. Pinning validates the
source and checks the version, navigation/cache epoch and same observation before
commit. Explicit Replace A/B is shown when occupied; Unpin clears the reference
and comparison projection. Native query edits never relabel a pinned envelope.
Graph replacements release their original observation only if it has no pin;
there is still one registry owner and ordinary FIFO eviction, with no pin priority.
Unpin releases pin-only retained observations; history/current ownership may keep
an original observation until ordinary eviction. A/B can reference the same read.

Comparison builds a temporary bounded index of typed entry keys and renders at
most50 rows. The source values are referenced, never cloned. Derived allocation
uses conservative UTF8/UTF16 key/text/DOM accounting against the existing20MiB
budget; refusal clears the transient DOM and leaves a usable explicit error.
Index and DOM cleanup is registered with observation invalidation and the global
comparison clear path. No400-row output crosses the200-item response contract.
Separate side panels show original request, seed, scope/population/status, query
and policy contexts, rank, own score/distance metric/direction and payload
completeness. Each row states membership; missing means not returned in that
bounded observation. Origin buttons retain side-specific exact choices; opening
native observation restores its actual retained data and safe original context.

Before opening/emitting/acting on comparison, validate all A/B disclosable owners
and origin handles through existing Check transactions, then verify slot versions,
cache epoch and selection/navigation. Background polling validates pins as well
as current selection. Changed context clears all observations as today; pinning
never relaxes that rule. Eviction, refusal, expiry, detach and cancellation clear
pin payload/DOM/closures synchronously and announce availability; no auto requery.
The comparison view itself has no independent observation or payload cache.

Real lexical/graph and cachedCPU person fixture observations exercise overlapping
and disjoint membership, equal body distinct entries, revisions and origins,
keyboard/pointer pin/select/Back, exact metadata and complete person evidence.
Pure human-authored adapters include zero/many origins, Unicode and >2^53 decimal
identities, all physical/class/namespace distinctions, no graph rank and bounded
union/intersection properties. Existing all8D01 realDB fixtures extend comparison;
held Check replies prove replaced/unpinned slots cannot resurrect. Reviewers are
source-read-only; runtime/build ownership stays serialized.

The direct Sol-high design review approved these changes with three concrete
integration conditions: combined content/metadata Check batches<=201, scrubbing
all consumers on forget/eviction, and source-only eviction preserving admitted
native results. All are implemented and exercised. Native view cleanup subscribers
share the existing registry owner. Back disposes discarded forward views and
releases only their unpinned, otherwise unreferenced observations. Source-only
eviction preserves a safe initiating focus label while rebuilding the admitted
result; policy invalidation removes that label with its result.

The direct code review identified two corrections: producing/representative arm
cannot split native identity, and standalone frozen graph evidence must validate
all target/terminal-edge/origin owners even after unpin. Both have separate
intended RED/GREEN tests, including real wall-clock expiry without generation
change, polling and Back. They introduce no new Engine route or relaxed policy.

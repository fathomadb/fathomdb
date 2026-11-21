---
title: Inspector 0.2.0 Slice 50 — stored graph design
status: ACTIVE
date: 2026-11-19
---

# Slice 50 — stored relationships and retained evidence

Original planning baseline at scope adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`, unchanged
Engine `6edc1155f`, format37. Current implementation follows the qualified
contract below; [status](status.md) records actual verification. Refines
[architecture](../../../design/high-level-design.md) and [scope](../scope.md);
[plan](plan.md) owns draft criteria, TDD, review and closure. Reconcile current
master mapping first; 60 comparison and 70 integration do not absorb this work.

## Existing versus new and route boundaries

Existing Inspector `storage.rs` has exact class/cursor/revision detail, chunks,
node/edge fields, source associations and policy classification. Session/worker/
protocol and static UI shell exist. New work is stored adjacency/history adapters,
complete relationship semantics and interactive SVG/keyboard navigation.
Public `graph_neighbors` gives nodes, 1–3 hops, 50 cap; it neither enumerates
full edges nor proves historical adjacency. Public `graph_expand` is serving
traversal; inactive/superseded context is rejected. Frozen `include_evidence`
and public `resolve_graph_evidence` expose exact selected target/terminal edge,
not all historical edges. `trace_dependency` is frozen reciprocal source-derived
trace; `read_dependency_closure` is keyed status. Private helpers/test hooks
may inform qualification and fixtures but cannot become product dependencies.

Proposed named Inspector-local current-layout reads: `stored_adjacency`,
`retained_revisions`, `recorded_source_links` and `recorded_dependencies`.
These are bounded allowlisted adapter operations, not arbitrary SQL/public Engine
APIs. Qualify exact columns, indexes, snapshot/policy semantics and outcomes in
real DB fixtures before adoption; no copied private predicates/ranker. If the
necessary semantics cannot be qualified without excluded dependency changes,
record a blocking prerequisite for 9/50 instead of an unavailable-positive pass.

## Data and interaction contracts

An observation records attachment/read context, exact seed reference, direction,
kind filter, depth, ordering, limits, examined links, emitted/retained objects and
complete/partial/cancelled/busy/error outcome. Stored reads use one bounded read
transaction; no claim of the same snapshot as a separate serving observation.
Nodes/edges carry class, exact cursor and recorded immutable revision evidence,
logical ID where present and typed endpoint references. Endpoint stored logical
IDs do not prove an exact historical node revision: expose observed resolution
as zero/one/many/unknown under its read context; never silently choose current.
Retained revision navigation is class-qualified and never substitutes latest.
Missing revision metadata stays explicitly absent; no manufactured durable ID.

Separate link types: canonical edge, recorded source attribution, registered
dependency, verified representation origin and supersession. Source ID alone
never promises source body or derivation; show role/completeness/unknown and
bounded recorded links. Node state/reason, supersession and half-open world-time
validity are distinct from edge t_valid/t_invalid/temporal metadata; use actual
route semantics when reporting eligibility. Stored presence is not serving
eligibility, and absence from a bounded traversal is not proof of nonexistence.
High investigation exposes supported actual request/evidence and record facts;
missing causes stay undetermined rather than inferred from visual proximity.

Use directed SVG with arrowheads, kind labels, distinguishable parallel curves
and self loops, selection and explicit expand/collapse. Preserve identities
through cycles. A synchronized keyboard relationship list exposes every visual
fact/action; roving focus, Enter/detail, expand/collapse, Back and visible focus
return work without a pointer. Layout has no metric meaning. Collapse removes
only its owned expansion; shared visible objects remain until no retained
expansion references them. Release hidden graph state and announce eviction.
Reuse shared selection/Back context; do not silently requery an evicted result.

## Policy, bounded work and errors

All node/edge/body/trace/evidence/reference/copy/export paths reapply independent
D01 gates, including stale observations. History/inactive viewable by default;
`--no-show-inactive` hides applicable content, `--show-erased` admits legitimate
retained queued content only with other gates, `--show-payloads` controls telemetry
payloads. Queued physical scope is not logical removal; absent bytes stay absent.
Reuse per-read transaction membership authority, never session-only freshness.
Render stored content inert; no automatic URL fetch or credentials in links.

Enforce shared response/chunk and aggregate observation budgets, graph ≤500
objects/1,000 examined links/depth 3 and queue/deadline/memory bounds. Distinguish
returned-object caps from work caps. Bound incident scans, joins and layout
before allocation; allowlisted keyset reads and request-owned interrupt/progress
controls follow shared runtime. Engine calls without interruption keep the
execution slot until completion or bounded session teardown; acknowledged cancel
is not completed execution. No late response may replace selection or leak gated
content. Default refusal, unsupported route, partial and empty are distinct.

## Challenges, research questions and review prerequisites

- Qualify historical endpoint ambiguity and pagination across parallel/self edges
  without conflating logical endpoints with revision membership. Seed independent
  typed adjacency/body manifests and test exact identities, not screenshots alone.
- Check current engine temporal/read/graph source and accepted lifecycle/dependency
  ADRs for explicit boundary instants; do not reuse obsolete node/edge assumptions.
  Frozen serving evidence remains separately labeled and authenticated.
- During execution, investigate only needed primary sources: SVG/browser and WAI
  keyboard semantics for focus/labels; SQLite/Rusqlite progress/interrupt and
  indexed keyset guarantees; engine public frozen evidence/trace contracts and
  cfg availability. No web research or library selection was performed here.
- Before Sol-high design review supply named adapter/query bounds, route/cfg
  qualification, D01 path inventory, independent fixture/time oracles and concrete
  DOM/keyboard/resource/cancel tests. Real DB tests, property invariants and Terra
  verification are required by the plan. Commit reusable diagnostic tools; defer
  generic layout libraries, global fuzz/MC/DC and speculative framework work.

## Qualified implementation contract (2026-11-20)

- `graph` accepts an exact node/edge handle, outgoing/incoming/both, optional
  exact edge kind, depth1–3 and an explicit expanded-node handle set (max100).
  Root nodes expand initially; edge roots expose that exact edge and endpoints.
  Each request computes a fresh bounded reachable graph; collapse removes its
  expansion ownership while paths reachable from other expanded nodes remain.
- Incident scans force the existing from/to indexes with rowid keysets; both
  directions deduplicate exact edge cursors. Charge every candidate before kind
  or D01 filtering. Share1000 link/classification units,200 node+edge objects and
  the2MiB response budget. Stop before allocating the next object.
- Historical endpoint resolution has no suitable index. Scan canonical rowids
  once per request, at most100k rows/16MiB logical keys, collecting only requested
  endpoint identities. Present each retained matching revision separately. If
  scan or policy coverage is incomplete, resolution is unknown; otherwise
  zero/one/many. Logical endpoints are references, never exact revision claims.
  Revision-history uses the same bounded scan for its own class; no latest alias.
- `investigate` returns class-qualified retained revisions, recorded source and
  derived links, registered dependency links and metadata (locator/hash/role/
  completeness), all sharing200 items/1000 examined relations/2MiB. Missing or
  withheld targets have no guessed body/handle. Each step is an explicit bounded
  read; following a link enables deeper investigation without a global closure.
- `graph_serving` invokes public frozen `graph_expand` with an explicit logical
  seed, depth<=3, result<=50, work<=1000, explanation and evidence enabled.
  Public sidecar identities identify actual selected target/terminal edges.
  Hydration and current D01 validation are separate later observations; do not
  claim the frozen snapshot for those reads. Refusal remains refusal. Native
  bounded absence has an undetermined cause; graph geometry has no rank/metric.
  No arbitrary raw-ID public resolver or copied serving predicates are added.
- SVG uses arrows, text kind labels, curved parallel edges and self loops. Native
  buttons in a synchronized relationship list provide keyboard selection,
  details, expand/collapse and Back; ArrowUp/Down/Home/End move between list rows.
  Selection is keyed by class+cursor, never label. The graph workspace preserves
  its original read context/focus on Back; expand/filter/refresh are new reads.
- Validate every displayed node/edge/investigation/native origin on restore and
  polling, with existing batched201-reference check. Graph observations collectively
  retain<=500 objects; replacing/collapsing releases prior hidden graph data.
  Existing10-observation/20MiB eviction remains binding; clear detached DOM owners.
- Real ordinary/governed writes seed known parallel/self/cycle edges, missing
  endpoint, equal labels, retained revisions, full Unicode edge bytes and source
  dependency. Authored edge/revision sets and explicit temporal instants are
  independent oracles. All8D01 overlaps use legitimate ExplicitMaintenance;
  fresh classification and absent physical bytes remain required. F06 reuses
 100k nodes/logs,10k incident edges and10MiB payload, measures examined work,
  bytes/RSS/cancel/quiescence/retry/detach; no synthetic product route.

Native allocation admission: before frozen graph expansion, scan only lengths
under a bounded read (100k total canonical/metadata rows,32MiB aggregate bytes,
25s). Charge selected target bodies at50× maximum node body, terminal edge
bodies at50× maximum edge body and canonical-source hydration at100× maximum
canonical-source body, plus1000× maximum identity/metadata width and32MiB
fixed overhead. Refuse if the conservative bound exceeds64MiB. This accounts
for repeated hydration of one large source; output truncation is insufficient.
Inspect all graph source-link/revision key/locator/hash lengths without loading
payloads. Native calls retain the execution slot and existing30s worker teardown.
Fresh output classification covers seed/predecessor/target/terminal edge before
any origin identities appear; an unpermitted origin withholds its whole item.
Explicit large-source/small-target and oversized-target refusal tests and unchanged-next-read are
required. This is conservative admission, not a claim of exact allocator peak.

Direct Sol-high design review approved the qualified contract after correcting
eager native source-hydration allocation accounting. Source and oversized-target
refusal checks use real governed writes; existing class-specific lifecycle
boundary tests remain the independent temporal oracle. Current Engine prohibits
purging an in-use unregistered provenanced source, so unavailable endpoint tests
and complete physical-erasure absence tests remain separate legitimate cases.

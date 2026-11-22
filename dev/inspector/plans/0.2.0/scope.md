---
title: Inspector 0.2.0 adopted scope
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Adopted connected exploration release based on the actual 0.8.29 record-browser port
---

# Inspector 0.2.0 adopted scope

## Current release disposition

[INS-020-LOCAL-RELEASE](release-decisions.md) amends this adopted scope: the
five-person study will not occur and is removed as a release requirement.
Functional T-01–T-05, High INS07/08 and every technical safeguard remain.
Completed20–60/70 technical checks and75 UX fixes precede80 readiness and90
local-only release to ignored `dist/inspector/0.2.0/`. No public registry
publication, other-platform support or human-validation claim is included.
The following adoption baseline is historical; current execution lives in each
slice status and the current release plan.

## Adoption and release boundary

Owner ruling, 2026-11-19:

> I agree. Setup the scope of 0.2.0 as proposed.

The adopted proposal is commit `8296258180e1fe8a3d865e7911d4a1324e3dd9d3`.
This file is the canonical release-scope authority. 0.2.0 delivers connected
exploration across records, text, vectors and stored relationships, plus a
bounded in-session comparison of two observations. Scope adoption does not
commission feature execution, approve detailed designs or establish runtime
qualification; release/slice planning and execution remain separate work. The
[current draft index](README.md), [release plan](plan.md) and
[workflow](slice-workflow.md) propose the ladder; legacy final integration 60
is allocated between new 60 comparison/interactions and new 70 final
installed/integrated qualification without changing the adopted product scope.

Baseline: the [0.8.29 port](../0.1/compatibility-0.8.29-port.md) at
`51b4e7c1fb4ce4b63adf4be0a40f1952eafce4ab` is unpublished Inspector 0.1.0,
with a working record browser on format 37. Its [worker](../../../../src/inspector/src/worker.rs)
accepts Catalog, Check, Page, Sample, Detail, Chunk, Tree, Associations, Export
and Close; it does not implement text search, vector neighborhoods or an
interactive graph. The port record owns executed checks and outstanding gates.

The owner adopts the actual unpublished 0.1.0 record-browser baseline and
carries the original full-v0.1 unfinished Slices 30/40/50/60, remaining Slice 20
foundational qualification and every mandatory Need/Should/INS/V01/X/F/T
obligation into 0.2.0 without dropping or weakening any. The historical Slice 20
status remains ACTIVE; no outstanding gate is closed by this decision. The
[original allocation](../plan-0.1.md) and draft paths remain available for reuse,
not a claim of a completed full v0.1. Application/package versions and the
unchanged Engine pins remain as implemented; this adoption performs no bump.

Canonical [requirements](../../requirements.md), [acceptance](../../acceptance.md)
and the [single verification matrix](../0.1/verification-plan.md) carry the
obligations. INSP-B01/B02/B03 are scoped carry-forward refinements; INSP-B04 is
the newly scoped comparison, specified in INS-05 and INS-05.7–05.10 with PLANNED
matrix row M40, owned by Slice 60 in the current draft mapping; legacy final
Slice 60 qualification is allocated to new 70. INSP-B05–B10 remain outside 0.2.0.

## In scope

| Capability | Bounded result | Carried obligation / new increment |
| --- | --- | --- |
| Record-first coordinated views | Keep canonical ID, revision and read context anchored across document, indexed text, vectors, stored relationships and provenance. Selection/Back preserve exact origin and request context; zero/one/many origins remain honest, derived canonical records distinct. | INS-04/05/09; INSP-B01/B02; existing integration work. |
| Native FTS/BM25 | Inspect indexed text, supported fields, effective route/tokenizer and verified origin; execute lexical queries with actual filters, available match evidence and method-specific scores/ranks. State unavailable explanations rather than generating them. | Existing Slice 30; INSP-B03. No arbitrary query console. |
| Stored vectors and supported semantic/hybrid retrieval | Inspect actual profile/model/dimensions, source windows and origin cardinality; use Find similar and bounded metric-faithful neighborhoods. Separate stored L2 profile/population/candidate stages from actual Engine serving/hybrid results and fused scores. Use only compatible cached CPU query models; model-free inspection still works. | Existing Slice 40; INSP-B03. No implicit download/repair or universal model chooser. 0.2.0 qualifies the selected Engine's supported single head window per source revision, displaying its actual row/profile/cursor/window and span metadata. A positive case with multiple emitted windows from one source revision is deferred; format layout capacity does not establish support. |
| Interactive stored graph | Select directed nodes/edges with their content and kinds; bound expansion/collapse and supported inbound/outbound filters; inspect current/retained lifecycle, provenance/dependencies and revisions. Provide equivalent keyboard operation. | Existing Slice 50. Stored edges, similarity links and provenance associations remain distinct. |
| Compare two observations in session | Pin two bounded results and compare typed-identity membership, available rank and each method's own value beside seed/query, filters, limits and read context. A graph result set, if included, has membership without an invented rank. | New INSP-B04 increment, beyond generic required correlation. No shared score scale, fabricated omission cause or matched-snapshot claim without evidence. No persisted experiments. |
| Integrated qualification | Apply disclosure, readiness, eligibility, resource, cancellation, cache, security and accessibility contracts to every new view. Qualify one installed Linux x86_64/Chromium pairing against the specific unchanged 0.8.29 Engine build/current format. Preserve the existing conditional narrow D-01 export and all existing distribution, scale and operating-budget targets. | Existing cross-cutting and Slice 60 obligations; not future binary compatibility or a full feature-parity claim. |

Use the person scenario from [INSP-B02](../../BACKLOG.md#worklist): biography and
metadata, a real indexed-text match, similar people through the selected supported head-window vector/profile,
preserving exact source revision, recorded span metadata and origin cardinality, known relationships with edge content, verified origins,
readiness/lifecycle and return to the original observation. Include equal
names/text, retained revisions and unresolved links. Authored-once content may
have multiple physical representations; matching text never establishes identity.

## Out of scope

- New Engine/SDK/MCP/schema/ranker/API changes; CRUD, schema editing, arbitrary query consoles, administration, recovery or repair.
- Live attachment to an actively writing application, initially nonempty WAL/journal support, remote/cloud or multiuser access, additional platforms, historical readers or migration. INSP-B10 remains deferred/unscheduled.
- Persistent workspaces, saved queries/history/bookmarks and general export/share (INSP-B05/B06). In-session observations and Back remain in; no follow-on release date is assigned.
- Rich generic filters/facets, statistical schema/distribution or anomaly analysis (INSP-B07 and broader RM-10).
- Advanced vector-stage comparisons, ANN fidelity benchmarks, relevance judgments/optimization suites, sampled 2D/3D maps, model training, cross-encoder tuning, GPU acceleration, graph analytics or advanced layout tooling (INSP-B08/B09). Required numeric neighborhoods, actual retrieval evidence and basic directed graph interaction remain in.
- Plugin frameworks, universal database/backend compatibility, tutorials or generated explanatory narratives.

Physical deletion remains correct. Existing ExplicitMaintenance queued content
is inspectable/exportable only if it legitimately exists and passes D-01;
Inspector does not retain/reconstruct content, delay deletion or create queues.
Inspection controls do not promise immutable bytes or effect-free attachment.
This CPU-only scope changes no CUDA artifact, driver or support rating;
formal release-planning review obligations remain for the eventual plan.

The qualified distribution target is the exact Ubuntu22/glibc2.35 Linux x86_64
userland + kernel7.0.0-43-generic/LandlockABI8 + Chromium156.0.8078.4 pairing,
four CPU cores, 8 GiB available RAM and scratch at least twice fixture size,
with the exact browser/tool pairing declared in the design. The separately compiled70/75 Ubuntu22 artifacts establish that exact pairing;
stock Ubuntu22/kernel5.15 lacks cached-CPU LandlockABI>=3. No blanket newer-OS
support is claimed. F-06 retains 100,000 canonical revisions, 100,000
mutation entries, 10 MiB content and 10,000 incident directed edges; all
[operating budgets](../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
remain binding. Scope adoption reduces none of these obligations.

## Feasibility, priorities and quality

First qualify unchanged dependency routes for lexical/vector/graph semantics,
then deliver each native view vertically with identity/context intact, then
integrate the person workflow and two-observation comparison. The existing
[high-level design](../../design/high-level-design.md) is the starting point;
its historical dependency findings require revalidation on 0.8.29.
[Search APIs](../../../../src/rust/crates/fathomdb-engine/src/search_api.rs) expose
filtered text/snippet, frozen snippet and evidence/view routes;
[graph APIs](../../../../src/rust/crates/fathomdb-engine/src/graph_api.rs) expose
neighbors/expansion; [projection metadata](../../../../src/rust/crates/fathomdb-engine/src/projection_registry.rs)
and [loaded model weights](../../../../src/rust/crates/fathomdb-embedder/src/loader.rs)
provide useful seams. Public methods alone do not prove complete edge payload,
origin, cancellation or cache-only behavior. Direct stored reads need semantic
qualification. A required Engine gap needs a separate scope proposal; an
unsupported label cannot waive a required positive feature.

Apply the [quality testing roadmap](../../../notes/fathomdb-quality-testing-roadmap.md)
through real databases and the existing harness: independent person identity,
adjacency, L2 and lexical-fact oracles; property round-trips/limits; targeted
fault, cancellation, late-result and resource checks; browser keyboard/cross-view
and functional investigation tasks; all eight D-01 flag combinations with real queued
and physically absent cases; installed default build, close and read qualification.
New evidence is the working browser foundation, format-37 composition and the
fixed real 8,192-target queue-budget defect, supporting per-transaction freshness
and bounded-work regressions. Search/graph interactions and person/comparison workflows are now implemented.
Defer workspace-wide MC/DC, generic fuzzers, exhaustive allocation campaigns and
unbounded platform matrices: they do not fit this Inspector-owned change.
Reuse existing tests rather than deriving expected results from Inspector output.
Prepare a checkout-owned environment before feature execution/final required
verification; the port's full gate stopped at missing `.venv`, so no full green
claim carries forward from the scoped port checks.

## Success conditions

1. A user follows person record → actual lexical match → similar people → stored relationship/edge detail → exact original record/context, using verified links and keyboard or pointer; equal-content identities remain separate.
2. Two pinned observations retain their own query/seed, filters, limits and read contexts; membership/ranks/metrics are truthful, capped/unknown absence stays qualified, and no unrelated score families share a fabricated scale.
3. Stored materialization, projection readiness, runtime availability, eligibility and query selection remain distinguishable; errors/refusals/empty results never substitute for a required positive view.
4. High-degree graphs, supported head-window vector populations and large values remain bounded; cancellation, late responses and close release owned work without replacing the selected observation or leaking gated content.
5. The installed supported artifact exercises every required view against the declared Engine/browser pair with independently checked outcomes. Scope, unavailable capabilities and remaining support limits are stated without a market/parity claim.

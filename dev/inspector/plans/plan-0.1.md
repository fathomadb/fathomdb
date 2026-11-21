---
title: Inspector v0.1 plan
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.1
desc: Two release-wide preparation slices and five feature slices for Inspector v0.1
status: PROPOSED
---

# Inspector v0.1 plan

Historical original v0.1 allocation; the [adopted 0.2.0 scope](0.2.0/scope.md)
now owns release boundaries. Unfinished Slices 30/40/50/60 and remaining
foundational qualification carry forward without dropping any mandatory
obligation. Source package 0.1.0 remains unpublished; Slice 20 ACTIVE/gates
are unchanged. The draft ladder below is retained for reuse, not an execution
commission or a new claim that full v0.1 is complete. Current proposed execution
is owned by the [0.2.0 index](0.2.0/README.md), [master plan](0.2.0/plan.md)
and [workflow](0.2.0/slice-workflow.md). Its new 60 owns comparison/interactions;
new 70 owns final installed/integrated qualification. Historical slice numbers
and evidence below retain their original meaning.

## Status, goal, and footprint

**Scope confirmed by the owner on 2026-10-28:** all assessed Need and Should
items are in v0.1. That was the original planning baseline; the record foundation is now
implemented in the unpublished 0.1.0 port. Detailed contracts and
unresolved policy below remain proposed; this is not a release-green claim.
Inspector's product line is 0.1 (initial artifact version 0.1.0 when packaged),
independent of the FathomDB database version and the `dev/0.8.30` branch.

Deliver a local browser GUI and local service that inspect FathomDB structures
using document/structured-value, lexical, vector, graph, and coordinated
association/result views. A SQLite-grid-only delivery fails scope. The
Inspector presents factual data and controls; documentation supplies concepts.

Application-specific source belongs under `src/inspector/`. Engine, schema,
SDK, MCP and agent-access implementations/contracts/dependency pins are read-only
dependencies and references in this plan; no work on them is authorized. CPU-only content/lexical/graph
inspection must not depend on a GPU; vector queries may initialize a compatible
cached runtime. No hosted model or implicit download is part of the plan.

## Scope and acceptance ownership

All INS-01 through INS-09, including High INS-07/08 depth, and all cross-cutting
criteria are required. All Need and Should action rows below are mandatory;
Should is review provenance, not a v0.1 deferral category.

Slice 15 establishes the requirements/AC and release-wide verification baseline
for every row below; Slice 16 establishes the architecture and capability map
that satisfies that baseline. The ownership column identifies subsequent
feature implementation/completion, not permission to defer baseline planning.

| Scope ID | Required deliverable | Slice allocation and completion owner | Acceptance |
| --- | --- | --- | --- |
| V01-N01 | Explicit disclosure policy and obligation-specific handling. | 15 obtains disclosure ruling and criteria; 20 implements admission/read boundary; 50 extends trace/history handling; 60 qualifies integration. Owner 20. | V01-N01; INS-X06; F-04/F-05 |
| V01-N02 | Current-write incomplete-provenance coverage. | 20 displays metadata; 50 navigates available provenance; 60 qualifies. Owner 20. | V01-N02; F-01 |
| V01-N03 | Node/edge lifecycle and eligibility distinctions. | 20 defines typed/read context; 30/40 apply their operation rules; 50 completes graph/temporal cases. Owner 50. | V01-N03; F-04 |
| V01-N04 | Vector metric/population/stage attribution. | 40 implements; 60 integrates. Owner 40. | V01-N04; INS-09.3; F-07 |
| V01-N05 | Actual lexical route/field/tokenizer/scoring attribution. | 30 implements; 60 integrates. Owner 30. | V01-N05; INS-09.2; F-07 |
| V01-N06 | Current-schema admission; Engine startup allowed. | 20 implements; 60 qualifies installed behavior. Owner 20. | V01-N06; F-05 |
| V01-N07 | Cache-only embedding and optional reranking. | 40 implements; 60 qualifies integrated ownership. Owner 40. | V01-N07; INS-X01 |
| V01-S01 | Labeled engine-owned data, roles and count scope. | 20 implements; later views retain labels. Owner 20. | V01-S01; INS-01/02 |
| V01-S02 | Engine-backed reuse assessment, scratch/cancellation ownership. | 16 maps/selects unchanged capabilities; 20 implements/revalidates lifecycle; 30 handles lexical scratch; 40 model ownership; 60 integrated verification. Owner 60. | V01-S02; INS-X01/03 |
| V01-S03 | Non-destructive sidecar refusal UX/documentation. | 20 implements and documents; 60 validates installed messages. Owner 20. | V01-S03; F-05 |
| V01-S04 | Automated tests and actual interaction evaluation. | Every feature slice owns its tests/interactions; 60 completes integrated/human qualification. Owner 60. | V01-S04; INS-09; T-01–T-05 |

The exact signals and F-01–F-07 fixtures are owned by
[acceptance.md](../acceptance.md); numerical/local IDs are not added to the
locked FathomDB global acceptance register. Work outside v0.1 is listed in
[ROADMAP](../ROADMAP.md), not silently omitted.

## Release ladder — two preparation slices and five feature slices

Seven slices are planned in order: **15 → 16 → 20 → 30 → 40 → 50 → 60**.
The five original feature slices remain mod-10. The owner added 15 and 16 on
2026-10-29 as release-wide prerequisites; no separate setup Slice 0 or 10 exists.

| Slice | Feature scope | Draft plan | High-level draft design | Dependency |
| --- | --- | --- | --- | --- |
| 15 | Release-wide needs/requirements/AC, policy and test/verification planning baseline. | [Plan](0.1/slice-15/plan.md) | [Guidance](0.1/slice-15/design.md) | First |
| 16 | Release-wide architecture/high-level design, capability feasibility and evolution through FathomDB 1.x. | [Plan](0.1/slice-16/plan.md) | [Guidance](0.1/slice-16/design.md) | 15 |
| 20 | Local Inspector foundations, safe attachment, disclosure, inventory, canonical/operational browsing and record detail. | [Plan](0.1/slice-20/plan.md) | [Design](0.1/slice-20/design.md) | 15, 16 |
| 30 | Indexed text, FTS/BM25 route/evidence inspection and lexical retrieval with exact origin navigation. | [Plan](0.1/slice-30/plan.md) | [Design](0.1/slice-30/design.md) | 15, 16, 20 |
| 40 | Vector representations, cache-only query runtime and metric-faithful similarity neighborhoods. | [Plan](0.1/slice-40/plan.md) | [Design](0.1/slice-40/design.md) | 15, 16, 20; deliver after 30 |
| 50 | Interactive graphs, provenance/dependencies, retained revisions and class-correct lifecycle navigation. | [Plan](0.1/slice-50/plan.md) | [Design](0.1/slice-50/design.md) | 15, 16, 20, 30, 40 |
| 60 | Integrated investigations, result/outcome evidence, accessibility/resources, local packaging, docs and v0.1 qualification. | [Plan](0.1/slice-60/plan.md) | [Design](0.1/slice-60/design.md) | 15, 16, 20–50 |

Slices 15 and 16 deliver reviewed planning/design baselines and verified
handoffs, not runnable application features. Slice 15 maintains one shared
`plans/0.1/verification-plan.md` with criterion/fixture/oracle/layer/owner/stage
traceability; Slice 16 maps it to the shared high-level architecture and updates
all five delivery drafts. Completion of planning is not runtime qualification.
Slice 15 execution state is recorded in its linked status; later drafts remain unexecuted.

Each feature slice is a complete vertical feature with its own requirements/AC review,
design, implementation, tests, documentation, Sol-high design/code reviews,
Terra verification and short status record. Slice 60 integrates and qualifies
completed features; it does not absorb unfinished basics from earlier slices.
The [shared execution workflow](0.1/slice-workflow.md) is binding and names
model/effort, RED/GREEN, delta enumeration, test scope and cleanup. Only the record foundation has been implemented; remaining view/integrated
work is carried into 0.2.0.

### Coverage of Inspector requirements

| Requirement | Implemented feature slices | Final qualification |
| --- | --- | --- |
| INS-01/02 | 20 structure/inventory/detail | 60 |
| INS-03 | 20 browse/identity/detail; 30 text constraints; 50 graph navigation | 60 |
| INS-04 | 20 declarations; 30 text origins; 40 vector origins; 50 graph/provenance links | 60 |
| INS-05 | 20 typed identity; 30/40/50 preserve observations; 60 comparison/navigation | 60 |
| INS-06 | 20 basic state; 30/40 route readiness; 50 class-specific lifecycle | 60 |
| INS-07 | 20 completeness/retained identity; 50 dependency/revision navigation | 60 |
| INS-08 | 30 lexical, 40 vector, 50 graph/trace evidence; 60 coordinated outcome inspection | 60 |
| INS-09 | 20 document/structured detail; 30 lexical UI; 40 vector UI; 50 graph UI; 60 coordinated interactions | 60 |
| INS-X01–X11 | Every slice owns relevant preservation, scope, resource, observation, value, disclosure, access, accessibility and narrow export checks | 60 |

Execute sequentially in the existing task checkout by default. No new branch
or worktree is required. Keep one writer per checkout; reviewers/verifiers are
read-only and agents are commissioned directly, not through MCP. Do not use
Steward or Orchestrator. If an extra slice-owned checkout was necessary,
integrate it back into this task branch and clean up only its owned resources.

Feature drafts retain baseline `037a8522e`; new Slice 15/16 drafts start at
`d05912c99`. Every executor enumerates plan/code/feature/allocated-item deltas
from its own baseline and consumes completed predecessor outputs. Shared
requirements/design changes feed back into the verification matrix and all
affected slices rather than creating competing acceptance registers.
At slice execution create its `status.md` with brief scope/review/TDD/verification
and cleanup outcomes; no board/receipt machinery is introduced here.

## Inspector-only boundary and capability gaps

New implementation belongs under `src/inspector/`; needs/req/AC/design and
public prose belong under the Inspector sections. Minimal product-specific
build/test/docs wiring outside those directories must be enumerated in the
slice delta/plan. No Engine/MCP/SDK/schema/agent-access code, public contract,
algorithm, loader, connection-pool or dependency-pin work is included.

The previous high-level proposal suggested Engine-side read-only factoring.
That is superseded by the owner's Inspector-only boundary. Slice 16 must
produce a release-wide source-grounded capability map of unchanged existing interfaces and
feasible Inspector-local adapters for all five features. Slice 20 and each later slice
validates its specific route before implementation. Private/test-only APIs and copied private modules are not assumed usable.
The owner permitted Engine startup on 2026-10-29; qualify unchanged Engine
open/options, ownership and normal startup/background/shutdown effects rather
than require an immutable Engine-free runtime. INS-X01 now distinguishes
Inspector inspection operations from those Engine effects.

A required capability unavailable within this boundary is an explicit blocker
and scope question, not permission for Engine/MCP changes or for silently
omitting a required view. Real dependency libraries may be consumed unchanged
and existing source read for conformance; no new separate runtime dependency
project is commissioned. All prior Need/Should scope remains allocated.

## Decisions and prerequisites

| ID | Status | Decision and dependent work |
| --- | --- | --- |
| D-01 — operator disclosure | Adopted; owner ruling 2026-10-30 | Normally accessible and retained history/inactive data viewable by default; `--no-show-inactive` hides history/inactive. Erasure-pending content refuses by default; `--show-erased` displays and permits narrow local export of still-retained content. Telemetry structure/counts default; `--show-payloads` permits payloads. Independent overlap gates on all paths; no reconstruction. Canonical requirements/INS-X06/X11 and M20/M39 own details. |
| D-02 — initial attachment | Planning baseline | One existing local current-schema database; external writers stopped. Inspector may own/start the Engine and its workers. Initially WAL-present, historical-schema, other-application live attachment and forensic immutable mode are outside the baseline. |
| D-03 — architecture details | Selected S16-2026-10-31 | Canonical HLD selects Rust supervisor/session/model helper, DOM/SVG modules, exact current-layout readers, public Engine lexical/hybrid operations, cache-only Caller model and separate stored-population L2 neighborhood. No dependency changes or forced MCP session. |
| D-04 — distribution envelope | Selected S16; unqualified until 60 | One offline Linux x86_64 tar, Ubuntu 22.04 target, Chrome for Testing 156.0.8078.4 / Playwright 1.62.1; source tree at 728884bfe and locked Rust 1.95 build. Actual measured support statement belongs to 60; no publication or second platform implied. |

Before implementation, establish a checkout-owned environment and pinned tools;
Slice 20 implementation and executed evidence are recorded in its status;
that does not establish a completed integrated v0.1 application.
Never borrow another checkout's Python environment for full verification.
Revalidate against the selected engine base: public open is current-schema
fresh-only, ordinary writes can have incomplete provenance, and test-gated
structural writers do not establish production prevalence.

## Quality strategy review

Read and applied [FathomDB quality/testing analysis](../../notes/fathomdb-quality-testing-roadmap.md).

- Adopt real-database admission/preservation/refusal tests and source-grounded
  conformance for shared retrieval semantics; SQLite file validity alone does
  not prove FathomDB eligibility or Inspector fidelity.
- Adopt property/boundary tests for typed references, pagination round trips,
  identity namespace collisions, validity instants, result limits and graph
  scope. Author failing tests before implementation and keep oracles independent
  of Inspector output; do not regenerate golden fixtures to fit behavior.
- Adopt targeted faults and leak checks at attach/cancel/detach, worker/model
  boundaries and partial outcomes. Specify process/native-call behavior rather
  than promise every model call is interruptible.
- Adopt actual browser/keyboard interactions and documentation-assisted unseen
  transfer tasks. JSON payload correctness cannot replace required GUI forms.
- Defer global MC/DC targets, broad SQL/VFS fuzzing, all-allocation failure
  sweeps, broad scale-by-platform campaigns and historical compatibility
  corpora: they are not justified by this initial UI/service scope.
- New evidence changes the risk focus: the Inspector adds typed navigation,
  Inspector-owned Engine lifecycle/adapters, query ownership and cached
  model admission. These seams need proof even where ordinary SDK tests pass.

## Validation and delivery definition

During document-only planning, run Markdown, public-doc, design-lifecycle,
local-link and strict MkDocs checks. Do not claim runtime verification.
For source/test/executable work, use repository lint, typecheck and test order
and the full agent verification gate on the final code phase; a failed
checkout-owned environment preflight is reported, not bypassed. Future UI
checks must run the actual app, not merely assert element labels exist.

Slice 60 also requires a packaged/installed artifact that launches, attaches a known
supported database in the disclosed Engine-backed mode, exercises content/lexical/vector/graph flows,
and exits cleanly. Platform claims match measured artifacts. A four-of-five
core formative target is proposed; agree the rubric before sessions and run
High-depth functional checks as well. All Need/Should acceptance remains
required even if one human task happens not to exercise it.

## Contract and change register

- Original authority remains in FathomDB ADRs/interfaces and shared engine
  owners; the Inspector does not introduce raw SQL or query verbs under doctor
  or recover. Proposed new binary/API contracts require their own review.
- The high-level design and this plan had previously ambiguous version/scope
  language; they now distinguish Inspector v0.1 from FathomDB 0.8.x. Historical
  review records retain their original commits and claims.
- Intended product changes are an Inspector service/GUI exposing read operations;
  qualified Engine startup/background/shutdown may change product files. No
  Inspector editing or schema migration is introduced. Engine/SDK/MCP/schema refactors are not part of these Inspector slices;
  preserve those dependencies unchanged.
- Original design paths are redirects to the dedicated engineering section.
  Public docs and navigation state planned rather than shipped capabilities.
- Additive milestones discovered during implementation must trace to existing
  v0.1 scope; optional enhancements stay in ROADMAP unless the owner changes
  scope. This plan does not mutate FathomDB's current release selector or publish.

- Owner clarification, 2026-10-29: Engine startup is permitted when required.
  Requirements/INS-X01/F-05 now distinguish Inspector requests from ordinary
  Engine effects, and slice/public documentation is reconciled. A direct
  gpt-6.1-sol-high design review found four stale slice startup prohibitions;
  they were corrected and the reviewer approved the narrow recheck with no
  remaining material findings. No implementation/runtime qualification is claimed.

- Owner addition, 2026-10-29: preparation Slices 15 and 16 precede all five
  feature slices. Slice 15 owns release-wide requirements/AC/verification
  planning; Slice 16 owns release-wide architecture and through-1.x evolution
  assessment. These plans do not execute either slice or add future features.

- Direct Sol-high review of the Slice 15/16 draft addition found completion and
  downstream architecture-ownership wording to reconcile. Those findings were
  resolved, including the exact Slice 30 wording correction requested on
  recheck; the reviewer approved with no other material gaps. This reviews
  populated plans, not execution of either preparation slice or application tests.

- Direct Terra verification of the populated drafts passed scoped Markdown,
  strict MkDocs and local-link/dependency/ownership checks, with no material
  gaps. No application/runtime tests or full code gate ran for this prose-only
  change; neither preparation slice is marked executed or complete.

## Slice 15 execution baseline (2026-10-30)

[Verification baseline S15-2026-10-30](0.1/verification-plan.md) now assigns all
canonical criteria and Need/Should signals, fixtures/oracles and evidence stages.
Canonical AC adds origin cardinality/effective-route/class-boundary cases and
INS-X07–X11 for local access, accessibility, installed lifecycle and aggregate
bounds. Initial CPU/Linux/Chromium budgets are explicit qualification targets;
Slice 16 selects/proves the exact feasible composition, Slice 60 measures its
support claims. No .28/.29 artifact compatibility or runtime pass is implied.
All successor plans/designs consume the same matrix. Slice 15's
[status](0.1/slice-15/status.md) records completion. D-01 was explicitly
adopted by the owner on 2026-10-30, including its narrow export addition. Four-of-five remains
a formative diagnostic; required actual evaluation follows the fixed rubric.

## Slice 16 execution baseline

[S16-2026-10-31](0.1/slice-16/status.md) selects architecture in the
[canonical HLD](../design/high-level-design.md); feature drafts 20–60 consume
its components, contracts, capability evidence and M01–M39 mapping. B16-01 is
resolved by the 2026-10-31 owner ruling: physical deletion remains correct;
retained-erasure content/export is conditional on legitimate existence, with
selected Engine absence explicitly qualified. The status
owns gate/completion evidence. Application/runtime qualification remains planned.

## Slice 20 development execution note — 2026-10-31

[Slice 20 status](0.1/slice-20/status.md) records implemented record foundations
and remaining gates. Actual development qualification ran on Ubuntu 24.04.5 /
glibc 2.39. Its tested release candidate requires GLIBC2.39, so it is not an
Ubuntu 22.04-compatible distribution. D-04's selected22.04-compatible build and
installed support qualification remain pending 60; no silent target change or
publication is implied.

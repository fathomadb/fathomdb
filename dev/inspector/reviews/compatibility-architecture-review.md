---
title: Inspector compatibility architecture — Astra-medium review
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Independent evidence review and compatibility architecture recommendations
---

# Inspector compatibility architecture — Astra-medium review

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Review basis and verdict

At the owner's request, a directly spawned **gpt-6-astra, medium** subagent
independently reviewed the [research findings](../research/compatibility-architectures.md),
[.28/.29 source comparison](../design/compatibility-0.8.28-0.8.29.md), existing
requirements, high-level design and Slice 20 context. Baseline: `9085c3655`
plus the first uncommitted research draft, on `dev/0.8.30`. The reviewer was
read-only and made targeted checks of official implementation sources.
No Anthropic/MCP model invocation was used for this review.

Astra found no major factual contradiction in the sampled claims checked.
The examples are sufficient for this architectural comparison; more product
surveys would add little. The core recommendation is sound: a small
Inspector-owned compatibility boundary. The evidence does not justify parallel
engines per producer release, a plugin framework, or a support promise for
both .28 and .29.

This is review evidence and engineering advice, not an approved detailed
design, new requirement, human support-policy ruling or runtime qualification.
The parent incorporated the clarifications below into the research report;
that does not establish an Inspector implementation. A subsequent narrow
read-only recheck by the same Astra-medium reviewer confirmed all five findings
resolved and no remaining material mismatch in either durable document.

## Findings and report disposition

| Priority | Finding in initial draft | Remediation / disposition |
| --- | --- | --- |
| P2 | The compatibility model omits the browser/service protocol boundary. | Added one bundled UI/service pair, explicit protocol/build-pair validation before attachment, launch/session identity, and stale-reference rejection. Capabilities do not repair incompatible payload semantics. General negotiation is deferred. |
| P2 | Evidence categories do not distinguish operation support from session prerequisites. | Added the separate prerequisite axis and concrete vector example. Support, stored representation, unchecked/missing/usable runtime, and actual request evidence cannot collapse into one Boolean or one giant status enum. |
| P2 | Datasette's per-table count timeouts are described too broadly as bounded inventory work. | Narrowed to per-table bounded counts and stated the missing aggregate work/time/response bound. No claim of bounded enumeration or admission follows from this example. |
| P3 | Discovery wording can make admission checks sound circular. | Distinguished minimum bounded file/lock/sidecar/layout admission checks from subsequent broader discovery; admission must not become a full scan or feature trial. |
| P3 | The one-adapter recommendation leaves the current-schema policy implicit. | Made the selected build's current compiled schema and qualified structural contract explicit. Same schema number does not prove a structurally valid file or support for both producers. |

These are documentation corrections and recommendations for the existing
Slice 20 detailed-design work. They do not create five new workstreams or
change existing mandatory Inspector functionality.

## Evidence assessment

The sampled pgAdmin loader really walks a version mapping and falls back on
missing templates. That proves implemented dispatch/fallback policy, not the
semantic correctness of fallback for every server.
[Tagged loader](https://raw.githubusercontent.com/pgadmin-org/pgadmin4/REL-9_9/web/pgadmin/utils/versioned_template_loader.py).

SQLAlchemy's PostgreSQL dialect derives flags from server versions. These
are policy conclusions, not evidence that a particular request succeeded.
[Tagged dialect](https://raw.githubusercontent.com/sqlalchemy/sqlalchemy/rel_2_0_44/lib/sqlalchemy/dialects/postgresql/base.py).

The LSP completion client and CDP target code support the report's narrow
claims about advertised operation support and local model gating. Neither
establishes safe storage admission.
[LSP client](https://raw.githubusercontent.com/microsoft/vscode-languageserver-node/release/client/9.0.1/client/src/common/completion.ts),
[CDP target](https://raw.githubusercontent.com/ChromeDevTools/devtools-frontend/main/front_end/core/sdk/Target.ts).

Datasette's count loop applies a timeout to each table, not a total inventory
deadline; the revised research now states that limitation.
[Tagged Database implementation](https://raw.githubusercontent.com/simonw/datasette/0.65.1/datasette/database.py).

Popularity evidence is appropriately qualified: familiar/widely used product
examples, not a measured ranking of all six. Clean architecture is judgment
about specific boundaries, not certification of entire products. Pinning CDP
to a commit would improve reproducibility; disclosed moving links do not block
the architectural conclusion.

Runtime conditions coexist with compile-time flags. Build flags select code
and platform dependencies; runtime conditions select among qualified operations
and observed prerequisites. Neither independently proves an arbitrary file
or request compatible.

## Recommended minimal v0.1 approach

Use one packaged Inspector UI/service pair, one pinned query/library build,
and one Inspector-local storage implementation for the qualified current
schema. A module boundary and constructor parameters may suffice. No trait
registry, configuration language or dynamic loader is required to prepare for
hypothetical versions.

Keep storage decoding/admission knowledge separate from query execution and
result normalization: they change for different reasons. Localize necessary
layout/version conditions there. Views consume typed FathomDB facts and
request evidence; configuration can select supported choices or budgets but
cannot implement decoding, eligibility or evidence normalization by itself.

Ordinary dependency injection composes session, reader and query collaborators
with explicit ownership. A plugin system would add executable trust, API
compatibility, lifecycle and distribution obligations with no demonstrated
v0.1 benefit. Internal composition is sufficient.

Prefer unchanged usable libraries and pure helpers when they satisfy the
required semantics. Lack of a safe immutable Engine session is a feasibility
problem, not permission to invoke normal `Engine::open`. Inspector-local
adapters must preserve document/structured values, lexical routes, vector
metrics, directed edges, identity, lifecycle and provenance. A generic SQL grid
cannot satisfy the existing product.

## Recommended sequencing and falsification

These priorities guide affected work already allocated to the slices. They do
not add or remove approved scope.

**Need before affected implementation:** Establish Slice 20's usable safe
interfaces, minimum admission checks, UI/service protocol/session check, and
operation/prerequisite/outcome distinctions. Record missing required access as
blockers. Put final decisions into the relevant slice design and shared
high-level design through the existing review workflow. A support-policy
change needs an explicit owner decision; this review does not grant one.

**Should during already planned qualification:** Use real producer fixtures
and independently qualify the pinned serving backend. If support for both
pinned .28/.29 producers is approved, run both through the same Inspector
build. Never infer producer identity from schema 34.

Tests that could disprove the recommendation include:

- A current-schema producer fixture whose required meaning the common adapter
  cannot decode faithfully.
- Same-schema malformed/missing structures passing admission and producing
  false facts later.
- A required lexical/vector/graph operation requiring Engine mutation,
  downloads or cache repair.
- Missing/corrupt cached assets causing network activity or a false ready state.
- Cancelled/budget-exhausted work displayed as a complete empty result.
- Normalization silently merging equal-content records or distinct revisions.
- An incompatible stale UI decoding responses, or old references working after
  reattachment.
- A large inventory exceeding aggregate bounds despite per-query timeouts.
- Success, refusal, cancellation or detach changing database, sidecar or cache
  contents.

A failure should drive the smallest concrete adapter/contract correction. It
does not automatically justify plugins, parallel engines, weaker acceptance
or work in Engine/MCP/SDK/schema components.

**Later, only with a demonstrated need:** Older-schema readers, historical
corpora beyond approved support, independently shipped UI/service compatibility
ranges, or external adapter plugins. Treat these as candidates through the
existing [roadmap](../ROADMAP.md) promotion process, not v0.1 commitments.

No implementation or full regression run is warranted to complete this
research task. Finish the targeted clarifications and scoped documentation
validation; implementation fixtures run in the slices that own those features.

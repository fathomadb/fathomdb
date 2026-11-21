---
title: Inspector shared verification plan
date: 2026-10-30
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Release-wide fixture, oracle, test ownership and qualification baseline
---

# Inspector shared verification plan

This legacy path remains the single verification matrix for [adopted0.2.0](../0.2.0/scope.md).
[INS-020-LOCAL-RELEASE](../0.2.0/release-decisions.md) removes the unperformed
five-person study. T-01–T-05 and High functional goals remain required.20–60
feature checks,70 installed technical qualification and75 UX remediation are
complete at their recorded source/artifact revisions.80 adds tested local CI,
package integrity/promotion and accurate documentation;90 performs the local
release. Historical per-slice allocation/source observations below retain their
original execution scope; a PLANNED note in that history is not current state.

## Current allocation crosswalk

The [0.2.0 master](../0.2.0/plan.md#feature-coverage-and-carried-work) assigns
one unique accountable owner for every M01–M40 row. S9-006 reconciles the current
owner/stage cells to that map. 60 owns M28/M40 comparison/person interaction;
70 adds final installed/integrated technical qualification across all rows. Earlier
features retain complete local checks and explicit contributions. Criteria,
methods, fixtures and historical executed evidence retain their original meaning.
Allocation is not an execution or runtime-pass claim. Current0.2.0 record-workspace
execution and focused/native/resource checks are recorded in
[Slice20 status](../0.2.0/slice-20/status.md); its new real-browser workspace cases
cover exact scalar origins, separate facts, retained tree/chunk/association
contexts, focus and reversed/failed stale replies. Historical results below keep
their original source scope; current feature and installed execution is recorded by the owning slice statuses.

## Slice75 UX remediation contribution

[Slice75](../0.2.0/slice-75/plan.md) remedies the seven expert findings within
existing UN/INS/M-row ownership: attachment feedback, typed membership versus
exact-origin correspondence, contained content/actions, bounded stored labels,
declared-property inputs, retained graph branches and compact scoped evidence.
Its [status](../0.2.0/slice-75/status.md) records actual regressions, source review
and refreshed installed qualification. The original expert report and Slice70
package/human-session evidence retain their historical scope. The former five-person study is removed by owner decision; agent checks
never count as human sessions.

## Authority and execution state

Slice 15 baseline **S15-2026-10-30**, source `9486c5ba9`, draft comparison
`d05912c99`. [Needs](../../needs.md), [requirements](../../requirements.md)
and [acceptance](../../acceptance.md) own outcomes/criteria. This document owns
methods, fixtures and responsibilities, not another acceptance register.
At that historical Slice 15 baseline, no Inspector application/test suite existed
(`src/inspector/README.md` only); all executable rows were **PLANNED**.
Current Slice 20 execution is recorded in [the executed-evidence section](#slice-20-executed-evidence);
later completed feature/installed qualification is recorded by20–75 statuses. D-01 was explicitly adopted
by the owner on 2026-10-30; its defaults and flags below are settled policy. Architecture source feasibility is mapped by
[S16-2026-10-31](../../design/high-level-design.md), with **B16-01 resolved by the 2026-10-31 owner ruling**
that retained-erasure content is conditional on legitimate existence; Slice 16 status records
review and limitations. Planning an oracle does not prove that the application
exposes the fact; only the explicitly executed evidence below constitutes a pass. Slice 15 status records
reviews/checks and completion. Missing mandatory positive capability
blocks its owner even if the negative unavailable case passes.

Every matrix row applies its canonical criterion verbatim, plus the shared
scope, observation, resource, security and accessibility conditions. Owner is
the accountable slice in the current master; “70” adds integrated
qualification and never substitutes for earlier local completion. Slice 16
maps all rows to feasible components/routes before any feature implementation.

## Source evidence and proof limits

Evidence below was inspected at `9486c5ba9`; recheck selected builds in Slice 16.
No implementation delta exists between `d05912c99` and this source revision:
`9486c5ba9` only adds/reconciles preparation-slice documents. The Inspector,
not the existing Engine, is the missing product.

| Source under `src/rust/crates/` | Established fact / consequence for verification |
| --- | --- |
| `fathomdb-schema/src/lib.rs` (`SCHEMA_VERSION`, canonical/operational/artifact tables) | Current schema is 34 in this revision; use compiled authority rather than hard-coded future support. Current edge temporal values are INTEGER epoch seconds; historical ISO-text comments/reviews are not the test oracle. |
| `fathomdb-engine/src/open.rs` (`admit_current_database`, open setup) | Nonempty noncurrent files refuse; absent/empty paths can bootstrap through Engine, so Inspector preflight must reject them first. Normal open has WAL, registry/profile and maintenance effects. Startup allowance does not waive fixture or cache constraints. |
| `fathomdb-engine/src/read_api.rs`, `temporal.rs`, `graph_api.rs` | Point logical-ID reads can choose the most recent matching cursor even with history enabled; they do not establish exact-revision lookup. Nodes have state/windows; edge invalid-time and dependency filters differ. `graph_neighbors` returns node records, not a complete edge-detail API. |
| `fathomdb-engine/src/write_commit.rs`, `identity.rs`, `erasure.rs` | Ordinary writes can register legacy/incomplete revisions. Collection mode branches between log insertion and latest-state replacement. Pending telemetry redaction holds identifiers, not proof of retained canonical payload. |
| `fathomdb-engine/src/projection_registry.rs`, `projection_generation.rs` | Declarations, generations and record materialization are separate. Recorded per-attribute tokenizer overrides need not be effective. Test-only structural writers are not production prevalence evidence. |
| `fathomdb-engine/src/search.rs`, `filter.rs`, `vector_storage.rs` | Lexical route-specific BM25/BM25F and temporary tokenization; staged binary candidates/f32 L2 ranking. Independent distance facts and population/shortlist facts are both necessary. |
| `fathomdb-embedder/src/loader.rs` | Default loading may fetch/create caches. Checksum/local-assets mechanisms are candidates, not an already qualified Inspector no-download route. |

Historical reviews remain evidence with their policy/migration qualifications.
The [.28/.29 comparison](../../design/compatibility-0.8.28-0.8.29.md) names
pinned sibling snapshots; it does not qualify those artifacts or import their
snippets/degradations here. Slice 16 records adopted/rejected callable reuse
candidates with exact provenance, including MCP/agent-access, and determines
feasible full-edge, exact-revision and cache-only routes within Inspector scope.
No private-module copying, Engine edits or pin changes follow from this plan.

## Layers, durable harnesses and stages

| Layer | Planned organization | Execution responsibility |
| --- | --- | --- |
| U — unit/contract/property | `src/inspector/tests/contracts/` | Owning feature: deterministic typed DTO/reference/codec/pagination/unknown-variant tests; genuine invariant/round-trip properties where applicable. |
| D — real-database integration | `src/inspector/tests/integration/` and `fixtures/` | Owning feature: real temporary stores, intended manifests and qualified unchanged Engine operations; never mocked database results. |
| B — browser/accessibility | `src/inspector/tests/browser/` | Owning feature: launch actual UI/service; interact and inspect rendered facts/navigation, not label or payload-only assertions. |
| R — resource/fault/performance | `src/inspector/tests/resources/` | Owning feature then 70: bounded deterministic pressure/faults, monotonic timings, handles/worker and peak-memory accounting. |
| I — artifact/compatibility | `src/inspector/tests/installation/` | 20 owns launch skeleton; 70 installs final bundled artifact, tests declared compositions and all views. |
| H — historical human transfer | Unperformed study pack in70 | Removed from0.2.0 by INS-020-LOCAL-RELEASE; no participant qualification claim. |

The locations are proposed test organization, not an instruction to create six
frameworks. Slice 16 selects one minimal toolchain and gives concrete runnable
commands in this table before implementation. Each feature names exact tests
before its RED and records RED failure/GREEN commits and focused commands in
its status; existing tests are read fully before asserting their coverage.
Harnesses/probes/seeds needed to reproduce results are committed in Inspector
wiring, never only `/tmp`. Generated large databases and raw timing output are
not required tracked receipts; deterministic builders and intended manifests are.

Run lint → typecheck → focused tests, then relevant browser/resource checks.
Source/test/executable changes require the final repository code-phase gate
with a checkout-owned environment. Missing tools/preflight failures/skips stay
unverified. Documentation planning uses `./scripts/agent-lint-md.sh`; public-doc
changes additionally use docs lint and strict MkDocs. No artificial RED/GREEN,
application gate or runtime claim belongs to this Slice 15 document change.

## Fixture ownership, construction and independent oracles

Slice 20 owns the common deterministic builder, manifest schema, cleanup and
F-01/02/03/05 core. Slices 30/40/50 extend its lexical/vector/graph cases;
70 assembles a separate unseen transfer instance and minimum-scale F-06.
Each feature also exercises its own F-06 boundary subset before closing.
The builder records intended class/identity/revision/values and setup operations
before reading Inspector output. Assertions compare intended facts plus named
Engine semantics; neither Inspector serialization nor a second call through
the same adapter is an independent oracle.

| Fixture | Construction and oracle | Feasibility / boundary |
| --- | --- | --- |
| F-01 | Ordinary canonical node/edge/collection writes; explicit source + distinct derived node + edge, multiple kinds/roles, dependencies, declarations/materializations. Hand-authored manifest includes full content, labels, exact links, counts by scope and ordinary-write incomplete provenance. | 20 seeds core; 30/40 add actual representations; 50 sources/dependencies. Provenance/structural cases needing controlled internal states must name supported setup seams; do not represent test-hook setup as production Inspector access. |
| F-02 | At least two logical IDs with equal body, two retained revisions, class/namespace bare-value collisions, anonymous content ambiguity, zero/one/many origins, unresolved and expired/session-mismatched references. Compare manifest revision IDs, retained payload and origin sets, not latest lookup. | 20 contract/core setup; 30/40 result origins; 50 exact retained navigation. Unsupported real passage modes use contract-level negative carriers, never fabricated stored passages. |
| F-03 | Register distinct latest-state and append-only collections, overwrite current key twice, append repeated keys twice, repeat key in unrelated collection; manifest expects only final current payload and both log entries. | 20 ordinary supported writes. No manufactured current-to-log history. |
| F-04 | Separate inactive/superseded node, expired edge, validity boundary, dependency-blocked target, unconfigured/pending/failed/ready representation, ready with unavailable CPU model, eligible out-of-top-k, unknown/missing source and erased target. Keep retained history, owed canonical physical closure and telemetry-only queue distinct. | 20 core, 30/40 representation, 50 class/closure/history cases. Slice 16 locates supported setup/generation/fault seams or flags infeasibility; controlled fixture setup is not an Inspector mutation control. Use the adopted policy and eight-combination oracle below. |
| F-05 | Real missing/empty/invalid/current/old/new-schema files, missing/held lock, initial WAL/journal, pending work and failed startup; paired equivalent Engine fixture from same intended seed/options. Independent effect allowlist and caller content invariants precede comparison. Add malicious origin/Host/credentials and stale tabs. | 20: fingerprint product+sidecars/cache before preflight; after admitted startup inspect logical post-state/events by phase. Old/new sentinels test refusal, not actual historical/future compatibility. Never mutate a live production store to construct faults. |
| F-06 | Deterministic 100,000 revisions + 100,000 mutation entries, 10 MiB mixed UTF-8 payload/structured tree and 10,000 incident directed edges; random seed fixed and logged. Independent counters track examined work, response bytes, pending/running jobs, retained observations, memory and cleanup. | 20 chunks/queues/lifecycle; 30 lexical budgets; 40 model bounds; 50 high-degree; 70 aggregate. Include empty/limit-1/limit/limit+1, single oversize value and repeated navigation cycles. |
| F-07 | Explicit directed adjacency A→B, B→C, C→A, A→A and parallel A→B with distinct edge IDs/kinds/content; a disconnected equal-label node. One source/derived pair with known text/vector origins; lexical match and nonmatch. Include inert markup, Unicode, large integers, null/empty/missing, empty representation. | 30 lexical, 40 vector, 50 graph all extend 20 record/tree; 60 end-to-end. Geometry/similarity/equal text never creates adjacency or identity. |

### Numerical and lexical reference cases

For M11/M17/M23/M26 and F-07, under S9-010, the 0.2.0 positive uses ordinary
supported single head-window writes/admission and verifies exact row/profile/
source cursor/revision, window_number=0, byte_start 0 and recorded byte_end/
body_length with unknown coverage explicit. Independent L2/population/ties/origin
tests remain; multiple sources/revisions do not count as multiple windows of one
source. Within-source multiple-window positive is deferred, not passed as
unsupported. Cache-only CPU Engine serving positive remains mandatory.

Use small hand-checkable stored vectors and an explicit query, with known f32
bytes, dimensions, profile and centering metadata. For L2 calculate
`sqrt(sum((q_i - x_i)^2))` independently in f64 from stored f32 values; compare
within `1e-5 * max(1, reference_distance)`. Hamming distance uses an independent
bit-XOR/popcount and must match exactly. Derive binary signs from the declared
centering transform, including zero/tie cases. Include equal distances,
dimension/profile mismatch, NaN/nonfinite rejection and candidates intentionally
excluded by a bounded binary stage. Assert membership of the declared stage
and tie equivalence classes, not an agent-generated golden rank. An exact
operation must check every member of its explicitly bounded population; a
staged route must not claim global nearest neighbors. Slice 16/40 records any
metric-specific precision change before running tests.

For lexical tests use a tiny explicit corpus with known token occurrences,
field lengths and query operands: body-only, edge content, projected attribute
text differing from body and kind/body/status BM25F where actually supported.
Include no match, repeated terms, stemming/diacritics, OR operands and parser
errors. Compute matched identities from the written corpus and effective
route/tokenization contract. For each selected scoring family use a small
independent formula from its documented parameters and corpus statistics, or
raw SQLite FTS5 BM25 on the independently seeded same index for Engine-vs-FTS
conformance; that shared FTS calculation alone is not proof of source/origin
correctness. Match spans must slice the intended UTF-8 text exactly if returned;
missing spans remain absent, never query-string guessed highlights. Do not
assume 0.8.29 snippet helpers are callable on the selected branch.

### Fault control, effects and teardown

Use supported barriers/generation observations to establish pending→ready and
before/after reads; no sleep-based race oracle. For node/edge temporal checks
use the supported bound instant and integer before/equal/after boundaries.
Fault fixtures inject one selected first failure and persistent failure at
Inspector attach/model init/request/response/close ownership seams; record
seed, fault point, action sequence and intended outcome for replay. Add late
native completion after UI cancel, queue overflow, interrupted owned SQLite
request, close during active work and worker startup failure. Ordinary close
and fault teardown must stop new work, settle/terminate owned workers within
the bound, release handles/product lock and delete only fixture-owned scratch.

No pretend universal OOM recovery: bounded memory pressure and process failure
prove only their declared failure model. Process kill/reopen does not prove
power-loss durability. No intrusive Engine/VFS fault hooks are commissioned.
If a required state cannot be constructed through available test/setup seams,
mark the row BLOCKED-FEASIBILITY before implementation rather than weaken it.
Observe worker progress and caller content invariants against requirements'
allowed effect classes; equivalent Engine timing/bytes alone is not a pass.

## Traceability matrix

Rows retain the partial executed evidence explicitly recorded below; all
unexecuted feature/integrated stages and the new M40 remain PLANNED. No scope
adoption or policy ruling constitutes a runtime pass.
Each row names a positive and boundary/negative case; fixture section supplies
the setup/independent oracle. Expected evidence is actual assertions and named
commands with pass/fail/skip in the owning status, plus browser actions where
listed. At implementation add exact test paths/commands and actual revisions
to the row or linked owner status. No other mandatory ID is implicitly optional.
UN mappings below refer to the need inventory; V01 signals share the same tests.

| Row / need | Canonical criteria and signals | Positive / boundary or negative; fixture oracle | Layers; first owner → final stage | Expected evidence / state |
| --- | --- | --- | --- | --- |
| M01 / UN-02,04,14 | INS-01.1–01.3; V01-N02/S01 | Distinct node/edge/collection/projection and source/derived/origin; empty/unconfigured/unavailable; kinds versus roles, ordinary incomplete provenance. F-01 intended catalog/links. | D+B; accountable 20; contributions 20 → 70, extensions 30/40/50; final 70 | Catalog/count assertions and actual open/detail; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M02 / UN-02,04 | INS-01.4 | All workflows usable with ordinary docs, no lesson/generator prerequisite. F-07. | B; accountable 20; contributions 20 → 70; final 70 | Actual route/task observation; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M03 / UN-01–04,08 | INS-02.1–02.3; V01-S01 | Open every populated category; count revision/logical/current/log/projection separately with scope; sample cannot prove field absence. F-01/03 manifests plus F-06 caps. | D+B+R; accountable 20; contributions 20 → 70; final 70 | Exact scoped counts/sample facts, bounded navigation; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M04 / UN-01,09 | INS-02.4; V01-N06/S03; INS-X01 | Supported attach/header; missing/empty/held/missing-lock/corrupt/noncurrent/initial-sidecar refuses with preservation and safe guidance. F-05. | D+B+I; accountable 20; contributions 20 → 70; final 70 | Phase-specific effects, fingerprint refusal assertions, UI reasons; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M05 / UN-03,06,13 | INS-02.5 | Log repeated entries versus latest replacement; equal cross-collection keys unrelated, absent current value. F-03 write manifest. | D+B; accountable 20; contributions 20 → 70; final 70 | Mode/count/entry identity assertions and detail; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M06 / UN-05,06,12 | INS-03.1 | Browse kind/collection then identity/text/field/source constraints; unsupported category constraint refuses before executing. F-01/03/07 known matching IDs. | U+D+B; accountable 20; contributions 20 base, 30 text → 70; final 70 | Request validation, discovered selection, zero-match versus failure; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M07 / UN-06 | INS-03.2; INS-X05 | Full text/tree and missing/null/empty/unavailable; 10 MiB Unicode/chunks, unsafe HTML, large numbers exact. F-01/06/07 written values. | U+D+B+R; accountable 20; contributions 20 → 70; final 70 | Chunk/copy round trips, rendered inert content, no lost values; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M08 / UN-07 | INS-03.3 | Exact directed endpoints and edge properties; missing endpoint never similar-name substitute. F-07 adjacency. | D+B; accountable 50; contributions 50 → 70; final 70 | Directed graph/exact edge local delivery COMPLETE; [50 status](../0.2.0/slice-50/status.md) records review/final verification; installed70 completed;75 refreshed UX |
| M09 / UN-07,17 incidental | INS-03.4 | Follow and back retains filter/order/page/selection; new read context separate, evicted state announced. F-01/02/06. | U+B; accountable 20; contributions 20 base, 30/40/50 each → 70; final 70 | Actual back/forward interactions and state bounds; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M10 / UN-06,14 | INS-03.5; INS-05.1–05.2 | Equal body/different IDs, revisions, class/namespace collisions; ambiguity never guessed. F-02 independent identity/revision sets. | U+D+B; accountable 20; contributions 20, result extensions 30/40/50 → 70; final 70 | Property/reference round trips and exact selected values; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M11 / UN-14,15 | INS-04.1–04.2; INS-09.5 | Declaration versus verified materialization and zero/one/many origins, graph participation distinct; equal text missing linkage stays unknown. F-01/04/07. | D+B; accountable 20; contributions 20 base, 30/40/50 → 70; final 70 | Origin-set/revision assertions and association navigation; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M12 / UN-05,08,16 | INS-04.3 | Lookup/lexical/hybrid/traversal inputs, filters/mode distinct; unavailable option before run. F-07 known requests and invalid inputs. | U+D+B; accountable 20; contributions 20 base, 30/40/50 → 70; final 70 | Typed request/operation, validation and controls; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M13 / UN-15,16 | INS-04.4; INS-06.1–06.3 | Unconfigured/pending/failed/ready/runtime-unavailable/unknown separately scoped/timed; known eligible out-of-top-k not missing storage. F-04 controlled generation/model/candidate facts. | D+B; accountable 20; contributions 20 base, 30/40/50 → 70; final 70 | Supported barrier/generation plus observed labels; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M14 / UN-08,13,16 | INS-06.4 | Core T-05 distinguishes four factual axes without requiring High diagnosis. F-04/transfer fixture. | U+D+B+I; accountable60; contributions70/75; final installed70 | Real fixture/browser/installed storage/readiness/eligibility/result-scope assertions in40/60/70/75. Historical human interpretation study is UNPERFORMED and removed; [70 study pack](../0.2.0/slice-70/study.md) is preserved |
| M15 / UN-08,14,16 | INS-05.3–05.4; INS-X04 | Preserve exact request/options/payload/read context across worker progress, refresh and detail; replacement/session expiry refuses, representative branch not all contributors. F-02/05/07. | U+D+B; accountable 20; contributions 20 base, 30/40/50, interaction 60; final 70 | Prior observation retained vs later detail, stale/unknown-variant tests; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M16 / UN-08,16 | INS-05.5 | BM25/BM25F/fused scores and distance separately named; no cross-family scale or invented absent ranks. F-07 independent lexical/vector references. | D+B; accountable 40; contributions 30/40 → 70; final 70 | 40 local implementation/qualification recorded in [status](../0.2.0/slice-40/status.md); final source/Terra closure as recorded there; installed/all-view technical qualification completed70/75; historical human study removed |
| M17 / UN-06,14 | INS-05.6 | Unique exact revision opens; zero/many origins unresolved/choices, passage not logical; reject database/class/session collision. F-02 origin cardinalities. | U+D+B; accountable 20; contributions 20 reference, 30/40 origins, 50 history → 70; final 70 | Property/negative carrier tests and chooser behavior; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M18 / UN-13,15 | INS-06.5; V01-N02/N03 | Node state/supersession/window versus edge invalid-time/closure, integer before/equal/after; no invented edge state/t_valid filtering. F-04 intended state at bound instant. | U+D+B; accountable 20; contributions 20 node, 50 edge/closure → 70; final 70 | Boundary predicates and class-correct detail/observations; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M19 / UN-07,12–14 | INS-07.1–07.2 | Bounded source/dependency/supersession navigation and retained revisions, distinct link labels; incomplete/missing target not fabricated. F-01/02/04 recorded link sets. | D+B; accountable 50; contributions 50 → 70; final 70 | Exact recorded link/revision and keyboard local delivery COMPLETE; [50 status](../0.2.0/slice-50/status.md) records review/final verification; installed70 completed;75 refreshed UX |
| M20 / UN-09,12,13 | INS-07.3; INS-X06; V01-N01 | All eight launch combinations, default normally accessible/history/inactive viewing, erasure-content/telemetry-payload withholding and intersecting gates on every data/evidence/copy/link path; missing/erased source never reconstructed. F-04/05 independent classifications and policy truth table. | U+D+B+I; accountable 20; contributions 20 launch/enforcement, 30/40 payloads, 50 history → 70; final 70 | Parse/default/help tests, typed supplied-content gate contracts + real available-class/atomic-delete absence/flags and worker/stale-context assertions; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) (B16-01 resolved; conditional Engine content) |
| M21 / UN-13,15,16 | INS-08.1–08.3 | Available request/per-hit origins/ranks/checks with time/scope; post-query check distinct, absence cause undetermined without oracle. F-04/07 known inclusion and bounded exclusion. | D+B; accountable 30; contributions 30 lexical, 40 vector, 50 graph, 60 integrated; final 70 | Actual returned evidence versus unknown states; Slice 30 native lexical contribution executed; [status](../0.2.0/slice-30/status.md); installed technical qualification completed70/75; human study removed |
| M22 / UN-14,16 | INS-08.4; V01-N05 | Effective lexical route/fields/tokenizer/scoring versus configured override; indexed text ≠ body, missing spans not fake. F-07 corpus/formula/UTF-8 oracle. | D+B; accountable 30; contributions 30 → 70; final 70 | Per-route attribution, numerical/match cases; Slice 30 native lexical contribution executed; [status](../0.2.0/slice-30/status.md); installed technical qualification completed70/75; human study removed |
| M23 / UN-14–16 | INS-08.4; V01-N04 | Representation/metric/direction/population/stage/centering; staged shortlist ≠ global exact; mismatch/nonfinite rejected. F-07 f64 L2 and bit reference. | U+D+B; accountable 40; contributions 40 → 70; final 70 | 40 local implementation/qualification recorded in [status](../0.2.0/slice-40/status.md); final source/Terra closure as recorded there; installed/all-view technical qualification completed70/75; historical human study removed |
| M24 / UN-04,06 | INS-09.1 | Actual readable text/structured tree with metadata separate; nested expansion and null/missing/empty, full chunk access. F-07/06. | B; accountable 20; contributions 20 → 70; final 70 | Render/expand/select/copy/back actions; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M25 / UN-05,14,16 | INS-09.2; V01-N05/S02/S04 | Index content → lexical query → ranked match → exact origin → back; route-dependent evidence, error/cancel/empty distinct. F-07 lexical oracle. | D+B; accountable 30; contributions 30 → 70; final 70 | Actual lexical controls/results/evidence interaction, not grid; Slice 30 native lexical contribution executed; [status](../0.2.0/slice-30/status.md); installed technical qualification completed70/75; human study removed |
| M26 / UN-14–16 | INS-09.3; V01-N04/N07/S04 | Profile/dimension/representation/readiness → neighborhood → numeric metric visual → record; cache missing separate. F-04/07 numerical oracle. | D+B+R; accountable 40; contributions 40 → 70; final 70 | 40 local implementation/qualification recorded in [status](../0.2.0/slice-40/status.md); final source/Terra closure as recorded there; installed/all-view technical qualification completed70/75; historical human study removed |
| M27 / UN-07,14 | INS-09.4; V01-S04 | Directed diagram select node/edge, kind filter, expand/collapse, self/parallel/cycle/disconnected, truncation; no geometry inference. F-07/06 adjacency. | D+B+R; accountable 50; contributions 50 → 70; final 70 | Exact visible identities plus SVG keyboard/actual pointer local delivery COMPLETE; [50 status](../0.2.0/slice-50/status.md) records resource/final verification; installed70 completed;75 refreshed UX |
| M28 / UN-07,14,16 | INS-09.6–09.7; V01-S04 | Record→lexical→vector→graph→original observation; equal bodies/revisions distinct, labeled numeric alternatives. Optional transform, if shipped, clearly not exact geometry. F-02/07. | B; accountable 60; contributions 30/40/50 local, 60 person interaction; final 70 | Development person workflow implemented: actual biography/index/match/head-vector/L2/cachedCPU/edge/source/history and original Back/focus pass; Sol-high approved0135d4dd5; independent Terra20comparison/16native and final3focused pass.60 single clean full gate PASS41748e588 (206/206 suites);70 installed technical qualification completed; human study removed. |
| M29 / UN-09 | INS-X01; V01-N06/S03 | No Inspector mutation dispatch; independently allowed Engine effect classes, preflight preservation, startup failure disclosure, normal workers/generations and close. F-05 intended post-state + equivalent Engine attribution. | U+D+B+R; accountable 20; contributions 20 → each feature → 70; final 70 | Operation allowlist, phase effects, handle/lock ownership; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M30 / UN-09,15 | V01-N07; INS-X01 | Network-denied valid CPU embedding assets work; missing/partial/corrupt assets cause no fetch/create/repair; independent browsing works. Optional reranker both cases if used. F-04/05. | D+R+B; accountable 40; contributions 20 startup, 40 model operation → 70; final 70 | 40 local implementation/qualification recorded in [status](../0.2.0/slice-40/status.md); final source/Terra closure as recorded there; installed/all-view technical qualification completed70/75; historical human study removed |
| M31 / UN-08,10 | INS-X02–X03; INS-X10 | Every list/count/sample/traversal/query limit scoped; F-06 cap boundaries, queue overflow, repeated requests, cancel/native-late results and shutdown meet aggregate budgets. | U+D+B+R; accountable 20; contributions 20 infrastructure, 30/40/50 feature work → 70; final 70 | Measured rows/links/bytes/time/memory/jobs, partial/busy/cancel states; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M32 / UN-06,10 | INS-X05 | Exact identity/value copy and chunk pagination/round trips; UTF-8 boundaries, large integer precision, oversize scalar. F-02/06/07 written bytes. | U+D+B; accountable 20; contributions 20 → 70; final 70 | Property/copy/reconstruction assertions; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M33 / UN-09 | INS-X07 | Allowed loopback UI works; foreign origin/Host, wrong credentials, stale build/session rejected before read; inert stored content, no automatic fetch/log payload leakage. F-05/07 attack strings. | U+D+B; accountable 20; contributions 20 shell, each feature render → 70; final 70 | Socket binding, request rejection and browser/network/log assertions; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M34 / UN-04,06,07 | INS-X08; V01-S04 | Keyboard/focus/equivalent labels across all native views incl graph expansion and vector values; pointer-only fails. F-07. | B; accountable 70; contributions 20/30/40/50 locally → 70; final 70 | Installed keyboard/pointer/native view/focus/Back checks PASS in the72 independently verified installed cases; [70 status](../0.2.0/slice-70/status.md). Human fact-parity evaluation remains unperformed and is removed by owner decision. |
| M35 / UN-01,09,10 | INS-X09 | Exact installed artifact offline launch/attach/all views/close and actionable refusal/missing models; no development server; correct package/build/session pairing. F-05/07 target environment. | I+B+R; accountable 70; contributions 20 launch, 70 final; final 70 | Installed technical qualification PASS on exact Ubuntu22/glibc2.35+kernel7/LandlockABI8+Chromium156 pairing, actual CPU/all views/native window close, owned cleanup and lock release; [70 status](../0.2.0/slice-70/status.md). Unpublished; no blanket newer-platform support. |
| M36 / UN-08,09,10 | V01-S02 | Adopt/reject MCP/agent-access/Engine reuse with exact branch/build facts; qualified scratch/connection ownership and honest interruption/native bounds. F-05/06. | Design 16; accountable 20; contributions D+R in 20/30/40 → 70; final 70 | Source capability map then owned cancel/fault/teardown tests; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) |
| M37 / UN-02,04–08,14–16 | T-01–T-05 | Familiarization then unseen transfer, ordinary docs, correct actions and interpretations; misconceptions fail task, no facilitator teaching. F-01/02/04/07 unseen variant. | U+D+B+I; accountable60; contributions70/75; final installed70 | Five participant sessions UNPERFORMED and removed by owner decision. Frozen distinct familiarization/unseen fixture/key/task/rubric and honest result form are ready in [70 study pack](../0.2.0/slice-70/study.md); automated fixture checks are not human evidence. |
| M38 / UN-07,12,13,15,16 | INS-07/08 High investigation | Trace source/dependency/revision and inspect available inclusion/check evidence; unavailable original/unknown omission causes retained honestly. F-02/04/07. | D+B in 50/60; accountable 50; installed contributions70/75; final 70 | Functional source/dependency/history/frozen-evidence and original observation Back local delivery COMPLETE; [50 status](../0.2.0/slice-50/status.md) records final verification; separate High human70 is UNPERFORMED and removed by owner decision; [study pack](../0.2.0/slice-70/study.md) ready |
| M39 / UN-09,18 narrow | INS-X11; V01-N01 | Explicit `--show-erased` local export of selected physically retained erasure-pending content; inactive/telemetry overlap gates remain binding, absent content unavailable, no general dump. F-04 manifest bytes/structured values and F-06 bounded stream/cancel/failure. | U+D+B+R+I; accountable 20; contributions 20 local export/launch, 50 history overlap → 70; final 70 | Typed supplied-content exact stream/identity/context/gate/partial/error/cancel tests; real Engine deletion/absence/flags/no reconstruction, positive E2E only if legitimately supported; Source20 base and later feature extensions complete; [20 evidence](#slice-20-executed-evidence) retains its original scope. Installed technical checks: [70 status](../0.2.0/slice-70/status.md); refreshed UX: [75 status](../0.2.0/slice-75/status.md) (B16-01 resolved; conditional Engine content) |
| M40 / UN-08,14,16 | INS-05.7–05.10; INS-X02–X10; T-04 | Pin two F-07 person observations with independently known overlap/disjoint typed identities, distinct revisions/classes/namespaces, zero/one/many origins and actual method values; graph membership has no invented rank. Retain differing seed/query/options/filters/limits/population/read contexts. F-02/04/06 cover caps, policy withholding/all D-01 combinations, unsupported/unknown absence, stale/refused/expired/evicted references, late results and pin/cancel bounds. | U+D+B+R; accountable 60; contributions 60 comparison; final 70 | Typed adapter/property and actual keyboard/pointer A/B/original/Back, all8 queued-policy, graph/native, expiry, late/cancel/FIFO cases implemented; Sol-high approved0135d4dd5; independent Terra7unit/20comparison/16native plus3focused byte/admission/heap cases pass.60 single clean full gate PASS41748e588 (206/206 suites);70 independently qualifies installed/all-view behavior; humanT04 is unperformed and removed by owner decision. No rerun, persistence, common snapshot or general export. |

## Evidence-method limits for current qualification

For M31/M36, `tests/browser/resources.mjs` owns a real Engine child but its
acknowledged one-shot pause is synthetic. Queue, <=250 ms cancel acknowledgement,
late-payload withholding and <=30 s reap do not prove interruption of active
native search/model work. Its RSS readings are samples and growth measurements,
not an instantaneous peak. Each feature qualifies its actual native operation;
70 adds aggregate peak/resource and owned-close evidence.

For M35, `tests/browser/package.mjs` checks the default release tar, identity,
checksums/help and absence of development runtimes. `tests/launch.rs`'s ignored
native case observes real bootstrap consumption then sends SIGTERM. Neither is
installed all-view, offline read plus actual native-window-close qualification.
Every new lexical/vector/graph/compare view still requires its actual interaction,
keyboard, cancellation and finite close cases, with final installed proof in 70.

M20/M39 retain real ExplicitMaintenance queued-positive display/export and all
eight disclosure-flag combinations, including the 8,192-target/200-unrelated
budget and fresh-next-transaction withdrawal regression. Synthetic supplied
content and atomic absence remain distinct; they do not waive legitimate queued
positive coverage or any other required view.

## Slice 20 executed evidence

The [active status](slice-20/status.md) owns actual revisions, executed
RED/GREEN pairs, review dispositions and remaining gates. This is the same
matrix, not a competing acceptance list. The entries below qualify Slice 20
foundations; full rows with query, graph, installed or human stages remain
partial until their assigned owners execute them. No later feature is declared
unavailable as a completion substitute.

| Rows / Slice 20 part | Re-runnable source evidence | Current scope |
| --- | --- | --- |
| M01–M07, M24 | `tests/integration/{records,operational,chunks,associations}.rs`; browser `{document,metadata,projection,catalog-caps,operational-copy,association-choice,nullable-copy,sample}.mjs` | Real public-producer data, all record classes, exact retained revisions, distinct current/log identities, verified attributes, full metadata/text/tree/copy and truthful caps. Text queries remain 30; human transfer remains 60. |
| M09–M13, M15, M17–M18 base | `tests/integration/{records,eligibility,associations,disclosure}.rs`; browser `{related,context,references,cache,eviction,association-choice}.mjs` | Exact handles, class/role/provenance, node and class-correct raw boundaries, actual source/attribute relationships, observation/expiry/Back and bounded cache. Query/candidate evidence and graph/history extensions remain 30/40/50. |
| M04, M20, M29–M30, M33, M35–M36 base | `tests/contracts/{policy,process,http,browser_launch,build,export}.rs`; integration `{admission,preflight_guards,session,access,disclosure}.rs`; `tests/{worker,cli,launch}.rs`; browser `{api,refusal,startup-cancel,startup-cancel-ui,parent-death,slow-http}.mjs`; `tools/qualify-startup.mjs` | Preserving refusals and qualified Engine lifecycle, closed environments before dependencies, one-use local access/build pairing, actual GUI launch and hostile model-free startup. Installed all-view/model-query and general security/human claims are not implied. |
| M31–M34, M36 base | Ignored `tests/worker.rs` F06/allocation fixtures; browser `{resources,control,scalar-copy}.mjs`; `tools/qualify-scale.mjs` | Shared row/item/byte/work bounds, real native catalog cancel/fault/detach, queue/parent/startup ownership, actual large Unicode/number clipboard and keyboard return, explicitly sampled memory. Later query/graph aggregate qualification remains assigned. |
| M20, M39 common/conditional | `tests/contracts/export.rs`, `tests/integration/disclosure.rs` | Supplied-content typed policy/stream tests plus real atomic deletion/absence/all flags/no reconstruction. No legitimate retained E target exists in this selected fixture, so positive DB export is unavailable; no mocked or fabricated storage state. |

Paths above are relative to
[the Inspector source](../../../../src/inspector/README.md); commands,
prerequisites and parameters are indexed in
[the committed tool index](../../../../src/inspector/tools/README.md).
The real F06 run used 100,004 nodes, 100,000 log entries, 10,000 edges and
10,800,042-byte content; default/debug composition and observed 25 ms memory
sampling are distinct from installed release and instantaneous peak claims.
Actual executor: Ubuntu 24.04.5/glibc 2.39, Chromium 156.0.8078.4,
Playwright 1.62.1. The tested development package requires GLIBC 2.39 and is
not Ubuntu 22.04-compatible; the selected compatible distribution and full
installed/human qualification remain 60 work. Fresh Sol code review, Terra
verification and final source gates remain pending at this record.

## Adopted disclosure test oracle

Canonical authority is [D-01](../../requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30),
explicitly ruled by the owner on 2026-10-30. The following independent truth
table is authored from that ruling, never generated from Inspector output.
`I` is retained-history/inactive, `E` still-retained owed-erasure content, and
`T` telemetry payload. “Yes” means this gate permits display, not guaranteed
storage/retrieval membership. Narrow export additionally requires `E` and
`--show-erased`; every applicable gate must permit it.

| --no-show-inactive | --show-erased | --show-payloads | I record gate | E content gate | T payload gate |
| --- | --- | --- | --- | --- | --- |
| absent | absent | absent | Yes | No | No |
| present | absent | absent | No | No | No |
| absent | present | absent | Yes | Yes | No |
| absent | absent | present | Yes | No | Yes |
| present | present | absent | No | Yes | No |
| present | absent | present | No | No | Yes |
| absent | present | present | Yes | Yes | Yes |
| present | present | present | No | Yes | Yes |

For each invocation exercise pure I/E/T cases plus I∩E, I∩T, E∩T and I∩E∩T.
Normally accessible unclassified data remains viewable in every combination.
Telemetry structure/counts do not expose payload fields; scope excludes hidden
inactive records. Confirm supplied-flag order/duplicates are equivalent and
invalid switches/values refuse before Engine open. Verify help/defaults and
fixed session policy rather than adding a confirmation dialog.

Run list/detail/chunk, record/index/vector/graph labels and links, query result/
match/evidence, prior-observation refresh, clipboard and export for each
applicable class. Check refusal/unknown/missing outcomes separately. Reclassify
under supported worker barriers between read and copy/export: no stale context
bypasses the gate and already absent content cannot be recreated. Ordinary
search ranking/eligibility still matches its declared Engine operation.
M20 owns launch/path checks; M39 adds exact bounded local-file export with
no flag, denied overlapping gate, partial write, cancellation and disappearing
content. The 2026-10-31 owner ruling makes positive Engine display/export conditional
on legitimately existing readable retained-erasure content. For the selected
atomic-delete Engine, real integration verifies deletion/absence, all flags,
no eligible export target and no reconstruction: unavailable/not-applicable is
the correct content result. Pure typed policy/export contracts exercise supplied
content with independent intent, including exact streams, overlap gates,
partial/error/cancel/disappearance; they neither mock a database nor establish
an Engine-positive pass. If a supported Engine legitimately exposes this state,
its positive end-to-end display/export becomes required. Record actual scope;
never fabricate retention or delay physical deletion. Other required positive
capabilities remain unchanged. No generated golden supplies this oracle.

## Qualified environment regression cases — N16-A01

F-05/F-07 add hostile-parent-environment fixtures for the selected composition.
Set production `FATHOMDB_PERF_EXPERIMENTS`, writer/reader PRAGMA overrides and
`FATHOMDB_PERF_SEARCH_LIMIT` before launching the real Inspector. Hand-authored
header sentinels and known candidate identities precede Inspector output; compare
selected ordinary versus hostile-parent launches with identical DB/options.
Assert persistent header values/effect classes unchanged, lexical/hybrid candidate
facts unchanged, CPU/model identity correct, all worker entries exactly allowlisted,
no fetch/cache write, and no parent value/secret in logs or errors. Do not depend
on test-only Engine builds to remove production hooks.

M15/M22 bind actual query context/effective population to the environment-policy
revision in the build description; M29 checks spawn/effects/header sentinels;
M30/M33 check cache/network/log/GUI separation; M35 qualifies installed launch;
M36 verifies source composition and child ownership. Feature 20 owns cleared
preflight/session/helper spawn and fixture infrastructure, 30/40 lexical/hybrid/
model assertions, 70 installed repetition. The canonical HLD owns exact worker
and browser allowlists; do not create a second config or per-view override.
All these methods are PLANNED until feature tests execute.

## Historical human evaluation operating rubric — not a0.2.0 gate

The original unperformed protocol is preserved in the [70 study pack](../0.2.0/slice-70/study.md).
No recruiting or sessions are required for0.2.0. Its audience, assistance and
four-of-five diagnostic are historical research, not substitutes for technical
checks and not claims of validated demand. Functional T-01–T-05 and High
investigation are verified with independent real-database/browser/installed
facts at their existing matrix owners. No participant score is fabricated.

## Quality analysis adopted and deferred

Read [quality/testing roadmap](../../../notes/fathomdb-quality-testing-roadmap.md).
Adopt independent intended state plus conformance, boundary/property tests at
identity/pagination/codec/validity/limit seams, focused first/persistent faults,
cleanup/resource measurements, and a few plausible wrong-condition/missing-cleanup
mutations where useful to evaluate test strength. Mutation/coverage percentages,
throughput and broad performance comparisons are diagnostic, not added gates.

Changed evidence: Inspector adds a browser, local credential/origin boundary,
Engine-owned worker progress, native model admission and cross-view references;
existing SDK tests cannot prove their behavior. Prior no-startup preservation
campaigns must instead separate preflight preservation from allowed post-start
Engine effects. Match/rank similarity does not prove correct origins/populations.

Defer workspace MC/DC, general SQLite SQL/B-tree fuzzing, all-allocation OOM,
VFS/power-loss sweeps, historical readers, unapproved producer/platform matrices
and large sanitizer campaigns: outside this Inspector composition/change scope.
Retain reproducible minimal fault seeds and actual artifacts for declared support.
Carry stable-contract regression fixtures, supported failure models and measured
support windows into Slice 16's through-1.x assessment; do not build speculative
future campaigns, dynamic plugins or new Engine metadata in v0.1.

## S16 architecture and executable harness handoff

The [canonical HLD](../../design/high-level-design.md#requirements-to-components-and-verification)
maps every row M01–M39 to A/S/Q/P/O/R/U/K and unchanged source routes. These
remain planned methods, not existing commands or tests. Selected harness commands
below become executable Inspector-owned wiring in feature 20; invoking them now
would not establish a RED or a pass. Feature owners name their individual tests
before RED, and extend the same runner rather than create competing registries.

| Layer | Selected command interface to implement | Evidence owner |
| --- | --- | --- |
| U/D | `cargo test --manifest-path src/inspector/Cargo.toml --test contracts` / `--test integration` | Feature 20 establishes Inspector manifest/targets; each feature adds own named cases. Test feature gates never enter product artifact. |
| B | `node src/inspector/tests/browser/run.mjs --slice=20` (then 30/40/50/60) | Playwright 1.62.1 with exact selected Chromium executable; actual UI/service, keyboard and malicious-content cases. |
| R | `cargo test --manifest-path src/inspector/Cargo.toml --test resources -- --test-threads=1` | Serial process/model/SQLite faults and peak/resource measurements under declared CPU target. |
| I | `node src/inspector/tests/installation/run.mjs --artifact=PATH` | Installed offline tar, not dev server; 20 shell then 70 all views. |
| H | Historical unperformed70 study pack | Removed by owner ruling; no0.2.0 execution or simulated participants. |

S16 chooses separate stored-population exact L2 inspection for M23/M26; it does
not promise public serving vector-stage distances. Retained ineligible rows may
participate under operator policy and are labeled stored scope; ordinary Engine
hybrid retains Engine eligibility/binary-shortlist/f32 attribution. M22/M25
index match evidence can be a separately labeled S read; no unsupported snapshot
claim or invented span. M30 optional reranker is not compiled; required cached
CPU embedding is positive scope. M36 source design is reviewable; cancellation,
resource and compatibility claims await feature/runtime evidence.

F-04 concrete test seams: ordinary PreparedWrite/ProvenancedNode and dependency
registration for intended facts; deterministic failing/blocking fixture Embedder;
existing test-only scheduler freeze, storage/commit failure and projection pause
controls for pending/failed/ready; controlled legitimate closed-file states for ordinary non-erasure cases. The selected Engine has no legitimate retained-erasure
content: physical closure requires zero remaining content before commit.
B16-01 is resolved by the owner's conditional-existence ruling, not a new hook.
Never seed an invalid proof or delay deletion to create a positive. Debug-only hooks
require a test child/debug helper; they are absent from installed product.
The absent retained-erasure target is valid unavailable/not-applicable scope.
Positive typed-content contracts and real atomic-delete absence integration
remain distinct. Other mandatory positive states must survive actual admission;
a planned control is not proof. No fixture may depend on keeping an external
writer or erasure-delaying snapshot attached. M20/M39 continue to require every
applicable overlap case on production read/export logic; retained-erasure Engine
positives are conditional on legitimate readable content, never manufactured.

## Current 0.8.29 port qualification

The [port record](compatibility-0.8.29-port.md) separates this branch's executed
checks from imported historical evidence. M20/M39 include the legitimate
format-37 ExplicitMaintenance queued cursor cut, all eight flag combinations,
real bounded export and unrelated records, in addition to Autonomous atomic
deletion/absence. The implementation never changes persisted erasure policy
or starts physical maintenance. Format-37 session/preflight reads also require
clean visibility coalescing; generation/nonce alone are insufficient.

## Slice30 lexical contribution

M06/09–17/20–22/25/28–34/36 consume the implemented native lexical workflow;
remaining vector/graph/comparison/installed/human stages retain their owners.
Independent authored corpus/numeric/identity and UTF8 evidence oracles are in
`src/inspector/tests/integration/lexical.rs`; real response-envelope/worker and
mandatory ignored F06 checks are in `src/inspector/tests/worker.rs`. Actual
keyboard/native/error/late observations are in `src/inspector/tests/browser/lexical.mjs`;
resource command and measured limits are recorded in the canonical
[Slice30 status](../0.2.0/slice-30/status.md). This contribution does not close
installed Ubuntu22/human T03/T05 or other views, and creates no second matrix.

## Slice 40 executed evidence

The same M16/M23/M26/M30 criteria use actual source and tests under
[src/inspector](../../../../src/inspector/README.md), indexed in its
[tool index](../../../../src/inspector/tools/README.md). Stored f64 L2/sign-bit,
identity/keyset/property and all8D01 tests use ordinary real vectors and legitimate
queued history. Controlled existing-span NULL and vector overflow cases test
unknown/refusal, never fabricate windows or count as serving positives. Native
Default dense contribution, asset fault/race/cache invariance, exact retained sink
and Chromium metric/score/origin/Back cases are separate executed evidence.
Shipping selects Inspector `cached-cpu`; the separate feature-off build passes
real-store/FIFO no-cache admission and unchanged SDK feature-off behavior.
[40 status](../0.2.0/slice-40/status.md) owns exact outcomes, failures and final gates;
no second matrix or installed/human pass is implied.

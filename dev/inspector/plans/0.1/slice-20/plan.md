---
title: Inspector v0.1 Slice 20 — execution plan
date: 2026-10-31
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Foundations and record inspection
---

# Slice 20 — Foundations and record inspection

## Draft and feature scope

Original draft baseline: `037a8522e`. Execution is ACTIVE after completed
Slices 15/16. The reconciliation below records accepted deltas;
[status](status.md) records source, actual tests and outstanding gates.

A runnable local Inspector that safely attaches a supported database through an owned Engine lifecycle and browses canonical and operational data with complete record detail.

Allocated scope: INS-01, INS-02, INS-03; INS-05 identity foundation; INS-06 state foundation; INS-09.1; V01-N01, V01-N02, V01-N06, V01-S01, V01-S03; initial V01-S02/S04; INS-X01–X11.

- Apply Slice 15's recorded D-01 policy/criteria and Slice 16's chosen runtime/UI/package and support envelope; report new contradictions rather than rerule policy or redo architecture.
- Build Inspector-local preflight and owned Engine session lifecycle, typed request/result/error references, loopback transport and a minimal real browser shell. Disclose/qualify normal Engine effects, preserve cache constraints and refuse missing/unsupported/locked inputs or external-writer conflicts.
- Deliver structure navigation, scoped counts/observed fields, paged node/edge/operational browsing, typed identity lookup, document/structured detail, exact-value copying and back navigation.
- Separate append-only logs from latest-state values; show incomplete provenance, engine-owned bookkeeping and structural role without invented history or source links.
- Revalidate Slice 16's all-feature capability inventory against the selected unchanged dependencies; make later-slice missing capabilities visible now rather than promise Engine refactors.

Exclusions: No text/vector querying or interactive graph is claimed complete here; no generic SQL console, migration, recovery, live/WAL attachment, Engine changes or global source refactor.

Owner delta, 2026-10-29: Engine startup is permitted when required. Follow
[the current attachment/effect contract](../../../requirements.md#engine-startup-allowance-2026-10-29).
External writers remain stopped; Engine-owned workers may progress. Qualify
normal Engine effects and per-read context rather than promise byte preservation.
Cache-only/no-download, exact identity and Inspector-only source scope remain.

## Release-wide prerequisites

Complete [Slice 15](../slice-15/plan.md) and [Slice 16](../slice-16/plan.md)
before this feature's implementation. Consume their recorded requirements/AC,
shared verification matrix, architecture/capability decisions and assigned
fixture/oracle/test obligations. Reconcile the draft against those baselines;
refine feature-local design/tests rather than repeat central planning. Feed
shared changes back to canonical documents/matrix and affected owners. Feature
correctness and tests remain slice-complete; Slice 60 is additional qualification.

## Required execution sequence

The [common slice workflow](../slice-workflow.md) is binding in full: reconcile
changes → update needs/req+AC → finish/review design → implement RED/GREEN →
Sol code review → Terra verification → status/integration/cleanup. Execute this
slice after planning it; a plan-only result is not slice completion.

1. Enumerate changes to this draft, related in-repo work, assigned features/
   functions and allocated findings/items since `037a8522e`. Read predecessor
   statuses and inspect actual implementation, not just documentation.
2. Evaluate each delta and keep/approve, adjust, reject or mark already done
   with a short evidence/reason row in this plan. Keep scope complete without
   speculative extension. An optional `change-plan.md` may guide a substantial
   revision; no duplicate decision paperwork is required.
3. Complete the slice's needs, requirements, acceptance and fixtures:
   Reconcile D-01 and the root inventory/detail criteria; define exact schema/lock/sidecar/refusal behaviors, disclosure, typed identities and the minimal session/UI contracts. Add F-01/F-02/F-03/F-05 facts and count/sample tests needed for this slice.
4. Expand [draft design](design.md) to the needed implementable contracts and
   have a directly spawned **gpt-6.1-sol-high** subagent review additions and
   changes. Resolve material findings before affected implementation.
5. Implement the feature using the steps below and per-behavior failing RED
   tests followed by separate GREEN/refactor work. Use real databases.
6. Have a fresh directly spawned **gpt-6.1-sol-high** code-review subagent inspect
   the implemented diff, then a direct **Terra** (`gpt-6.1-terra`) subagent
   independently test/verify the final revision and blast radius.
7. Update `status.md`, address material findings, integrate and clean up only
   extra slice-owned checkout/branch resources if any were created. Normally
   use the existing task checkout. Do not use Steward, Orchestrator or MCP.

## Planned implementation

- Implement the Slice 16 Inspector-local adapter/session boundary and prove its behavior on a supported real database; do not build a broad transport abstraction.
- RED/GREEN preflight refusal, qualified Engine effects and owned shutdown; then bounded identity/detail/list reads and their independent browser flows, one behavior per cycle.
- Implement inventory/detail renderers and operational mode navigation, then complete Engine-effect attribution, per-read observations, invalidation and return navigation.
- Document local launch for the actually supported development target and the non-destructive refusal messages; map remaining features to their owning slices.

Keep all implementation under `src/inspector/` and its narrowly enumerated
Inspector test/build/docs wiring. Changes to Engine, MCP, SDK, schema and
agent-access code/contracts are outside this slice, even if earlier proposals
suggested shared refactoring. A genuine dependency gap is a blocker, not
permission to expand scope or downgrade acceptance.

## Testing and slice completion

- F-05 successful/refused attach, missing/held lock, old/new schema, WAL/journal, pending Engine work, detectable replacement, generation progress, cancel/detach, no-extra-Inspector-mutation and cache constraints. Preflight refuses initial WAL/journal before Engine open; successful startup has no byte-preservation claim.
- F-01/F-02/F-03 exact identity collisions/revisions, plain/structured/large values, incomplete provenance, count scope, current values versus retained log entries.
- Actual browser attach → category → record → related item → back; empty/error states, keyboard selection and copied exact identity.

Run focused lint/type/behavior checks first. For an individual finding, rerun
the reproducer and affected/adjacent tests; full regressions require a broad
blast radius, unresolved risk or an explicit required final gate. Broader
Slice 60 qualification does not excuse missing tests or an unusable feature
here. Follow repository TDD, review, environment and retry rules.

**Completion:** Local launch and admission plus browse/detail are usable and tested; D-01 is settled for these reads; the revalidated Slice 16 baseline provides feasible Inspector-only routes for all mandatory capabilities. A newly discovered required gap reopens the affected baseline and blocks handoff until resolved; it is not a completion exception. A shell with placeholders does not close this slice.

The executor writes a short `status.md` with reconciled scope/revision,
requirements/design changes, Sol dispositions, RED/GREEN references, Terra
results, remaining gaps and cleanup. The status records actual implementation and verification scope. See the [master plan](../../plan-0.1.md) for full scope ownership.

## Slice 15 execution handoff — S15-2026-10-30

Consume [canonical acceptance](../../../acceptance.md),
[requirements/envelope](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
and [the single verification matrix](../verification-plan.md), source baseline
`9486c5ba9`. Check [Slice 15 status](../slice-15/status.md) before starting:
D-01 is adopted on 2026-10-30; implement its exact launch flags, independent
overlap gates and narrow retained-erasure export. This handoff does not waive
predecessor completion. Original draft baselines remain
for later delta reconciliation.

Assigned matrix rows (also M39): **M01–M07, M09–M15, M17–M18 (node), M20, M24, M29–M36**. Own deterministic fixture core F-01/02/03/05, typed reference/codec properties, document/tree, chunk/copy, preflight/effect observations, loopback credentials/origins, queue/cancel/shutdown and installed launch foundation. Build positive CPU/cache-independent browsing, not labels/placeholders.

This adjusts the draft by adding exact fixture/oracle/layer obligations,
INS-X07–X11 and explicit aggregate resource/security/accessibility/installed
contracts. It does not implement them. Preserve feature-local TDD and review;
replace planned methods with named tests/commands and executed evidence as the
feature is built. Required positive gaps block completion, even where an
unavailable/unsupported negative case works. No competing acceptance list or
Engine/MCP/SDK/schema/pin work is authorized.

D-01 final handoff: apply [the adopted policy](../../../requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30)
and matrix M20/M39. Slice 20 owns launch parsing/help/defaults, common gate
enforcement and narrow local export; 30/40 apply gates to their representations/
results/evidence; 50 extends history/graph/provenance and overlap cases; 60
qualifies all eight flag combinations and export in the installed artifact.
No flag alters Engine eligibility/ranking, restores physically absent content
or bypasses another applicable gate. No general export tooling is added.

## Slice 16 architecture handoff — S16-2026-10-31

Consume the [canonical architecture](../../../design/high-level-design.md) and
[Slice 16 status](../slice-16/status.md) before implementation. Central composition,
operation/reference/policy/resource/protocol contracts are selected there; local
refinement cannot silently replace them. The original draft did not establish execution; the status now records it. B16-01 is resolved by the 2026-10-31 owner ruling: retained-erasure content
is conditional on legitimate existence. Physical deletion remains unchanged.
Consume completed Slice16 before starting; typed supplied-content export
contracts are distinct from real Engine deletion/absence/flag integration.

Owned components: **A/S/P/O/R/K + U document/tree**. Implement the selected Rust supervisor/session process and plain browser shell, one-use local bootstrap, paired protocol, exact typed locators, guarded schema/marker/mean admission, bounded current-layout readers, independent policy gates and narrow retained-erasure export. Public logical point reads never replace exact revision lookup. Package has no Engine operator/test/benchmark/reranker features. Establish child/helper ownership and positive controlled fixtures before claiming lifecycle completion.

Assigned architectural verification: **M01–M07, M09–M13, M15, M17–M18, M20, M24, M29–M36, M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Implement cleared worker environments before any dependency initialization, exact allowlists and separate private browser launch; add real hostile-parent F-05 header/effect/cache/log tests. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.

## Execution reconciliation — 2026-10-31

| Delta since `037a8522e` | Evidence and impact | Disposition |
| --- | --- | --- |
| Source/build unchanged; no Inspector app | Git diff contains 45 documentation files only; `src/inspector/README.md` is sole source file. | Keep scope; all runtime work is new. |
| S15 complete, canonical M01–M39 and budgets | `slice-15/status.md`, current requirements/AC/matrix. | Adopt exact independent oracles, assigned positive/negative/browser obligations. |
| S16 selected Rust process/browser architecture | `slice-16/status.md`, canonical HLD A/S/P/O/R/K. | Adopt; refine operation wiring locally, no alternative server/framework. |
| Conditional retained erasure | Owner ruling and current D-01/M20/M39. | Adjust original unconditional positive fixture to supplied-content contract plus real atomic-deletion/absence. |
| N16-A01 ambient environment | Accepted postcompletion adversarial correction at `16693b72c`. | Adopt closed cleared worker/browser environment before dependency initialization; real hostile-parent tests. |
| Existing dependencies/callability | Current schema34 source, Engine open/close/config, exact canonical/operational layout, public queries and CPU constructor. | Revalidate all-feature routes without dependency edits; retain future query/vector/graph owners. |
| Minimal build/test wiring | New unpublished Inspector workspace member, Cargo.lock resolution, Inspector gate hook in agent checks, product docs. | Authorized minimal wiring only; no dependency pin/public Engine/API change. |

Per-behavior implementation order: launch policy/typed gates; cleared child
composition; guarded preflight; owned Engine attach/close; exact bounded
canonical/operational reads and policy; reference/observation expiry; chunk/copy
and structured token tree; loopback authentication/protocol; real browser
navigation; aggregate queue/cancel/teardown; package skeleton. Each behavior has
its own executed failing RED commit and separate GREEN, not one app-wide RED.

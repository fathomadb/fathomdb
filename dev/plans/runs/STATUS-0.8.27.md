---
title: FathomDB 0.8.27 release status
status: ACTIVE
---

# FathomDB 0.8.27 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.27.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.27:status-current-state -->**Next is Slice 85 (ENGINE-BOUNDARIES), PLANNED.** Completed on local `release/0.8.27` per release state: 0 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 1 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 2 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 3 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 4 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 5 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 6 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 7 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 8 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 9 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 10 (`3097d191511d81a221b038ccd2e14f074dcafa6d`) · 20 (`b455bb73fb2b04c91f50e6e5dbdc16752325453b`) · 30 (`6ba3be95cd043570da1deafbe4e2f78c878d8a87`) · 40 (`fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`) · 50 (`1f5b8614813b5a363ec5f81fcb580d48da4a4e8f`) · 60 (`d5a5bd39b3ee8a04bd080df451204564c6849bd1`) · 70 (`36fc2352cf243e022315ea302368d9424096aebd`) · 80 (`8e4499637e9d40ac6fcb9579f352b9f643e86709`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.27:status-current-state -->

Prework Slices 0-9, bounded preparation Slice 10, correction-safe erasure
Slice 20, real-surface comparator Slice 30, and engine-foundation Slice 40 are
complete on `release/0.8.27`. Slice 40 preserves runtime behavior, schema,
public API, feature gates, and publication state while moving shared engine
foundations to private modules.

Engine-domain Slices 50, 60, 70, and 80 are also complete. Slice 80 preserves
the read, search, graph, evidence, public, hidden, schema, SQL, wire, and
feature-gated contracts while moving their implementation into private
semantic modules.

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.27:status-next-action -->**Commission Slice 85 (ENGINE-BOUNDARIES)** — engine carrier ownership and dependency-boundary enforcement. **Remaining ladder:** 85 → 90 → 100 → 110 → 120 → 130 → 140 → 150.<!-- END GENERATED release-state:0.8.27:status-next-action --> |

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.27:status-live-open-count -->ONE<!-- END GENERATED release-state:0.8.27:status-live-open-count --> live open decision:

- authorize tagging/publication only after the complete ladder and release
  qualification pass.

## Completed release-branch ladder

| Slice | Scope | Status and evidence |
| ---: | --- | --- |
| 0 | Environment and infrastructure | Complete at `a3e6cff6`; existing worktree/base and local/unavailable capabilities recorded. |
| 1 | Dependencies and pins | Complete at `a3e6cff6`; updates retained, postponed, or allocated with evidence limitations explicit. |
| 2 | Repository/documentation cruft | Complete at `a3e6cff6`; disposition inventory only, no deletion. |
| 3 | Needs, requirements, acceptance, allocation | Complete at `a3e6cff6`; release-local contracts, global acceptance unchanged. |
| 4 | Architecture/code alignment | Complete at `a3e6cff6`; code-grounded module and ownership map. |
| 5 | Verification adequacy | Complete at `a3e6cff6`; deep existing evidence and missing non-vacuous oracles allocated. |
| 6 | Stale-documentation evidence | Complete at `a3e6cff6`; exact current/historical correction set allocated. |
| 7 | Build and delivery evidence | Complete at `a3e6cff6`; open recurring causes separated from closed regressions. |
| 8 | Reserved | Complete at `a3e6cff6`; intentionally empty. |
| 9 | Proposal review and closeout | Planned at `a3e6cff6`, closed at `73c53ffd`; every proposal ruled, Slice 10 inputs design-reviewed, and verification passed. |
| 10 | Bounded repository preparation | Complete at `3097d191`; current truth, dependency security, Action comments, digest authority, and Slice 30 prerequisites pass review. |
| 20 | Correction-safe source erasure | Product complete at `b455bb73`; exact atomicity, at-rest, binding, and unchanged-fixture Memex evidence pass review. The shared Python artifact gate is corrected at `6ba3be95`. |
| 30 | Current inventory and comparison guardrails | Complete at `6ba3be95`; deterministic 13-row real-surface baseline, mutation-tested comparator, candidate-bound artifact gates, independent design/code review, and focused verification pass. |
| 40 | Engine foundation and test seams | Complete at `fdd7fb64`; private error, identity, temporal, and hook modules preserve root paths and cfg gates. Final surface comparison, 124-suite gate, strict security, workspace Clippy/check, and independent review pass. |
| 50 | Engine erasure, lifecycle, dependency, and provenance | Complete at `1f5b8614`; private semantic modules preserve root paths and close nonterminal soft-closure residue with reviewed RED/GREEN evidence. |
| 60 | Engine write, ingest, and consolidation | Complete at `d5a5bd39`; six private write-domain modules, full-state write-boundary characterization, and bounded abort-seam carry-over correction. Public/hidden structural surfaces and release probe equal; test inventory additive-only; 127/127 gate and live AC-037 pass. |
| 70 | Engine projection, embedding, and reranking | Complete at `36fc2352`; ten private modules preserve public paths and runtime shape. Canonical, security, feature-complete, workspace, and artifact gates pass. |
| 80 | Engine read, search, graph, and evidence | Complete, with historical binding at `8e449963`; private read-domain modules preserve public/hidden surfaces and runtime behavior. Follow-up `9700991f` plus lint-only `31e78529` close the graph-result codec property gap and await independent rereview before the Slice 80 SHA fields are rebound. The live AC-037 re-run at `66e27983` is historical only. |
| 85 | Engine carrier ownership and dependency-boundary enforcement | Planned and uncommissioned. Settle root reader/telemetry/evidence carrier ownership without field widening, remove three facade-induced cycles, and add a non-vacuous normal-lint dependency-direction gate before Slice 90. |

## Verification boundary

The final whole-work review covered the complete Slice 20/30 plans, assigned
functions, implementation, tests, baseline, records, and generated artifacts.
Committed RED/GREEN rounds closed stale Python native shadowing, debug NAPI
leakage, multiline cfg ownership, distinct-filesystem capacity, scratch
ownership, and fixture-cleanup defects. Independent design review, code review,
and focused verification passed at clean `2967593c`; the final `6ba3be95`
change only made an older packaging assertion follow the canonical wrapper.

The reviewed baseline has 13 rows and SHA-256 `06212f66…`; a clean later
capture compared equal with no metadata or row diff. The canonical gate passed
lint, typecheck, strict security 0/0/0, Rust, TypeScript, executable NAPI, and
123 of 124 registered suites. Its sole failure was that obsolete packaging
assertion. After correction, the full Python suite passed 1,533 tests with 27
documented skips and the candidate receipt validated. Slice 150 retains fresh
installed-package and platform qualification.

Slice 40's final clean candidate `fdd7fb64` produced a fresh 13-row capture
that compared equal to the immutable baseline with empty metadata and row
diffs. The canonical gate passed all 124 registered suites with no failures,
skips, or exclusions; strict security was 0/0/0; the Python native receipt was
candidate-bound; and full-workspace Clippy/check passed. The locked Slice 20
erasure matrix remained unchanged and passed 5/5.

Slice 50's final clean candidate `1f5b8614` split the lifecycle, provenance,
dependency, and erasure domains into private modules and closed nonterminal
soft-closure residue without changing public paths. Independent design/code
review and verification passed. Public and hidden structural surfaces are
equal; hidden inventory has only reviewed additive tests; the canonical gate
passed 127/127 with strict security 0/0/0; workspace Clippy/check and the
candidate-bound Python receipt passed.

Slice 60's final clean candidate `d5a5bd39` moved the write facade,
validation, commit, provider transport, ingest, and consolidation into six
private modules. `actuation.rs` kept its existing domain, and every public path
is unchanged. Before the moves, a new write-boundary suite compared a full
`sqlite_master`-derived snapshot at ten refusal boundaries, plus a
validation-before-mutation property. It passed unmodified production and
failed under every recorded mutant.

Post-closeout adversarial review added real refusal coverage for both commit
exits. Owner follow-up then proved that the private abort marker could survive
validation and that its hook could survive an unrelated commit failure;
`81d723b1` is RED and `d5a5bd39` is GREEN. The final boundary suite passes
15/15 debug and 14/14 release with `test-hooks`.

The final 13-row public surface is equal with empty metadata and row diffs.
All eight hidden structural rows and the 41-item release probe are equal; test
inventory has zero removals or changes and reviewed additions only. Target
coverage is 261 targets with 53 feature-complete-only. The full canonical gate
passed 127/127 with no skips or exclusions. On the named 2026-09-26 unconfined
executor, the AC-037 live layer ran and passed; security was 0/0/0. Workspace
Clippy/check and the candidate-bound Python receipt passed.

Slice 70's final clean candidate `36fc2352` moved projection runtime, worker,
commit, registry, rebuild, vector storage and equivalence, mean, embedding,
and standalone reranking into ten private modules in ten verbatim batches.
`lib.rs` shrank from 25,899 to 19,001 lines, and every public path is
unchanged. A new residue suite proves that a failed projection commit leaves
no terminal, sidecar, vec0, or failure audit. The pre-existing red slice35
audit was repaired. The public surface is equal; the hidden surface is
additive only. The canonical gate passed after a path-only Windows WAL guard
retarget. Strict security was 0/0/0 with both AC-037 live layers, run
through a temporary per-binary AppArmor `userns` profile for
`/usr/bin/unshare`. The feature-complete gate passed 349/357
with 8 documented ignores on the RTX 3090s. Workspace Clippy/check and the
candidate-bound Python receipt passed.

Slice 80's implementation candidate `8e449963` moved fusion, filter, search
types and execution, search APIs, telemetry, read verbs, the reader-pool
implementation, and graph types/codec/execution/traversal into private
modules. Three characterization tests killed their specified mutants before
the moves. Independent code review closed two P2 architectural findings; the
reviewed clean candidate is `3e60cc5d`.

The canonical gate passed 127/127. Workspace Clippy/check and the
candidate-bound Python receipt passed. The 13-row public capture is exact; the
33-row hidden capture is additive only, with 261 additions, 0 changes, and 0
removals. Strict security was claimed 0/0/0 including live AC-037 through an
already-active capability. The post-hoc design review (2026-09-27) found no
captured pass lines or grant/revert record, so the live AC-037 layer is
UNEVIDENCED at `3e60cc5d`. A HITL-granted re-run through
`dev/release/ac-037-live-netns-hitl-runbook.md` passed 0/0/0 with both live
layers at `66e27983`, and the grant was reverted. The
feature-complete gate passed 349/357 with 8 documented ignores. The verifier
removed its disposable Python environment, restored the tracked stub, and
left the tree clean.

The post-hoc adversarial design review of Slice 80 (2026-09-27) returned
PASS-WITH-FIXES. Fix-1 `b8af4d86` made two changes, with no behavior change:

- it restored four widened carrier fields to private by keeping
  `TelemetrySink` and `EvidenceCapture` at the root;
- it narrowed six over-visible items.

The records list three facade-induced module cycles and add the Slice 140
test-seam carry-over. Planning-only Slice 85 owns removal of those three cycles
and durable carrier placement; Slice 90 consumes the settled boundary. See
`features/slice-80/design-review.md`.

Follow-up `9700991f` adds generated coherent result-codec typed round-trip and
positional evidence-corruption properties. Both killed their specified
temporary production mutants and passed after exact restoration. This closes
`TC-aa4bea08-f281-47eb-8022-d63250d1daac` without changing production or
creating a generated golden oracle. The recorded Slice 80 implementation,
reviewed-candidate, and closeout SHAs remain historical pending independent
rereview.

The `66e27983` live AC-037 receipt applies only to that historical candidate;
it does not qualify later HEAD or the final candidate. By owner ruling, a fresh
live run is deferred until after Slice 130, and Slice 150 must execute the HITL
runbook against the exact final candidate with grant/revert evidence.

## Boundaries

- This release starts at schema 34 and currently proposes no schema migration.
- Publication is unauthorized.
- Slices 30, 40, 50, 60, 70, and 80 were directly authorized by the
  repository owner and are complete; Slice 85 and later slices require
  separate commission.
- No temporary branch or worktree was created for prework; the existing
  `release/0.8.27` worktree remains the active release workspace.

---
title: "0.8.29 Slice 7 — delivery delay review proposal"
status: PROPOSED
target_release: 0.8.29
slice: 7
---

# Slice 7 — delivery delay review proposal

## Goal, maturity and boundary

Propose the smallest preparation that discovers delivery blockers before costly
builds and preserves meaningful product evidence. This slice performs planning;
Slice 9 selects changes, Slice 10 implements approved preparation, and Slice 160
owns integrated qualification/readiness. Publication remains separately authorized.
No dispatch, installs, authentication/policy changes, cleanup, code or Git edits.

Read [brief](../../planning-brief.md), [evidence register](../../planning-evidence.md),
[.28 completion](../../../0.8.28/features/slice-95/status.md), its
[qualification record](../../../0.8.28/features/slice-95/qualification-evidence.json)
(searches of relevant delivery entries), and the source/design sections linked below.
Memory index and existing `full-gate-run-frequency.md`,
`fathomdb-ci-single-maintainer-minimal-not-gated.md`,
`release-dod-requires-full-workspace-gate.md`, `release-publish-gotchas.md` and
`agent-worktree-stale-base-trap.md` informed this proposal. No chats were read
independently here; transcript claims are attributed to the evidence register.

.28 shipped: registry consumers, engine package self-dependency recovery, npm
OIDC promotion, SDK trusted publisher and standing runner restoration are fixed.
OIDC-only run42750034084 skipped35 build/publication jobs; readiness42726255745
proved job delivery, not builds/inference. The failed CPU/full local campaigns
remain failed with scoped closure; they are not uninterrupted final-tree green.
.29 includes these fixes. New setup/resource, route and changed-input needs must
be distinguished from repaired .28 defects and from unavailable privileges.

## Local needs, requirements and acceptance draft

IDs below are release-local planning labels, not canonical acceptance additions.

| Need | Requirement | Falsifiable acceptance proposal |
| --- | --- | --- |
| P7-N1 affordable feedback | P7-R1 check prerequisites before compilation | P7-A1 names command/tier, tool pins, checkout-owned venv/native identity, memory/disk/concurrency and earliest failure; every missing item has an owner/disposition. |
| P7-N2 truthful verification | P7-R2 preserve scope and real failure | P7-A2 maps each proposed change to existing checks, logs, actual counts/exit and reuse invalidators; no skipped publisher or assembled campaign is called an executed pass. |
| P7-N3 predictable CI | P7-R3 retain proportional routing and bounded resources | P7-A3 identifies affected jobs, cold/cache setup and build/upload/cache-post peak headroom; runner identity, job delivery and product execution remain separate evidence. |
| P7-N4 releasable packages | P7-R4 cover packaging and registry prerequisites | P7-A4 lists Axis W/E, rewritten dependencies, native targets and publisher identities, separating locally testable inputs from real-publish-only behavior. |
| P7-N5 safe, useful scanning | P7-R5 preserve current-tree enforcement/history advisory | P7-A5 distinguishes confirmed credentials, corroborated nonsecrets, unresolved findings and scanner/setup failure without broad allowlisting, raw-secret retention or history rewrite. |
| P7-N6 early access decisions | P7-R6 expose protected-route needs | P7-A6 specifies .29 first-tag/workflow/environment prerequisites, authorized actor and stop points; no .28-only exception or existing registration implies .29 permission. |

## Architecture audit, design and fit review

Read [release design](../../../../design/release.md),
[proportional CI design](../../../../design/ci-cd-simplified-redesign-20260821.md),
[crate topology](../../../../adr/ADR-0.6.0-crate-topology.md),
[verifier](../../../../../scripts/agent-verify.sh),
[CI workflow](../../../../../.github/workflows/ci.yml),
[release workflow](../../../../../.github/workflows/release.yml),
[local rehearsal](../../../../../scripts/release/local-dry-run.sh),
[publish helper](../../../../../scripts/release/cargo-publish-if-new.sh),
[tag route guard](../../../../../scripts/release/check-tag-cuda-route.sh) and
[history scanner](../../../../../scripts/security/gitleaks-history.sh).
The architecture already separates cheap routing/setup, execution and publishing;
retain it. Draft one compact delay register with cause, affected input/job,
fixed/residual/access classification, smallest intervention and evidence reuse.
No new workflow family, required-check aggregator, merge queue or scheduled soak.

Apply [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
recommendations for reusable harnesses, bounded failure controls and package/platform
checks. Defer new global coverage/fuzz programs to their product owners; no new
mandatory gates. [CUDA usage review](../../../../design/cuda-version-support-review.md)
belongs to Slice120; do not expand GPU support to solve a routing delay.
Before a selected behavior change, research only the unresolved mechanism in
GitHub runner-group/environment/OIDC documentation and crates.io/npm/PyPI official
publishing documentation. Record retrieval date and distinguish provider limits
from repository bugs; omit generic best-practice paperwork or speculative upgrades.

## Dependencies, sequencing and newly discovered prerequisites

- Hard: 9 consumes this proposal; 10 needs recorded 9 decisions before execution.
- Coordinate with 0 on setup/hosts and 1 on pins; with 5 on coverage and 6 on
  documentation claims. These are draft exchanges, not execution dependencies.
- Conditional: selected CI/tooling repairs in10 precede affected20/30 measurements;
  final changed product/platform inputs flow to160, not a duplicate matrix here.
- Source discovery: the tag guard permits bootstrap only for `v0.8.28`; GitHub's
  first-tag routing problem therefore needs a bounded .29 design/owner decision.
  Approve an exact route/exception before use; retain reviewers/group restrictions.
- Setup evidence reports Node26/npm12 versus Node25/npm11.12.1 contracts, no owned
  venv/TS modules and18GiB free at98%; these are reported observations, not probes
  repeated here. Resolve capacity/tool ownership before expensive execution.
- Runners168/31/22 were restored; availability, scopes and candidate access must
  be refreshed at execution. Keep active standing installation; no re-registration
  by inference. R580/primaryCUDA12.6 stay fixed; never run K620.
- Workflow, bootstrap, package manifests and release design are shared surfaces:
  one writer per checkout; serialize approved edits with dependency/feature work.

## Tasks and deliverables

1. Draft `requirements.md` and `design.md` with P7 mappings and a single bounded
   delay register: retain fixed .28 controls; propose a repair only for a demonstrated
   residual, current mismatch or unsupported .29 route. Assign each to10/160/owner.
2. Draft `test-mapping.md`: reuse existing environment/bootstrap, workflow-scope,
   registry-safety, publish-idempotency and Gitleaks fixtures; identify only missing
   behavioral cases. Include wrong/missing tool and preflight-log failure controls.
3. Describe resource checks through restore/build/test/upload/cache post-processing;
   preserve the advisory parallel report's purpose and actual exit/counts. Budget
   concurrency and task scratch, and propose scoped remedies before cleanup requests.
4. Map W bump via `set-version.sh`, independent E drift check, leaf rehearsals,
   dependent-crate registry-resolution limits, native package smokes and final
   installed-registry reopen/close/exit. Existing SDK OIDC repair needs no repeat.
5. Separate source/generated-text scans from noisy full-history advisory; reuse
   safe classifications and propose bounded changed-findings triage, not rescanning
   all history to obtain a green badge. Authentication-only proof is a separate row.

## Validation, independent review and decision points

Proposal completion means all P7 rows have evidence, ownership, test mapping and
explicit unknowns; links resolve. Parent integrates drafts and runs Markdown lint.
Independent review challenges duplicate work, hidden publication/auth changes,
false-green language and the .29 first-tag route. No review outcome is presumed.
Approved later executable fixes use failing test first, visible RED then GREEN;
focused checks while editing, actionlint for workflows, one final clean-tree full
`agent-verify.sh` per code phase. Default/all includes lint/typecheck/security/test;
heavy alone does not. Use owned venv/pinned tools; ptrace denial needs an unchanged
strict rerun on a capable executor. Records-only changes use scoped validators.
Rerun failed/affected suites; broader changes justify broader qualification.
Reuse .28 evidence only with explicit source/config/artifact-input equivalence.
Missing access, protected-route approval or package bytes stop the affected claim;
unresolved details go to9. No fresh timing saving, host availability or pass is claimed.

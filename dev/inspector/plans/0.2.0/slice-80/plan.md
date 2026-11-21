---
title: Inspector 0.2.0 Slice 80 — release readiness
product: FathomDB Inspector
target_release: 0.2.0
status: APPROVED
desc: Documentation convergence, tested local CI and packaging/distribution readiness
---

# Slice 80 — release readiness

## Authority, prerequisites and outcome

The owner explicitly commissioned this slice and then90 in
[INS-020-LOCAL-RELEASE](../release-decisions.md) (Inspector ledger seq-1).
Planning baseline `2a56af3e7a7031343322f4db9956ebf0a1ea949c` has completed75,
all seven expert UX fixes, Sol/Terra approval and206/206 source suites passed.
Slice70 completed technical qualification; its former five-person study gate is
removed by decision. Reconcile and close its current disposition honestly, without
claiming the unperformed sessions happened. Readiness work does not wait on them.

Deliver accurate project, user-facing and consuming-developer/contributor docs,
a runnable locally tested CI path and a qualified versioned Inspector package
contract that Slice90 can release without inventing infrastructure or commands.
[Design draft](design.md) guides implementation. This slice prepares and qualifies
a candidate;90 owns the final `dist/` release. No registry deployment in0.2.0.

## Scope and acceptance

The R80 labels are slice work/checks, not a second global acceptance register.
Existing functional needs/INS/V01/X requirements remain authoritative.

| Work | Observable acceptance |
| --- | --- |
| R80-01 · reconcile decision and project state | Current requirements/acceptance/scope/matrix/index/readiness status no longer require five participants or describe them as blockers. T01–05 and High functional goals remain; past study forms/report results are clearly historical/unperformed. Slice70's former blocker closes by owner decision, not a fabricated pass. The release ladder is75 complete→80 readiness→90 local release. |
| R80-02 · user documentation | Installation/extraction/checksums, Chromium prerequisites, exact supported OS/kernel/browser/Engine/schema pairing, launch/attach/close, real text/vector/graph/compare workflows, disclosure flags, existing-model requirements and actionable refusal/troubleshooting guidance agree with the actual0.2.0 package and75 UI. No Node/Python/development server needed by users, no claim of registry availability, participant validation or unsupported platforms. |
| R80-03 · developer and project documentation | Source architecture/contracts, build/feature/tool/environment setup, local verification and fixture/extension guidance, package/version identity, release notes/changelog, known limits and roadmap explain the implemented product. Canonical links work. Historical evidence remains tied to its actual artifact. Unrelated repository documentation is not a cleanup campaign. |
| R80-04 · establish and test CI locally | One documented Inspector-owned entry point runs real relevant source/unit/browser and packaging/installed checks with explicit prerequisites, useful failure status and no silent required-positive skips. Reuse typed repo gates and existing harnesses. Execute the documented path on this checkout; record actual commands/environment/results, including meaningful failure propagation. Validate any affected workflow with actionlint and existing routing fixtures. Local success is labeled local; no remote hosted-CI claim or paid runner requirement. |
| R80-05 · packaging and distribution method | Define stable product/version/platform artifact names, tar/extracted contents, license/notices, internal and archive checksums, build/Engine/schema/features/ABI identity and supported installation/removal. Build an actual locked/offline cached-CPU0.2.0 release candidate in the qualified isolated Ubuntu22 target, extract it and test the actual executable. Existing models are not bundled/downloaded, credentials/fixtures/test-only features/dev runtimes are absent. |
| R80-06 · local release protocol and future channels | Implement/test a minimal local promotion method or documented commands for90: source/candidate check, existing-destination handling, staging/commit to `dist/inspector/0.2.0/`, no partial success, no unrelated overwrites, checksum/metadata validation and failure cleanup. Verify existing gitignore rules cover the complete destination. Document future registry/download channels and remaining publish/version/signing/CI requirements as roadmap work; implement no registry uploader or credential path now. |
| R80-07 · handoff | A clean reviewed source revision, real tested candidate paths/hashes, exact local-CI result/limits, concise docs and runnable90 instructions are available. No unresolved required readiness or material review findings remain. Artifacts are candidates until90 actually completes its release. |

## Design and implementation sequence

1. Reconcile the decision across current Inspector surfaces. Preserve the original
   expert report, historical failures and unperformed study evidence with clear
   supersession pointers. Do not mechanically delete every historical human mention
   or rewrite a source test as passed. Avoid changing global Engine release state.
2. Inventory actual docs/CI/package behavior and the75 candidate. Identify real
   missing commands/contracts before adding scripts. Inspect shared CI selection;
   keep any necessary shared wiring narrow to Inspector, independently reviewed.
3. Finish design and obtain Sol-high review. Prefer one small Inspector-local CI
   entry point invoking existing gates and one local staging/promotion method.
   Future registries are documented extension points, not a plugin system or new
   multi-platform release matrix. No registry secret or remote publish command.
4. Apply docs and minimal scripts/tests with real RED/GREEN for behavior changes.
   Check failure propagation, absent prerequisites, wrong metadata/checksums,
   populated output destinations and interrupted staging with small owned fixtures.
   Do not build a general tamper/audit framework or spend a full build on every case.
5. Execute local CI and package qualification on a final coherent candidate. Test
   no-dev-runtime extracted help/build identity and actual installed all-view/person
   workflow/native close against real fixtures, including the published0.8.29 review
   dataset and75 corrections. Reuse unaffected scale/cache evidence truthfully;
   new script results do not silently recertify all source/platform combinations.
6. Sol-high code review, fix/recheck findings, then independent Terra verification
   of the actual documented local-CI and package/release rehearsal. Perform applicable
   source/docs gates once at final clean phase, preserving exact pass/fail/skips.

## Draft reconciliation and execution plan

At execution,395662894 adds the settled local-release ruling and80/90 ladder
since75. Keep the seven75 UX fixes, genuine206-suite source pass and completed
70 installed qualification; do not reopen product functions. Adjust stale active
needs/AC/matrix/HLD/project/user/developer docs and close70 by owner disposition.
Historical study/report evidence remains unperformed and explicitly superseded.
Keep existing shared CI routing (already correct), fixture-test it. Add only the
missing local CI command, outer checksum/isolated build cleanup, compact clean
source/build candidate binding and atomic no-overwrite promotion. Reject new
registry uploaders, platform matrices, study recruitment and duplicated suites.
No assigned change-plan work beyond these readiness gaps remains.

Sol-high design review approved with source-binding and isolated Ubuntu22 release
example clarifications. Implementation uses per-behavior RED commits and GREEN
commits for candidate validation, atomic promotion/interrupt cleanup, package
completeness and CI prerequisite/failure propagation. Filesystem/command fixtures
exercise those contracts only; actual compatible build/native real-DB checks are
mandatory independently. Fresh Sol-high code review and Terra verification follow.
One final clean full source gate is the source stage of default local CI; do not
repeat it merely to close documentation. Applicable docs/MkDocs/routing checks
complete the readiness evidence.

## Completion and90 handoff

Write concise `status.md` with decision disposition, changed docs/CI/package
contracts, RED/GREEN and review results, executed checks, actual candidate source
revision/build/hash/path and any explicit limits. COMPLETE means readiness is
proved and90 has concrete release instructions; it does not mean the local release
has already occurred. Keep the qualified candidate outside final `dist/` or clearly
in an owned staging location until90. Preserve75/70 historical candidates.

If closure changes docs only, retain the candidate's actual tested source/build
identity rather than relabeling it with the later status SHA.90 must verify that
its selected bytes match this handoff and qualify any substantive later change.

## Execution method

Read applicable AGENTS and indexed memory, the current source and prior statuses,
canonical Inspector needs/requirements/acceptance/HLD, [scope](../scope.md),
[workflow](../slice-workflow.md), [decision](../release-decisions.md) and the single
[verification matrix](../../0.1/verification-plan.md). Enumerate relevant changes
since this draft and assigned functions/drafts; keep/adjust/reject/already-done
with concise reasons. Revise the plan to be complete without overbuilding.

Add or adjust precise Inspector requirements/AC only where needed. Review actual
design changes with a fresh directly spawned `gpt-6.1-sol` high before affected
implementation. Use genuine per-behavior RED execution/commit then separate GREEN;
real databases, independent intended facts, no database mocks, generated goldens,
assertion weakening or skipped checks. Docs-only/version-only work needs no fake RED.
Obtain fresh direct Sol-high code review and remediate material findings; use direct
Terra (`gpt-6.1-terra`) independent verification. No Steward/Orchestrator or MCP
model wrapper. Keep reviews coherent and source-read-only; serialize mutable builds.

Use focused lint/type/tests while working, existing tools before new ones and
reproducer/blast-radius reruns for fixes. Source/test/executable changes require
one final clean full `./scripts/agent-verify.sh` per code phase; metadata/artifact
promotion alone uses applicable scoped checks, not another unchanged206-suite run.
Preserve actual failures/skips and candidate provenance. Markdown uses scoped
validators, public docs add docs lint/strict MkDocs, and formatting is only guarded
`npm run format:md -- explicit-paths`. No hook bypass, prettier or raw Markdown fix.

Reapply the quality roadmap pragmatically: real DB composition, installed artifact
checks, bounded fault/cleanup and meaningful packaging/CI failure tests. Defer global
MC/DC/fuzz/benchmark campaigns and new orchestration/evidence frameworks. Keep status
and documentation concise; no supplemental audit packet phase.

Use the existing `inspector/dev/0.2.0` release checkout sequentially at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`; primary stays read-only.
Extra concurrent writers require isolation; integrate/clean only verified owned
extras. Preserve the owned target symlink, model caches, original review databases,
old artifacts and other sessions. Signal only exact self-recorded owned PIDs/PGIDs.
No Engine/SDK/MCP/schema/ranker/API/pin/other-package-version changes. No GPU/driver
changes, new model fetch/repair, broader support target or paid infrastructure.
Shipping explicitly selects `cached-cpu`, fixture controls are absent; bare Cargo
workspace builds intentionally remain model-free. D-01, exact identities, metric
semantics, resource limits, cache/network isolation and physical deletion stay intact.
No git tag, remote push, registry upload or main merge is authorized by these slices.

## Execution prompt

The owner authorized sequential execution; this filled template commissions the slice
when its actual prerequisites above are satisfied.

```text
Work on and complete Slice 80 for Inspector inspector/dev/0.2.0 (and in a worktree).

Ensure the slice has planned to do any needed work regarding requirements+acceptance criteria, design, implementation, and testing; each should be slice-complete and correct. The slice might have slice-related "change plan" documentation for guidance. The slice's plan should:
- identify and enumerate changes since slice's plan was written (review drafted slice plan, review in-repo related work done, review assigned feature/functions, review allocated draft items).
- Evaluate the enumerated output, and if needed, update/approve/reject/adjust the drafted plan, ensure it is complete but not overbuilt.
- add/adjust needs, requirements, acceptance criteria.
- write/update design (design additions and changes should be reviewed by a design review subagent (gpt-6.1-sol-high).
- The plan should then describe implementation with TDD (RED/GREEN method) and code review subagent (gpt-6.1-sol-high).
- Use a subagent (terra) to test/verify.
- Write Slice status record. Clean-up after itself: While a wt or branch should not be necessary for a slice, if a new worktree or branch was used, merge back and clean-up the worktree.

Keep ceremony to a minimum. Stay focused on the work-at-hand. When resolving a specific finding, do not run full regressions unless the changes are comprehensive enough to warrant the overhead. Generally, solve for the work outlined in the slice; check for impacts in the blast radius; be pragmatic. (do not use Steward or Orchestrator.)

After the planning for requirements+acceptance criteria, design, and implementation is planned: implement the slice according to the plan.
```

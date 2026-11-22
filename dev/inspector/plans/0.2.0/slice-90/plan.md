---
title: Inspector 0.2.0 Slice 90 — local release
product: FathomDB Inspector
target_release: 0.2.0
status: APPROVED
desc: Perform the approved local-only release to gitignored dist artifacts
---

# Slice 90 — local release

## Authority, sequencing and result

[INS-020-LOCAL-RELEASE](../release-decisions.md), Inspector ledger seq-1,
authorizes this actual release after [Slice80](../slice-80/plan.md) completes.
Do not start before80 has resolved material review findings and supplied a clean,
tested candidate/CI/docs handoff. The five-person study is removed, not pending.
Completed75 UI fixes and all technical safety/identity/resource criteria remain.

Create the local Inspector0.2.0 release in the release worktree's
`dist/inspector/0.2.0/`. The root `dist/` tree must be gitignored; no binaries enter
Git. This is a local release with release notes/status and verified deliverables,
not a public registry release. [Design](design.md) defines the handoff.

## Execution reconciliation — 2026-11-21

Reviewed the drafted plan,80's completed implementation/handoff, assigned local
release functions, canonical needs/requirements/AC/HLD and the single matrix.

| Change since draft | Disposition |
| --- | --- |
|80 now COMPLETE with Sol design/code approval, strict206-suite source CI and81 installed checks at `7449ee0108d503cbed3b6a33e2c841072d6de0ce` | Already done; retain actual source/build scope, do not rerun unchanged campaigns. |
|80 supplied source-sealed candidate, inner/outer hashes, tested archive extraction/modes and atomic no-overwrite staging | Keep existing implementation; use exact candidate, not a rebuild or relabel. Existing destinations always refuse, including identical ones. |
| Three Inspector closure Markdown files follow qualified source at execution HEAD `b7b0ce1df4a3780026ba4cc93d6c2f459eda804b` | Approved documentation-only descendant; source verification passed before promotion. |
| Current needs/requirements/AC/matrix/HLD already incorporate the removed study and local80/90 distribution | No new product requirement/design/API needed; R90-01–06 cover release acceptance. No human evidence inferred. |
| Future-tense user/project/contributor availability remains after80 | Adjust existing docs to actual local availability after promotion; retain exact support and no-registry limits. |

Fresh direct Sol-high applicability/design review APPROVE: existing80 method and
R90-01–06 are complete without new code. Execute promotion, final-destination
integrity and independent Terra installed smoke/trace; review the final release
diff with Sol-high. No RED/GREEN change is needed for unchanged artifact copying
and Markdown closure. Any actual behavior correction instead follows the test-first
method below. [Status](status.md) records the performed result.

## Scope and observable acceptance

| Work | Observable acceptance |
| --- | --- |
| R90-01 · select exact approved candidate | Verify actual branch/revision,80 COMPLETE/reviews/results, artifact hashes and build/version/features/Engine/schema/platform metadata. Candidate source/build provenance is accurate even if later closure docs have another SHA. Any substantive unqualified source or packaging delta is resolved before release. Do not rerun unchanged broad checks just for metadata. |
| R90-02 · create local dist | Use80's tested method to stage and release into `dist/inspector/0.2.0/`. Include the tar.gz, directly usable extracted executable/bundle or documented equivalent, license/install README/build description, internal and archive checksums and compact release metadata/notes. Artifacts identify Inspector0.2.0/Linuxx64, cached CPU enabled/fixture controls absent and the precise qualified pairing. |
| R90-03 · integrity and destination safety | Verify archive extraction and checksums plus selected executable bytes from the final destination. Preserve previous artifacts and unrelated dist entries. On existing identical release, verify/return idempotent success only if the protocol permits it; mismatched/populated destinations refuse without overwrite. A failed/interrupted staging run leaves no falsely successful final release. No tokens, benchmark payloads/model assets, developer environments or test helpers are distributed. |
| R90-04 · final installed operation | Run the executable from the final dist bundle or a fresh extraction of its final archive without Node/Python as product runtimes. Check help/build/version/feature identity and a real installed person/all-view/compare/Back workflow on an owned fixture, normal native close, lock/private-process cleanup and offline behavior. Existing model prerequisites remain explicit. Test tools may use Node/Python; the released application must not depend on them. |
| R90-05 · ignore and release record | `git check-ignore` covers every binary/archive/metadata file under final dist; `git ls-files dist` contains no release payloads and tracked status is clean after normal status/docs commits. Record local release source/build/hash/path, executed results, exact support limits and no-registry distribution. Current user/project/developer docs point to local installation and accurately describe release availability. |
| R90-06 · honest completion | Source and compiled Engine pins/schema stay unchanged; any required corrections are reviewed/tested rather than relabeled. All owned processes/staging/locks are cleaned while final release artifacts remain. No git tag/push, GitHub Release, npm/PyPI/crates/OCI/OS-registry upload, remote deployment or main merge occurred. |

## Release execution sequence

1. Read80's actual handoff and reconcile the plan. Verify released bytes and
   support/CI evidence rather than trusting a status label. Record any new delta
   and keep/adjust/already-done disposition. Obtain Sol-high review for any actual
   design/code additions; this release should chiefly execute80's tested method.
2. Perform the local release into an owned staging directory using the80 method,
   verify before committing final outputs, and handle collisions as specified.
   Do not overwrite the prior70/75 candidate or unrelated `dist/` artifacts.
3. Validate archive paths/contents, all checksums/metadata and the actual final
   executable. Verify package architecture/glibc/feature statements from80 rather
   than broadening support from the host's successful launch.
4. Direct Terra independently verifies final dist integrity, gitignore status,
   documented installation and the actual final native workflow/close. Fix a
   concrete release finding with its reproducer and affected checks. New script
   behavior requires genuine RED/GREEN and Sol-high code review; artifact copying
   or metadata-only updates do not create artificial RED tests or another full
   source regression campaign.
5. Commit only concise release notes/status/documentation updates. Record the
   actual candidate/source commit and release-record commit separately. Validate
   docs/metadata and any required gate for real code changes; a previous full gate
   is cited at its actual SHA, never invented as an exact-final-HEAD execution.
6. Confirm COMPLETE with final paths/hashes, reviews/verification/cleanup and clean
   tracked worktree. The final binaries remain locally available in ignored dist.

## Distribution boundary

The0.2.0 deliverable is local only.80 documents future registry methods and their
remaining prerequisites;90 does not exercise them, request credentials or push any
release tag. Avoid `v*` tags because the repository's Engine publisher can be
triggered by them. A future public distribution needs its own explicit commission.

Do not let a removed human-study gate reappear. Preserve the truthful historical
record that no sessions occurred; functional/expert/automated evidence continues
under the current criteria. Final status must distinguish local release availability
from future public registry availability without claiming untested platforms.

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
Work on and complete Slice 90 for Inspector inspector/dev/0.2.0 (and in a worktree).

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

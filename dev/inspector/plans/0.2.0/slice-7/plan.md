---
title: Inspector 0.2.0 Slice 7 — release-delay evidence and remedies
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Read-only build and delivery investigation with scoped proposals for Slice 9
---

# Slice 7 — release-delay evidence and remedies

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Mandate and boundary

This DRAFT plans a future evidence review, not a release run or repair campaign.
Deliver a concise, source-grounded account of failures that defer release after
product builds, with the smallest useful remedies and explicit owners. The
[adopted scope](../scope.md) is approved; this slice does not reopen it.
The [master plan](../plan.md) and [shared workflow](../slice-workflow.md) own
current numbering: comparison is 60, final installed qualification is 70.
Historical active Slice 20 and old 60 references do not establish current closure.

Inspected read-only baseline: clean `llm/0.8.29-inspector-additional` at
`609dd85c41fb7980e05faa1132a248d376d8ec89`, based on Engine
`6edc1155f039f87cd0a37660cb27ae4373bbce71`; workspace packages 0.8.28,
Inspector unpublished 0.1.0, current format 37. Future release branch
`inspector/dev/0.2.0` must inherit the approved final planning-branch HEAD,
including port, scope and integrated plans; do not cut from raw main/dev/0.8.29.

In: build prerequisites, worktree/preflight mismatch, verifier scope and cost,
CI routing, package composition and installation, Gitleaks disposition, registry
visibility/authentication, and other evidenced delivery failures. Distinguish
product defect, tool/environment failure, expected refusal, stale evidence and
optional/advisory red. Record actual elapsed delay only when logs support it.
Out: builds, tests, installs, bootstrap, library sweeps, source/config/workflow
changes, credential changes, secret scanning campaigns, dispatches, tags,
publication, registry writes and cleanup outside slice-owned scratch.

## Preparation and prerequisites

- Slice 0 supplies environment proposals and the release checkout under the owner ruling;
  Slice 1 owns dependency/pin proposals, 5 test adequacy, 6 stale documentation.
  Read available outputs, but do not wait for unrelated investigations or redo them.
- Before execution enumerate changes since this draft in plan/scope, relevant
  scripts/workflows/package files, assigned functions and allocated proposal items.
  For each choose keep/adjust/reject/already-done with source or run evidence and
  reason. Revise this draft once around substantive changes, without overbuilding.
- Recheck applicable AGENTS and actual memory: `dont-be-distracted-by-ceremony.md`,
  `fathomdb-ci-single-maintainer-minimal-not-gated.md`,
  `verify-design-against-code-not-just-architecture.md`,
  `full-gate-run-frequency.md`, `tdd-red-must-fail-per-finding.md`,
  `keep-measurement-tools-in-repo.md`, `release-publish-gotchas.md`, and
  `agent-worktree-stale-base-trap.md` in the indexed external memory store.
  Source and current instruction win over obsolete commands in historical memory.
- Use the selected report worktree sequentially, one directly spawned agent
  as sole writer. No Steward/Orchestrator. Extra isolation only for necessary
  concurrent writers; preserve release worktree and unrelated work.
- A missing checkout environment does not prevent reading evidence. Do not borrow
  a venv, bootstrap it here, or mark required runtime qualification unavailable-pass.
  The linked owner decision assigns release-checkout creation and entry to Slice 0.

## Evidence already inspected and draft candidate allocation

Paths below are repository-relative; inspect their current bodies before deciding.
These are investigation labels, not new canonical acceptance IDs.

| Item | Evidence / current distinction | Minimal proposal to evaluate; owner |
| --- | --- | --- |
| S7-A | `dev/inspector/plans/0.1/compatibility-0.8.29-port.md` records two full-gate stops at missing checkout-owned `.venv`; scoped Rust/browser passes are separate. `scripts/agent-verify.sh` already checks own Python, pip and pinned tools before lint. | Keep early failure; make approved checkout preparation usable before final gate. Slice 0 owns proposal, 9 decides, conditional 10 may implement; 20/70 own actual required source/installed qualification. Do not add a duplicate preflight. |
| S7-B | `scripts/preflight.sh --worktree/--expect-closed` reads the Engine live release-state selector; its worktree message still says build on MAIN. Inspector uses a separate approved planning ancestry and forbids primary-checkout execution. | Determine exact compatibility of existing preflight with Inspector branch/plan. Propose the narrow invocation/documentation or separately approved adapter needed, without fabricating Engine ladder closure or disabling protections. Slice 0 owns bootstrap, external harness owner owns generic changes. |
| S7-C | `.github/workflows/ci.yml` rust paths are `src/rust/**`, `Cargo.toml`, `Cargo.lock`; `verify` heavy selects language/harness flags. `src/inspector/**` alone is absent; `verify-fast` still runs for non-Markdown. | Source-backed routing-gap candidate: fixture-test Inspector-only Rust/assets/package paths and the smallest appropriate job mapping. CI/harness owner; 9 considers, conditional 10 implements only if approved. Do not claim an observed hosted skip without its run log. |
| S7-D | `src/inspector/tools/package.sh` builds locked/offline default Linux x86_64 archive, checks fixture control false, but names 0.1.0. Package README records Ubuntu24.04/glibc2.39 binary cannot run on22.04. Port did not repeat installed qualification. | Propose selected22.04-compatible build/extract/runtime route and one source of Inspector package version when bump is approved. Inspector packaging owner; 9 chooses prerequisite actions, 70 owns final artifact support/qualification. No Engine pin/platform expansion. |
| S7-E | `ci.yml` current-tree Gitleaks is report-only; `release.yml` history advisory is independent; dispatch-only `gitleaks-history.yml` exists. `dev/design/ci-cd-simplified-redesign-20260821.md` records historical100/121 mismatch; this is not a fresh scan result. | Keep existing separation. Classify latest available safe reports as current finding/history mismatch/tool failure; route actual remediation to security owner. Do not claim advisory red blocks publish, widen ignores, expose secrets or repeat an already-landed redesign. |
| S7-F | `dev/design/release.md` records earlier SDK registry bootstrap/trusted-publisher repair; release helpers already wait for exact crate/PyPI/npm versions. `release.yml` dry-run skips actual PyPI/post-publish paths and v-tags publish for real. | Classify auth/first-publication, resolution, propagation and installed smoke separately. Reuse bounded exact-version wait/idempotent recovery where relevant; no fixed sleep, token work, dispatch or publication. External Engine release owner; Inspector remains `publish=false` and local archive, so registry work is not automatically an Inspector prerequisite. |
| S7-G | `agent-verify.sh` separates Markdown and fast/heavy/full tiers; security AC-036 needs ptrace; AC-037's live exemption is explicit. Port left four native/allocation/scale cases unrun. | Preserve strict semantics and distinguish executor failure, ordinary pass and explicit pending case. Consolidate real blocker ownership with5/20/70; no broad gate reruns, quarantine or unsupported positive substitution. |

Review additional candidate logs only when they explain a concrete release delay:
workflow syntax, artifact upload/download, stale generated views, cancellation,
job timeout, native linkage or mismatched build/fixture pair. Stop expanding when
new evidence no longer changes a remedy or ownership. Do not inventory all CI jobs.

## Local requirements, acceptance and design assessment

Read [needs](../../../needs.md), [requirements](../../../requirements.md),
[acceptance](../../../acceptance.md), [HLD](../../../design/high-level-design.md)
and the existing single [verification matrix](../../0.1/verification-plan.md).
Keep additions local pending9; do not edit the canonical register in this slice.

| Canonical obligation | Local DRAFT treatment and observable check | Owning slice |
| --- | --- | --- |
| INS-X09; all INS native views | No new product need/AC required: installed default artifact already must exercise every positive view offline and close cleanly. Remedy proposal identifies exact build/OS/browser, artifact and remaining checks; build success alone is insufficient. | 70 final qualification; feature owners20–60 supply working views. |
| INS-X01/X07; F-05 | No new policy: fixture hooks remain absent from shipped artifact; offline/cache and paired-build requirements stay binding. Review package/test route coverage and identify missing proof explicitly. | 20 foundations; 40 model route; 70 installation. |
| INS-X03/X10; F-06 | No reduced budgets: list ignored/native/resource obligations still owed and their owner; proposed CI scoping must not make mandatory tests disappear from release qualification. | 5 adequacy; each feature; 70 integration. |
| Cross-cutting verification, no new canonical ID | Draft process remedy only: each delivery finding has source/run context, current versus historical state, delay evidence or unknown, minimum action, owner, effort/risk and a falsifiable validation approach. | 7 proposal; 9 decision; approved10 action. |

## Future execution and review

1. Refresh the baseline and evidence table. Read existing safe logs/review records;
   record exact commit/workflow/job and first actionable failure versus final exit
   when available. No hosted access is presumed; absent logs mean unknown, not green.
2. Trace blockers through local build → verifier → CI → archive → installed use,
   keeping registry publication as a separate external branch. Distinguish existing
   remedies from remaining defects. Assess the genuine architecture challenges in
   [design](design.md), revise only source-backed proposals, and ask the primary-source
   research questions there only if they resolve a live uncertainty.
3. Produce a compact finding/remedy table in these documents, not a second evidence
   database. Include include/postpone recommendation, rough effort/risk, dependencies
   and owner for9; no predetermined10 bundle or approval-by-silence.
4. Commission a fresh direct gpt-6.1-sol-high reviewer for substantive design changes;
   review requirements/acceptance fit and source correctness. A fresh Sol-high final
   deliverable reviewer checks findings, allocation and no-action bounds. These are
   prose reviews: no artificial RED/GREEN and no code implementation here.
5. Commission Terra to verify document links, source references, claim strength,
   current/historical distinctions and ownership. Run relevant Markdown checks and
   `git diff --check` when integrated; no full agent-verify for this prose-only slice.
   Passing document checks is not a runtime/full-workspace pass. Rerun only affected
   checks after specific corrections; existing final product gates remain binding.
6. On execution only, write a short status record with actual deliverables, checks,
   unresolved mandatory prerequisites and handoff to9. COMPLETE means assigned review
   is complete, not all recommended remedies implemented or the product qualified.
   Integrate/check and remove only slice-owned extra worktrees/branches if used;
   do not bypass hooks, force-push, tag, publish, or touch the primary checkout.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
through focused real-DB, fault/resource/cancel, independent lexical/numeric/adjacency
and browser/keyboard/human qualification already allocated to features/70. New
port evidence supports investigating environment and composition gaps rather than
rerunning semantic proofs here. Defer engine fuzzing/global MC/DC, exhaustive
allocation campaigns and new generic CI frameworks. Any later approved measurement
tool must be committed and indexed with reproducible parameters; raw logs may stay
in owned scratch. This slice creates no tool just to add paperwork.

## Future execution prompt

For this prework slice, “implementation” in the mandatory prompt means completing
the evidence review and proposal documents above. It does not authorize applying
any proposed change, bootstrap, test/build campaign, workflow run or release step.
Code TDD applies only to separately approved future changes, with a failing RED
run/commit per behavior, separate GREEN, unchanged tests as the oracle, real DBs
and no generated goldens; it does not turn this prose task into code work.

```text
Work on and complete Slice 7 for Inspector inspector/dev/0.2.0 (and in a worktree).

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

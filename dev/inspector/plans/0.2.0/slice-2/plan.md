---
title: Inspector 0.2.0 Slice 2 — cruft classification proposal
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: DRAFT
desc: Read-only repo-wide inventory and evidence-backed disposition proposals
---

# Slice 2 — repo-wide cruft review and proposal

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Goal, boundary and baseline

Prepare a reviewable classification of repository material as **keep, deprecate-in-place, archive or delete**, with reasons and exact dependencies, for the Slice 9 human decision. Slice 2 delivers an inventory and proposals; it applies none of the proposed cleanup. This DRAFT is not the completed inventory or an audit verdict.

Authority: [adopted scope](../scope.md), future [master plan](../plan.md), [shared workflow](../slice-workflow.md), and [draft design](design.md). Inspected source: `/home/coreyt/projects/fathomdb-worktrees/0.8.29-inspector-additional`, branch `llm/0.8.29-inspector-additional`, proposal base `829625818`; unchanged Engine base `6edc1155f`, packages 0.8.28/current format 37. Scope adoption subsequently landed at `609dd85c41`; execution must record its own actual revision and reconcile the delta.

Include program documents, development needs/requirements/AC, architecture, design, implementation, tests/fixtures/tools, plans/status/evidence, supporting/intermediate notes, and public docs. Inspect shared Engine/SDK/MCP source and policy only as read-only evidence. Repository-wide visibility does not commission Engine/shared-code cleanup, API changes, library updates or public pin changes.

Exclude removal, moves, archive application, source/test edits, environment setup, dependency sweeps, runtime qualification and public publication. Do not modify canonical acceptance registers or rewrite historical approvals, RED/GREEN, status or evidence to look current/complete. Historical staleness alone is no defect.

## Prerequisites and evidence to consume

1. A directly commissioned Slice 2 agent, sole writer in the release checkout created and entered by Slice 0 under the linked owner decision. This slice performs no checkout bootstrap.
2. Read final scope/master/workflow and Slice 0/1 outputs; check applicable AGENTS.md and indexed memory. Relevant inspected entries: `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`, `tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`; preserve their document-only/TDD distinction and reusable tools.
3. Re-read [Inspector ownership](../../../requirements.md#document-ownership), [HLD](../../../design/high-level-design.md), [legacy allocation](../../0.1/README.md), [verification matrix](../../0.1/verification-plan.md), historical Slice 15/16/20 statuses and [port evidence](../../0.1/compatibility-0.8.29-port.md).
4. Read [dev index](../../../../README.md), [document map](../../../../DOC-INDEX.md), [plans archive convention](../../../../plans/README.md), [design lifecycle rules](../../../../design/README.md), `dev/design/document-lifecycle.json`, [archive rules](../../../../archive/README.md), [notes ownership](../../../../notes/README.md) and the ADR decision index. Verify the exact current release selector rather than inferring currency from filenames.
5. Coordinate the evidence key/ownership convention with Slice 6 before stale-document review, and with Slices 3–5 for requirements, architecture and test-analysis overlaps. Do not wait for feature implementation to classify clearly historical material.

## Local DRAFT needs, requirements, acceptance and design allocation

These are slice-local review obligations, not new canonical product IDs or a second acceptance register. No product need or behavior change is presently justified by this planning inspection. Canonical [needs](../../../needs.md), [requirements](../../../requirements.md) and [acceptance](../../../acceptance.md) remain authoritative.

| Local draft allocation | Existing affected authority | Observable completion check | Owner |
| --- | --- | --- | --- |
| D2-N: contributors can locate current authority and preserved history | UN-08/UN-09 are related user outcomes, not new cleanup requirements; Inspector document-ownership section | Every proposed disposition names current owner/successor or explicitly records none/unknown; a redirect is not dismissed as duplicate content. | 2 proposal; 3 reconciles canonical allocation; 9 decides |
| D2-R: classify the entire defined corpus with evidence and reference impact | INS-01–09 and all INS/V01/X/F/T obligations must survive; Engine global registers remain locked | Coverage table accounts for each tracked path exactly once, with group inheritance only when safe; source/test changes remain proposals and Engine/shared changes identify external ownership. | 2 |
| D2-A: preserve reproducibility and truthful status | INS-X04/09, F-01–F-07, V01 and T/human evidence are protected inputs, not passes earned here | No historical status, approved ADR, RED/GREEN record, fixture or required measurement tool is proposed for deletion merely due to age, ignored status or absent execution. | 2; 5 assesses test adequacy |
| D2-D: one evidence source for overlapping stale-content/disposition findings | HLD ownership and current-versus-planned distinction | Slice 6 and Slice 2 link one finding key with separate content remedy/disposition fields; no conflicting copies or duplicate work estimates. | 2 + 6; 9/10 consume |

Future review may identify a genuine canonical change. Keep its exact proposed wording and affected IDs local, route to Slice 3/9, and leave global `dev/acceptance.md` unchanged. Never invent an AC to justify cleanup.

## Reconcile before starting

Enumerate changes since this draft in scope/plans, source and feature allocation, lifecycle catalogs/indices, referenced evidence, and approved retention decisions. Evaluate each as **keep / adjust / reject / already done**, with reason and current source citation. Revise the draft once as needed; do not reproduce already-adopted scope repairs or treat a preexisting prune as approval for another.

Confirm what exists versus planned: the inspected private worker enum accepts Catalog/Check/Page/Sample/Detail/Chunk/Tree/Associations/Export/Close; text/vector/interactive graph remain future work. `fixture-control` operations are test-gated, not a public mutation API. Their existence or limited use cannot establish dead code. This slice neither qualifies public retrieval routes nor proposes copying rankers or inferring provenance.

## Future review and proposal steps

1. Build a lightweight exhaustive inventory from tracked paths at the recorded revision, covering root/config/CI/scripts, `dev/`, `docs/`, `src/`, examples, experiments, fixtures/data and licenses. Reconcile tracked metadata with ignored/untracked/local artifacts only to record ownership/exclusions; do not traverse external symlink targets, inspect secrets, or assume local scratch is disposable.
2. Partition into small coherent groups by product/subsystem and authority/lifecycle, with path-level exceptions. Record counts, exclusions and unreadable/missing paths. Group keep rows are acceptable; any non-keep candidate needs exact paths and individual evidence. No uncovered group may be silently marked reviewed.
3. Read authoritative owner/index/status, actual content and relevant consumers. Collect inbound Markdown anchors, generated navigation, code read/write paths, Cargo/module/test discovery, scripts/CI/package manifests and external-link uncertainty. Search basenames, old paths and known aliases; inspect hits to distinguish readers from output declarations. No-reference search alone proves neither dead code nor safe deletion.
4. For each non-keep candidate, identify successor and retained unique value, active use, reproducibility inputs, legal/license/source attribution, ownership and retention constraints. Unknown ownership, legal duty, external use or required evidence blocks destructive recommendation; propose keep pending a named decision. Do not invent a legal hold or retention period.
5. Apply the decision rules in design.md. Assess risk, user/contributor confusion, maintenance value, effort and exact reference/index/catalog edits needed if later approved. Separate Inspector-owned improvements from Engine/shared/program work needing another commission. Do not turn list size or bytes saved into a priority target.
6. Share stale-content evidence with Slice 6 once. Let it assess users, consuming developers and contributors and propose factual text remedies; Slice 2 owns keep/deprecate/archive/delete and relocation safety. Slices 3/4/5 own scope/design/test adequacy decisions, not this inventory.
7. Deliver one concise `proposal.md` with coverage and candidate tables; add a compact machine-readable inventory only if path volume requires it. Include command patterns/revision for reproducibility, not full logs, giant receipts or a new catalog authority. A new executable inventory/measurement tool requires separate approval and normal TDD; do not build one for ceremony.
8. Send decision-ready rows to Slice 9, including understood/risk/effort/include-versus-postpone, unresolved prerequisites, ownership, conflict with existing rules and any proposed wording/targets. Slice 9 consolidates 0–8, holds the actual human gate, and refines conditional 10 with at most four FIX-n cycles. No timeout/default option is approval. Slice 10 implements only individually approved rows; an approval of Inspector scope is not approval to delete files.

## Representative inspected candidates — prompts for review, not findings

| Concrete evidence surface | Initial review hypothesis and protection |
| --- | --- |
| `dev/inspector/needs-requirements-acceptance.md` | **Keep** likely: explicitly preserves earlier links and delegates to separate canonical owners. A short redirect has purpose. |
| `dev/inspector/plans/plan-0.1.md`, `plans/0.1/slice-{15,16,20}/status.md`, old 30/40/50/60 drafts | **Keep historical in place** likely. 15/16 completion and ACTIVE 20 retain historical meaning; unfinished duties carry to 0.2.0. Decide present-day navigation/authority labels separately. |
| `dev/inspector/design/README.md`, `reviews/README.md`, `requirements.md` execution-allocation prose | Inspect legacy 0.1 link/target wording against new 0.2.0 ladder. **Deprecate-in-place or keep with a proposed current pointer**, if needed; Slice 6 owns factual stale-prose finding. No historical status edits. |
| `dev/architecture.md`, `dev/design/document-lifecycle.json`, accepted/superseded ADRs | Superseded architecture already points to v2; **keep** provenance and catalog class/owner. Propose future lifecycle repair only with evidence; no new authority or blanket move to archive. |
| `dev/notes/`, research/intermediate notes, `dev/archive/README.md` | Evaluate active citations, unique rationale and attribution individually. **Archive** is only a candidate for retired non-run material after reference/retention checks; no actual archive target is selected now. |
| `dev/plans/runs/` raw outputs and `src/inspector/tools/`, `examples/`, tests/fixtures | Keep required tools/oracles/read inputs. Archive README records two mistakenly pruned `runs/*` inputs restored because tracked Python READS them. **Delete** requires exact redundant/disposable evidence; no current file is declared deletable here. |
| `docs/inspector/`, Engine public docs, MkDocs navigation | Current Inspector index already distinguishes adopted 0.2.0 from unpublished 0.1.0. Review individual claims/duplicate navigation with Slice 6; no assumption all future-facing prose is cruft. Engine docs remain external ownership. |

The future exhaustive inventory must cover code/tests as well as prose. Static suspicion, test-only access, absence from one feature path and ignored long cases cannot justify removal. Public callable, private implementation and test-only consumers must be distinguished explicitly.

## Design challenges, research and quality fit

The main challenge is lifecycle and ownership, not a new application architecture: age, duplication and supersession are different from disposability. The existing design catalog supports `maintained/reference/experiment/historical/proposal/deferred/superseded`; the proposal must map to it without inventing schema values. Plans intentionally archive in place. The [archive](../../../../archive/README.md) supplies a concrete read-input hazard; the Inspector redirect supplies an intentional duplication example.

Research only if execution uncovers a concrete question: which official MkDocs/Git/package-manager rule makes a reference dynamic, and what upstream license/fixture distribution terms require retained attribution or source? Consult primary project/license documentation for that specific uncertainty during execution, record unresolved ownership for HITL, and avoid a broad web sweep. No external research or legal conclusion is performed by this draft.

Apply [quality-roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) emphasis on durable contracts, independent oracles and replayable regression tools by protecting them from uninformed pruning. Working format-37 queued-scope regressions and unrun long/native cases increase the value of accurate evidence retention; they are not audit passes. Runtime/fault/property/browser/human qualification stays with feature owners and Slice 70. Defer global coverage/MC/DC/fuzz campaigns and cleanup benchmarks; Slice 0 owns formal environment and CPU-only CUDA-policy review, with no driver/support change here.

## Future review, scoped verification, status and cleanup

- Review requirements/allocation and the actual classification design with a directly spawned fresh `gpt-6.1-sol` high reviewer. Review coherent proposal changes once; any proposed product/source remedy remains unimplemented. A separate code review or artificial RED/GREEN is unnecessary for this prose-only deliverable.
- Have directly commissioned Terra verify inventory coverage arithmetic, a meaningful sample from each corpus, **every archive/delete row**, inbound-reference evidence and historical/ownership safeguards. Mandatory missing coverage is unresolved, not an unavailable pass; retention ambiguity may be closed as explicit keep-pending-decision.
- Run `./scripts/agent-lint-md.sh` and `git diff --check` for delivered Markdown; inspect links/anchors and output scope. No public docs edit is planned; if separately authorized later, use `./scripts/agent-lint-docs.sh`. Use existing catalog/view checks only if applicable metadata changes are approved, never hand-edit generated release regions. Do not run runtime/full-workspace tests solely for this proposal.
- Run only scoped checks again after specific corrections. Report preexisting failures and their exact scope; do not repair unrelated gates, borrow another checkout's `.venv`, install tools or claim runtime green. Future approved source/test/executable changes in 10 require per-behavior failing RED commit, separate GREEN, real databases/no generated goldens or mocks, fresh Sol-high code review and Terra verification plus required final gates.
- Create concise `status.md` only during execution: actual revision, deliverables, coverage, reviews/checks and remaining decisions. COMPLETE means the assigned proposal and mandatory checks are complete, never that cleanup or feature qualification happened. No execution status is created by this draft.
- Each future slice uses its own directly spawned agent, sequential sole writer in the selected report worktree. Extra isolation only when concurrency/necessity warrants it. Preserve that checkout and unrelated work; integrate and clean only slice-owned extra worktrees/branches after verified integration. Do not execute on primary/main, bypass hooks, force-push, tag or publish. Proposed deletion is not scratch cleanup authorization.

## Future execution prompt

For this prework slice, “implementation” below means completing the inventory, reviewed classification proposal, scoped document checks and status. It never means applying cleanup, deprecation, archive, source or environment proposals; those require individual Slice 9 approval and Slice 10 commission. Prose-only work has no artificial RED/GREEN or runtime-green claim. The specific scope above overrides the generic final sentence.

```text
Work on and complete Slice 2 for Inspector inspector/dev/0.2.0 (and in a worktree).

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

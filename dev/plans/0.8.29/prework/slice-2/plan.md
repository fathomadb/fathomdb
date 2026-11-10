---
title: 0.8.29 Slice 2 — repository cruft disposition proposals
status: PROPOSED
target_release: 0.8.29
slice: 2
---

# Slice 2 — repository cruft disposition proposals

## Goal, maturity and boundary

Produce an evidence-backed keep/deprecate-in-place/archive/delete register for Slice 9.
This is proposal work: no cleanup, Git mutation, contract changes, source/test fixes or ledger writes.
Baseline inspected: `6edc1155f039f87cd0a37660cb27ae4373bbce71`, branch `llm/0.8.29-chat`, schema constant 37.
Repository paths and representative content were inspected; a complete file-by-file census remains slice work.
The [brief](../../planning-brief.md), [reconciliation proposal](../../reconciliation-plan.md) and
[release plan](../../../plan-0.8.29.md) control scope; existing parent edits are not cleanup candidates.
Preserve implemented MCP, queue, Block 1, B2-2/3 and partial B2-6 signals regardless of branch ancestry.

## Current checkpoint disposition

November 10 owner ruling: **keep the query scaffold for now**. This supersedes
the conditional deletion candidate in the original draft below. No query-test
retirement or associated instruction cleanup is selected for Slice10.

## Sources and slice preparation

Read [agent rules](../../../../../AGENTS.md), [engineering index](../../../../README.md),
[documentation map](../../../../DOC-INDEX.md), [design lifecycle](../../../../design/document-lifecycle.json),
[ADR index](../../../../adr/ADR-0.6.0-decision-index.md) and [archive precedent](../../../../archive/README.md).
Inspect [needs](../../../../needs.md), [requirements](../../../../requirements.md),
[acceptance](../../../../acceptance.md), [traceability](../../../../traceability.md), maintained interfaces,
[architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md), code/tests and public navigation.
Memory read: `agent-harness-extracted-to-agent-aware.md`, `keep-measurement-tools-in-repo.md`,
`competitor-benchmark-untracked-register.md`, `acceptance-md-locked-no-feature-acs.md`,
`verify-design-against-code-not-just-architecture.md`, `disk-cleanup-check-live-scratch.md`.
These entries exist in the supplied external memory store; verify present facts rather than replay old status.
Draft `needs.md`: N29-S2-01 find current authority; N29-S2-02 preserve reproducible evidence and consumer contracts.
Draft `requirements.md`: R29-S2-01 classify every inventoried cohort; R29-S2-02 bind proposals to exact paths/evidence;
R29-S2-03 record consumers, authority, risk, owner, successor/recovery, execution slice and reversibility.
Draft `acceptance.md`: A29-S2-01 complete category coverage; A29-S2-02 auditable high-risk dispositions;
A29-S2-03 no unauthorized mutations. These are release-local proposal IDs, never new canonical AC numbers.
Draft `design.md`: classify artifact → inspect content/consumers → retain provenance → propose disposition → Slice 9 ruling.
Architecture reading must distinguish historical snapshots, maintained authority, executable dependencies and publication inputs.
Hard aspects: historical imperatives resembling current instructions, generated pages, tracked fixtures and ignored evidence.
Primary-source research is local: Git tracked/ignored inventories, read-vs-write consumers, manifests, lifecycle catalog and nav.
No external freshness claims or network research are required for this inventory.

## Initial concrete candidate register

Each row is a proposal seed; expand it into exact-file entries with evidence and confidence before Slice 9.
Keep means maintained or retained as identified; deprecate-in-place means qualify authority without relocation.
Archive means preserve bytes/provenance with an explicit historical classification; moves require a reviewed destination map.
Delete requires affirmative nonfunctional/redundant evidence and approval, not age, naming or missing merge ancestry.

| ID / area | Observed evidence | Proposed disposition / allocation |
| --- | --- | --- |
| C29-02-01 program/docs | `dev/DOC-INDEX.md` still calls .28 plan/state/board active with work remaining; .28 is published in the brief. | Deprecate stale status prose in place; Slice 6 verifies wording, selected correction in 10. Preserve .28 records. |
| C29-02-02 product needs/requirements/AC | `dev/{needs,requirements}.md` retain locked IDs and .28 format36 applicability; source schema is 37; `dev/acceptance.md` is locked. | Keep traceability and thresholds; send applicability delta to Slice 3. No deletion or canonical edits here. |
| C29-02-03 architecture | `dev/architecture.md` and architecture v1 already say SUPERSEDED with a v2 successor. | Keep in place as reviewed history; no repeat supersession work or path churn. |
| C29-02-04 design | `dev/design/document-lifecycle.json` already distinguishes maintained, proposal and historical documents; agent-access work list still describes original defects. | Keep catalog and accepted decisions; propose bounded as-built annotations through Slices 4/6, retain original design/evaluation evidence. |
| C29-02-05 root recovery plans | `plan-slice-85-recovery.md` says IN_PROGRESS/qualification blocked; review plan says COMPLETE; .27 plan and recovery receipt cite the root plan. | Deprecate execution-state claims in place against .27 final status; keep owner authorization, SHAs and paths. |
| C29-02-06 root release note | `0.8.23-release-todo.md` is already historical but its banner points to .25 as active. | Keep historical content; qualify only the active-authority pointer through Slice 6/10. |
| C29-02-07 root question note | `CORPUS-01-supersession-questions.txt` asks for confirmation; committed 2026-08-23 human-review result records both cases insufficient. | Archive candidate with explicit closed-question status and link to retained result; retain unique context, no renewed adjudication. |
| C29-02-08 notes | `dev/notes/perf-canonical-runner-2026-MM.md` contains real May measurements and .6.1 consumers; `phase9-handoff.md` is dated .6.0 guidance. | Archive-in-place/classify historical; keep measurements and old paths. A placeholder-looking filename does not prove empty content. |
| C29-02-09 query test | `fathomdb-query/tests/property_template.rs` only asserts `s.clone() == s`; `src/lib.rs` has real token/literal properties. | Delete candidate for 10 if approved: demonstrate retained named properties, check target references and update applicable instruction citations. |
| C29-02-10 other tests | Engine/schema/Python `property_template` files contain durability/registry invariants; Python `test_scaffold.py` checks cursor advancement. | Keep; do not generalize the query finding by filename. Slice 5 evaluates adequacy, not this inventory. |
| C29-02-11 source warning scan | MCP hints fields are accepted uniformly; `ToolResult` is used for output schema, with explicit dead-code rationale. | Keep public/schema surface; send any actual warning residual to Slice 20. No source removal by warning grep. |
| C29-02-12 developer test directories | `test/` and four child READMEs describe shared assets; executable suites also live under `tests/` and package roots. | Keep pending consumer/owner census; propose one accurate routing explanation, not merging/deleting test trees. |
| C29-02-13 library-consumer docs | Query README says all sub-three-character queries fall back although code retains short content tokens; Python README says no network call while optional weights may download. | Deprecate inaccurate sentences in place via Slice 6 and approved docs preparation; retain package README publication inputs. |
| C29-02-14 public/contributor docs | MkDocs links worktree consolidation as Operations; page describes a real preservation-first operator tool; generated contributor map names its generator. | Keep tool/page; propose contributor-navigation fit review in 6. Keep generated pages, regenerate from source only in approved phase. |
| C29-02-15 experiments/program evidence | `dev/performance-benchmarking/README.md` now indexes committed results; historical memory called this untracked. Measurements/tools remain evidence. | Keep register, tools, minimized regressions, failed/rejected receipts and available ignored payloads; inventory availability separately. No bulk prune. |
| C29-02-16 refs/workspaces | `exp/agent-access*`, `feat/aa-b1-*`, scale experiments, `dev/0.8.30`, backup/recovery refs and many linked worktrees exist. | Keep/preserve; record owners and integration disposition only. No ref/worktree deletion, checkout, or automatic merge. |

The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) supports retaining meaningful
boundary/reopen properties and minimized seeds, and consolidating reusable tools; defer broad harness consolidation.
New evidence revises the old scaffold diagnosis: three templates now hold real properties; only the query template is tautological.
The [CUDA review contract](../../../../design/cuda-version-support-review.md) is maintained policy, not cruft;
leave fresh usage collection and policy/artifact proposals with their owning slices.

## Work breakdown and deliverables

1. Capture tracked, untracked and ignored path cohorts without reading secrets or copying datasets into planning docs.
2. Expand the register across program/product/dev/design/notes/source/tests/consumer/developer/public audiences; state excluded cohorts.
3. For each changed/disposable candidate, inspect inbound links, literal paths, test discovery, generators, manifests and runtime reads.
4. Separate completed history, obsolete authority, actual unused behavior and unavailable evidence; name competing owners explicitly.
5. Write `inventory.md`, `dispositions.md`, `needs.md`, `requirements.md`, `acceptance.md`, `design.md`, `research.md`, `test-mapping.md`.
6. Send scored proposals and newly discovered prerequisites to 9; send semantic truth findings to 3–6 and tooling findings to 7.
7. Review fit against as-built .29; record proposed revisions and unresolved evidence, without claiming reviewer/HITL acceptance.

## Dependencies, conflicts and new prerequisites

Hard: supplied baseline/brief and accessible source; Slice 9 consumes this proposal; execution requires recorded Slice 9 approval then 10.
No hard implementation dependency on 0/1 or 20–150; exchange findings with 3–7 while prework research proceeds in parallel.
Preferred: classify first, let 3/4 settle authority and 5 assess tests, then let 6 own audience-specific wording before scoring in 9.
Single-writer conflicts: DOC-INDEX/detail maps, design lifecycle catalog, interfaces, MkDocs nav and generated public pages.
New prerequisite NP-S2-01: a consumer/read-input proof per proposed deletion or move; archive precedent records two pruned JSON inputs later restored.
New prerequisite NP-S2-02: reconcile the .27 root recovery records to final evidence before changing their current-status qualification.
New prerequisite NP-S2-03: verify query test discovery, retained property names and AGENTS citations before approving its deletion.
Conditional: any later ref retirement needs separately authorized preservation/owner-map evidence; this slice creates no retirement manifest.
Do not touch shared caches, symlinks, live scratch or worktrees; environments and installations belong to approved Slice 10 preparation.

## Validation, independent review and completion signals

A29-S2-01 passes only if every requested audience/category has a named cohort, sampled evidence, proposed owner and explicit coverage limit.
A29-S2-02 passes only if every delete/move candidate has exact paths, consumers, retention/recovery, risks and a decision dependency;
any unresolved consumer forces keep/pending, with a concrete investigation rather than a guessed safe deletion.
A29-S2-03 passes only if checkout/Git/environment/ledger mutations attributable to this draft are absent; the isolated draft is the only write.
Independently review the register adversarially for lost evidence, generated inputs, public contracts and already-completed cleanup.
Proposal completion requires local N→R→A→artifact/review mapping, no unresolved high-risk disposition presented as approved, and Slice 9 intake.
After parent import, run scoped Markdown validation and link checks; no product test execution or workspace-green claim is required here.
Later approved behavior/script changes require visible failing tests before fixes and the full source gate; docs use scoped validators.
For the no-op test deletion, preserve substantive property assertions and demonstrate retained discovery; do not invent a failing product test.
Stop for owner decisions on destructive scope, canonical authority changes, evidence loss or unresolved semantics; record options for Slice 9.
Open: whether the query deletion, root-note archival and contributor-nav change justify maintenance cost; no ruling is presumed.

---
title: Inspector 0.2.0 Slice 4 — architecture and source alignment review
date: 2026-11-19
status: DRAFT
target_release: Inspector 0.2.0
---

# Slice 4 — architecture and source alignment proposals

Current disposition: owner S9-010, applied in [Slice 10](../slice-10/status.md),
qualifies supported single head windows in [0.2.0 scope](../scope.md) and defers
within-source multiple-window positive. Earlier findings/draft proposals below
retain their historical meaning; cache-only CPU Engine serving remains mandatory
and unqualified in 40.

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Goal and authority

Review whether the architecture, capability claims and very high-level code organization can deliver the [adopted scope](../scope.md) on the unchanged selected Engine. Produce source-grounded proposals and explicit prerequisites, not architecture or application changes. This is prework for Slice 9's actual human decision, not permission to implement recommendations.

Draft inspected baseline: adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`, Inspector port `51b4e7c1fb4ce4b63adf4be0a40f1952eafce4ab`, Engine ancestry `6edc1155f039f87cd0a37660cb27ae4373bbce71`, packages 0.8.28/current format 37. Inspector remains unpublished package 0.1.0. The actual worker implements record browsing, not lexical queries, vector neighborhoods or an interactive graph. Historical 0.1 Slice 20 remains ACTIVE; its checks and missing checkout-owned `.venv` full-gate stop are not a release-green claim.

Use the [master mapping](../plan.md), [shared workflow](../slice-workflow.md), [canonical needs](../../../needs.md), [requirements](../../../requirements.md), [acceptance](../../../acceptance.md) and [HLD](../../../design/high-level-design.md). Old 0.1 feature designs and matrix owner numbers are evidence inputs: current feature owners are 20 record foundation, 30 lexical, 40 vectors/models, 50 graph/history, 60 comparison, 70 installed/integrated qualification.

## Bounds and prerequisites

- Inputs: Slice 0 environment/composition proposal; Slice 1 dependency/pin assessment; Slice 2 cruft classifications; Slice 3 local DRAFT needs/requirements/AC and architecture allocation. Consume their actual outputs before finalizing this review; do not infer agreement from their existence.
- In: component responsibilities, capability-to-source agreement, current versus proposed behavior, minimal module seams, ownership/consistency/security/resource risks and concrete downstream allocations. Read-only inspection may follow a dependency into Engine/SDK/MCP.
- Out: exhaustive code audit, dependency sweeps, cleanup/refactor, new protocols or abstraction frameworks, code/config/dependency changes, runtime tests and changes to canonical scope/acceptance/HLD. No Engine/SDK/MCP/schema/ranker/API edits, public pin upgrades or waivers of mandatory positives.
- Drafting these files performs no execution. During future execution the commissioned direct Slice 4 agent is sole writer in the selected report worktree, used sequentially; never execute in the primary checkout. Its base must include the approved final planning-branch HEAD, not raw `main`/`dev/0.8.29`.
- Slice 0 creates and enters the release checkout under the linked owner ruling. Slice 4 creates no checkout or environment; an unavailable prerequisite is named, not repaired here.

## Preparation at execution

1. Read repository instructions and relevant real memory entries, especially `dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`, `tdd-red-must-fail-per-finding.md` and `keep-measurement-tools-in-repo.md`.
2. Enumerate changes since this draft in the approved plans, canonical scope/INS/M40, Slice 0–3 outputs, Inspector source/features, exact dependency source and allocated draft items. Compare against the actual target branch/worktree and record only changes affecting conclusions.
3. Evaluate each item as keep, adjust, reject or already done, with source/decision evidence and reason. Revise this plan and its local design before the substantive review; complete the assigned review without overbuilding it.
4. Read the quality roadmap and accepted decisions relevant to the changed seams. Resolve contradictory input as a local proposal for Slice 9; do not silently rewrite canonical requirements or accepted ADRs.

## Evidence inputs and review method

Use the [local design matrix](design.md#source-agreement-matrix) as the starting map, not a finished audit. Cite exact source paths/symbols at the executed baseline. For each capability record: user outcome/IDs → component owner → production callable or named storage route → production/private/test-only classification → what exists → missing Inspector work or dependency prerequisite → owning slice and observable check. Source existence never establishes runtime qualification.

Read only the relevant bodies and declarations: `src/inspector/src/{worker,session,process,server,admission,storage,policy,window}.rs`, `src/inspector/assets/app.js`, `src/inspector/Cargo.toml`; Engine `src/{open,search_api,search_types,graph_api,read_api,vector_storage,dependency_trace}.rs`; embedder `src/{loader,candle_bge}.rs`; current schema and `dev/interfaces/rust.md`. Expand a seam only to resolve a concrete uncertainty, not into a repository-wide code review.

Review [transactional visibility coalescing](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md), [vector admission](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md), [explicit erasure maintenance](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md), [frozen reads](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md), [frozen pagination](../../../../adr/ADR-0.8.25-frozen-pagination-and-operational-state.md), [compact evidence](../../../../adr/ADR-0.8.25-compact-source-evidence.md) and [embedder identity](../../../../adr/ADR-0.6.0-vector-identity-embedder-owned.md). No accepted mechanism is replaced by an attractive but incompatible design.

## Local DRAFT needs, requirements, acceptance and design items

These are review findings/proposals, not a second acceptance register. Slice 3 reconciles any overlap; Slice 9 decides their disposition. No new user need is currently evidenced: UN-06–10 and UN-12–16 already cover the outcomes.

| Local item | Affected canonical contract | Proposed clarification / observable review check | Proposed owner |
| --- | --- | --- | --- |
| D4-01 — capability truth | INS-04.3, INS-08.4, V01-S02, M12/M22/M36 | Reconcile HLD's historical absent-snippet/sibling claims with actual public snippet, degradation and queue APIs; retain a route-specific adoption/rejection rationale and exact evidence limits. | 30/40; common capability foundation 20 |
| D4-02 — cache-only admission prerequisite | V01-N07, INS-X01, M30/M36 | Production `EmbedderChoice` is Default/None; HLD's helper-to-Caller route is not callable in a normal product. Seek an unchanged supported cache-only/public query route, then require a reviewed no-fetch/no-repair path and real positive CPU query before claiming semantic/hybrid readiness. The missing selected seam is a feasibility risk, not proof that every alternative is impossible. | 9 decision; 40 resolves prerequisite |
| D4-03 — vector entry/cardinality | INS-04.2, INS-05.6, INS-08.4, INS-09.3, M17/M23/M26 | Model stored vector row/profile/window separately from canonical origin; check multiple windows, exact source ranges and zero/one/many origins. Distinguish bounded stored L2 from serving-stage/fused values. | 20 reference seam; 40 feature |
| D4-04 — complete stored graph | INS-03.3, INS-07, INS-09.4, M08/M18/M19/M27 | A serving neighbor result returning nodes does not provide complete stored edge payloads/history. Map directed adjacency and exact edge detail separately from serving traversal/evidence and class-correct lifecycle. | 50 |
| D4-05 — policy/read freshness | V01-N01, INS-X04/X06/X11, M20/M39 | Preserve transaction-local bounded queued-scope membership and clean coalescing admission; withdrawal/requeue must be reclassified on the next read even without a convenient generation change. All alternate payload/evidence/pin paths obey independent gates. | 20 common; 30/40/50/60 extensions |
| D4-06 — two observations | INS-05.7–05.10, INS-X04/X10, M40 | Existing observation cache is not two-pin comparison. Retain each actual query/seed/options/filter/limit/population/context, typed identity, actual rank and own metric; no false common snapshot or fabricated graph rank. Reconcile cache invalidation with safe pinned metadata and D-01 withdrawal. | 20 foundation; 60 comparison |
| D4-07 — owned execution | INS-X02/X03/X10, V01-S02, M31/M36 | Existing kill/reap/late-response guard is not a general Engine interruption API. Name request connections, Engine child, model helper, scratch, queue/cancel boundaries and aggregate accounting; no abandoned native work claim. | 20/30/40/50/60 locally; 70 integrated |
| D4-08 — minimal code alignment | INS-09, all cross-cutting contracts | Propose ordinary injected concrete collaborators and only necessary typed modules; demonstrate an actual responsibility conflict before extraction. No plugin host, generic backend registry or parallel private ranker. | feature owner; 10 only if 9 approves prework |

## Review deliverable and allocation

1. Compare Slice 0–3 proposals with HLD components A/S/Q/P/O/R/U/K. Identify agreements, unresolved contradictions, duplicated responsibilities and missing prerequisites; do not re-run their whole reviews.
2. Produce the source agreement matrix and a concise proposed HLD change list (create/update/delete or retain sections, not edits applied to HLD). Separate implementation gaps from inaccurate architectural claims and from unqualified existing behavior.
3. Review high-level code alignment: worker allowlist/dispatch, session/connection ownership, reader/policy/reference coupling, server queue/lifecycle, browser observation/history coupling and future feature seams. Propose the smallest module/DI changes justified by real use; do not prescribe every future file/function.
4. Attach each proposal to feature 20/30/40/50/60/70 by default. An independently useful prework implementation may be a candidate for conditional 10 only after Slice 9 individually approves it. No pre-decided cleanup/refactor bundle.
5. For a missing Engine capability, state the exact missing production contract, affected mandatory positive, alternatives rejected, external owner and evidence needed to unblock it. Escalate to Slice 9 for separate scope/dependency resolution. Neither unsupported UI state nor test-only injection completes that requirement; do not commission Engine work here.
6. Hand findings to Slice 5 for traceability/test adequacy and Slice 6 for stale-doc allocation, preserving current versus historical evidence. Slice 7 owns actual build/runtime/release checks; this report does not claim their results.

Deliver only the slice's review/proposal plan and design plus the concise execution status when actually executed. Keep rationale and source agreement together; avoid separate audit packets, hash inventories or per-helper receipts.

## Challenges and targeted investigation

- Cache-only compatible serving is the principal known feasibility conflict. Public standalone loaded-weight construction does not establish production Engine provider attachment; `Default` invokes the normal loader. Future 40 must first investigate unchanged cache-only/public query routes and actual loader behavior. Prove a supported route or carry a concrete unresolved prerequisite to 9/40; this planning finding is not an executed audit or proof that all alternatives fail.
- Public snippet APIs now exist but do not cover projected-text/BM25F identically; actual index highlights, sidecar caps and same-transaction context must be attributed by route. MCP transport/formatting/admission may be deliberately rejected without claiming it is absent.
- Background workers, clean coalescing, queued scope withdrawal and browser invalidation can change disclosure or context without changing canonical identity. Two pins cannot promise durable content or a common snapshot.
- Stored multiple-window vectors and complete directed edge payloads require named layout reads; neither Engine fused scores nor neighbor nodes substitute for those facts.
- Planned web research only if a remaining source question needs it: primary SQLite/rusqlite transaction/interrupt and sqlite-vec metric docs for owned-read semantics; official Candle constructor/device documentation for no-fetch construction; WAI-ARIA/Chromium docs for a concrete new interaction question. First inspect locked source/interfaces. Do not browse or conduct library/benchmark sweeps merely to complete a research section.

## Review, scoped verification and completion

- Commission a fresh direct `gpt-6.1-sol` high read-only design reviewer for this actual review/design proposal batch. Ask for source fidelity, accepted-ADR conflicts, missing mandatory positives, allocation and overbuilding; it must not implement the drafts. Address substantive findings coherently, not with ritual per-helper loops.
- Use a directly commissioned Terra verifier read-only for document/source checks where appropriate: IDs/paths, production versus feature-gated APIs, route/owner consistency, and omission/waiver detection. No runtime checks are warranted for this prose-only prework and no artificial RED/GREEN tests or code-review gate are manufactured.
- Verify links/IDs/claims and run the required scoped Markdown validator on execution changes; no full workspace gate solely for these reports. Check only impacted documents again after fixes. Required feature/release gates remain with their owners and no foreign Python environment is borrowed.
- Apply the quality roadmap proportionately: propose real DB fixtures, independent lexical/numeric/adjacency facts, property invariants, targeted fault/resource/cancel tests and browser/keyboard/human verification for future features. Defer unapproved Engine fuzzing/global MC/DC/allocation campaigns. New evidence is the working format-37 foundation and its per-read queued-scope cache correction; do not erase that lesson with a session-only cache.
- COMPLETE requires delivered review proposals, explicit unresolved prerequisites/owners and meaningful scoped checks. Missing mandatory product positives remain open for the feature/release owners, never an unavailable pass. Create a short truthful status record only during execution, including changes in disposition and remaining blockers.
- Preserve the selected report worktree and unrelated work. If additional isolation was necessary, integrate and clean only slice-owned extra branches/worktrees after verified integration; no hook bypass, force push, tag or publication. Keep any future reusable measurement tool in-repo with its command; this slice needs none.

## Future execution prompt

Slice 4 is review/proposal-only prework. In this prompt, “implementation” means producing and checking the assigned review documents, never applying their architecture, dependency, environment or code recommendations. TDD/code review apply only if later separately authorized code work exists; no artificial RED/GREEN for reports. Slice 9 decides recommendations and conditional Slice 10 scope. This specific no-action boundary overrides the generic final instruction below.

```text
Work on and complete Slice 4 for Inspector inspector/dev/0.2.0 (and in a worktree).

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

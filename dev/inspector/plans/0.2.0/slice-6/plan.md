---
title: Inspector 0.2.0 Slice 6 — stale documentation evidence review
status: DRAFT
date: 2026-11-19
target_release: 0.2.0
---

# Slice 6 draft plan

The [owner checkout decision](../slice-0/checkout-decision.md) requires future
Slice 0 to create and enter the release branch/worktree from the approved final
planning HEAD. Reuse that release checkout sequentially for 0–9 reports, with
one writer. Other environment/infrastructure changes remain proposals for
actual Slice 9 disposition and conditional Slice 10 implementation. Recording
this ruling and updating drafts performs no checkout creation or slice execution.

## Goal, authority and baseline

Produce an evidence-led proposal for stale documentation affecting Inspector users,
developers consuming libraries/APIs/SDKs, and contributors. Review evidence repo-wide;
proposed actions remain Inspector-scoped unless separately authorized. This slice
executes review and drafting only, never the proposed corrections or a whole-repo rewrite.

The [adopted scope](../scope.md) is authoritative; approval is complete, detailed
execution is not. Inspected planning checkout: `llm/0.8.29-inspector-additional`,
adoption `609dd85c41fb7980e05faa1132a248d376d8ec89`, unchanged Engine ancestry
`6edc1155f039f87cd0a37660cb27ae4373bbce71`, packages 0.8.28/current format 37.
Inspector is an unpublished 0.1.0 record browser; native text/vector/interactive graph
and integrated qualification remain work. This draft inspected source/docs only;
it ran no tests, fetched no web material and inspected no private transcript store.

Use the [master mapping](../plan.md) and [shared workflow](../slice-workflow.md)
when commissioned. Old numbered owners in `plans/0.1/` are historical until mapped:
new 60 owns two-observation comparison; new 70 owns final installed integration.
Do not equate historical ACTIVE Slice 20 with completion of new 20.

## Scope and prerequisites

- In: documentation discovery across `README*`, `docs/`, `dev/`, source READMEs,
  public API docstrings, examples and contributor/build instructions; current chat's
  authorized durable port, available review/status/build records and callable source.
- In: identify wrong, incomplete, ambiguous, intentionally historical and already
  corrected claims; propose exact replacement wording and a bounded implementation allocation.
- Out: modifying reviewed docs/source/contracts, deleting cruft, running examples,
  executing full audit remedies, dependency upgrades, CI/environment repairs, browsing
  private histories, external correspondence and new Engine/SDK/MCP requirements.
- Prerequisites: available approved planning evidence and the selected report checkout;
  available evidence authority; Slice 2 inventory and Slice 3/4 contract/architecture
  proposals for deduplication, Slice 7's actual build/CI evidence when available.
  Review may start before peers finish; reconcile their results before handoff to 9.
- Future release branch `inspector/dev/0.2.0` must preserve this planning branch's
  approved final HEAD and port/scope/plans, never start from raw main/dev/0.8.29.
  Slice 0 creates and enters that release checkout under the linked owner ruling;
  this slice reuses it and creates no checkout as a workaround.

## Evidence inspected for this draft

| Evidence | What it establishes; remaining limit |
| --- | --- |
| `src/inspector/src/worker.rs`, `src/inspector/Cargo.toml`, `src/inspector/README.md` | Actual Rust application and browser foundation; worker operations do not implement future search/graph views. Build commands are instructions, not executed results. |
| `docs/inspector/{index,getting-started/index,reference/index,operations/index,contributing/index}.md`, `dev/inspector/README.md` | Adoption already distinguishes source baseline, future capabilities and allowed Engine effects; use as already-correct controls, not automatic new findings. |
| `dev/inspector/plans/0.1/compatibility-0.8.29-port.md`, `dev/inspector/reviews/compatibility-0.8.29-port-review.md` | Recorded 39 then 41 ordinary Rust cases, ten browser cases at port, four original explicit long/native/allocation cases unrun; full gate stopped at missing checkout-owned `.venv`. Remediation does not add browser/install/full-gate qualification. |
| [HLD](../../../design/high-level-design.md), `src/rust/crates/fathomdb-engine/src/search_api.rs`, `dev/interfaces/rust.md` EP-17, `dev/interfaces/mcp.md` | HLD introduction qualifies historical S16 findings, but lexical table still says no sibling snippet API. Current public filtered/view/frozen snippet routes exist; projected-text/BM25F do not gain snippets by inference. MCP envelope has its own byte limits and semantics. |
| `src/inspector/src/session.rs`, [requirements](../../../requirements.md), `dev/interfaces/rust.md` admission contract | Request-owned read connections coexist with ordinary Engine ownership/effects; read-only UI does not prove immutable forensic attachment. |
| `dev/adr/ADR-0.6.0-decision-index.md`, `dev/adr/ADR-0.8.28-{transactional-visibility-coalescing,vector-compatibility-open-policy}.md` | Accepted decisions need source/implementation-time qualification. A historical accepted proposal mentioning removed Caller support versus Inspector's draft Caller route needs Slice 4/9 reconciliation, not a Slice 6 API change or automatic declaration of current absence. |

Read root `AGENTS.md`; applicable actual memory entries were
`dont-be-distracted-by-ceremony.md`, `verify-design-against-code-not-just-architecture.md`,
`tdd-red-must-fail-per-finding.md`, `keep-measurement-tools-in-repo.md`, indexed at
`/home/coreyt/.claude/projects/-home-coreyt-projects-fathomdb/memory/MEMORY.md`.
The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) guides
truthful independent-oracle, failure-model and resource claims. Apply claim-to-test
inspection here; preserve real-DB/numeric/adjacency/lexical/property/cancel/browser/
keyboard/human obligations for feature owners. Defer generic engine fuzzing, global
MC/DC and all-allocation campaigns; prose review creates no such gates. New evidence
is the working port, format-37 composition and real queue-budget regression.

## Future preparation and review procedure

1. Enumerate changes since this draft in scope/master plans, relevant source,
   interfaces, adopted feature behavior, available evidence and allocated peer items.
   Record each material delta as keep/adjust/reject/already-done with a reason and
   witness. Revise this plan/design once where needed; do not revive superseded scope.
2. Inventory the three audiences and document entry points. Search repo-wide with
   `rg`; inspect each flagged statement in context. Include public docstrings and
   consuming-developer examples, not only Inspector Markdown. Record searched roots,
   material exclusions, search terms and why coverage stops; no exhaustive claim from
   a few grep hits. No indiscriminate library or web sweep is commissioned.
3. Inventory authorized evidence: supplied/current chat content and its durable port,
   available transcripts explicitly accessible under current authority, review records,
   source contracts, build descriptions, test logs/status and release documents.
   Record origin/date/revision and access scope. Missing/truncated/unavailable chats
   become named evidence gaps; do not search hidden history stores or infer their content.
4. For each candidate, read the whole relevant section and identify the claim's
   audience, owner, tense, target version, source/revision and claimed qualification.
   Cross-check callable signatures, feature gates, schema and actual test/run evidence.
   Distinguish public-callable, private storage fact, test-only seam and unavailable
   route; source existence proves neither product use nor runtime qualification.
5. Classify: wrong now / misleading omission or ambiguity / intentionally historical /
   already corrected / unresolved evidence. Preserve dated status/review evidence;
   propose a current redirect/qualification when useful rather than rewriting history.
   A draft promise is not a shipped fact; a historical absence is not current absence.
6. Write one finding per actual semantic defect with exact existing excerpt/location,
   source/time-bounded witnesses, audience impact, proposed replacement text, action,
   severity rationale, owner and prerequisites. Never label a mandatory unverified
   positive as a pass or erase the obligation by wording it unsupported.
7. Reconcile stable keys with Slice 2 cruft, Slice 3 draft contracts and Slice 7 CI
   evidence. Use `doc:<repo-relative-path>#<heading>:<claim-topic>` plus aliases for
   renamed paths/peer keys; do not mint duplicate remediation tasks. Slice 2 owns
   removal/retention, 3 owns contract wording, 4 owns architecture feasibility, 7 owns
   gate evidence; Slice 6 supplies audience-facing implications and replacement prose.
8. Submit concise `findings.md` with inventory/gaps and proposed wording/actions,
   alongside updated slice-owned draft plan/design. Slice 9 scores/rules on proposals;
   Slice 10 implements only individually approved early corrections. Feature-owned
   usage wording lands with 20/30/40/50/60; final support/installed wording belongs to 70.
   Repo-external-owner findings remain explicit separately authorized referrals.

## Local DRAFT needs, requirements, acceptance and design proposals

These are proposals local to this slice, not a second canonical acceptance register.
Do not edit global `acceptance.md` or canonical Inspector needs/requirements/AC here.

| Local proposal / affected canonical IDs | Observable completion check | Owning allocation |
| --- | --- | --- |
| No new user need: truthful guidance already supports UN-01/08/09/14/15/16 and INS-01/04/06/09. | Each audience has inspected entry points or an explicit coverage gap; no invented transcript/user-demand evidence. | 6 review; 3 decides canonical wording need. |
| Clarify planned/current/historical status and capability evidence; INS-04/06, V01-N05/N06, V01-S02. | Every proposed present-tense route claim has current source/interface witness; snippet routes separated from projected-text/BM25F and from Inspector implementation. | 6 proposal; 4 feasibility; 30/40 implementation docs. |
| Preserve inspection-versus-Engine-effects boundary; UN-09, INS-09, V01-S02/S03, D-01. | Proposed wording permits startup/background/shutdown effects, preserves sidecars and independent gates, promises no immutable forensic mode or erasure reconstruction. | 6 proposal; 20/50 feature docs; 70 support. |
| Qualify verification scope; existing F/T and verification-matrix obligations, INS-07/08. | Run claims name build/date/command, ordinary versus ignored/native/install scope and missing `.venv`; required positives remain pending. | 7 run-evidence authority; 6 wording; 70 final support. |
| No product architecture change needed for this review. | [Draft design](design.md) is an evidence model and ownership proposal only; no runtime protocol/API/schema edit is introduced. | 6; substantive HLD correction proposals reviewed with 4/9. |

## Concrete verification and closure

A fresh directly spawned `gpt-6.1-sol-high` design reviewer checks substantive draft
architecture/wording changes against source, not just internal consistency. A fresh
direct Sol-high content/code reviewer checks the final report's claims and allocations;
a direct Terra verifier reopens witnesses, checks relative links and audience coverage.
These are future commissions, never nested Steward/Orchestrator roles or concurrent writers.

Verify each finding's excerpt exists at the recorded baseline; replacements resolve
only that finding; every current capability statement has source and contract support;
all qualification claims retain unrun/blocked caveats. Check duplicate keys/owners and
trace every allocation to master mapping. Confirm historical S15/16/20 records remain
historical, comparison stays 60 and integration stays 70. Review all evidence gaps.

Run `git diff --check` and `./scripts/agent-lint-md.sh` for commissioned report/draft
changes. Public-doc lint applies only if separately authorized public docs actually
change; no runtime tests/full source verification for report-only prose. Missing
mandatory evidence stays unresolved/blocked, never unavailable-pass. Rerun only failed
checks or affected checks after fixes; no artificial RED/GREEN for prose. Any later
approved behavior fix requires its own failing RED commit, separate GREEN, unchanged
oracle tests, real DBs and applicable code/final gates in its owning slice.

Write a short status only during execution: reviewed baseline/coverage, findings and
gaps, actual scoped checks, review disposition and handoff to 9. COMPLETE means this
assigned review and checks finished; it does not mean proposals were implemented or
Inspector qualified. Sole writer uses the selected report worktree sequentially;
preserve unrelated work/release checkout. Extra isolation only when necessary, integrate
and clean only slice-owned extra worktrees after verified integration. No hook bypass,
force push, tags, publishing or primary-checkout execution. No borrowed Python venv.

## Future execution prompt

This is prework: “implementation” below means performing this report/proposal slice.
The final sentence authorizes no proposed doc/source/environment/dependency change.
Draft corrections and canonical-contract changes await Slice 9 and individually approved
Slice 10 or feature-owner work; private evidence access needs existing authority.

```text
Work on and complete Slice 6 for Inspector inspector/dev/0.2.0 (and in a worktree).

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

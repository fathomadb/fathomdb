---
title: 0.8.29 initial proposal register and HITL decision log
status: PROPOSED
date: 2026-11-09
target_release: 0.8.29
---

# Initial proposal register and HITL decision log

These seed proposals come from the source/reconciliation audit and initial slice
drafts. Prework 0–8 is not declared complete. Slice9 revises scores using actual
outputs and presents coherent batches interactively. Workspace recovery/start and bounded preparation scope were authorized November
10 as recorded below; feature decisions and final preparation acceptance remain
pending. H/M/L understanding reflects evidence confidence; risk is impact;
S/M/L effort is comparative scope, not duration or cost.

| ID | Proposal / gap | Understood | Risk | Effort | Recommendation / slice | Hard input or uncertainty | HITL |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P01 | Reuse/establish owned .29 workspace, switch session, preserve other writers | H | M | S | Include /10 |Owner-authorized replacement attached here; environment/state work still needs9 | Workspace done; preparation pending |
| P02 | Owned venv/native/TS setup and source-pinned Node/npm alignment | H | H | M | Include /10 |0 observed Node26/npm12 versus25/npm11 pins; no owned venv/TS deps | Scope approved; final acceptance pending |
| P03 | Disk/scratch/GPU/runner availability budgets | M | H | M | Include /10 |Current replacement:16GiB free at99%; GPU query unavailable; historical host success is not readiness | Scope approved; final acceptance pending |
| P04 | Activate .29 state/board/views after .28 completion | H | M | S | Include /10 |9 decision; no live state today, dependent preflight cannot pass yet | Scope approved; final acceptance pending |
| P05 | Library/Dependabot/security sweep and selected updates | M | M | M | Include sweep; updates conditional /1→10 |Pin rationales, current advisory/compatibility evidence; no blind latest-version policy | Rustls/DOMPurify and survey scope approved; other updates unselected |
| P06 | Source/doc/branch cruft proposals and stale-document corrections | M | M | M | Include review; edits conditional /2,6→10 |Consumer/inbound-reference proof, history preserved, no blanket deletion | Bounded docs approved; query scaffold kept |
| P07 | Draft need/requirement/AC/architecture CRUD and source alignment | M | H | M | Include /3–5 |Exact gap/test mapping and settled contract authority | Bounded applicability/profile edits approved; feature semantics pending |
| P08 | Bounded CI/preflight/package/registry delay remediation | H | M | M | Include /7→10 |Already repaired .28 publisher/packaging issues are not new work | Bounded preparation scope approved |
| P09 | Queue race/parallel-test/warning/tripwire residuals | M | M | M | Include /20 |Reproduce after .28 fixes; preserve real tests/oracles | Gated scope approved /20 |
| P10 | Current scale, open, embedding and write-interleaved baseline | H | M | M | Include /30 |Stable10/20 harness and data/native identities | Gated scope approved /30; existing targets retained |
| P11 | Eligibility cache and batch-invariant metadata work | M | H | M | Conditional implementation /40 |30 benefit under writes; format37 snapshot/invalidation proof | Conditional scope approved;30 benefit and reviewed snapshot safety required |
| P12 | Fusion record identity, exact deferred text and snippet optimization | M | H | L | Include design; implementation after approval /50 |Identity ADR/census, reference algorithm, Unicode equivalence | Census/ADR scope approved; build waits for identity approval and30 |
| P13 | Compact text, MCP flags and filtered text/paging support | H | H | L | Include /60 |Accepted defaults/B2-8 retained; current provider and budget contracts | Remaining60 scope approved; settled default retained |
| P14 | Additional degradation labels and SDK/facade/native parity | H | M | M | Include /70 |Bounded reason scope; no wholesale EP03 approval | Bounded70 scope approved; exact reason/error semantics still reviewed |
| P15 | Unified residual B2 design and build | M | H | L | Include /80,90 |Built B2-2/3 and partial6 preserved; follow-up exit before90 | 80 design approved;90 requires accepted80 AND Block1 exit |
| P16 | Interrupted-erasure coverage reconciliation | M | H | M | Include bounded scope /100 |Existing .28 coverage comparison; no new SLA/availability effort | Pending |
| P17 | Safe lifetime/teardown after CUDA context reset | M | H | L | Include design; remedy conditional /110 |Permitted fault venue; possible qualified fork prerequisite | Pending |
| P18 | Required CUDA usage review and second CUDA13 artifact | M | H | L | Include /120 |Quantitative shares incomplete; compiler6111901 exposure/mitigation, delivery design and ratified policy | Pending |
| P19 | Tegra/non-Tegra CUDA platform, SDK close and soak evidence | M | H | L | Conditional units /130 |Venue approval/availability; reserved-vs-USED proof; plateau criterion | Pending |
| P20 | Required fork repairs in .29; broader maintenance in .30 | M | H | L | A /140; B/C /0.8.30 | Concrete110/120 repair need; qualify before pins; .30 work is not a .29 dependency | Ruled: steward seq-367 |
| P21 | Entity/alias, coverage/complementarity, diversity families | L | H | L | Select bounded experiment or postpone /150 |9 ruling, held-out decision rule, current eligibility baseline | Pending |
| P22 | Concurrent single-flight/batching experiment | M | M | M | Conditional /150 |Built queue, measured bottleneck and explicit resource/cost bound | Pending |
| P23 | SDK hints stage2/default experiment | M | M | M | Conditional /150 |Measure benefit/overhead; separate public-surface/default decision | Pending |
| P24 | Overflow tripwire monitoring and answer-level/latency probes | M | H | M | Monitor; conditional probes /9,20,30,150 |Chunking itself remains parked; no inferred approval | Pending |
| P25 | Combined qualification, notes/docs and release readiness | H | H | L | Include /160 |Selected work and explicit dispositions; actual publication separate | Pending |
| P26 | Verifier optimization/large-file refactor/history refs | L | M | L | Preserve and assess; not automatic product scope /2,7 |Accepted behavior may already exist under different commits; scope needed | Pending |
| P27 | .29 first-tag CUDA routing prerequisite | H | H | M | Include design/decision in7/9, approved change in10 before160 |Current bootstrap guard is .28-only; preserve group/environment protections | Narrow .29 route scope approved; preserve protections |
| P28 | B2 state/verdict presentation and coverage semantics | M | H | M | Include contract reconciliation /3,60,80 |Hints ADR versus compact placeholders; score precision/parity; known/unknown spans and coverage denominator need explicit contract | Contract reconciliation approved /60,80; no inferred calibration |

## Recorded directions

| Decision | Source | Effect |
| --- | --- | --- |
| Release number0.8.29 | Owner reply in this chat, November9 | Corrects prompt typo; all drafts use .8.29 |
| Draft-only slice planning | Owner latest request, November9 | No product/environment/CI/dependency/cleanup execution |
| Prework0–8, interactive9, approved preparation10; feature numbering20+ mod10 | Owner latest request | Governs this proposed ladder |
| One GPT-6.1 Sol high drafter per slice | Owner latest request | Independent slice drafts; parent alone imports checkout changes |
| Compact text default and B2-8 guard approved | Existing October27/31 decisions, preserved in reconciliation |Do not reopen settled decisions when reviewing residual scope |

## Slice9 decision capture

For each batch append the proposal IDs, source evidence and unresolved point,
user's exact ruling/conditions, resulting allocation/defer reason and changes to
the decision-bound Slice10 plan. Keep draft scoring distinct from the ruling.
Record independent review findings and FIX1–FIX4 changes; final HITL acceptance
must follow material revisions. Up to four cycles is a ceiling, not a quota.

Bounded Slice9 scope approvals are recorded in the decision log and rows above.
No final preparation acceptance or implementation go-ahead is recorded yet.

November 10 source/advisory findings and explicit sweep limits are in
[prework intake](prework/slice-9/intake.md). Proposed preparation choices are in
[the decision worksheet](prework/slice-9/preparation-scope.md); no new feature or
preparation approval is inferred from starting this proposal work.

[Fresh CUDA review](research/cuda-version-adoption.md) adds primary compiler/
package evidence while retaining unknown adoption shares and unchanged accepted
importance ratings. Compiler exposure/mitigation is a proposed120 delivery gate.

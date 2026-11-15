---
title: 0.8.29 proposal register and HITL decision log
status: PROPOSED
date: 2026-11-10
target_release: 0.8.29
---

# Proposal register and HITL decision log

These proposals are reconciled with completed direct-run prework and actual
owner rulings.0–7 are completed proposals and8 remains reserved. Actual4/5 outputs and
independent plan/final reviews were reread at52a85c38c before9 PLAN_READY. The [direct mandate](direct-execution.md)
supersedes generic final-permission waiting: approved work advances after required
independent reviews. Specific unresolved owner choices remain explicit.
U/R/E confidence/risk/comparative effort is no time/cost estimate or product proof.

| ID | Proposal / gap | Understood | Risk | Effort | Recommendation / slice | Hard input or uncertainty | HITL |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P01 | Reuse/establish owned .29 workspace, switch session, preserve other writers | H | M | S | Include /10 |Owner-authorized replacement attached here; approved10 environment/state work needs reviews and actual prerequisites | Workspace done; preparation approved through reviews |
| P02 | Owned venv/native/TS setup and source-pinned Node/npm alignment | H | H | M | Include /10 |0 observed Node26/npm12 versus25/npm11 pins; no owned venv/TS deps | Approved scope; actual prerequisites/reviews remain |
| P03 | Disk/scratch/GPU/runner availability budgets | M | H | M | Include /10 |Dated initial16GiB observation superseded by10 read-only18:21UTC refresh; numeric build/campaign budgets remain unmeasured; historical host success is not readiness | Approved scope; actual prerequisites/reviews remain |
| P04 | Activate .29 state/board/views after .28 completion | H | M | S | Include /10 |9 decision; no live state today, dependent preflight cannot pass yet | Approved scope; actual prerequisites/reviews remain |
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
| P16 | Interrupted-erasure coverage reconciliation | H | H | M | Included /100 |Actual eight-phase process-crash and physical coverage reviewed by4/5; compare exact remaining transaction/checkpoint gaps before adding controls | Inclusion assigned; actual fault/control design only |
| P17 | Safe lifetime/teardown after CUDA context reset | H | H | L | Included remedy design /110 |Typed-loss then SIGSEGV characterization and unbounded child wait; positive bounded teardown design, permitted timing/venue, conditional140 repair | Inclusion assigned; exact model/venue/remedy gates remain |
| P18 | Required CUDA usage review and second CUDA13 artifact | M | H | L | Accepted next-artifact direction /120 |Comparable-month shares/actual compiler6111901 exposure unknown; selection/identities/PTX/driver/library/route design needed | Accepted13.0 next artifact; exact design/policy-change ruling retained |
| P19 | Selected Tegra/x86 CUDA12.6 SDK close and soak evidence | M | H | L | Independent conditional units /130 |CUDA13 A is .31; non-Tegra ARM+GPU B/C excluded from .29 by seq387; selected D/E counters/settling/plateau remain | Only selected D/E local readiness remains; no non-Tegra ARM+GPU .29 gate |
| P20 | Required fork repairs in .29; broader maintenance in .30 | M | H | L | A /140; B/C /0.8.30 | Concrete110/120 repair need; qualify before pins; .30 work is not a .29 dependency | Ruled: steward seq-367 |
| P21 | Entity/alias, coverage/complementarity, diversity families | L | H | L | Select bounded experiment or postpone /150 |9 ruling, held-out decision rule, current eligibility baseline | Specific decision pending |
| P22 | Concurrent single-flight/batching experiment | M | M | M | Conditional /150 |Built queue, measured bottleneck and explicit resource/cost bound | Specific decision pending |
| P23 | SDK hints stage2/default experiment | M | M | M | Conditional /150 |Measure benefit/overhead; separate public-surface/default decision | Specific decision pending |
| P24 | Overflow tripwire monitoring and answer-level/latency probes | M | H | M | Included constant/monitoring /20,30; optional probes /150 |Accepted tripwires only; chunking remains parked and paid probes need selection/caps | Existing20 monitoring scope retained; optional probes undecided |
| P25 | Combined qualification, notes/docs and release readiness | H | H | L | Included /160 |Selected actual outcomes, W/E canonical reconciliation, AC054 finalizer coverage, assembled/current installed-consumer proof | Included in release work; actual publication separate |
| P26 | Verifier optimization/large-file refactor/history refs | H | M | M | Retain existing behavior /2,7; optional additions unselected |Completed2/7 distinguish implemented tooling and historical recovery; no broad refactor/cleanup needed | Assessment complete; no new product work selected |
| P27 | .29 first-tag CUDA routing prerequisite | H | H | M | Include design/decision in7/9, approved change in10 before160 |Current bootstrap guard is .28-only; preserve group/environment protections | Narrow .29 route scope approved; preserve protections |
| P28 | B2 state/verdict presentation and coverage semantics | M | H | M | Include contract reconciliation /3,60,80 |Hints ADR versus compact placeholders; score precision/parity; known/unknown spans and coverage denominator need explicit contract | Contract reconciliation approved /60,80; no inferred calibration |

## Recorded directions

| Decision | Source | Effect |
| --- | --- | --- |
| Release number0.8.29 | Owner reply in this chat, November9 | Corrects prompt typo; all drafts use .8.29 |
| Draft-only slice planning | Owner request, November9; superseded for approved execution by November10 mandate | Retain as origin history; current direct pipeline governs approved work |
| Prework0–8, interactive9, approved preparation10; feature numbering20+ mod10 | Owner latest request | Governs this proposed ladder |
| One GPT-6.1 Sol high owner per slice with independent Astra-medium plan/final review | November10 direct mandate | Parent dispatch/import; same owner finalizes/implements only reviewed approved scope, no generic extra permission |
| Compact text default and B2-8 guard approved | Existing October27/31 decisions, preserved in reconciliation |Do not reopen settled decisions when reviewing residual scope |

## Slice9 decision capture

For each batch append the proposal IDs, source evidence and unresolved point,
user's exact ruling/conditions, resulting allocation/defer reason and changes to
the decision-bound Slice10 plan. Keep draft scoring distinct from the ruling.
Record actual independent plan/final verdicts and bounded FIX deltas. Material
new scope requires its specific owner decision; approved work requires no generic
final acknowledgment. Up to four cycles is a ceiling, not a quota.

Bounded owner rulings and the current direct mandate govern the rows above.
Approved preparation advances through required reviews and actual prerequisites;
policy/fault/spend/optional/publication decisions remain item-specific.

November 10 source/advisory findings and explicit sweep limits are in
[prework intake](prework/slice-9/intake.md). Proposed preparation choices are in
[the preparation mapping](prework/slice-9/preparation-scope.md). Proposal completion
is not source/environment/product qualification.

[Fresh CUDA review](research/cuda-version-adoption.md) adds primary compiler/
package evidence while retaining unknown adoption shares and unchanged accepted
importance ratings. Compiler exposure/mitigation belongs120 design; actual policy/toolkit changes need owner ratification.

Completed prework routes exact current changes through
[Slice9 receiving corrections](prework/slice-9/correction-register.md). P21/P22/P23
and optional P24 probes still require individual bounded selection or explicit
deferral; no research campaign is authorized by listing them. P16/P17/P25/W
inclusion is not reopened. Mandatory unknowns cannot be waived by a proposal PASS.

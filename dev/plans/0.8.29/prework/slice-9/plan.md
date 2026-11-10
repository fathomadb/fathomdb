---
title: "0.8.29 Slice 9 — proposal review and HITL decisions"
status: PROPOSED
target_release: 0.8.29
slice: 9
---

# Slice 9 — proposal review and HITL decisions

## Goal, maturity and inspected sources

Convert actual prework into explicit scope decisions and an independently reviewed,
HITL-accepted preparation plan. This original plan describes the checkpoint;
November 10 research, bounded scope rulings and independent-review progress are
now recorded in [intake](intake.md), [decisions](decision-log.md) and
[review log](review-log.md). Final acceptance, preparation execution, state
activation and product qualification remain pending.
The [brief](../../planning-brief.md), [master](../../../plan-0.8.29.md),
[reconciliation](../../reconciliation-plan.md), [evidence](../../planning-evidence.md)
and [seed register](../../proposal-register.md) were inspected, as were available
prework plans 0–5 and [reserved 8](../slice-8/plan.md). Plans are not their outputs;
6/7 outputs were unavailable at drafting. Source evidence remains attributed to
its originating audit, not a fresh Slice 9 source review or current test pass.
Preserve integrated MCP/queue/Block 1, B2-2/3, partial B2-6 and format-37 storage.
Compact `text` default and B2-8 guard are approved prior directions; residual
implementation, field design, scope and qualification remain separate questions.

## Slice prep: local needs, requirements, acceptance and design

- S9-N1: the owner needs small, evidence-backed choices with visible uncertainty.
- S9-N2: implementers need exact approved scope, conditions and receiving slices.
- S9-R1 / S9-A1: intake accounts for every slice 0–8 and every proposal; missing work has an explicit unavailable/deferred disposition, owner and impact, while 8 remains reserved.
- S9-R2 / S9-A2: each proposal has understanding, risk, effort, evidence, recommendation, allocation and pending/approved/rejected/deferred ruling; no omitted or silently approved row.
- S9-R3 / S9-A3: each Slice 10 task traces to an actual ruling and executable prerequisites; a task without that link fails review.
- S9-R4 / S9-A4: independent findings, FIX-n deltas and the final owner's exact acceptance/conditions identify the reviewed revision; unresolved scope cannot execute.
- S9-R5 / S9-A5: master/slice drafts and TC dispositions agree with final rulings; canonical contracts change only through separately approved receiving work.

Deliver `requirements.md`, `design.md`, `intake.md`, `decision-log.md`,
`preparation-scope.md` and `review-log.md` beside this plan when commissioned.
Use local IDs only; do not add canonical ACs. Design the records around
proposal→evidence→ruling→condition→task→validation, preserving superseded revisions.
Read each relevant accepted ADR through the [decision index](../../../../adr/ADR-0.6.0-decision-index.md)
before endorsing a contract change. Primary research resolves specific gaps from
actual source/tests, official vendor documentation or original experiment receipts;
record dates/identities and unavailable evidence rather than inventing results.
Fit review applies the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md):
prioritize owned-boundary faults, post-state witnesses, properties and honest skips;
defer blanket coverage/fuzz campaigns. Fresh quantitative CUDA review remains
incomplete under the [review procedure](../../../../design/cuda-version-support-review.md);
retain accepted policy and require owner ratification of proposed changes.

## Intake and provisional scoring

Collect 0 environment/resources; 1 dependencies/pins; 2 cruft; 3 draft contracts;
4 architecture fit; 5 meaningful test traceability; 6 stale docs/transcripts;
7 delivery delays; and 8 reservation. Record scope, output paths, evidence dates,
unknowns, proposed dispositions and cross-slice prerequisites for each. An absent
output is not a passed review; ask HITL whether to defer the affected proposal.
Reconcile duplicates by proposal/TC identity, preserve completed behavior, and
separate obsolete premise, existing implementation and unqualified residual.

These are original seed scores, HITL-pending at drafting; current rulings in the
decision log supersede that initial status. Rescore from
actual intake, citing exact evidence/limits per row. U=understood H/M/L;
R=impact risk H/M/L; E=S bounded edit/check, M several seams, L new design/external
qualification. Effort is comparative, not a time/cost promise. I=include proposal,
C=conditional include, P=postpone, D=design first; none authorizes implementation.

| Proposal | U/R/E | Initial recommendation / allocation | Evidence or missing condition |
| --- | --- | --- | --- |
| P01 workspace ownership | H/M/S | I /10 | Existing managed checkout; 0 ownership decision |
| P02 owned tools/native setup | H/H/M | I /10 | Missing owned venv/TS; Node/npm mismatch |
| P03 resource budgets | M/H/M | I /10 | 18GiB/98% audit; GPU availability unknown |
| P04 state/board/views | H/M/S | I /10 | .28 published; no live state; decision first |
| P05 dependency sweep/updates | M/M/M | I sweep; C updates /1→10 | Pin rationale, current applicability and compatibility |
| P06 cruft/stale docs | M/M/M | I review; C edits /2,6→10 | Inbound-reference proof and bounded file list |
| P07 contract/source alignment | M/H/M | I /3–5 | Draft traceability and accepted authority |
| P08 delivery delays | H/M/M | I /7→10 | .28 publisher repair already complete |
| P09 test/queue residuals | M/M/M | I /20 | Current reproduction after .28 fixes |
| P10 current scale/open/embed | H/M/M | I /30 | Stable harness and fixture/native identity |
| P11 eligibility cache/batch work | M/H/M | C /40 | Write-interleaved benefit and format37 proof |
| P12 fusion/text/snippets | M/H/L | D; C build /50 | Identity ADR/census and exact Unicode behavior |
| P13 compact MCP/filter/paging | H/H/L | I /60 | Preserve approved default; reconcile surface |
| P14 degradation/binding parity | H/M/M | I /70 | Bounded reasons; no wholesale EP03 approval |
| P15 residual B2 | M/H/L | D /80; C build /90 | Built signals preserved; Block1 follow-up exit |
| P16 interruption qualification | M/H/M | I bounded /100 | Compare .28 coverage; no reopened erasure SLA |
| P17 CUDA reset lifetime | M/H/L | D; C remedy /110 | Permitted fault venue; qualified fork if needed |
| P18 CUDA review/artifact | M/H/L | I /120 | Shares incomplete; delivery/policy decisions |
| P19 CUDA platform/SDK/soak | M/H/L | C units /130 | Venue, USED witness and plateau criterion |
| P20 fork/kernel maintenance | M/H/L | Owner seq-367: A /140; B/C /.30 | Only required repairs gate .29 pins; broader work has a .30 receiving plan |
| P21 candidate-selection families | L/H/L | C bounded experiment or P /150 | Held-out rule, native baseline and cost bound |
| P22 single-flight/batching | M/M/M | C /150 | Built queue and measured bottleneck |
| P23 SDK hints stage2/default | M/M/M | C experiment /150 | Separate experiment and default promotion |
| P24 chunking tripwires/probes | M/H/M | I monitoring; C probes /20,30,150 | Chunking remains parked; no new approval inferred |
| P25 combined readiness | H/H/L | I /160 | Selected work qualified; publication separate |
| P26 verifier/refactor/history | L/M/L | I assessment; C placement /2,7 | Compare actual behavior, preserve experiment refs |
| P27 first-tag CUDA route | H/H/M | I design/decision; C execution /7,9,10 | Current .28-only exception does not authorize .29 |
| P28 B2 presentation/coverage | M/H/M | I contract reconciliation /3,60,80 | Settle state/verdict and unknown/span/denominator semantics |

## Interactive rulings and ledger procedure

1. Present coherent batches of roughly 3–5 choices: preparation/resources; dependencies/cruft/docs/CI; contracts/verification/performance; MCP/B2/erasure; CUDA/forks; experiments/readiness. Split further when decisions differ; summarize evidence, alternatives, recommendation and consequence in plain language.
2. Ask only unresolved questions and wait for the owner's answer. Capture proposal IDs, evidence/version, exact ruling text, conditions, rationale, receiving slice/owner and deferred trigger. Clarify ambiguous scope; time, silence and reviewer approval are never HITL approval.
3. Update the seed register and `decision-log.md`; separate recommendation from ruling. Preserve approved defaults/B2-8, built behavior and settled closures. Disputed TC closures need evidence or explicit closure by decision, never a confirming run for a settled ruling.
4. For resulting canonical TC events, use the documented [ledger tools](../../../../agent-tools/ledgerwrite/README.md): fresh unfiltered `ledgerwatch` tail, fold by stable TC ID, reconcile duplicates, then `ledgerwrite --profile todos` on the release line. Updates retain kind/ID and use latest `--expected-prior-seq`; new items use `--open` with tool-allocated ID. Include ruling/evidence refs, preserve append-only history and verify the emitted event with ledgerwatch. On a stale update, reread/reconcile before retrying; do not hand-edit or force terminal transitions.

## Decision-bound Slice 10, review and final integration

1. Derive Slice 10 requirements/design/tasks solely from recorded preparation rulings: exact files/tools/versions, workspace ownership/switch order, resource stops, dependencies, rollback/preservation boundaries and falsifiable checks. Keep feature work in 20+; state/board activation remains approved 10 work. Carry conditional tasks as blocked until their conditions are evidenced.
2. Commission an independent read-only subagent review of intake completeness, decision fidelity, architecture fit, executable prerequisites and the resulting 10 plan. Parent alone edits shared plans/registers; any later concurrent implementer gets its own worktree. Record findings against a concrete revision; no fabricated approval.
3. Apply at most four FIX-n cycles, each tying finding→change→recheck. This is a ceiling, not a quota; stop when adequate. Repeated same-issue failure triggers rereading evidence/ADR/memory and narrowing the proposal. At four unresolved cycles, return issues to HITL or explicitly defer/narrow them; do not silently proceed.
4. Present final revisions and material scope/condition changes to HITL. Record acceptance/rejection/deferral of the identified final package; re-present later material edits. Update master allocation/history, dependencies and accepted slice drafts from that ruling, including explicit 120–150 carryover placement and old120→160 mapping. No new state/views are activated in 9.

## Dependencies, validation and stop points

Hard: 0–8 intake or explicit HITL unavailable/deferred dispositions; 10 waits for
recorded decisions, independent review and final acceptance. Prefer 3/4/5 convergence
before contract choices. Resource/fork/venue conditions block affected units only;
140 qualification→chosen pins→110/120 integration→130 reruns; 80 approval AND
Block1 follow-up exit gate90. Serialize shared master/register/ledger edits.
New prerequisites for the next reviewer: missing 6/7 outputs, bootstrap ownership,
actual resource budgets, disputed closures, identity design and CUDA venue/pin scope.
Validate S9-A1–A5, relative links and scoped Markdown after parent import; no full
product gate or behavioral tests for this planning slice. Later approved source/
script changes require failing-test-first implementation and applicable final gates;
mechanical version bumps retain the exception. Stop on contradictory authority,
missing condition or unresolved ruling; complete unaffected proposals without
claiming product green, execution permission or publication authorization.

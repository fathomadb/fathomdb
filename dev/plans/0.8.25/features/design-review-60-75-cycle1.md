---
title: 0.8.25 Slices 60/65/70/75 independent design re-review
date: 2026-09-01
cycle: 1
reviewer: independent Codex design review
verdict: FIX-2
scope: read-only re-review; no design or implementation edits
---

# Independent design re-review: Slices 60, 65, 70, and 75

## Verdict

**FIX-2.** Seven of nine cycle-0 findings are fully closed. Two P1 issues remain
in Slice 70. No other new P1/P2 issue was found, and scope, dependency order,
semantic-policy ownership, and historical-receipt disposition remain intact.

## Closure matrix

| Finding | Cycle-1 result | Basis |
| --- | --- | --- |
| DR-60-01 | CLOSED | Successful pagination now requires a fully materialized sub-cap graph; state 10,001 fails with no page or cursor, with adversarial proof mapped. |
| DR-60-02 | CLOSED | Seed, intermediate traversal, and terminal/output predicates are separate; the mixed-kind intermediate fixture is explicit. |
| DR-65-01 | OPEN in dependent Slice 70 | Slice 65 removes default-capable registry state correctly, but Slice 70 still names the removed state and default-promotion route. |
| DR-65-02 | CLOSED | Context-changing is mechanically defined; retrieval-only and answer-qualified capabilities are distinct and enforced by intended use. |
| DR-70-01 | OPEN | Integer arithmetic and ordering are much stronger, but the normative recurrence is dimensionally incomplete and does not fully specify mass-conserving allocation. |
| DR-70-02 | CLOSED | Every graph-cap overflow fails before diffusion; canonical graph discovery and no-partial-ranking proofs are explicit. |
| DR-70-03 | CLOSED | Paths are witness paths; bounded contribution accounting and omission/degradation signals are explicit. |
| DR-75-01 | CLOSED | Pre-publication packed-artifact proof is separated from the separately authorized post-publication registry gate. |
| DR-75-02 | CLOSED | The sealed manifest now binds the workload, schedule, freshness, failure, environment, affinity, and sampling inputs; policy versus advisory metrics are explicit. |

## Remaining findings

### P1 — C1-70-01: Slice 70 reintroduces the removed profile/default contract

Slice 65 now defines only `qualified_retrieval_only`, `qualified_answer`, and
`retired` ([slice-65/design.md:58](slice-65/design.md)); it also makes omitted-profile A0 immutable and forbids a promotion-capable form in 0.8.25
([slice-65/design.md:89](slice-65/design.md)). Slice 70 nevertheless says an accepted treatment installs only `qualified_opt_in` and that default promotion is available through a separate HITL decision
([slice-70/design.md:158](slice-70/design.md)). `qualified_opt_in` no longer exists, and the stated promotion route contradicts the shared registry authority.

**Required correction:** replace the Slice 70 acceptance paragraph with the
Slice 65 state model: retrieval-gated treatments may install as
`qualified_retrieval_only`; context-changing treatments may install as
`qualified_answer` only after the registered correctness, groundedness, and
attribution no-regression predicates pass. State explicitly that 0.8.25 has no
default promotion path and omitted profile remains compiled A0. Map a dependent-
contract fixture that rejects `qualified_opt_in`, default aliases, and answer use
of retrieval-only temporal/associative profiles.

### P1 — C1-70-02: the integer PPR recurrence is dimensionally incomplete

Slice 70 represents both the probability and seed vectors as integer mass summing
to `M = 10^15` ([slice-70/design.md:105](slice-70/design.md)), but writes the dangling term as `dangling_mass*seed(v)` without division by `M`
([slice-70/design.md:111](slice-70/design.md)). It also does not state whether largest-remainder allocation occurs across each node's outgoing arcs and across seed recipients as a whole. A literal implementation over-scales dangling redistribution; independent local rounding can also lose or create mass despite the asserted invariant.

**Required correction:** specify a mass-conserving integer algorithm rather than
an abbreviated real-valued equation. For every full iteration:

1. Allocate exactly `15*M/100` restart units across the seed vector by the
   canonical largest-remainder rule.
2. For each non-dangling node, allocate exactly `85*p(u)/100` units across its
   ordered outgoing edge revisions, with the defined remainder rule.
3. Sum the mass of all dangling nodes and allocate its exact 85% follow portion
   across the seed vector using `dangling_follow*seed(v)/M` and the same
   canonical remainder rule; the global restart allocation in step 1 already
   accounts for restart mass.
4. Define disposition of every division remainder so the resulting vector sums
   exactly to `M` before convergence is evaluated.

Equivalent exact arithmetic is acceptable, but the normative equation and
remainder domains must be unambiguous. Extend the hand-computable RED/GREEN cases
with uneven out-degree, multiple dangling nodes, and seeds whose weights do not
divide `M`; assert exact per-iteration mass conservation as well as final digest.

## Accurate review-record update

The FIX-1 resolution record accurately describes the intended corrections, but
its statement that all nine are resolved is premature until the two Slice 70
issues above are corrected. No historical receipt was promoted, no semantic
policy entered FathomDB, and no Slice 10+ implementation was performed by this
review.

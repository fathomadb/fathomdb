---
title: FathomDB 0.8.27 Slice 90 — design review
status: PASS
target_release: 0.8.27
---

# Slice 90 design review

The first independent read-only review used `gpt-6.1-sol` with high reasoning
against the draft and current `release/0.8.27` source. Its verdict was FAIL
with three findings:

| Finding | Resolution |
| --- | --- |
| D27 demanded embed-queue/fixed-worker metrics from historical `7a2f9bf9`, which has neither. | Split comparable operation throughput/latency from mandatory candidate-only dispatch/resource evidence in the protocol, design and change plans. Historical unavailability is explicit, never zero. |
| The source owner inventory was tied to the older Slice 85 binding despite recovery commits already in release ancestry. | Inventory current `e689000d4` source and keep `7a2f9bf9` only as the historical performance reference. Correct Slice 85 prose without asserting missing qualification or rebinding release state. |
| Direct embed queued expiry and started timeout had no exact public outcome. | Specify `Overloaded` for queue full/expiry, `Embedder` for started error/timeout, `Closing` for pending close, and request-state completion precedence; pin in RED tests. |

The second independent read-only review used `gpt-6-sol` with high reasoning.
It found the three design gaps closed but returned FAIL for one stale present
tense disk-blocker statement in the Slice 85 recovery records. Those records
now distinguish the historical failed public capture from the currently
available 127 GB and preserve the unresolved NVML blocker. Its narrow
re-review returned **PASS**. It found no further design or TDD-plan gap
against the accepted runtime ADR and current source.

The approved scope is the reconciled [plan](plan.md),
[design](design.md), [code change plan](slice-90-code-change-plan.md),
[test change plan](slice-90-test-change-plan.md), and
[D27 protocol](d27-runtime-qualification-protocol.json). Review approval is
for the plan only; it is not an implementation or qualification receipt.

## Current-source inventory amendment

A subsequent independent `gpt-6-sol` high read-only review returned **PASS**
on the prospective four-item amendment to the design and
[current-source inventory](current-source-inventory.md). It verified the
current uses and exact retained cfg/public identities for `EDGE_FACT_KIND`,
`MEAN_VEC_PIN_THRESHOLD`, `Engine::execute_for_test`, and
`Engine::run_one_thread_poison_for_test`, along with the corrected 13-row
public-capture evidence. This review approves those item-specific
dispositions only; it does not turn the pending hidden-surface, historical
D27 or Slice 85 recovery qualifications into PASS.

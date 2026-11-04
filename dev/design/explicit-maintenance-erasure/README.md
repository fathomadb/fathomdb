---
title: Explicit maintenance erasure — approved contract and implementation
status: ACTIVE
date: 2026-11-04
---

# Explicit maintenance erasure — approved contract and implementation

The completed [optimization write-up](../../notes/erasure-optimization-sequence-findings.md)
sets aside the Slices 81–84b optimization sequence. HITL requested a concrete
contract and explicit cleanup API proposal, Astra review, then approval before
implementation. HITL approved the contract on 2026-11-04 and assigned implementation to
[Slice 85](../../plans/0.8.28/features/slice-85/README.md). The reviewed
[production design](production-design.md) is implemented on the development line
with format 36. Final Astra review and independent qualification passed at
`a2fb4a647` on canonical `release/0.8.28`; the slice status records assembled
verification, measured costs and hosted CI limitations.

| Document | Purpose |
| --- | --- |
| [Durable findings](../../notes/erasure-optimization-sequence-findings.md) | Full delivered/rejected sequence, measurements and limits |
| [Contract and API](contract.md) | Explicit policy, queue visibility, maintenance ownership, guarantees, blockers and incomplete outcomes |
| [Implementation outline](implementation-plan.md) | Successor design, RED/GREEN, SDK/ownership qualification and approval gates |

The minimal proposal queues intent without claiming immediate logical revocation.
Expensive deletion/cleanup requires an explicitly acquired maintenance window.
Post-delete blockers can preserve a durable write fence even after handle release;
this failure behavior is part of the approved contract. There is no active SQL
deadline or immediate logical-revocation promise.

Astra approved this revised package for HITL proposal review on 2026-11-04 with
no blocking findings. Four initial findings were resolved: admissibility/queued
withdrawal, offline activated-ticket reconciliation, original-sink reattachment,
and older-binary refusal through an approval-required fresh-store format break.

HITL subsequently approved the contract, including unchanged logical eligibility
until maintenance, the fresh-store format revision and truthful incomplete/fenced
outcomes. The original end-of-Slice-83 guarantee/API decision is now ruled by that
approval; no numerical deadline was approved. Slice 85 completed concrete Astra
design review and implementation. HITL accepts observed test timings; the
committed bounded empirical screen reports actual outcomes without an SLA.

The contract and production design are maintained code-backed authorities. The
implementation outline remains a planning reference; current execution and final
qualification are recorded in the [slice status](../../plans/0.8.28/features/slice-85/status.md).

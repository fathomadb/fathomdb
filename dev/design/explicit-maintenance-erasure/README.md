---
title: Explicit maintenance erasure — approval package
status: ACTIVE
date: 2026-11-04
---

# Explicit maintenance erasure — approval package

The completed [optimization write-up](../../notes/erasure-optimization-sequence-findings.md)
sets aside the Slices 81–84b optimization sequence. HITL requested a concrete
contract and explicit cleanup API proposal, Astra review, then approval before
implementation. HITL approved the contract on 2026-11-04 and assigned implementation to
[Slice 85](../../plans/0.8.28/features/slice-85/README.md). This preparation
changes no production implementation or schema.

| Document | Purpose |
| --- | --- |
| [Durable findings](../../notes/erasure-optimization-sequence-findings.md) | Full delivered/rejected sequence, measurements and limits |
| [Contract and API](contract.md) | Explicit policy, queue visibility, maintenance ownership, guarantees, blockers and incomplete outcomes |
| [Implementation outline](implementation-plan.md) | Successor design, RED/GREEN, SDK/ownership qualification and approval gates |

The minimal proposal queues intent without claiming immediate logical revocation.
Expensive deletion/cleanup requires an explicitly acquired maintenance window.
Post-delete blockers can preserve a durable write fence even after handle release;
this failure behavior is part of the proposed contract, not hidden by a timeout.

Astra approved this revised package for HITL proposal review on 2026-11-04 with
no blocking findings. Four initial findings were resolved: admissibility/queued
withdrawal, offline activated-ticket reconciliation, original-sink reattachment,
and older-binary refusal through an approval-required fresh-store format break.

HITL subsequently approved the contract, including unchanged logical eligibility
until maintenance, the fresh-store format revision and truthful incomplete/fenced
outcomes. The original end-of-Slice-83 guarantee/API decision is now ruled by that
approval; no numerical deadline was approved. Implementation is assigned to
Slice 85 for a separate clean run. Exact ownership/guard/index design, metadata
limits and numerical qualification retain their specified pre-code/timing gates.

Lifecycle catalog classification remains `proposal` because the approved work is
unshipped; it becomes a code-backed maintained owner when Slice 85 lands.

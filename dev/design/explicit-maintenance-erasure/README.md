---
title: Explicit maintenance erasure — approval package
status: PROPOSED
date: 2026-11-04
---

# Explicit maintenance erasure — approval package

The completed [optimization write-up](../../notes/erasure-optimization-sequence-findings.md)
sets aside the Slices 81–84b optimization sequence. HITL requested a concrete
contract and explicit cleanup API proposal, Astra review, then approval before
implementation. This package changes no production contract or release allocation.

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

Approval must explicitly cover unchanged logical eligibility until maintenance
and the fresh-store deployment cost. Exact ownership/guard/index design, metadata
limits and qualification thresholds retain their preimplementation review gates.
No production implementation is authorized by review approval. The original
end-of-Slice-83 physical-erasure guarantee/API decision remains the decision of
record; the completed performance follow-ups did not resolve it.

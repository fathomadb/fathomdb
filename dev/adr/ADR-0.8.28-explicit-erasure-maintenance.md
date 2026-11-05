---
title: Explicit physical-erasure maintenance policy
status: accepted
date: 2026-11-04
target_release: 0.8.28 Slice 85
supersedes: ADR-0.8.28-resumable-physical-cleanup.md (explicit policy scheduling and admission only)
---

# Explicit physical-erasure maintenance

## Decision

Implement the owner-approved [contract](../design/explicit-maintenance-erasure/contract.md)
using the [production design](../design/explicit-maintenance-erasure/production-design.md).
Autonomous remains the fresh-store default and retains its current worker.
ExplicitMaintenance is persisted and honored on reopen, queues intent without
logical revocation, and executes only under an exclusive application-granted
maintenance handle. Direct explicit-profile erase/purge returns MaintenanceRequired.

Format 36 is a fresh-store boundary for both profiles. Earlier files are refused;
no migration or backwards-readable metadata opt-in is provided. Queue identity,
scope guards and terminal receipts are durable, bounded and store-bound.
Normal queue admission returns NeedsMaintenanceAdmission when bounded authoritative
resource admission cannot be established; handle admission performs heavy preflight.

Physical success retains exact registered-copy/sink/WAL obligations. After logical
deletion, all mutations remain fenced until completion, including after handle
release or reopen. Original-sink reattachment and offline completion share existing
physical authority and atomic terminal ticket finalization. No deadline, immediate
revocation, background queue runner or MCP surface is added.

## Consequences

Applications schedule and own service blackout. Queued data retains existing
read/rank/statistics semantics. Blocked post-delete attempts can prolong write
outage. Terminal completion certifies the accepted cut and registered boundary,
not future writes, external backups or universal storage remanence. Numerical
qualification records empirical observations accepted by the owner, not an SLA.
Autonomous scheduling in the prior ADR remains authoritative for that profile.

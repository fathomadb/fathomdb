---
title: Inspector design
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
desc: Design ownership for the Inspector
status: ACTIVE
---

# Inspector design

Current delivery scope is [owner-adopted 0.2.0](../plans/0.2.0/scope.md). Original v0.1
mandatory obligations and remaining qualification carry forward unchanged;
historical source/design baselines and V01 identifiers retain their meaning.
The unpublished 0.1.0 port is the record foundation, not full-view completion.
This scope adoption is not approval of future detailed designs or execution.

[High-level design](high-level-design.md) owns the selected S16-2026-10-31 architecture.
[Requirements](../requirements.md) own release scope, and
[acceptance](../acceptance.md) owns behavioral criteria. Future detailed
contracts belong here and must identify current engine reuse versus new work.
Accepted FathomDB ADRs and interfaces remain authoritative; this directory
introduces no SQL escape hatch or application commands under doctor/recover.

Current 0.2.0 delivery drafts and ownership are in
[plans/0.2.0/plan.md](../plans/0.2.0/plan.md): comparison/person interaction 60,
final installed/integrated qualification 70. Legacy drafts under
[plans/0.1](../plans/0.1/README.md) retain their original source/evidence scope. The shared high-level design is constrained
to Inspector-local delivery; it does not authorize Engine/MCP refactors.

## Compatibility analysis

[0.8.28 versus 0.8.29](compatibility-0.8.28-0.8.29.md) records the pinned
source differences relevant to implementation. It is not a completed runtime
qualification or an amendment to the supported-version requirement.

[Compatibility architecture research](../research/compatibility-architectures.md)
collects implemented examples from other tools. Its
[Astra assessment](../reviews/compatibility-architecture-review.md) recommends
minimal Inspector-local boundaries for detailed design; it does not amend the
current-schema policy or approve producer-version support.

[Slice 15](../plans/0.1/slice-15/plan.md) establishes the required behavior and
verification baseline. [Slice 16](../plans/0.1/slice-16/plan.md) completed the original
shared architecture baseline and evaluates evolution through FathomDB 1.x;
feature drafts consume its decisions rather than independently choosing
incompatible structures or contracts.

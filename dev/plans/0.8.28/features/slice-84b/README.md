---
title: Slice 84b — repeated certification scans and transactional envelope accounting
status: DRAFT
target_release: 0.8.28
---

# Slice 84b scope

Owner requested this follow-up after Slice 84's scheduling experiment failed.
This is a planning package, not release-state allocation or authorization for a
new measurement campaign. The existing physical-success contract remains intact.

**Both targets are required:** reduce repeated eligibility/structural-proof work,
and maintain resource-envelope totals from actual transaction effects. Execute
these sequentially so each has a separate correctness and work-reduction witness;
neither substitutes for the other.

Slice 84 measured node eligibility at 3.020 seconds and certification at 4.038
seconds, including envelope 1.519 and registered-cursor proofs 2.727 (overlapping
elapsed stages). Its boundary MAX cost only 0.077 seconds. Priority merely moved
waiting to the writer turn. Use the retained Slice 83 production semantics plus
Slice 84 observers as the baseline, **never the rejected priority candidate**.

| Document | Purpose |
| --- | --- |
| [Plan](plan.md) | Requirements, acceptance, staged TDD and proposed bounded measurement |
| [Draft design](design.md) | Concrete owner, proof/effect boundaries, deltas, commit and recovery |
| [Slice 84 status](../slice-84/status.md) | Negative qualification and frozen evidence |
| [Reviewed research](../slice-84/design.md#web-research-before-disposition) | Primary-source approaches and applicability limits |

In scope: exact query work, safe transaction-local reuse of immutable decoded
root/denial facts, a bounded cleanup-job certificate, explicit mutation-effect
coverage and exact resource accounting. No new index is presumed: a new durable
index or format change needs a reviewed schema/interface/ADR disposition first.

Out of scope: scheduling experiments, broader mutation admission, changed
physical completion/refusal meaning, reader cancellation, cleanup APIs, numerical
SLAs, staged replacement, general-purpose caches, new database engines or SQLite
forks. No FTS cursor=rowid assumption, head-only eligibility shortcut or weaker
orphan/residue proof is permitted. The broader guarantee/API decision remains
open. Final success requires both correctness and integrated service improvement;
operation-count reductions alone cannot close the performance objective.

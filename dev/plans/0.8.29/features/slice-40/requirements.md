# Slice 40 local needs and requirements

Status: proposed conditional requirements, not a canonical acceptance amendment.
The [plan](plan.md) owns the current predecessor/input holds. These requirements
refine master N29-S/R29-REL-S; existing release targets remain unchanged.

| Need | Requirement | Local acceptance |
| --- | --- | --- |
| N40-1: justify operational complexity | R40-1: select eligibility reuse or batch hoisting only from actual current 30 cost attribution and both read-mostly/write-interleaved schedules; distinguish existing optimization from new savings. | A40-1 |
| N40-2: preserve exact eligibility | R40-2: each memoized value is valid for the consuming SQLite snapshot, effective time and every semantic input; preserve pre-cap eligibility, route guards, typed fallback and frozen reproduce-or-fail. | A40-2 |
| N40-3: preserve publication | R40-3: hoist only metadata proved invariant throughout the admitted IMMEDIATE batch; preserve row-specific checks, physical tuple atomicity, mid-batch mean pin, rollback and post-commit publication. | A40-3 |
| N40-4: bound operational cost | R40-4: selected reuse has reviewed ownership, lifetime, entry/byte bounds and eviction; misses and unsupported proofs use the existing correct route. No per-write scan or retained SQLite snapshot is introduced without reviewed measured justification. | A40-4 |
| N40-5: make a conditional decision reviewable | R40-5: report selected, not-selected, deferred or blocked separately for each opportunity with evidence and uncertainty; a rejected cache does not authorize schema/counters or a new optimization. | A40-5 |

## Fixed contracts

- R40-6: preserve current format37, vec0 `chunk_size=64`, lifecycle/validity metadata, sidecars/spans, projection generations/cursors, query candidate budgets, filter behavior, ranking and public APIs. No database migration or pre-1.0 compatibility shim.
- R40-7: preserve fresh clean-coalescing admission, external ordinary SQLite row-trigger invalidation, branch-sensitive nonce and existing frozen token fixtures. Neither a shared latest atomic nor equal integer generations authorize another read snapshot.
- R40-8: source-dependent eligibility and nonterminal closure barriers remain independent of direct-node eligibility; relaxed history flags never bypass an unconditional barrier.
- R40-9: no cache helper, benchmark/probe or public hook is created in this preparation phase. Future measurement tools must be committed, rerunnable, indexed and tested for measurement correctness. Do not regenerate golden fixtures or add tamper/path/digest-hardening tests.
- R40-10: shared metadata must not absorb row bodies, scalar values, lifecycle/validity, cursor ownership or mutable mean state. Keep absent versus present-empty attribute encodings, complete vec0 bindings and no-attribute behavior unchanged.

Each selected behavior needs a runtime RED at its own staged/committed source
that fails for the intended contract or work-count reason, followed by a separate
GREEN. Existing-correct behavior is baseline coverage; passing tests cannot be
relabelled RED. No Slice 20 exception is inferred for Slice 40.

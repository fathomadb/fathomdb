# Operations

Operations docs cover the published 0.8.23 surface. APIs not yet available
from a registry are marked separately.

- [Erasure](erasure.md) — what `erase_source` / `purge` guarantee, what they
  do not, the erasure-audit record, the non-PII `source_id` rule, and
  `fathomdb doctor orphan-provenance`.
- [Worktree and branch consolidation](worktree-consolidation.md) — the local,
  manifest-gated preservation workflow for safely reducing repository
  worktrees and local branches.

See also [Reference — CLI](../reference/cli.md) for the full `doctor` /
`recover` verb tables, and
[Positions — recovery surface](../positions/recovery-surface.md) for why
recovery is CLI-only while erasure is not.

---
title: Worktree Consolidator Execution Status — requirements
date: 2026-08-15
desc: Requirements for durable, namespace-independent observation of a consolidation execution
status: PROPOSED
blast_radius: local consolidator read-only status mode and execution evidence only
refs:
  - dev/design/worktree-branch-consolidator-requirements.md
  - dev/design/worktree-consolidator-execution-status-acceptance-criteria.md
  - dev/design/worktree-consolidator-execution-status-design.md
---

# Worktree Consolidator Execution Status — requirements

## Learning

The first proof-backed execution continued in a host PID namespace after its
caller could not observe the lock-recorded PID. Durable preservation and
progress receipts, rather than PID visibility, correctly showed that the
execution was still active. A PID in a cooperative lock is therefore an opaque
diagnostic hint, not liveness or recovery authority.

## Requirements

### REQ-WTC-S01 — namespace-independent liveness boundary

`status` must not call `kill`, inspect `/proc`, invoke `ps`, or otherwise use
PID visibility to classify a lock as stale. A present consolidator lock means
`executing` regardless of whether its recorded PID is visible to the caller.
The status report may say that liveness is `unknown`; it must not expose a
"safe to clear", takeover, retry, or resume action.

### REQ-WTC-S02 — durable execution state machine

For an exact canonical manifest under a private evidence directory, `status`
must derive exactly one metadata-only state from deterministic execution paths:

| State | Required observation |
| --- | --- |
| `not_started` | No lock and no execution receipt or preservation/progress receipt. |
| `executing` | Lock present, with zero or more valid durable receipts. |
| `completed` | Lock absent, no fallback-partial receipt exists, and a valid successful final receipt covers every expected action. |
| `recovery_required` | Lock absent and a valid deterministic or fallback partial receipt exists, or preservation or progress evidence exists without a valid successful final receipt. |

Malformed, contradictory, non-canonical, foreign-repository, or non-direct
evidence is a safety error, never a guessed state. A final receipt while the
lock remains is still `executing` (finalizing), not `completed`. The existing
randomized `partial-<manifest-prefix>-<nonce>.json` fallback is part of the
state machine: a valid fallback partial dominates a coexisting success final.

### REQ-WTC-S03 — exact evidence and privacy

`status` binds the supplied manifest's exact canonical file hash and derives
only its preservation, progress, final-receipt, and exact-prefix fallback
partial paths. It scans that direct-child namespace only and rejects unexpected
progress numbering, malformed padding, duplicate fallback partials,
symlink/non-regular receipt entries, and records bound to another full manifest
hash. It checks exact schemas, repository identity, manifest hash, action
count, contiguous progress numbering, preservation linkage, completed-action
prefixes, bundle linkage, and final-result invariants. Its JSON report contains
only hashes, filenames, counts, state, and fixed operator guidance—never Git
diffs, source payloads, prompts, secrets, untracked contents, failure strings,
or a lock PID.

### REQ-WTC-S04 — observation cannot mutate or authorize recovery

`status` creates no receipt, archive, lock, ref, worktree, process signal, or
report file. It must leave the repository and evidence fingerprints unchanged.
It cannot make a later `consolidate` invocation pass, unlock a batch, or
replace the required fresh audit → manifest → dryrun chain after an incomplete
execution. Recovery remains an explicit human decision using preserved bundle
and evidence.

## Non-requirements

- Cross-namespace process discovery or termination.
- Automatic stale-lock removal, lease expiry, takeover, retry, resume, or
  deletion after interruption.
- Changing preservation, approval, freeze, expected-old-SHA, or remote-mutation
  boundaries.

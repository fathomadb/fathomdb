---
title: Worktree and Branch Consolidator — high-level requirements
date: 2026-08-13
desc: Product requirements for safe local consolidation of Git worktrees and local branches
status: PROPOSED
blast_radius: proposed local operator script, its evidence directory, local worktrees, and local refs only
refs:
  - dev/design/worktree-branch-consolidation.md
  - dev/design/worktree-branch-consolidator.md
---

# Worktree and Branch Consolidator — high-level requirements

## 1. Objective

Provide `scripts/worktree-consolidator.py`, a local operator tool that reduces
a repository to an approved worktree target without losing local work or
silently expanding cleanup scope. It supports the controlled pipeline:

```text
audit → manifest → independent approval → dryrun → freeze → consolidate → status → verify
```

The tool is preservation-first. It does not make semantic integration,
remote-ref, or release-scope decisions.

## 2. Requirements

### REQ-WTC-001 — authority-separated modes

The CLI provides exactly these modes:

- `audit`: observe one repository snapshot and classify every worktree/local
  head without modifying repository or filesystem state;
- `manifest`: solve a supplied snapshot, owner map, policy, and declared or
  inferred target into an immutable candidate manifest;
- `dryrun`: re-audit current state and simulate one independently approved
  manifest without mutating Git state; and
- `consolidate`: apply only the confirmed, freshly rehearsed manifest.
- `status`: observe one named execution's durable evidence and cooperative lock
  without making any repository, evidence, archive, lock, process, or remote
  mutation.

`manifest` never performs cleanup. `dryrun` never repairs, regenerates, or
widens a plan. `consolidate` never performs semantic Git operations such as
merge, checkout, rebase, reset, or push.

`status` never treats a missing or inaccessible lock PID as evidence that an
execution has stopped. A present lock is an active-or-unobservable execution
until its owner releases it; status may report durable progress but cannot
clear, take over, resume, or retry the execution.

### REQ-WTC-002 — bounded objective and classification

Every registered worktree and local head receives exactly one classification:
`protected-active`, `integration-required`, `archive-required`,
`merged-retirable`, or `unresolved`.

Target planning uses a strict policy and owner/release-role map. It fails closed
when ownership, release role, theme, recovery disposition, or target lower
bound is unknown. It must not infer safety from a branch name, age, ancestry,
or `git cherry` alone.

### REQ-WTC-003 — immutable, replay-resistant evidence chain

The manifest, approval attestation, baseline attestation, dry-run receipt,
freeze attestation, and execution receipt are canonical JSON with SHA-256
bindings. They bind to the resolved repository identity, snapshot, baseline,
owner map, policy, exact actions, and evidence locations as applicable.

Approval and freeze attestations are local operator evidence, not cryptographic
signatures. They establish a human review/freeze precondition and durable audit
trail; deployments requiring resistance to a same-host attacker must supply
externally signed attestations or a protected evidence store.

An executable plan must have a matching independent approval and successful,
unexpired dry-run receipt. Any changed hash, witness, path, owner map, policy,
baseline, target, or expiration fails before cleanup begins.

### REQ-WTC-004 — preservation and recovery

Before retirement, `consolidate` creates a standalone, hash-verified bundle
covering all local refs, every manifest-required old tip, and every selected
reflog/unreachable recovery candidate. It records the bundle hash, exact input
refs/candidates, verifier result, and tip coverage in a typed, append-only
preservation receipt before the first worktree removal or local-head deletion.
The final or partial execution receipt hash-binds that preservation receipt.

Dirty, untracked, or ignored worktrees are never removed. Reflog-only and
unreachable commits receive an explicit policy disposition; `unresolved`
blocks retirement.

### REQ-WTC-005 — local-only, exact execution

The tool never writes remote refs or invokes `git push`. It performs only
manifest-listed local worktree removals and local-head deletions. Worktree
removal retains the branch by default. Local-head deletion uses the witnessed
expected old SHA and fails if it changed or remains used by a retained
worktree.

The primary checkout, locked worktrees, detached worktrees, owned targets, and
non-clean worktrees are never retirement targets.

### REQ-WTC-006 — concurrency and crash safety

Consolidation requires an explicit freeze attestation. The freeze covers all
writes to retiring paths, including ignored/generated files. The tool also
uses a cooperative process lock, revalidates every target immediately before
action, and invokes `git worktree remove` without `--force`.

Bundles are written through a no-clobber temporary file, fsynced, verified,
published without overwriting an existing final file, and followed by a parent
directory fsync. A failed or changed action stops the batch and writes an
honest partial receipt.

### REQ-WTC-007 — safe evidence locations and privacy

Manifests, attestations, and receipts reside directly under one
operator-designated evidence directory outside all registered worktrees. The
archive and evidence directories must be resolved, non-symlink-ambiguous,
effective-user-owned, and non-group/world-writable. Dry-run binds their device,
inode, ownership, mode, and no-symlink attributes; consolidate rechecks them.

Reports contain only metadata and counts, never source payloads from working
trees, prompts, secrets, or untracked-file contents.

## 3. Non-requirements

- Automatic merge, cherry-pick, rebase, reset, checkout, clean, force-push, or
  remote branch deletion.
- Treating a successful dry run as proof that an operator has frozen unrelated
  writers.
- Consolidating a new candidate discovered after a manifest is approved.
- Replacing an ambiguous target with a similarly named or patch-equivalent one.

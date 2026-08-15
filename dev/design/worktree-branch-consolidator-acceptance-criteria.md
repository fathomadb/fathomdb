---
title: Worktree and Branch Consolidator — acceptance criteria
date: 2026-08-13
desc: Observable acceptance criteria for the local worktree and branch consolidator
status: PROPOSED
blast_radius: proposed local operator script, evidence directory, local worktrees, and local refs only
refs:
  - dev/design/worktree-branch-consolidator-requirements.md
  - dev/design/worktree-branch-consolidation.md
  - dev/design/worktree-branch-consolidator.md
---

# Worktree and Branch Consolidator — acceptance criteria

## AC-WTC-001 — audit is observational and complete

Given a primary checkout plus linked worktrees and local heads, `audit` emits a
canonical snapshot with repository identity, worktree/branch/ref classification,
cleanliness counts, baseline SHA, reflog/unreachable candidate data, and an
owner-map hash. It creates no file, ref, index, worktree, lock, bundle, or
receipt. A snapshot without complete owner evidence is marked non-executable.

## AC-WTC-002 — manifest planning is deterministic and bounded

Given the same snapshot, policy, owner map, and goal, `manifest` emits identical
canonical plan content and plan SHA. It blocks a target below the active-theme
lower bound or above policy maximum, unknown/missing owner evidence, unknown
reflog disposition, and policy/owner-map hash mismatch. The derived manifest ID
and bundle basename are non-recursive and deterministic.

## AC-WTC-003 — candidate approval is distinct from generation

A candidate manifest alone is not executable. `dryrun` and `consolidate`
require an unexpired approval attestation that binds the exact full manifest
SHA-256, repository identity, reviewer, approval decision, and time bounds.
Changing either document, using a future/expired attestation, or placing it
outside the evidence directory fails closed.

## AC-WTC-004 — dry run rehearses the exact plan

Given an approved manifest and matching current state, `dryrun` re-audits and
validates all target witnesses, owner/policy/baseline hashes, archive/evidence
destination attributes, and ordered prospective actions. It writes only its
receipt under the evidence directory and produces no Git mutation. Its receipt
binds the exact manifest, approval, baseline, owner map, snapshot, archive, and
evidence locations. Any drift yields a non-zero result and no receipt usable by
`consolidate`.

## AC-WTC-005 — consolidate is manifest-exact and preflight-gated

`consolidate` refuses to run without the exact manifest confirmation, matching
approval/baseline/dry-run/freeze evidence, matching owner map, and unexpired
time bounds. It rejects changed snapshot/baseline/policy/owner-map data,
replaced archive/evidence directories, primary/locked/detached/dirty targets,
or targets in use by retained worktrees.

## AC-WTC-006 — preservation precedes retirement

Before its first retirement action, `consolidate` creates and verifies a
standalone no-prerequisite bundle, proves every required tip and selected
reflog candidate is included, hashes it, and records a typed preservation
receipt containing verifier output and exact bundle inputs. The final execution
receipt binds that preservation receipt. An
existing final archive name, verification failure, missing coverage, or
publication failure stops all retirement actions.

## AC-WTC-007 — retirement actions are narrow and recoverable

For a valid small fixture manifest, consolidate removes only manifest-listed
clean linked worktrees and retains their branches unless a separately listed
local-head deletion follows. It deletes a local head only through expected-old
SHA comparison after confirming no retained worktree uses it. It never calls
`git push`, `git worktree remove --force`, `git reset`, `git clean`, or a
semantic integration command.

## AC-WTC-008 — interruption and race behavior is honest

Any failed action stops later actions and emits a partial execution receipt.
Detected drift or a Git safety refusal is an abort. The tool documents that
undetected writes, especially ignored/generated-file writes, are prevented by
the human freeze attestation rather than made atomic by the tool.

## AC-WTC-009 — end-state verification

After a successful fixture consolidation, a new audit proves retired targets
are absent, retained refs remain reachable, preservation evidence verifies,
and the fixed-point measures do not increase unresolved or dirty-uncaptured
state. No remote ref changes occur.

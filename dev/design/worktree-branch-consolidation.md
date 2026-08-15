---
title: Worktree and Branch Consolidation Protocol
date: 2026-08-13
desc: Preservation-first formal procedure for converging repository checkout topology
status: PROPOSED
blast_radius: local git worktrees and refs only; no product API or release contract
---

# Worktree and Branch Consolidation Protocol

## 1. Purpose and scope

This protocol makes local worktree and branch consolidation a controlled,
iterative operation. It prevents a cleanup from destroying active work,
uncommitted artifacts, or the only reachable copy of a commit.

It is a local-repository operations specification. It does not authorize a
specific deletion, force-push, remote branch change, reset, or checkout. Each
retirement still requires an explicitly approved, exact manifest.

The target working range is five to seven worktrees, set by the approved policy
for the current iteration:

1. clean primary checkout on `main`;
2. benchmark campaign;
3. EARP;
4. 0.8.24 performance integration;
5. 0.8.23 CUDA/Memex integration;
6. temporary legacy triage, removed after closure; and
7. one explicitly justified temporary integration worktree, if needed.

## 2. Model

Let repository state be:

```text
S = (W, R, C, P, K)

W = registered worktrees
R = local and remote-tracking refs
C = tracked, untracked, and ignored working-tree content
P = preservation evidence (bundles, hashes, artifact manifests)
K = classification and ownership records
```

Every worktree and local ref must have exactly one classification:

```text
protected-active | integration-required | archive-required |
merged-retirable | unresolved
```

`unresolved` is deliberately non-retirable. A result such as “no unique patch
according to `git cherry`” is evidence for review, not a classification.

## 3. Safety invariants

The following invariants must hold before and after every transition.

### I1. Recoverability

Every pre-existing commit and every pre-existing tracked, untracked, or ignored
file is either retained in a surviving worktree, incorporated into an approved
commit, or recoverable from a hash-verified archive.

### I2. No active-owner retirement

No worktree or ref with an active writer, an assigned owner, a locked worktree,
or an unresolved release-board role is retired.

### I3. Exact-manifest authority

Only targets enumerated in the approved manifest may be changed. Discovery of
additional apparently eligible targets does not expand that manifest.

### I4. Fresh-evidence requirement

Ancestry, worktree cleanliness, ref use, and recovery evidence are measured
against a freshly recorded repository snapshot immediately before a retirement
batch. Historic audit output is never sufficient evidence. The baseline ref's
SHA and an operator-attested fetch time are part of that snapshot; a locally
present `origin/main` without freshness evidence is suitable for analysis only,
not retirement.

### I5. Primary-checkout protection

The primary checkout is never switched, reset, removed, or cleaned while it
contains unresolved changes. Its content must first be separated into named,
reviewable commits or a verified archive.

### I6. Remote non-interference

This protocol does not alter remote refs. Any remote branch action is a separate
operation with separate explicit approval.

### I7. Quiescent, revalidated execution

Before RETIRE, an accountable operator attests that the named writers and
automations are frozen for the manifest's short validity window. The
consolidator's local lock coordinates only other cooperative consolidator
instances; it cannot prevent unrelated Git commands. It must therefore
revalidate the snapshot and each target immediately before every destructive
action. A failed revalidation invalidates the batch rather than selecting a
replacement target.

Worktree removal has no compare-and-remove primitive. It is safe only under the
operator-controlled freeze assumption, which must prevent all writes to the
retiring path, including ignored and generated files. Invoke `git worktree
remove` without `--force` and treat its own safety rejection as defense in
depth and an abort, never as proof that the tool made removal atomic.

## 4. Retirement proof obligation

For target `x`, `Retire(x)` is permitted only if:

```text
Retire(x) ⇒
  x ∈ approved_manifest
  ∧ x ∉ protected_active_set
  ∧ no active writer owns x
  ∧ content(x) satisfies I1
  ∧ recoverable(x, P)
  ∧ unused_by_retained_worktree(x, W)
  ∧ fresh_snapshot_valid(x)
```

For a worktree, `content(x)` includes tracked, untracked, and ignored files.
For a branch, `recoverable(x, P)` requires at least one of:

- its tip is reachable from a retained ref;
- its content was intentionally integrated and that relation was reviewed; or
- an execution receipt proves that a verified Git bundle contains the tip and
  its required ancestors.

The approved manifest cannot contain the hash of a bundle that does not yet
exist. It instead names the required old tips and deterministic archive scope.
The append-only execution receipt records the created bundle path, SHA-256,
`git bundle verify` result, and coverage of each required tip before any target
is removed.

Patch equivalence alone never proves recoverability because two branches can
have the same patch relation while producing distinct final trees or carrying
distinct provenance.

## 5. State machine

```text
AUDIT → CLASSIFY → INTEGRATE → MANIFEST → APPROVE → DRYRUN
  ↑                                                    ↓
  └───────── VERIFY ← RETIRE ← PRESERVE ← FREEZE ←────┘
                    ↓
               FIXED_POINT
```

### AUDIT

Record the complete current topology: worktree path, branch or detached state,
lock status, cleanliness, tracked/untracked/ignored change counts, branch tip,
ancestry, upstream/ref status, and active-process/owner evidence.

### PRESERVE

Before retiring a target:

1. create a complete local-ref Git bundle;
2. hash the bundle and store its manifest outside the target worktrees;
3. capture dirty/untracked/ignored working-tree artifacts by a reviewed method;
4. inspect reflogs, especially after an earlier cleanup, and include any
   recoverable lost tip in the preservation evidence; and
5. verify that the bundle contains every manifest-required old tip and every
   selected reflog recovery candidate before any retirement.

The bundle proves Git-object preservation; it does not preserve uncommitted
files. Artifact capture is independently required.

### CLASSIFY

Assign every worktree/ref exactly one class. The classification manifest must
state the owner, evidence, proposed successor or archive, and the reason a
target is not `unresolved`. Audit also records locally reachable reflog-only or
unreachable commit candidates. Each candidate must be explicitly retained in a
recovery bundle, promoted to a preservation ref, or classified `unresolved`;
silently omitting it is not allowed.

### INTEGRATE

Integrate active content by theme rather than choosing a branch mechanically.
The current intended themes are EARP, 0.8.24 performance, 0.8.23 CUDA/Memex,
benchmark campaign, and legacy triage. A theme may reduce to one branch only
after a code-grounded review verifies which commits and working-tree artifacts
belong in its integration head.

### MANIFEST

Solve the approved target-worktree goal into one exact retirement manifest for
a small batch. The manifest lists only targets whose proof obligation is
complete. It is a candidate plan, not execution authority.

### APPROVE

An independent reviewer approves the exact candidate manifest and its hash.
Approval does not authorize any targets discovered after manifest generation.
The approval is a separate immutable attestation, not a mutable field inside
the candidate manifest.

### DRYRUN

Re-audit the current repository without mutating it, then simulate the approved
manifest's ordered PRESERVE and RETIRE actions. The dry run must prove that the
manifest snapshot, baseline, target witnesses, archive destination, and
prospective post-state remain valid. It emits a short-lived receipt bound to
the manifest and approval-attestation hashes. A changed current state
invalidates the manifest rather than causing a refreshed plan to be substituted
silently.

### FREEZE

After a successful dry run, coordinate all potential writers and record the
freeze attestation described above. The final retirement transaction repeats
the dry-run checks because the dry-run receipt cannot eliminate a later race.

### RETIRE

Apply only the approved batch. Worktree removal and local branch deletion are
separate transition types. Branches are retained by default when removing a
worktree unless their separate branch proof is satisfied.

### VERIFY

Re-run AUDIT and prove I1–I7. Verify the bundle hash, retained integration
heads, artifact pointers, ref reachability, and the absence of retired targets
from remaining worktree use. Any failed proof returns the state to CLASSIFY.

### FIXED_POINT

The process is complete only when all of these predicates hold:

```text
unresolved_worktrees = 0
unresolved_local_refs = 0
dirty_uncaptured_worktrees = 0
duplicate_active_heads = 0
active_worktrees ≤ approved_target_worktrees
all_retired_targets_recoverable = true
```

## 6. Progress measure

Use this non-negative measure to prevent cleanup that merely moves risk:

```text
D(S) = 10 × unresolved_worktrees
     + 10 × dirty_uncaptured_worktrees
     +  3 × duplicate_active_heads
     +      surplus_clean_worktrees
     +      surplus_local_refs
```

`surplus_local_refs` is the count of local heads classified
`merged-retirable` after excluding protected/integration heads and heads named
as required recovery refs. `unresolved_local_refs` is the count of local heads
or selected reflog candidates whose disposition is unknown. Neither measure
permits deletion: they merely state the remaining, proven retirement work.

An approved iteration must strictly decrease `D(S)` without increasing either
`unresolved_worktrees` or `dirty_uncaptured_worktrees`. If it cannot, stop and
return to CLASSIFY. Reducing branch count while losing recoverability is not
progress.

## 7. Required evidence manifest

The consolidation manifest is durable state, not a chat summary. It contains:

| Field | Requirement |
| --- | --- |
| repository identity | resolved primary root and Git common-directory paths |
| snapshot | timestamp, HEAD, Git version, and baseline ref/SHA |
| policy | canonical policy hash, bound from MANIFEST through all later evidence |
| baseline freshness | operator-attested fetch time and bounded expiry, matching the snapshot baseline SHA |
| target | exact worktree path or ref name |
| class | one of the five classifications in §2 |
| owner | active owner or explicit `none` after inspection |
| content | tracked/untracked/ignored status and artifact-capture pointer |
| ancestry | tip SHA, upstream relation, and retained successor if any |
| recovery plan | required old tips, local-ref/reflog scope, and archive filename; never a future bundle hash |
| disposition | retain, integrate, archive, or retire; planning-only, never an implicit operation |
| executable action | for retired entries only: `remove-worktree` or `delete-local-ref` |
| approval | immutable independent approval attestation of the candidate hash |
| dry run | successful, short-lived simulation receipt bound to the manifest hash |
| freeze | short-lived operator freeze attestation and its expiry |
| verification | post-transition snapshot and invariant result |

The evidence series is append-only per iteration. Each candidate manifest is
immutable; a retrospective change creates a new manifest, attestation chain,
and evidence entry rather than rewriting prior evidence.

## 8. Initial convergence order

The order is mandatory because each later step relies on the earlier proofs:

1. preserve and resolve the primary checkout;
2. preserve every dirty or artifact-bearing worktree;
3. establish one integration worktree per active theme;
4. merge, archive, or explicitly abandon only after per-theme review;
5. retire direct-ancestor and exact-duplicate clutter in small manifests;
6. triage legacy residuals and remove the temporary triage worktree;
7. run one final fixed-point verification.

Do not begin with clean historical worktrees merely because they are numerous.
They are low-risk only after the preservation baseline exists and the active
topology is understandable.

## 9. Failure handling

| Failure | Required response |
| --- | --- |
| Target differs from approved manifest | Stop; no substitute target is allowed. |
| Dirty or untracked content appears | Return to PRESERVE. |
| Branch/tree relationship is ambiguous | Mark `unresolved`; return to CLASSIFY. |
| Bundle/hash verification fails | Stop all retirements; repair preservation evidence. |
| Reflog or unreachable candidate lacks a disposition | Mark `unresolved`; preserve it or return to CLASSIFY. |
| A deletion exceeds scope | Stop immediately; record exact refs/paths, inspect reflogs, and restore only with explicit approval. |
| New writer/process appears | Return to FREEZE. |
| Baseline, dry-run, or freeze attestation expires or mismatches | Stop; perform a new audit and approval cycle. |
| Per-action revalidation differs | Stop the batch; issue a partial receipt and return to AUDIT. |

## 10. Non-goals

- Rewriting history or force-pushing remotes.
- Treating a green test or an ancestor relation as proof that uncommitted
  artifacts are safe.
- Automatically deleting branches based on naming, age, or patch-equivalence.
- Using this protocol to schedule product work or decide release scope.

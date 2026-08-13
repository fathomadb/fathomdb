# Worktree and branch consolidation

`scripts/worktree-consolidator.py` is a local, preservation-first tool for
reducing a repository's worktrees and local branches. It is an operator tool,
not a release command and not a Git-history rewriting tool.

Use it when a repository has accumulated stale, merged worktrees or local
heads and you want to converge on a deliberate working set—often five to seven
worktrees—without treating “looks old” or “has no unique patch” as deletion
authority.

## What it does and does not do

The tool follows this pipeline:

```text
audit → manifest → independent approval → dryrun → freeze → consolidate
```

Each stage produces evidence for the next one. `manifest` is the planning step;
`consolidate` does not solve a new plan while it runs.

It can only remove a clean, non-primary linked worktree explicitly listed in
the approved manifest. It can delete a local branch only when that separately
listed action carries the expected old SHA and no retained worktree uses it.
Before either action, it writes and verifies a Git bundle that covers local
refs, manifest-required tips, and the selected recovery candidates.

It never runs `push`, `reset`, `clean`, `checkout`, `merge`, `rebase`,
`cherry-pick`, `commit`, or `git worktree remove --force`. It does not preserve
uncommitted files in a bundle: dirty, untracked, and ignored worktrees are
classified as non-retirable and must be archived or otherwise resolved before
planning.

## The safety model

An audit classifies each worktree and local head as one of:

| Classification | Meaning |
| --- | --- |
| `protected-active` | Primary checkout or an owned/release-role target; retain it. |
| `integration-required` | Still used by a retained worktree or integration role; retain it. |
| `archive-required` | Has tracked, untracked, or ignored changes; preserve its files first. |
| `merged-retirable` | Clean, unowned, linked worktree or unused local head whose tip is an ancestor of the chosen baseline. It is a candidate, not permission to remove it. |
| `unresolved` | Missing ownership evidence, detached/locked state, or unproven ancestry; stop and resolve it. |

The tool fails closed. A stale baseline, changed worktree, occupied evidence
path, expired approval, missing owner-map entry, or extra recovery candidate
stops the current run. A `git cherry` result is advisory only; it is never a
retirement proof.

## Before starting

1. Work from a clean, deliberate operator checkout. Do not use the primary
   checkout as a cleanup target.
2. Fetch the baseline you intend to use, normally `origin/main`, and record
   its resolved SHA and fetch time in the baseline attestation below.
3. Create two private directories outside every registered worktree:

   ```bash
   install -d -m 700 /secure/fathomdb-wtc/evidence
   install -d -m 700 /secure/fathomdb-wtc/archive
   ```

   The effective user must own both directories and neither may be
   group/world writable or a symlink. The evidence directory holds all input
   attestations, the manifest, and receipts. The archive directory holds the
   verified bundle.
4. Coordinate human and automated writers. The final freeze is an explicit
   operator promise; the tool's lock coordinates only other consolidator
   processes.

The tool accepts only canonical JSON: UTF-8, sorted object keys, no indentation
or trailing newline. Generate operator-authored JSON with a serializer that
produces those exact bytes. `jq -cS` is useful for sorting and compacting, but
its usual newline means its output is not directly acceptable. Do not
hand-format the examples below before use.

## 1. Audit the current topology

Create a complete owner map first. It must include exactly one entry for every
registered worktree path and every local `refs/heads/*` reference. Use `none`
only after reviewing that target; do not omit an uncertain target.

```json
{"entries":[{"evidence":"primary checkout","owner":"release-operator","release_role":"main","target":"/repo/main"},{"evidence":"reviewed merged worktree","owner":"none","release_role":"none","target":"/repo/old-slice"},{"evidence":"primary branch","owner":"release-operator","release_role":"main","target":"refs/heads/main"},{"evidence":"reviewed merged branch","owner":"none","release_role":"none","target":"refs/heads/old-slice"}],"schema":"fathomdb-worktree-owner-map/v1"}
```

Save it as `/secure/fathomdb-wtc/evidence/owner-map.json`, then audit:

```bash
scripts/worktree-consolidator.py audit \
  --repo /repo/main \
  --baseline origin/main \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json \
  --json > /secure/fathomdb-wtc/evidence/audit.json
```

`audit` makes no repository or report-file mutation. Its stdout is the snapshot,
so redirecting it is the operator's explicit decision. Review every class,
cleanliness count, local head, and `recovery_candidates` entry before planning.

## 2. Attest the baseline and write the policy

The baseline attestation binds the audit's exact `baseline` object. Substitute
the `repository` and `baseline` objects copied from `audit.json`; timestamps
must be RFC3339 and within the policy's maximum age.

```json
{"baseline":{"ref":"origin/main","sha":"<audit-baseline-sha>"},"expires_at":"2026-08-13T18:15:00Z","fetched_at":"2026-08-13T18:00:00Z","issued_at":"2026-08-13T18:00:00Z","repository":{"git_common_dir":"/repo/main/.git","primary_root":"/repo/main"},"schema":"fathomdb-worktree-baseline-attestation/v1"}
```

The policy specifies the desired working set. Every active theme maps to a
distinct existing **worktree path** classified `protected-active` or
`integration-required`; a branch name is not a theme target. Every recovery
candidate found in the audit must have exactly one disposition.

```json
{"active_themes":["campaign","earp"],"baseline_max_age_seconds":900,"dryrun_max_age_seconds":900,"legacy_triage_required":false,"primary_role":"main","reflog_candidates":{"<audit-recovery-sha>":"preserve-in-bundle"},"retire_local_heads":false,"target_range":[5,7],"theme_targets":{"campaign":"/repo/campaign","earp":"/repo/earp"}}
```

Use `{}` for `reflog_candidates` only when the audit listed none. Set
`retire_local_heads` to `true` only when separately reviewed unused local heads
may be included; worktree removal otherwise retains its branch.

## 3. Generate and review a manifest

Ask for an explicit target, or use `--infer-target` to apply the policy lower
bound. A numerical target that cannot be reached using only proven candidates
returns exit code `3` and changes nothing.

```bash
scripts/worktree-consolidator.py manifest \
  --repo /repo/main \
  --audit /secure/fathomdb-wtc/evidence/audit.json \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json \
  --policy /secure/fathomdb-wtc/evidence/policy.json \
  --baseline-attestation /secure/fathomdb-wtc/evidence/baseline.json \
  --evidence-dir /secure/fathomdb-wtc/evidence \
  --target-worktrees 6 \
  --output /secure/fathomdb-wtc/evidence/manifest.json \
  --json
```

Review the manifest's `entries`, `goal`, baseline, owner-map hash, required
tips, and planned bundle name. It is immutable candidate evidence: do not edit
it after review. If a plan is unsuitable, correct ownership/policy or resolve
the underlying work, then repeat from audit.

An independent reviewer creates a canonical approval attestation referring to
the SHA-256 of the exact `manifest.json` bytes:

```json
{"decision":"approved","expires_at":"2026-08-14T18:00:00Z","issued_at":"2026-08-13T18:00:00Z","manifest_sha256":"<sha256sum-manifest.json>","repository":{"git_common_dir":"/repo/main/.git","primary_root":"/repo/main"},"reviewer":"reviewer-id","schema":"fathomdb-worktree-approval-attestation/v1"}
```

## 4. Dry-run the approved plan

`dryrun` re-audits the current state and validates the exact actions, evidence
directory, archive directory, and all witnesses. It writes only an expiring
dry-run receipt; it does not create a bundle or mutate Git.

```bash
scripts/worktree-consolidator.py dryrun \
  --repo /repo/main \
  --manifest /secure/fathomdb-wtc/evidence/manifest.json \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json \
  --approval-attestation /secure/fathomdb-wtc/evidence/approval.json \
  --baseline-attestation /secure/fathomdb-wtc/evidence/baseline.json \
  --archive-dir /secure/fathomdb-wtc/archive \
  --evidence-dir /secure/fathomdb-wtc/evidence \
  --json
```

Do not regenerate a manifest to work around a dry-run failure. Treat a failure
as drift, correct the cause, and repeat the pipeline so approval covers the
new snapshot.

## 5. Freeze, then consolidate

Once dry-run succeeds, stop identified writers and create the short-lived
freeze attestation. Use the dry-run receipt filename and its SHA-256, along
with the manifest snapshot ID:

```json
{"dryrun_receipt_sha256":"<sha256sum-dryrun-receipt.json>","expires_at":"2026-08-13T18:20:00Z","issued_at":"2026-08-13T18:05:00Z","manifest_sha256":"<sha256sum-manifest.json>","operator":"operator-id","repository":{"git_common_dir":"/repo/main/.git","primary_root":"/repo/main"},"schema":"fathomdb-worktree-freeze-attestation/v1","snapshot_id":"<manifest-snapshot-id>","writers":["scheduled-indexer"]}
```

Confirm both the content hash and manifest ID at the terminal:

```bash
scripts/worktree-consolidator.py consolidate \
  --repo /repo/main \
  --manifest /secure/fathomdb-wtc/evidence/manifest.json \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json \
  --approval-attestation /secure/fathomdb-wtc/evidence/approval.json \
  --baseline-attestation /secure/fathomdb-wtc/evidence/baseline.json \
  --dryrun-receipt /secure/fathomdb-wtc/evidence/dryrun-<hash>.json \
  --freeze-attestation /secure/fathomdb-wtc/evidence/freeze.json \
  --archive-dir /secure/fathomdb-wtc/archive \
  --evidence-dir /secure/fathomdb-wtc/evidence \
  --confirm-manifest-sha256 "<sha256sum-manifest.json>" \
  --confirm "CONSOLIDATE <manifest-id>" \
  --json
```

Immediately before the first action, the tool reserves all deterministic
receipt names, builds and verifies the no-prerequisite bundle, and writes a
preservation receipt. It then re-audits and revalidates every entry before
acting. Successful actions receive progress receipts; the final receipt records
the post-snapshot and bundle linkage.

## Results and recovery

| Exit code | Meaning | Operator response |
| --- | --- | --- |
| `0` | Requested mode succeeded. | Retain evidence and verify the resulting audit. |
| `1` | Safety precondition failed. No requested cleanup should have started. | Inspect the error, resolve drift/evidence, and restart at the appropriate earlier stage. |
| `3` | `goal_inference_blocked`. | Reduce the target ambition or resolve/protect the uncertain work; do not force a deletion. |
| `4` | Partial batch after preservation. | Stop writers, inspect preservation/progress/partial receipts and the bundle, then decide recovery or a newly audited manifest. |

The archive bundle is a recovery artifact. Verify it with:

```bash
git bundle verify /secure/fathomdb-wtc/archive/refs-before-wtc-<snapshot>-<plan>.bundle
git bundle list-heads /secure/fathomdb-wtc/archive/refs-before-wtc-<snapshot>-<plan>.bundle
```

Run a new `audit` after any successful or partial execution. Do not reuse an
old manifest, approval, dry-run receipt, or freeze attestation for a later
iteration.

For the complete formal protocol and implementation details, see the internal
[consolidation protocol](https://github.com/coreyt/fathomdb/blob/main/dev/design/worktree-branch-consolidation.md)
and [implementation design](https://github.com/coreyt/fathomdb/blob/main/dev/design/worktree-branch-consolidator.md).

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
audit → owner-map review → manifest → independent approval → dryrun → freeze → consolidate
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
path, expired approval, missing owner-map review, missing owner-map entry, or extra recovery candidate
stops the current run. A `git cherry` result is advisory only; it is never a
retirement proof.

An unused non-ancestor local head needs a separately reviewed proof. The proof
does not decide obsolescence; the reviewed owner map and semantic disposition
do. It establishes only complete exact stable-patch coverage on the baseline,
same-tip/ancestor redundancy with one explicitly retained local ref, or an
exact matching remote-tracking ref. The verified recovery bundle remains the
preservation guarantee, and no remote ref is modified.

## Optional semantic triage in Codex

When many worktrees remain, use `scripts/worktree-semantic-triage.py` before
creating the owner map. It collects metadata only, packages bounded tasks for
Codex, and validates structured model suggestions into a **review-only** owner
map draft. It does not call a model, read an API key, inspect source-file or
untracked-file contents, create an attestation, or run Git cleanup.

```bash
scripts/worktree-semantic-triage.py collect \
  --repo /repo/main --baseline origin/main \
  > /secure/fathomdb-wtc/evidence/semantic-casebook.json

install -d -m 700 /secure/fathomdb-wtc/semantic-packets
scripts/worktree-semantic-triage.py packets \
  --runner codex \
  --casebook /secure/fathomdb-wtc/evidence/semantic-casebook.json \
  --output-dir /secure/fathomdb-wtc/semantic-packets --batch-size 8
```

Give each packet to Codex using the internal
`dev/design/worktree-semantic-triage-codex-prompt.md` prompt. Codex returns
one canonical scoped response per packet. Store those responses in a separate
owner-private empty directory, then merge them mechanically before validation:

```bash
install -d -m 700 /secure/fathomdb-wtc/semantic-responses

scripts/worktree-semantic-triage.py merge-decisions \
  --casebook /secure/fathomdb-wtc/evidence/semantic-casebook.json \
  --packets-dir /secure/fathomdb-wtc/semantic-packets \
  --input-dir /secure/fathomdb-wtc/semantic-responses \
  --output /secure/fathomdb-wtc/evidence/semantic-decisions.json
```

Validate the merged document before an operator reviews it:

```bash
scripts/worktree-semantic-triage.py validate \
  --casebook /secure/fathomdb-wtc/evidence/semantic-casebook.json \
  --decisions /secure/fathomdb-wtc/evidence/semantic-decisions.json \
  --owner-map-output /secure/fathomdb-wtc/evidence/owner-map-draft.json \
  --report-output /secure/fathomdb-wtc/evidence/semantic-report.json
```

The report is intentionally labeled `review_required`. Review the proposed
owners/dispositions and create a normal owner-map review attestation for the
accepted map; only then begin this guide's `audit → manifest → dryrun →
consolidate` pipeline. A model's suggestion is never deletion authority.

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

When the owner map marks a local head `none`/`none`, the snapshot's
`retirement_review.entries` gives a compact proof table in JSON. It checks the
three automatable facts: the head is not `main`, no worktree checks it out, and
its tip is reachable from the audited baseline. It also reports same-tip remote
refs and copies the owner-map evidence text. Filter it for a concise terminal
review:

```bash
scripts/worktree-consolidator.py audit \
  --repo /repo/main --baseline origin/main \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json --json \
  | jq -r '.retirement_review.entries[] |
      [.target, .ancestor_of_baseline, .checked_out_by_worktree, .is_main,
       (.matching_remote_refs | join(",")), .result, .owner_map_evidence] | @tsv'
```

`mechanically-eligible` means Git proves those predicates; it does not decide
that the branch's business purpose is obsolete. Review that final policy
judgment, then approve the owner map.

## 2. Review the owner map, then attest the baseline and write the policy

Before a manifest can be generated, an independent reviewer must attest the
exact canonical bytes of `owner-map.json`. This is deliberately before manifest
generation: an LLM or human may help classify the audit, but its proposed owner
map is not authority until a reviewer records this approval.

```json
{"decision":"approved","expires_at":"2026-08-14T18:00:00Z","issued_at":"2026-08-13T18:00:00Z","owner_map_sha256":"<sha256sum-owner-map.json>","repository":{"git_common_dir":"/repo/main/.git","primary_root":"/repo/main"},"reviewer":"reviewer-id","schema":"fathomdb-worktree-owner-map-review-attestation/v1"}
```

Save it as `/secure/fathomdb-wtc/evidence/owner-map-review.json`. If it is
absent, `manifest` and `dryrun` return exit code `3` with
`{"result":"owner_map_review_required",...}` and perform no cleanup.

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

### Optional proof chain for non-ancestor local heads

This capability uses `fathomdb-worktree-consolidator/v2`; regenerate audit and
manifest evidence instead of reusing a version-1 artifact. Create canonical
`fathomdb-worktree-retirement-proofs/v1` metadata with one proof per target:

- `stable_patch_coverage` lists every target-only commit, stable patch ID, and
  all exact baseline matches. Partial, empty, and merge-containing sets fail.
- `retained_local_ref` names one explicit retained local ref and its `same-tip`
  or `ancestor` relation.
- `remote_tracking_ref` names one direct, non-symbolic `refs/remotes/*` ref
  whose tip exactly matches the local target.

The file contains no diffs, contents, messages, prompts, or completions. An
independent reviewer writes a canonical
`fathomdb-worktree-retirement-proof-approval/v1` attestation binding its exact
SHA-256 and the owner-map hash. Author and reviewer identities must be distinct,
untrimmed, printable canonical strings.

Pass both files to `manifest`, then the same immutable files to `dryrun` and
`consolidate`:

```text
--retirement-proofs /secure/fathomdb-wtc/evidence/retirement-proofs.json
--retirement-proof-approval /secure/fathomdb-wtc/evidence/proof-approval.json
```

Every stage recomputes the relation from live Git. Drift requires a new audit,
proof review, manifest review, and dry run.

## 3. Generate and review a manifest

Ask for an explicit target, or use `--infer-target` to apply the policy lower
bound. A numerical target that cannot be reached using only proven candidates
returns exit code `3` and changes nothing.

```bash
scripts/worktree-consolidator.py manifest \
  --repo /repo/main \
  --audit /secure/fathomdb-wtc/evidence/audit.json \
  --owner-map /secure/fathomdb-wtc/evidence/owner-map.json \
  --owner-map-review-attestation /secure/fathomdb-wtc/evidence/owner-map-review.json \
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
  --owner-map-review-attestation /secure/fathomdb-wtc/evidence/owner-map-review.json \
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
  --owner-map-review-attestation /secure/fathomdb-wtc/evidence/owner-map-review.json \
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
| `3` | `goal_inference_blocked` or `owner_map_review_required`. | Reduce the target ambition, or have an independent reviewer approve the exact owner map; do not force a deletion. |
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

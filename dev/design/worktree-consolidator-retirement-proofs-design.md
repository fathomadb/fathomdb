---
title: Worktree Consolidator Retirement Proofs — design
date: 2026-08-15
desc: Hash-bound design for three live-revalidated local-ref redundancy proofs
status: PROPOSED
blast_radius: worktree consolidator inputs, manifests, receipts, and local refs
refs:
  - dev/design/worktree-branch-consolidator.md
  - dev/design/worktree-consolidator-retirement-proofs-requirements.md
  - dev/design/worktree-consolidator-retirement-proofs-acceptance-criteria.md
---

# Worktree Consolidator Retirement Proofs — design

## Authority sequence

```text
owner map → independent owner-map review
  → semantic proof set → independent proof review
  → manifest → independent manifest review → dryrun → freeze → consolidate
```

The semantic layer decides whether a local name may retire. Git proves only one
exact redundancy relation. The closed audit/manifest schema advances to
`fathomdb-worktree-consolidator/v2`; version-1 evidence is regenerated rather
than shimmed.

## Evidence schemas

`fathomdb-worktree-retirement-proofs/v1` contains `repository`, `baseline`,
`owner_map_sha256`, canonical `author`, `issued_at`, and unique `proofs`.
Common entry fields are `target`, `target_tip`, `proof_type`,
`semantic_disposition: retire-local-ref`, and bounded `evidence_id`.

- `stable_patch_coverage` adds ordered `source_commits` entries containing
  `commit`, `stable_patch_id`, and the complete sorted `baseline_matches` set.
- `retained_local_ref` adds `relation`, `retained_ref`, and `retained_tip`.
- `remote_tracking_ref` adds `remote_ref` and `remote_tip`.

`fathomdb-worktree-retirement-proof-approval/v1` contains repository and
owner-map bindings, `retirement_proofs_sha256`, canonical reviewer, approved
decision, issue time, and expiry. Reviewer must differ from author.

The v2 manifest always has `retirement_proofs`: `null` for direct-ancestor-only
plans or `{proofs_sha256, approval_sha256}`. A `proof-retirable` witness adds
`proof_type` and `proof_entry_sha256`. Dry-run receipts repeat both hashes plus
the existing owner-map-review hash.

## Revalidation algorithms

Stable-patch validation enumerates `target --not baseline`, rejects empty sets
and commits with more than one parent, computes config-independent stable patch
IDs from in-memory binary diffs, and compares the evidence with every exact
match in a baseline non-merge patch index. Empty patch output fails.

Retained-local validation resolves exact non-symbolic `refs/heads/*` refs,
requires explicit retained ownership, prohibits self/retirement targets, and
recomputes same-tip or strict-ancestor relation. Remote validation resolves one
exact non-symbolic `refs/remotes/*` ref and requires tip equality.

All proof files are direct children of the existing private evidence directory.
Manifest, dry run, consolidation under the cooperative lock, and immediate
pre-action checks recompute live relations. Earlier completed proof targets are
not re-resolved during later actions, but their immutable schemas/hashes remain
bound.

## Failure, preservation, and privacy

Malformed identities use Unicode category `C` rejection plus exact
`value == value.strip()` comparison. Schema/hash/approval failures stop before
receipts or Git writes. Drift after preservation becomes the existing partial
batch, with the old tip already recoverable from the bundle. Remote refs are
never update targets.

Patch bytes remain only subprocess input. Evidence and receipts persist no
diff, content, message, prompt, completion, secret, or untracked payload.

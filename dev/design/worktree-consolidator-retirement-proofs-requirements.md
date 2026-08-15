---
title: Worktree Consolidator Retirement Proofs — requirements
date: 2026-08-15
desc: Requirements for reviewed non-ancestor local-ref retirement proofs
status: PROPOSED
blast_radius: local consolidator evidence and local refs only
refs:
  - dev/design/worktree-branch-consolidator-requirements.md
  - dev/design/worktree-consolidator-retirement-proofs-acceptance-criteria.md
  - dev/design/worktree-consolidator-retirement-proofs-design.md
---

# Worktree Consolidator Retirement Proofs — requirements

## Requirements

### REQ-WTC-P01 — closed proof set

An unused non-baseline-ancestor local head may be retired only with exactly one
reviewed proof type:

1. `stable_patch_coverage`: every target-only commit is non-merge, non-empty,
   and has an exact stable-patch match on the captured baseline. The evidence
   lists the complete ordered source set and all matching baseline commits.
2. `retained_local_ref`: the target is the same tip as, or an ancestor of, one
   named local ref explicitly retained by the owner map and absent from all
   retirement actions.
3. `remote_tracking_ref`: one named, direct, non-symbolic `refs/remotes/*` ref
   exactly matches the target tip.

Partial coverage, unique merges, empty commits, local/symbolic remote evidence,
and generic integration/archive claims fail closed.

### REQ-WTC-P02 — semantic and owner authority

The existing exact owner-map review remains mandatory at manifest, dry run,
and consolidation. The proof set additionally identifies an accountable author,
the exact target tip, `retire-local-ref` disposition, and a metadata-only
decision identifier. A separate unexpired reviewer attestation approves the
exact proof-set and owner-map hashes. Reviewer and author are distinct canonical
identities: non-empty, unchanged by trimming, and free of Unicode control
characters.

### REQ-WTC-P03 — hash chain and live recomputation

The proof set binds repository identity, captured baseline, and owner-map hash.
The manifest binds the owner-map review, proof-set, and proof-approval hashes;
each proof-backed action binds its proof-entry hash and type. The dry-run receipt
repeats those hashes. Manifest, dry run, consolidation preflight, and the moment
before each proof-backed deletion recompute the selected relation from live Git.
Any moved/missing target, baseline, retained ref, or remote ref fails closed.

### REQ-WTC-P04 — preservation and mutation boundary

The existing standalone verified bundle, freeze, manifest approval, expected-old
SHA deletion, progress receipts, and partial-batch protocol remain mandatory.
The tool deletes no remote ref and performs no integration operation. Successful
tests must recover each deleted proof-backed tip from the bundle in a fresh
repository. Direct-ancestor retirement remains valid without proof inputs.

### REQ-WTC-P05 — privacy and closed schemas

Proof evidence contains only ref names, hashes, patch IDs, relations, timestamps,
accountable identities, and bounded decision identifiers. Unknown fields/types,
diffs, source contents, commit messages, prompts, completions, secrets, or
untracked payloads are rejected or never persisted.

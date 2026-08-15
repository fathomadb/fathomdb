---
title: Worktree Consolidator Retirement Proofs — acceptance criteria
date: 2026-08-15
desc: Observable criteria for reviewed non-ancestor local-ref retirement
status: PROPOSED
blast_radius: local consolidator evidence and local refs only
refs:
  - dev/design/worktree-consolidator-retirement-proofs-requirements.md
  - dev/design/worktree-consolidator-retirement-proofs-design.md
---

# Worktree Consolidator Retirement Proofs — acceptance criteria

## AC-WTC-P01 — stable-patch proof

Complete exact coverage succeeds. Partial coverage, wrong identities, empty
commits, and unique merge commits fail before a manifest is written.

## AC-WTC-P02 — retained-local proof

Same-tip and strict-ancestor relations succeed only for one explicitly retained
local ref. Missing, moved, self-named, unretained, or also-retired anchors fail.

## AC-WTC-P03 — remote proof

One exact direct remote-tracking ref succeeds. A local, symbolic, missing,
nonmatching, or moved ref fails, and no remote ref changes during execution.

## AC-WTC-P04 — accountable approval chain

Missing, rejected, expired, altered, same-author, or hash-mismatched proof
approval fails. Leading/trailing whitespace and every Unicode control character
fail in author/reviewer identities. Unknown schemas, proof types, and payload
fields fail. Existing owner-map review missing/mismatch/expiry gates remain
covered at manifest, dry run, and consolidation.

## AC-WTC-P05 — live drift

Moving target, retained ref, baseline, or remote ref after a successful dry run
causes consolidation to stop before bundle publication or deletion.

## AC-WTC-P06 — end-to-end preservation

Each proof type completes manifest → approval → dry run → freeze → verified
bundle → expected-old-SHA deletion. The local ref disappears, its exact old tip
is recoverable from the bundle, remote refs are byte-identical, and only listed
local refs change. Direct-ancestor behavior remains green without proof files.

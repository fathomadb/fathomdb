---
title: FathomDB 0.8.28 Slice 40 — goal
status: ACTIVE (phase 1, W1–W2 done; R1 next)
target_release: 0.8.28
observed_on: 2026-10-15
---

# Slice 40 goal

Fix two FTS ranking defects found by the COMP-01 NFCorpus benchmark
(FathomDB 0.3087 nDCG@10 against LanceDB 0.3265):

- **A.** RRF fusion credits one arm more than once for a body that several
  rows share, so a byte-identical pair deep in the text arm outranks the
  true rank-1 hit. Each arm must contribute at most once per body;
  cross-arm agreement still accumulates.
- **B.** The query compiler drops 1–2 character tokens ("C", "D3", "II",
  "1"). They must be kept; the stop list stays.

The slice is complete when every AC40 in [requirements.md](requirements.md)
passes, the full gate is green on the final tree, and the codex review
approves. Steps are in [plan.md](plan.md); state is in
[status.md](status.md).

Paste the block below to continue the slice in a fresh session.

```text
Continue FathomDB 0.8.28 Slice 40 (FTS ranking defects) in worktree
/home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-40 on branch
slice/0.8.28-40-fts-ranking-defects. Read AGENTS.md, the memory index, and
dev/plans/0.8.28/features/slice-40/{requirements,design,test-plan,plan,status}.md.
Resume from status.md. Tests are read-only during the fix; do not edit
release-state-0.8.28.json, STATUS-0.8.28.md or generated blocks.
```

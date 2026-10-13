---
title: FathomDB 0.8.28 Slice 31 — goal
status: ACTIVE (owner approved Slice 31 and the W0 merge-forward on 2026-10-11; aligned with plan revision 17: the forks' qualified commits and the published registry crates, the Candle crates 0.11.0 from K9)
target_release: 0.8.28
observed_on: 2026-10-11
---

# Slice 31 goal

Paste the block below as the session goal (or the first prompt of a fresh
session) to continue Slice 31 until it is complete. The plan is
[plan.md](plan.md); the plan governs where this summary is shorter.

```text
Complete FathomDB 0.8.28 Slice 31 exactly as specified in
dev/plans/0.8.28/features/slice-31/plan.md (on release/0.8.28): a CUDA
memory pool done with excellence. Read the plan, AGENTS.md and the memory
index first. Read the plan with
`git show origin/release/0.8.28:dev/plans/0.8.28/features/slice-31/plan.md`
or from a slice worktree; never switch branches in the primary checkout
(/home/coreyt/projects/fathomdb). If the committed plan is older than
revision 17, stop and report. Fork work is governed by each fork's own
plan: coreyt/cudarc branch fathomdb/0.19.10, fathomdb/plan.md (clone
/home/coreyt/projects/fathomdb-forks/cudarc) and coreyt/candle-fathomdb
branch fathomdb/0.8.28-cudarc-fork, fathomdb/plan.md (clone
/home/coreyt/projects/fathomdb-forks/candle-fathomdb); read both. W0 step 2
(merge-forward) is done at 9fa71e93a. Resume from status.md
(dev/plans/0.8.28/features/slice-31/status.md): before the slice merges it
is on branch slice/0.8.28-31-cudarc-fork in worktree
/home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-31; after the merge,
on branch slice/0.8.28-31-post in worktree
/home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-31-post. If neither
exists yet, start at plan § 9 W0.

Outcome:
- coreyt/cudarc: branch fathomdb/0.19.10 cut from upstream v0.19.10
  (ec1cac71; no tag pushed) with FathomDB's allocator fallback (items 1-4)
  and private memory pool (items 5-6) re-applied as small upstream-style
  commits, fixed for all 19 cudarc CUDA features (cuda-11040 through
  cuda-13040), packaged as cudarc-fathomdb. Planned, test-planned, coded,
  reviewed and qualified inside the fork (its fathomdb/ requirements,
  design, test plan, review, status and qualification receipts, including
  the driver-only 13.0/13.2/13.4 GPU cells and the Orin cells).
- coreyt/candle-fathomdb: branch fathomdb/0.8.28-cudarc-fork depending on
  cudarc-fathomdb at the qualified cudarc commit C, with its own fathomdb/
  record and receipts.
- fathomadb/fathomdb: release/0.8.27 merged forward into release/0.8.28;
  slice/0.8.28-31-cudarc-fork pins both forks, deletes
  third_party/cudarc-0.19.7 and every reference to it, keeps exactly one
  cudarc 0.19 in the graph, and is merged into release/0.8.28 after
  qualification on the Jetson AGX Orin (10.83.10.13; ssh/scp approved) and
  on windchill3 (CUDA 12.6), with the CUDA 13.0 toolkit cells on an
  owner-approved GCP VM with driver branch 580 (plan § 15 Q15). FathomDB pins only fork commits whose fork reviews and receipts
  pass (plan § 3.1 pin chain); it does not re-run the forks' own tests.
- A tidy, unposted cudarc memory-pool proposal on coreyt/cudarc branch
  proposal/mem-pool based on upstream main, with issue and PR drafts in the
  fork's fathomdb/proposal/.
- Publication-ready crates; after the owner publishes, the patch-drop
  follow-up is merged and a crates.io consumer with no patch table builds
  tegra-pool and passes the Orin GPU smoke.

Rules:
- Main thread runs the slice. Do not use the steward or orchestrator roles
  or agents. The main thread creates every tag, branch, worktree and clone.
  Subagents per plan § 11: implementer subagents (one per checkout; briefs
  give path, branch, output.json path, and replace the definition's codex
  reviewer with an Opus review), research subagents for NVIDIA/cudarc/Candle
  documentation, and a Claude agent on the Orin only for long qualification
  runs. One writer per branch: never commit on the slice branch while an
  implementer is running there.
- Every review is an independent Opus subagent (model "opus", effort high,
  general-purpose, read-only), not codex, per plan § 12: in the cudarc fork
  R-FC-REQ, R-FC-DES, R-FC-TEST, R-FC-CODE, R-FC-VERIFY, R-PROP, R-FC-PUB;
  in the Candle fork R-FK-PLAN, R-FK-CODE, R-FK-VERIFY, R-FK-PUB; in
  FathomDB R-REQ, R-DES, R-TEST, R-CODE-3, R-VERIFY, R-CODE-4. Record
  model, findings and dispositions in the reviewed repository's review.md
  (FathomDB's review.md gets one line per fork review); fix findings RED
  then GREEN before the blocked step. Reviewers re-run each RED sha.
- Forks (plan § 3.1): both are GitHub forks, so NEVER run gh pr create,
  gh issue create or gh repo set-default in a fork clone and never open a
  PR from the web UI (the base defaults to upstream). Upstream push URLs
  stay disabled. Every commit on a fathomdb/* branch carries a Fork-Class
  trailer (U, L, P, W, D; none on proposal/*); engineering records are D
  commits touching only fathomdb/. Pins name qualified code commits;
  receipts follow as D commits naming them. FathomDB consumes the
  published registry crates (cudarc-fathomdb =0.19.10 from C, the Candle
  crates =0.11.0 from K9, plan § 15 Q21) and keeps only candle-kernels as
  a git patch at K9 (plan revision 17, § 9 W6). Publish from the qualified
  commits (cudarc from C, Candle from the re-qualified K9).
- TDD for all code: the failing test first (RED), then the code (GREEN),
  then refactor. Test files are read-only during fix-to-spec.
- Cite primary CUDA release notes and Driver API docs for every CUDA-version
  decision in design.md. Search outside PATH (/usr/local/cuda-*, $HOME/opt,
  pip nvidia/cu13 wheels, /opt) before concluding something is missing.
- windchill3 safety (owner rulings Q4, Q6): NEVER install, update or remove
  any NVIDIA driver (it stays 580.178.04; no R615) and NEVER change
  /usr/local/cuda (stays 12.6). No CUDA 13.0 toolkit is installed on
  windchill3 (Q15); 13.0 toolkit cells run on a GCP VM, and Spot VMs are
  for test and verification only, never development. Record nvidia-smi driver and readlink -f /usr/local/cuda before
  and after every install or qualification run; stop on any change.
- Never write to release/0.8.27 (merging it forward into release/0.8.28 is
  allowed). Never push to Memex or any upstream repository; never open
  upstream issues or PRs. Fork and FathomDB branch pushes need no prompt;
  push explicit refs.
- Owner actions — stop and ask with the exact command or diff ready, and
  continue every step that does not depend on them: each GCP VM session
  (plan § 15 Q15); each pinned-rev or registry-version
  advisory review (R-PIN); each crates.io publication (R-PUB, W8); any
  Orin reflash (not in Slice 31; Tegra on CUDA 13 is UNMEASURED, Q5).
- If the owner declines an owner action: record it in status.md and with
  ledgerwrite, mark the dependent cells UNMEASURED by owner decision
  (R-VERIFY confirms), and continue. A declined or pending R-PIN or R-PUB
  pauses the goal at that point; it does not fail it.
- The full gate runs in the slice worktree with its own .venv created by
  scripts/bootstrap.sh inside it; never borrow or modify the primary
  checkout's .venv; never pip install -e or maturin develop against it.

Done when:
1. Pre-merge: AC31-01..08, 10, 11, 13..16 and AC31-12 so far pass with
   evidence in status.md, qualification.md and the forks' receipts; CUDA
   13.1-13.4 runtime cells and Tegra on CUDA 13 are recorded UNMEASURED
   citing owner rulings Q4 and Q5; every fork review through R-FC-VERIFY and
   R-FK-VERIFY, and R-CODE-3 and R-VERIFY, are recorded with no open
   finding; the slice is merged into release/0.8.28 and its worktree
   removed.
2. Post-merge: AC31-09 passes and R-PROP is recorded with no open finding;
   the proposal is not posted. The owner holds FC9 and FC9b (plan § 15
   Q17); until released, the goal skips this step.
3. Post-publication (after the owner publishes): R-FC-PUB, R-FK-PUB and
   R-CODE-4 are recorded with no open finding, AC31-08 (re-checked from
   the registry) and AC31-12 (final) pass, and the follow-up is merged.
If the owner has not yet published, the goal pauses after step 2 with the
publication commands and dry-run output ready; it is not failed.
```

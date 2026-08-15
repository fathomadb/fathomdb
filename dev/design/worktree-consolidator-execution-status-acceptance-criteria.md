---
title: Worktree Consolidator Execution Status — acceptance criteria
date: 2026-08-15
desc: Verifiable criteria for namespace-independent execution monitoring
status: PROPOSED
blast_radius: local consolidator read-only status mode and execution evidence only
refs:
  - dev/design/worktree-consolidator-execution-status-requirements.md
  - dev/design/worktree-consolidator-execution-status-design.md
---

# Worktree Consolidator Execution Status — acceptance criteria

| ID | Criterion | Verification witness |
| --- | --- | --- |
| AC-WTC-S01 | A present lock yields `executing` even when any PID-liveness probe would fail; the report states `liveness: unknown` and offers no lock-clear/resume authority. | Unit test makes `os.kill`, `/proc`, and `ps` access fail if attempted and observes an unchanged repository/evidence fingerprint. |
| AC-WTC-S02 | A running execution reports zero, partial, and finalizing progress from deterministic receipts; a final success receipt is not `completed` until the lock is absent. | Fixture tests build canonical preservation/progress/final receipts and exercise each transition, including unreadable, malformed, directory, and broken-symlink locks without reading their contents. |
| AC-WTC-S03 | With no lock, a valid success receipt that covers all ordered actions reports `completed`; a deterministic partial final, randomized fallback partial, or absent final after preservation/progress reports `recovery_required`. A fallback partial dominates a coexisting success final. | Fixture tests assert exact state, counts, and fixed guidance for ordinary final-write failure and final-path-published-then-fsync-failure. |
| AC-WTC-S04 | Missing, non-canonical, foreign, malformed, non-contiguous, or action-prefix-mismatched receipts fail closed. | Negative tests assert non-zero status and no Git/evidence mutation, including bad preservation/final cross-links, 0000/N+1/malformed progress names, duplicate/foreign fallback partials, and symlink/FIFO receipt entries. |
| AC-WTC-S05 | `status` is read-only and metadata-only. | Fingerprint test proves no Git/evidence/lock writes; JSON test rejects PID and payload fields. |
| AC-WTC-S06 | Existing audit, manifest, dryrun, proof, and consolidate behavior remains green. | Existing targeted consolidator suite passes with new status tests. |

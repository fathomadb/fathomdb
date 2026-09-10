---
title: 0.8.25 Slice 72 status
status: COMPLETE
candidate: 2e14f5ba81888a72ccc8616759d6413056268d95
---

# Slice 72 status

Slice 72 is complete. Generic preflight now derives its active release,
baseline, and dependency closure from tracked release state. The installed CE
profile passes on CPU and the pinned RTX 3090 without changing CE semantics,
shipping defaults, score tolerance, or the 10% performance limit.

## Acceptance

| Criterion | Result |
| --- | --- |
| S72-AC1–AC4 | PASS — the live state and exact ladder SHA are authoritative; active, PENDING, and COMPLETE baselines fail closed on malformed identity, reachability, or path escape. Plain health and linked-worktree landing behavior remain covered. |
| S72-AC5 | PASS — exact baseline and candidate CPU/CUDA wheels import from isolated installs and produce the registered standalone reorder plus stable CE-scored `Engine.search` results. |
| S72-AC6 | PASS — CPU resolves CPU; every CUDA process resolves `cuda:0` on `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b` (`NVIDIA GeForce RTX 3090`) with matching-PID allocation evidence. Model bytes are staged read-only and unchanged under the offline policy. |
| S72-AC7 | PASS — all four candidate median-p95 ratios are within `1.10`: CPU standalone `1.0043`, CPU Engine `1.0508`, CUDA standalone `0.9594`, CUDA Engine `1.0962`. CPU/CUDA rank and `1e-2` score-tolerance checks pass. |
| S72-AC8 | PASS — the manifest, 64 raw process logs, four validated cells, and combined receipt bind commits, artifacts, model bytes, runtime identity, timing, RSS/VRAM, and verdict. |

Authoritative evidence is
[`dev/plans/runs/0.8.25-slice-72/receipt.json`](../../../runs/0.8.25-slice-72/receipt.json).
The measured candidate is `2e14f5ba`; the branch carries later manifest and
closeout-only commits.

## Focused verification

- release-current, preflight landing/state, agent-test tier, release-state-view,
  CE validator, scoped Ruff/ShellCheck, and diff checks pass;
- candidate reranker CPU tests: 5 passed;
- candidate reranker CUDA tests: 2 passed on the pinned RTX 3090;
- independent design and code reviews pass; and
- a separate read-only evidence audit reviewed retained results without a
  duplicate campaign.

No full regression, Windows, cross-SDK, hosted CI, full CUDA rehearsal,
publication, tag, registry, or `main` integration was run. Slice 75 retains the
full release verification scope. Slice 73 is next.

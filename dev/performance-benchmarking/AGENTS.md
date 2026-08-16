# Performance-program agent instructions

These instructions apply to all work under `dev/performance-benchmarking/`.

- Follow [TRACK-RUNNER.md](TRACK-RUNNER.md) for every PROGRAM track. Begin with
  `./scripts/track-runner.sh check`, `./scripts/track-runner.sh status`, and
  `./scripts/track-runner.sh brief <TRACK-ID>`.
- Read PROGRAM, PROGRAM-GOALS, the track charter, and the named dependency
  contract before changing code or planning an execution.
- Use one isolated worktree and one writer per active track. The coordinator
  alone edits PROGRAM, TRACK-RUNNER, [TRACK-RUNNER-STATUS.md](TRACK-RUNNER-STATUS.md),
  shared helpers, and integration state; it updates the status board at every
  commission, handoff, review, integration, blocker, and close.
- Complete targeted tests and `./scripts/agent-verify.sh`; request independent
  read-only review before the coordinator integrates a worker commit.
- A plan, a dry static check, or a worker commit does not authorize corpus,
  GPU/model, paid, or external execution. Preserve blocked observations as
  evidence and use the normal safe `experiments` receipt contract.

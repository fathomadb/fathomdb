# Execution, Subagent, and Review Traps

Hard-won operational rules governing day-to-day agent execution in FathomDB.

## Worktree & checkout discipline

- **Place worktrees on `/mnt/data/worktrees/<name>`** (cut off `origin/main`),
  never on the root SSD under `/home/coreyt/projects/fathomdb-worktrees/`.
- **One writer per checkout.** Never run concurrent file-mutating sessions or
  subagents in the same working tree — concurrent writers race on `HEAD`,
  branch switches, and uncommitted edits. Isolate each writer in its own
  worktree or serialize them; read-only subagents (research, audit, review) may
  share a checkout.
- **Verify state from git, never from narration.** Before claiming a branch,
  commit SHA, or test result, run `git rev-parse` / `git status` and read the
  actual command exit code directly.
- **Push explicit branch refs** (`HEAD:refs/heads/<branch>`) after concurrent
  or multi-worktree work rather than assuming current `HEAD` tracking.
- **Sandbox note for worktrees:** Linked worktrees store git metadata under the
  parent repo's `.git/worktrees/`. Sandboxed `git commit` / `git push` commands
  that mount the parent `.git` read-only will fail with `Read-only file system`
  and must run with write access to the parent `.git` metadata.

## Reviews, watchdogs & progress reporting

- **Adversarial review fallback:** Every plan, design, and slice diff requires
  an independent read-only adversarial review before landing. When an external
  reviewer is unavailable or rate-limited, fall back to an independent
  adversarial review subagent rather than stalling.
- **Full-workspace gate before green claims:** Scoped checks are for inner-loop
  iteration; never make a release or full-workspace green claim without running
  full-workspace `clippy`, `check`, and `agent-verify.sh` on a clean tree.
- **2-core constrained concurrency stress before tagging:** High-core dev
  workstations mask preemption races that fire on 2-core hosted GitHub Actions
  runners (`0.8.32` `read_visibility_generation` race `15d19f573`). Run
  `bash scripts/check-low-core-concurrency.sh` (pinned via `taskset -c 0,1`)
  during qualification before cutting a release tag.
- **Watchdog & ETA cadence (≤15 min):** Never let a subagent or long local run
  wait silently for more than ~15 minutes without checking progress and
  reporting an ETA to the user. Use event-driven timers (`schedule` with
  `TimerCondition`) instead of tight polling loops.
- **Shared-host benchmarks:** Never stall indefinitely waiting for an idealized
  zero-load CPU on a shared workstation. Use interleaved paired runs / CPU-time
  controls or ask the user to confirm running on the current load floor.

## Act first & clear HITL prompts

- **Act first within agent-appropriate scope:** Before asking the user about
  repo state, CI runs, GitHub runners/environments, reachable build hosts, or
  published registry metadata, probe them directly with read-only / authorized
  tools (`gh api`, `nvidia-smi`, SSH, registry APIs). Escalate only genuine
  human gates (interactive auth/2FA, HITL design rulings, destructive ops,
  `sudo` host edits, and irreversible `v*` tag publication).
- **HITL prompt structure:** Whenever requesting a human decision, keep it
  clear and concise, present concrete numbered options, and state a clear
  **recommendation with justification**.

## Standing scope discipline

- **Physical erasure effort is complete (defects only):** Core physical erasure
  architecture is done; address only concrete defects and preserve existing
  `ROW_OWNED_PROJECTIONS` / `TABLE_MANIFEST` invariants rather than reopening
  erasure design in future release plans.

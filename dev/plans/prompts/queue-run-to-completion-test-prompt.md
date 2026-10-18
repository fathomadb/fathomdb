# Prompt — admission queue: test run-to-completion and the p90 start check

You are working in the FathomDB repository on branch `exp/agent-access`, or
on a fresh worktree cut from it. Read `AGENTS.md` and the memory index first
and follow them: TDD, no mocked database, one writer per checkout (your own
worktree), the GPU mandate (RTX 3090 `cuda:0` only), never `--no-verify`,
never push a `v*` tag. The rule against product code until the owner has
reviewed (HITL) still holds for `fathomdb-mcp` and the engine. This task
changes **the simulator and the docs only**.

## Read these first, in this order

You must understand both the research and the design before you change
anything. Do not skim.

1. `dev/design/agent-access/queue-admission.md`. This is the design of
   record.
   - § 2: every queue and lock on the path, H1 to H13.
   - § 6.1: the acquire sequence, the wait deadline, **"Started calls run to
     completion"**, the provisional early-shed rule, the busy codes and
     `abandoned`/`stuck`.
   - § 6.2: EP-18 part A as P2, idle-first dispatch.
   - § 7.3: the decision rules.
   - § 10: owner decisions and what is still open.
2. `dev/design/agent-access/queue-manager-research.md`. This is the
   research behind the queue type.
   - The exemplars in § 3: Kubernetes APF, CoDel/FQ-CoDel, nginx, Envoy and
     Netflix adaptive concurrency, tower, PgBouncer, SEDA.
   - Sizing in § 4 and agent-side behaviour in § 6.
   - The sources in § 11.
   - Its arrival test and `retry_after_ms` were overruled. § 10 records why.
3. `dev/experiments/queue-sim/REPORT.md` and `README.md`. These are the
   simulator, its calibration (§§ 1–3) and its findings.
   - The incident reproduction (§ 5).
   - Depth and head-of-line (§ 6).
   - The main fuzz (§ 7).
   - The cutoff study (§ 8).
   - Its limits (§ 11).
4. `dev/design/agent-access/queue-lock-trace.md`. This is a code trace of
   every queue and lock that one MCP call passes through.
5. `dev/design/mcp-adapter/engine-proposals/EP-09-interruptible-reads.md`.

## Background

The design was changed on 2026-10-18.

**Before:** the 10 s `call_deadline_ms` also applied to a call that was
already running. When it fired, the adapter abandoned the call. The read
kept its worker, permit and locks, and its result was thrown away.

**Now:**
- The deadline bounds waiting and starting only.
- A started call runs to completion and returns its result.
- A separate `run_limit_ms` (proposed 60 s from the permit) catches stuck
  work.

This makes the p90 start check provisional. It was meant to stop late
starts from becoming abandoned work. Now a late start becomes a late answer,
so the check only trades a late success for a quick busy.

## What to do

1. **Make the simulator express the rule. TDD: write failing tests first.**
   - Add an admission option that separates the wait deadline from a run
     limit.
   - Under it, a started call completes unless it passes `run_limit_ms`,
     counted from the permit.
   - Count abandonment at the run limit, and on client cancel if you model
     cancel.
   - Keep the old behaviour as a selectable arm, so results stay comparable.
   - All 46 existing tests must still pass; tests are read-only during
     fix-to-spec.
2. **Preregister before running.** Write the arms, the metrics and the
   decision rule into a new section of `REPORT.md`, or a sibling file,
   before any run.
   - Arms:
     - the old rule (deadline applies to running calls);
     - run to completion with the p90 start check;
     - run to completion with a start check on the wait deadline only;
     - optionally p95 and p99 start checks.
   - Run-limit values: 30 s, 60 s and 120 s.
   - Bands: the realistic and stress bands of REPORT § 7.
   - Slow tails: the existing ones, plus reads of 10–30 s to stress the run
     limit.
   - Metrics:
     - successes, busy and abandoned, per call;
     - latency p50, p95, p99 and p99.9 of successful calls;
     - the share of successes answered after 10 s;
     - goodput;
     - abandoned work held at the run limit;
     - retries caused, using the client models of REPORT § 2.
3. **Run it crash-safe.** Use the existing `runner.py` pattern: streaming
   output, resume, a progress and ETA log, and error rows per job. Keep raw
   output out of git; commit the summary tables.
4. **Answer these questions:**
   - How many former late timeouts become successes, and how late are they?
   - Does the p90 start check still pay for itself, or should any call that
     gets a permit before its wait deadline start?
   - What `run_limit_ms` value, and how often is it hit?
   - Does run to completion change the queue depth or the head-of-line
     results?
   - What changes once EP-09 lands?
5. **Write durably.**
   - Update `REPORT.md`, and mark the new section's date.
   - Propose the edits to `queue-admission.md` §§ 6.1, 7.3 and 10 as a
     clearly marked proposal. Do not change the owner decisions recorded
     there.
   - Pass `./scripts/agent-lint-md.sh`.
   - Commit with the session trailer the harness gives you, and push the
     branch.
6. **Gate:**
   - Get a Fable-medium review of the simulator changes and the
     conclusions.
   - Fix what it finds, each fix with a failing test first.
   - Then report to the owner.

## Report back

- The preregistered rule and the answer it gives.
- The table of arms by band.
- The recommended start check and `run_limit_ms`.
- What the review found.
- Open questions for the owner.

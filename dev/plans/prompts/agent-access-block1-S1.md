# Slice S1 — adapter admission queue (queue-admission § 6.1)

Branch `feat/aa-b1-s1`, worktree
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s1`, cut from
`feat/agent-access-block1` after S0 lands. Crate: `fathomdb-mcp` only.

## Read first

- `AGENTS.md` §§ 1, 3, 5, 6, 10.
- `dev/design/agent-access/queue-admission.md` §§ 1, 3, 6.1, 8, 9 (tests
  1–27), 10. § 6.1 is the specification; this brief does not restate it.
- `dev/interfaces/mcp.md` §§ 2, 3, 6.
- `src/rust/crates/fathomdb-mcp/src/{registry,bridge,tools,error,seams,config,edge}.rs`
  and `tests/bridge.rs`, `tests/lifecycle.rs`, `tests/common/`.

## Scope

Build § 6.1 as designed:

- Per-store `Arc<tokio::sync::Semaphore>` with `admission_cap` permits,
  `acquire_owned`; `in_flight` derived from permits (the separate
  `AtomicUsize` goes); `queued` via a `QueueTicket` drop guard; `abandoned`
  counted by `bridge::call` and released by the guard's `Drop`.
- The acquire sequence, steps 1–7, including the re-check under the registry
  mutex and the fall-back to the open wait on `Draining`/`Closed`.
- The per-store, per-tool-kind window of the last 128 completed service times
  (`search_text`, `search_hybrid`, `fetch`, `expand`, `orient`); `need` = p90,
  or a 50 ms floor below 8 samples. Make the window size, the p90 quantile,
  the 8-sample threshold and the 50 ms floor named constants; the p90 check is
  provisional.
- Wait deadline `call_deadline_ms` (default 10 s) bounds waiting and starting
  only. New keys `queue_limit` (default 32) and `run_limit_ms` (default
  60 000); `bridge::call` races the blocking closure against `run_limit_ms`
  measured from the permit; `tools.rs` passes the two separately. Client
  cancel (`context.ct`) of a queued call leaves the queue; of a running call,
  returns an error result at once and marks the call abandoned.
- Busy: `busy/admission` with `busy.reason` ∈ {`queue_full`, `deadline`,
  `engine_overloaded`}; `busy/store_switch`; the `busy` sub-object
  `{reason, in_flight, queued, abandoned, stuck}`; no `retry_after_ms`
  anywhere. Output schema updated.
- Drain/idle/shutdown rules of § 6.1, progress `queued: q waiting, c running`
  each second for queued calls with a progress token, tracing at stderr.
- Docs in the same slice: every `dev/interfaces/mcp.md` change in § 8's
  table, `dev/design/mcp-adapter/architecture.md` § Threads, and
  `dev/design/mcp-adapter/requirements.md` MR-033.2.

## Test seams

Extend `src/seams.rs` (feature `test-seams`) only as the tests need, each
extension in its own commit before the RED that uses it:

- per-call release (release the oldest held call, one at a time), for the
  arrival-order tests;
- a hold of a given duration, for service-time tests;
- seeding a store's service-time window with given samples is allowed for
  tests 7–10, so they do not need 128 real calls of 300 ms. Real calls must
  still be used to show that samples are recorded at all (test 9 or 14).

Seams never change behaviour when the feature is off. No mocked database:
every test runs real stores through the in-process protocol harness.

## RED tests

The 27 adapter tests of § 9, numbers 1–27, by the names given there. The two
existing contract tests (`tests/bridge.rs` `the_admission_cap_counts_abandoned_calls`
and `concurrent_calls_never_exceed_the_admission_cap`) are replaced by tests 1
and 2 in the same commit as the interface change; say so in that commit body.
Suggested order (so no single test forces a big-bang GREEN): 1, 2, 5, 4, 21,
18, 17, 6, 12, 3, 10, 7, 8, 9, 11, 14, 15, 13, 16, 20, 19, 22, 23, 24, 25, 26,
27. Change it if a dependency demands, and say why.

- Each RED commit fails at its own sha for the stated reason, with `RED
  evidence:` in the body; each GREEN carries `GREEN evidence:`.
- A test that passes on arrival is not RED. If one of the 27 already passes
  once its predecessors are green, commit it with the GREEN it belongs to and
  name it as a pin in the body. Do not invent a failing variant.
- Timing assertions use the tolerances in § 9 or wider; run each timing test
  10 times at its GREEN sha and record the count.
- Tests are read-only once committed RED.

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

These override the text above where they differ.

1. **Split.** S1a (queue core: semaphore, ticket, window, busy object, config
   keys, steps 1–5 and 7, progress for queued calls) on `feat/aa-b1-s1a`, then
   S1b (run to completion: `bridge::call` races `run_limit_ms`, `context.ct`
   on running calls, abandoned/stuck, drain/shutdown with waiters) on
   `feat/aa-b1-s1b`. S1a tests in this order: 1, 2, 5, 4, 21, 18, 17, 6, 12,
   3, 10, 7, 8, 9, 11, 19(pin). S1b: 14, 15, 13, 20, 16, 23, 24, 22(pin).
2. **Cancel is not observable on the cancelled call (blocker).** rmcp 3.5.1
   drops the response of a cancelled request (`service.rs:1500-1503`,
   `:1648-1652`); `context.ct` does fire. Tests 16 and 21 assert the effects
   through a second call (its `busy` object: `queued` drops, `abandoned: 1`,
   the next waiter is admitted), not through the cancelled call's reply.
   Correct § 6.1 step 6 and its citation in `queue-admission.md` in the same
   slice. A cancelled running call returns `timeout/cancelled` (rmcp discards
   it, but the handler's result is defined); add it to `mcp.md` § 6.
3. **Re-evaluate `need` at step 4.** `need` sets the step-3 timeout and is
   recomputed from the window at step 4. Test 3: seed 7 `fetch` samples at
   500 ms (cold → 50 ms floor at step 3); the running call completes as the
   8th sample at about deadline − 200 ms; the waiter gets the permit with
   200 ms left, `need` is now 500 ms, step 4 refuses with `deadline`, and the
   permit is free. State the re-evaluation in `queue-admission.md` § 6.1.
4. **Test 26 moves out of S1.** `EngineError::Overloaded` is raised only by
   the embed queue. If an `Overloaded` SDK error is constructible in the
   adapter's unit tests, make 26 a unit test of `ToolError::from_sdk` in S1a;
   otherwise it moves to S3. No error-injection seam that fakes the engine.
5. **Pins, declared in advance** (commit with the GREEN they belong to,
   labelled as pins): 19 (direct JSON check that no busy reason carries
   `retry_after_ms`; give the new `busy` struct `deny_unknown_fields` so the
   schema also rejects it), 22 (a queue implies `in_flight ≥ 1`).
6. **Test 17 needs an observable:** a seam between permit acquire and guard
   build; with cap 1 a call held there already shows `in_flight: 1` in a
   concurrent busy object and blocks idle/LRU drain.
7. **Test 18 panic leg:** add a seam "panic after the ticket, before acquire"
   in its own commit; or drop the panic leg and say so.
8. **Tool kind.** `Registry::acquire` takes a `ToolKind`; classify search by
   `canonical.search_mode()` (known before acquire for fresh and continuation
   calls, `tools.rs:594-596`, `:634`). `feedback` (acquires with `None`
   deadline, `tools.rs:1175`) takes a permit, never waits, leaves no sample.
9. **`in_flight` derivation** replaces `wait_idle` (`registry.rs:630-640`) and
   the `in_flight == 0` reads at `registry.rs:412, 426, 598`; the
   `AtomicUsize` is deleted.
10. **Progress message** needs a `String`: change `ProgressFn` and reuse S0's
    one-sender-per-call path.
11. **Config:** add `queue_limit` and `run_limit_ms` to `TOP_LEVEL_KEYS`
    (`config.rs:145`) with validation (`queue_limit ≥ 1`, `run_limit_ms ≥
    call_deadline_ms`), a RED each; `mcp.md` § 2 rows and the `timeout` row.
12. **Test 2 tolerance:** `call_deadline_ms` ≥ 1000, assert `elapsed ∈
    [deadline − 150 ms, deadline − 10 ms]`.

## Rules

- Scoped checks only: `cargo test -p fathomdb-mcp` (the crate's dev-dependency
  enables `test-seams`), `cargo clippy -p fathomdb-mcp --all-targets -- -D
  warnings`, `cargo fmt --check`, `./scripts/agent-lint-md.sh` for docs. Do
  not run `./scripts/agent-verify.sh`.
- No engine change. If one seems needed, stop and report.
- Block 2 (B2-7) must stay an addition: the hints design reserves no busy
  field; the `busy` sub-object is defined here.
- Audit RED shas in a throwaway detached worktree before reporting.
- Never `--no-verify`, never `pip install -e`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report

The commit list (sha, RED/GREEN/pin, test), failure evidence, scoped check
results quoted, any deviation from § 6.1 or this brief with the reason.

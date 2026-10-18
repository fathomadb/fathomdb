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

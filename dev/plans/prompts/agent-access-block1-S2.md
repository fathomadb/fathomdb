# Slice S2 — EP-18 part A: idle-first reader dispatch

Branch `feat/aa-b1-s2`, worktree
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s2`, cut from
`feat/agent-access-block1`. Crate: `fathomdb-engine` only.

## Read first

- `AGENTS.md` §§ 1, 3, 5, 6, 10.
- `dev/design/agent-access/queue-admission.md` § 4 (head-of-line risk),
  § 6.2 part A, § 9 tests 28–29.
- `dev/design/engine.md` § Writer / reader split (reads must not queue behind
  one connection).
- `src/rust/crates/fathomdb-engine/src/reader_pool.rs` in full.

## Scope and acceptance criteria

Replace round-robin-only dispatch with "try an idle reader first, otherwise
round-robin" (owner decision P2).

- Mechanism per § 6.2: each worker gets a rendezvous channel
  (`sync_channel(0)`); dispatch tries `try_send` on each worker in turn
  starting from the round-robin cursor, and only if every `try_send` returns
  `Full` falls back to today's blocking round-robin `send`. If a rendezvous
  channel cannot carry the fallback (a blocking `send` on a rendezvous channel
  waits for that worker's `recv`, which is the fallback behaviour wanted —
  confirm this), say so; do not add a mutex on the hot path.
- If the change of channel capacity alters any existing behaviour (shutdown,
  `Disconnected` handling, the `live_workers` count, AC-021, tc5-benchmark
  hooks), keep that behaviour and say how.
- Each worker still owns its connection for life.
- AC-1: with one worker held by a test hook inside a read, seven following
  reads all complete while it is held (today, round-robin sends one of them
  to the held worker, where it waits).
- AC-2: with all eight workers held, the next read queues on the round-robin
  worker and completes when that worker is released.
- Update the `ReaderWorkerPool` doc comment (it says round-robin) and
  `dev/design/engine.md` if it describes dispatch. File EP-18 as
  `dev/design/mcp-adapter/engine-proposals/EP-18-reader-dispatch-and-pool-size.md`
  from queue-admission § 6.2 (parts A and B, status: part A accepted and
  built, part B pending the live sweep). No `pub` API change is expected; if
  one is needed, stop and report.

## RED tests (one failing commit each, then its GREEN)

| Test | Notes |
| --- | --- |
| `a_slow_read_does_not_delay_reads_while_a_reader_is_idle` | Use the engine's existing `test-hooks` to hold one worker inside a read (find the existing pause/hold seam; add a test-only one behind `test-hooks` if none fits, in its own commit before the RED). Must fail at its RED sha: at least one of the seven reads is stuck behind the held worker. |
| `with_every_reader_busy_dispatch_falls_back_to_round_robin` | All eight held; the ninth read completes only after its round-robin worker is released. This may pass on arrival against today's code (it pins the fallback). If it does, commit it with the GREEN of the first test and say so in the body; do not label it RED. |

Tests are integration tests under `src/rust/crates/fathomdb-engine/tests/`,
registered in `Cargo.toml` with `required-features = ["test-hooks"]`, on a
real store. Timing assertions use generous margins (seconds, not
milliseconds) and must not be flaky: run each 20 times at its RED and GREEN
shas and record the counts in the commit bodies.

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

1. **Shutdown (blocker).** `shutdown()` wakes workers with
   `try_send(ReaderRequest::Shutdown)` (`reader_pool.rs:1238`) and joins with
   the senders alive. On a rendezvous channel a busy worker returns `Full`,
   the Shutdown is lost, and `join` hangs. Fix it (blocking `send(Shutdown)`
   per worker, or drop the senders before joining so `recv` errors), and add a
   test, RED first: closing the engine while a reader is held mid-read
   returns once the read finishes (`close_while_a_reader_is_busy_returns`).
   It fails (hangs → bounded by a timeout thread) only after the channel
   change, so order it: commit the dispatch RED, then this test as RED
   against an intermediate commit that changes the capacity, or fold the
   shutdown fix into the dispatch GREEN and show the test passing there with
   the failure demonstrated on a scratch build. Say which in the bodies.
2. **Callers now wait in `send`, not in `recv`.** Today's capacity is 4
   (`reader_pool.rs:499`). Latency is the same; state this in the doc
   comment.
3. **Test 28 determinism.** The only hold seam is for worker 0
   (`pause_reader_with_timeout_for_test`, `wal_runtime.rs:238-245`, 5 s
   auto-release). Align the cursor to 0 with
   `next_reader_worker_index_for_test()` (pattern `tests/reader_pool.rs:184-185`)
   before holding; issue the seven reads sequentially from a spawned thread,
   each with a `recv_timeout` well under 5 s; assert elapsed time.
4. **Test 29 needs an eight-worker hold seam** (its own commit, behind
   `test-hooks`, sending `HoldWalSnapshotBounded` to every sender). The test
   reads the cursor before the ninth read, releases a non-cursor worker and
   asserts the read is still pending (500 ms), then releases the cursor
   worker and asserts completion, all inside the 5 s auto-release, asserting
   elapsed time.
5. **`Disconnected` on `try_send`:** skip that worker and continue; the
   fallback `send` returns `Err` as today (`:1225`).
   `wal_native_state_inventory_for_test` (`:1081-1093`) retries `Full`; its
   meaning becomes "busy"; say so in a comment.
6. Rewrite the comments at `reader_pool.rs:496-498` and `:1207-1209` too.
   Add an EP-18 row to `dev/design/mcp-adapter/engine-proposals/README.md`.
7. AC-021 (`tests/lifecycle_reliability.rs`) needs `--features
   operator,test-hooks`; run it.

## Rules

- TDD per `tdd-red-must-fail-per-finding`: each RED commit body carries
  `RED evidence:` with the failing assertion; GREEN carries `GREEN evidence:`.
  Audit RED shas in a throwaway detached worktree.
- Tests are read-only once committed RED.
- Scoped checks only: the two new tests, `cargo test -p fathomdb-engine
  --features test-hooks --lib reader_pool`, the existing reader-pool and
  AC-021 tests (find them), `cargo clippy -p fathomdb-engine --all-targets
  --features test-hooks -- -D warnings`, `cargo fmt --check`,
  `./scripts/agent-lint-md.sh` for docs. Do not run `./scripts/agent-verify.sh`.
- Never `--no-verify`, never `pip install -e`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report

The commit list (sha, RED/GREEN, test), failure evidence, scoped check
results quoted, any deviation with the reason.

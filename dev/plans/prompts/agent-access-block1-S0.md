# Slice S0 — MCP server defects (agent-access item 6)

Branch `feat/aa-b1-s0`, worktree
`/home/coreyt/projects/fathomdb-worktrees/aa-b1-s0`, cut from
`feat/agent-access-block1`. Crate: `fathomdb-mcp` only.

## Read first

- `AGENTS.md` §§ 1, 3, 5, 10.
- `dev/design/agent-access/README.md` item 6.
- `dev/interfaces/mcp.md` §§ 1 and 6.
- `dev/design/mcp-adapter/engine-proposals/EP-03-hybrid-search-degradation.md`
  ("Adapter change once it lands", last paragraph).

## Scope and acceptance criteria

1. **Ordered progress notifications.** `edge.rs::progress_fn` spawns one
   task per notification (`edge.rs:107-108`), so two in a row can arrive out of
   order. Send them in order: one sender per call, for example an unbounded
   `mpsc` channel drained by a single task that ends when the sender drops.
   - AC: for one call, the progress values a client receives are strictly
     increasing and in send order, under a runtime with several worker
     threads and many notifications in a burst.
2. **Exit code on stdin close.** Contract (decided in
   `dev/plans/agent-access-block1-plan.md`): exit 0 whenever stdin closes,
   before or after `initialize`; exit 1 when the handshake fails for any other
   reason (malformed or wrong first message). `main.rs:93-99` returns 1 on any
   `serve` error today.
   - AC: the built binary, with a valid config, exits 0 when stdin is closed
     with no bytes written; still exits 0 when stdin closes after a full
     session; exits 1 when the first message is not a valid `initialize`
     (whatever rmcp does with garbage — check it, and if rmcp cannot tell EOF
     from a protocol error, say so before writing code).
   - Update `dev/interfaces/mcp.md` § 1 with both cases.
3. **Vector side disabled.** A hybrid search on a store whose dense side is
   disabled gets SDK `VectorEquivalenceMismatch` before any work and falls
   through `error.rs:244` to `internal`. Map it to `not_ready` with an
   actionable remedy naming `mode: "text"`, keeping `detail` = the `ErrorKind`
   name. Do not change the engine.
   - AC: a real store with the dense side disabled (find how the engine sets
     `dense_disabled`; use the real path, no mocks), `search` with
     `mode: "hybrid"` returns `error.class == "not_ready"`, a code naming the
     cause, and a remedy containing `mode` and `text`; `mode: "text"` on the
     same store still succeeds.
   - Update `dev/interfaces/mcp.md` § 6 error table with the new code.

## RED tests (one failing commit each, then its GREEN)

| Test | File |
| --- | --- |
| `progress_notifications_arrive_in_order` | `tests/lifecycle.rs` or a new `tests/progress.rs` |
| `stdin_closed_before_initialize_exits_zero` | `tests/process.rs` (new; spawns the built binary via `env!("CARGO_BIN_EXE_fathomdb-mcp")`) |
| `a_malformed_first_message_exits_one` | same file; this one may pass on arrival — if so, it is a contract pin, not RED: commit it together with the GREEN of the previous test and say so |
| `hybrid_on_dense_disabled_store_is_not_ready_use_text` | `tests/search.rs` |

The ordering test must fail at its RED sha for the stated reason. A
notification race may not reproduce reliably; make it reliable (many
notifications in one burst from a seam, a multi-thread runtime, repeat) and
record the failure rate across 20 runs at the RED sha in the commit body. If
it cannot be made to fail reliably, stop and report rather than committing a
flaky RED.

## Review fixes (Fable-medium brief review, APPROVE-WITH-FIXES) — binding

These override the text above where they differ.

1. **Exit-code contract, precisely.** Exit 0 on
   `ServerInitializeError::ConnectionClosed`; exit 1 on any other
   `ServerInitializeError` (the enum is `#[non_exhaustive]`: use a wildcard).
   rmcp 3.5.1 ignores unparsable lines (`transport/async_rw.rs:166-190`) and
   answers wrong-shaped JSON with an error then continues, so "garbage" input
   ends at EOF → exit 0. The exit-1 pin uses a JSON-RPC **notification** as
   the first message (`{"jsonrpc":"2.0","method":"notifications/initialized"}`),
   then closes stdin (`ExpectedInitializeRequest`, `server.rs:682`). Rename the
   pin `a_notification_before_initialize_exits_one`. `mcp.md` § 1 states:
   stdin close before or after `initialize` → 0; unparsable lines are ignored
   by the transport; a handshake protocol error → 1; stdout closed
   (`TransportError`) → 1.
2. **Progress burst seam.** Progress fires at most about once a second today
   (`registry.rs:391-396`), so add a `test-seams` hook first, in its own
   non-behavioural commit before the RED: e.g. `Seams::burst_progress(tool,
   n)` makes dispatch call the call's progress function `n` times before the
   tool body. The RED asserts `len == n` and strictly increasing values.
   Record the failure count over 20 runs.
3. **Dense-disabled test needs the default embedder.** Gate the test with
   `#![cfg(feature = "default-embedder")]` (or a `cfg` on the function) and run
   it with `cargo test -p fathomdb-mcp --features default-embedder --test
   search` (weights must be in the local model cache; if they are not, stop
   and report). Recipe, no mocks (mirrors
   `fathomdb-engine/tests/vector_equivalence_probe.rs:232-300,351`): open with
   the default embedder and write a vector-kind node, close; with rusqlite
   (already a dev-dependency) negate `reference_vec` for every row of
   `_fathomdb_embed_probe` and `DELETE FROM _fathomdb_open_state WHERE
   key='vector_equivalence_verified_fingerprint'`; reopen through the adapter
   and assert `orient.dense_disabled.disabled == true` as a precondition before
   the hybrid assertion. Do not add a `fathomdb-engine` dev-dependency
   (`tests/manifest.rs` forbids it).
4. **Error code.** Class `not_ready`, code `vector_disabled`, remedy mirroring
   `embedder_not_ready` (`error.rs:137-144`, `use mode "text"`), `detail` =
   the `ErrorKind` name. Add the code to the `not_ready` row of `mcp.md` § 6.
5. **Harness.** Use the existing `Spawned` helper in `tests/common/mod.rs`
   (`:380-470`) and `child.wait()`, not a new process harness. "Exit 0 after a
   full session" already passes: it is a pin, committed with the GREEN.

## Rules

- TDD per `tdd-red-must-fail-per-finding`: each RED commit body carries
  `RED evidence:` with the failing assertion; each GREEN body carries
  `GREEN evidence:`. Audit your RED shas in a throwaway detached worktree.
- Tests are read-only once committed RED.
- No mocked database. No engine change.
- Scoped checks only: `cargo test -p fathomdb-mcp --features test-seams`
  (check the crate's dev-dependency enables it), `cargo clippy -p
  fathomdb-mcp --all-targets -- -D warnings`, `cargo fmt --check`,
  `./scripts/agent-lint-md.sh` for the doc change. Do not run
  `./scripts/agent-verify.sh`.
- Never `--no-verify`, never `pip install -e`, never push.
- Commit messages end with
  `Claude-Session: https://claude.ai/code/session_01RBbKrBEAJUjsNgeFwfAUbr`.

## Report

The commit list (sha, RED/GREEN, test), the failure evidence, scoped check
results quoted, and any deviation from this brief with the reason.

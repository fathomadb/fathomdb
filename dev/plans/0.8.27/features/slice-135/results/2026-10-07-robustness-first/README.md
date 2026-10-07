# Slice 135 first system-robustness result — 2026-10-07 UTC

**Status:** first diagnostic result, not the Phase 1 robustness matrix or a
release gate. The Phase 1 protocol was still `DRAFT_NOT_FROZEN` at execution.

## Identity and method

- Tested product source base: `8cbd330c8f83f35ef46d538ea23811c08af8066b`
  (`llm/0.8.27-slice-135`, post-Slice-132). The only source difference in the
  isolated execution branch was the new test file. Its clean execution commit
  was `4f3a533aa12cae175d2c272fe03dc9b367a1abb0`.
- Fixture: `src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs`,
  SHA-256 `75af50bbc609b54216fb976fab014bb42f5d3b027c7539055917e7cf032057a8`.
  `Cargo.lock` SHA-256:
  `9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
- Executable SHA-256:
  `7ff7943df4651d27543dcde264123fe4bf654eca70890ff77153b5703a421364`.
- Host: `windchill3`, Linux `7.0.0-38-generic`, x86_64;
  `rustc 1.95.0`, `cargo 1.95.0`. `CARGO_TARGET_DIR` was isolated at
  `/tmp/slice135-robust-target`; `CARGO_INCREMENTAL=0`,
  `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=6`. No model or network
  provider was used. Every case opened a real temporary SQLite database
  through FathomDB `Engine` and reopened it with a fresh `Engine` after close
  or process kill.
- Raw build, negative-control, positive and scoped-Clippy output are retained
  as [build.log](build.log), [negative.log](negative.log),
  [positive.log](positive.log), and [clippy.log](clippy.log).
  [summary.json](summary.json) independently parses the `SLICE135_ROBUSTNESS`
  records and checks exact arrays, overlap, error, resource deltas and
  supplementary SQLite integrity. The raw logs remain authoritative.

## Exact commands and outcomes

From the isolated execution worktree at the commit above:

```bash
env CARGO_TARGET_DIR=/tmp/slice135-robust-target CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=6 cargo test --offline -p fathomdb-engine --test slice135_robustness_first --no-run
timeout 30s /tmp/slice135-robust-target/debug/deps/slice135_robustness_first-abf418941aafe485 --exact slice135_negative_missing_record_must_fail --ignored --nocapture
timeout 30s /tmp/slice135-robust-target/debug/deps/slice135_robustness_first-abf418941aafe485 --nocapture --test-threads=1
env CARGO_TARGET_DIR=/tmp/slice135-robust-target CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=6 cargo clippy --offline -p fathomdb-engine --test slice135_robustness_first -- -D warnings
cargo fmt --all --check
```

Build exit `0` (12.88 s). Deliberately wrong missing-row oracle exited `101`:
expected one body, observed `null`. The positive suite exited `0`: three
passed, two intentionally ignored child/control tests, total 0.35 s. Scoped
Clippy, workspace rustfmt and `./scripts/agent-lint-md.sh` exited `0`.
The Markdown check first stopped because this linked worktree lacked
`node_modules`; it passed after linking the primary checkout's installed
tooling, as the script instructs. The full `agent-verify.sh` gate was
**not run in this isolated branch**; it remains required after integration.
These are focused checks, not a full-workspace green claim.

## Observed matrix

| Case | Setup and controlled point | Expected and observed reopened state | Bounded completion and cleanup |
| --- | --- | --- | --- |
| Concurrent read/write | One durable anchor; two writers each add eight unique nodes while two readers point-read the anchor. | All 17 bodies were present immediately and after fresh reopen, exactly matching the fixture list. | 206 reads, 204 while a writer was active; 56 ms for the concurrent phase plus verification; FD count 4 → 4; `integrity_check=ok`; WAL absent after close. External suite timeout 30 s. |
| Process kill/restart | Seed `base`, close, launch child. Kill once after `Engine::open` but before the gated write. Launch another child and kill after `Engine::write` returned and a point read found `after`. | First reopen: `base` present, `after` absent. Second reopen: both present. Child statuses were Unix signal 9. | 120 ms across both phases; each child marker had a 5 s deadline; child guard kills/reaps on early failure; `integrity_check=ok`; WAL absent after close. This models process termination, **not** torn or reordered power-loss writes. |
| Injected write refusal | Seed `base`, set `force_next_commit_failure_for_test`, then attempt `failed`; write `recovery` afterward. The hook fires after validation and before transaction `BEGIN`. | Typed `EngineError::Storage`; `failed` absent both immediately and after reopen; `base` and subsequent `recovery` present. | FD count 4 → 4; `integrity_check=ok`; WAL absent after close. External suite timeout 30 s. |

The temporary databases were removed by their owning `TempDir` after each
test. The positive receipt recorded `wal_bytes_after_close: null` for all
three cases. The FD readings are single-process snapshots under a serial test
run; they are diagnostic and do not replace a repeated leak test.

## Invalid attempts and limits

No positive attempt was invalidated or retried. The negative-control failure
was intentional and is retained separately. No production source changed.

This first result does not exercise projection completion, a crash within a
commit or queue transition, provider failure, persistent injection, SQLite
busy/full-disk, close/cancellation under load, interrupted erasure, binding
layers or randomized schedules. It does not compare with 0.8.26. Those remain
open rows in the Phase 1 robustness matrix. A green result here establishes
only the three named boundaries on the source identity above.

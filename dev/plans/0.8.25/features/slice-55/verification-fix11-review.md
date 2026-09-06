---
title: 0.8.25 Slice 55 FIX-11 independent verification
status: FAIL
verified_candidate: b92eabe5
updated: 2026-09-06
---

# Slice 55 FIX-11 independent verification

Independent verification cannot close candidate
`b92eabe5fe0fcf9d847a59b14a2e4a263ed703cc`. FIX-11 correctly repairs the
installed-wheel smoke fixture and verification plan without changing product
code. Windows and the focused, artifact, fast, heavy, all-tier, selected-feature,
Clippy, and check routes pass. The required parallel default-feature workspace
test route remains scheduling-sensitive and can deadlock.

## FIX-11 and Windows closure

- Two independent exact-source, fresh-wheel, fresh-venv Windows smokes passed
  the typed trace assertions and immediate deletion of `corrupt.fathom` after
  explicit `Engine.close()`. No deletion, garbage collection, retry, or ignored
  cleanup error was used.
- The two wheel SHA-256 values were
  `2eff8b56902dc60a6543c527d31772b5fa7bb5a5e872a2757421b8b1fa2937bb`
  and
  `5dd1023c48c07a013f2e7b1817b2a35a6e3dab3fbd49c566fd452c5ac639cbd8`.
- Windows focused Rust passed 66 integrity, 20 dependency-trace plus 1 expected
  ignored performance, 17 explanation, 10 wire, 6 legacy, 2 facade, and 3 CLI
  tests.
- The exact Windows N-API build passed 31/31 Slice 55 TypeScript tests and the
  local native-artifact harness. Its native SHA-256 was
  `9adec7b7bba2e0b5c01bd9cd2baed2c8375fa18df6f3c902bfe48d38b06a5618`.

This evidence confirms the original Windows failure as a fixture-owned
stdlib SQLite handle rather than a FathomDB product leak.

## Other passed evidence

- Linux Ruff, canonical project Pyright, wrapper compatibility, package-local
  TypeScript/N-API, focused Rust/wire, real-database CLI, release-state views,
  Markdown checks, and `git diff --check` passed.
- The exact Linux wheel passed installed smoke and the immediate unlink oracle;
  SHA-256 was
  `9cb4449a44b8f522940cef7e74dcccc414bd9bb5d0caf7b4af1b176515c319cf`.
- The Linux local/offline native-artifact harness passed with N-API SHA-256
  `9345716ec6def0d933c1d22db5bd3a51d715b32c2ed2d6c6965cb71776069267`.
- The 50,000-row performance fixture passed at 3,200,000 VM steps, 53 ms, and
  zero measured RSS delta.
- Fast passed 103/103 suites, heavy passed 3/3, and all-tier passed 106/106;
  none reported a skip or exclusion.
- The serial selected-feature engine route, default workspace Clippy, and
  default workspace check passed.

## Blocking parallel-workspace evidence

The required default-feature command is:

```text
timeout 3600s cargo test --workspace --all-targets
```

The initial sandbox run reached the CLI ptrace test and failed only because
ptrace was denied. The unchanged unconfined run passed ptrace, then failed
`erasure_completeness::op_store_record_erasable_by_key` with
`ErasureIncomplete` at `wal_checkpoint`: all five attempts were BUSY and 278
WAL frames remained. The unchanged isolated test passed in 0.10 seconds.

One bounded rerun of the exact parallel workspace command then deadlocked in
`tests::wal_attribution_projection_worker_typed_refusal_then_post_release_sampler_is_recorded`.
At 128 seconds all 11 threads in PID 296365 were sleeping in `futex_do_wait`.
The test held
`/tmp/.tmplI1HCl/wal-attribution-projection.sqlite.lock`; its WAL was
1,149,512 bytes. The verifier terminated only that process tree, yielding exit
130. No third parallel attempt was made.

The exact focused WAL-attribution test passed 1/1 in 0.19 seconds. A diagnostic
serial `cargo test --workspace --all-targets -- --test-threads=1` also passed.
Those controls classify a scheduling-sensitive parallel test-harness liveness
defect; serial success does not waive the required parallel gate.

The minimal owning surface is the unbounded `ready.wait()` / `release.wait()`
rendezvous around `pause_projection_worker_after_wal_transaction_for_test`.
The WAL BUSY erasure symptom must also be classified during correction.

The verifier removed its temporary Linux and Windows artifacts and recovered
host free space to 165 GiB. The exact candidate remained unchanged and clean;
no release artifact was staged, tagged, uploaded, or published.

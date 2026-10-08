---
title: Slice 135 exact-candidate projection-commit fault replay
status: AUDITED_ROBUSTNESS_SUBSET_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Projection-commit fault replay — 2026-10-08

The clean candidate source checkout was
`3f29d649d0213e595c0dab251a449d92fd625792`. Its
[`tc91_projection_commit_hardening.rs` fixture](../../../../../../../src/rust/crates/fathomdb-engine/tests/tc91_projection_commit_hardening.rs)
had SHA-256
`49236d66b3c3c9d98bd076dea47711ed0e9a0b48503c40088e643c50d1f92cb0`.
The executed debug test binary had SHA-256
`857d661d436207eae15c1675eeb82e879b24deff80093b3d4f24dd16ab333ec4`.
The initial targeted `cargo test --offline --locked -p fathomdb-engine --test
tc91_projection_commit_hardening -- --nocapture --test-threads=1` passed all
six tests. The same binary was then run **ten separate times** with GNU Time
resource capture and a 30-second process timeout per run. No product source,
test source or binary changed between runs. The retained [manifest](manifest.json)
binds their identities and command.

The [independent replay checker](../../../../../../../scripts/slice135_tc91_audit.py)
reopened every stdout, stderr and resource report. Its [audit](audit.json)
accepted **60/60 test executions** across ten runs, including the two
expected injected panic messages in each run; no unexpected test failure,
child swap or major page fault occurred. Peak test-process RSS ranged
31,748–34,856 KiB. The [negative controls](negative-controls.json) show that
one changed test outcome and one false swap report were both rejected. The
checker's focused test also passes. Resource values describe the test process,
not an end-to-end latency comparison.

| FathomDB-owned fault or schedule | Assertion in the real-database fixture |
| --- | --- |
| One-shot projection commit busy | Durable caller write remains pending; lifecycle event reports `SQLITE_BUSY`; redispatch commits its vector without a false exhausted-embed failure. |
| One-shot non-SQLite storage failure | Event reports `StorageError` with the correct source; pending work redispatches and reaches a vector. |
| Subscriber panic after commit failure | Worker cleanup still permits redispatch and bounded drain. |
| Embedder panic followed by failed panic-terminal commit | A false durable `ProjectionPanic` terminal is avoided; normal retry reaches a vector. |
| Commit failure at mean-vector pin threshold | Rollback does not consume or duplicate the pin event. |
| Stop before failed-commit cleanup, then reopen | Fresh engine reconstructs pending work from durable state and commits its vector. |

All cases use `TempDir` real SQLite databases and debug-only fault hooks.
The stop/reopen test asserts product state after fresh open; the temporary
databases are removed by the fixture, so this archive does not retain an
independently reopenable database. The injected faults are one-shot and do
not model a persistent disk failure, process death inside a commit, or power
loss. Those rows remain open in the full robustness matrix.

The local raw archive is `/tmp/slice135-tc91-current-3f29`. Its verified
33-file `SHA256SUMS` has SHA-256
`6ef1eaa62000ab071e1d8e44e4b921454e9ef56cee747297411a2f295b65dba9`.
The retained manifest, audit and negative-control hashes are respectively
`9f869ff06cca48f92b50d7f37e542da822abce77e0d32cb235490e69d9e8673d`,
`067c16838560db29c888621cd4b5c221997e13b4fbf96241a47a4371b6fc07ec`
and `e0c5abe684cb8d36a20b776c914e51f1b0f2f12978077fb040dd6d76dca774f2`.
This is an exact-candidate robustness subset, not a complete Phase 1
checkpoint or a full workspace gate.

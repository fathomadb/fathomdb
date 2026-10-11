# Slice 135 erasure interruption and retry probes — 2026-10-07 UTC

**Status:** two focused real-database regressions on candidate source
`fedb56df9cd48ef9c9a63d4731eaeeac540bf26e`. Product engine bytes are
the same as the paired E01–E12 candidate; this receipt extends only the
robustness matrix. It does not simulate process death within SQLite commit.

The existing fixture is
`src/rust/crates/fathomdb-engine/tests/correction_safe_erasure.rs`, SHA-256
`3c155c946ebe589d0de54a9fe2a593cc49f0e4b3f51846e40b7326b6d4d00b94`.
The executed test binary was
`target/debug/deps/correction_safe_erasure-f9e6ca231a316005`, SHA-256
`d188d2e8280b5d4342106521d88fef2a8978c48a40a83ff72ed5a76bc7c1d1e8`.
Both selected cases passed with `--exact --nocapture --test-threads=1` under
`timeout 30s`; the retained [precommit](precommit.log) and
[postcommit](postcommit.log) logs show one passed and zero failed in each run.
GNU Time measured 0.10 s and zero swaps for each direct test-binary run;
[precommit](precommit-resource.txt) and
[postcommit](postcommit-resource.txt) resource reports are retained.

| Controlled interruption | Asserted product and physical state |
| --- | --- |
| Precommit proof failure | A real SQLite `BEFORE DELETE` trigger refuses one dependent deletion. `erase_source` returns `Storage`; a whole-database-plane snapshot is unchanged, no audit or physical closure is committed, and telemetry bytes are untouched. |
| Postcommit at-rest failure | A telemetry sink is rotated after records were written. The primary erase commits, but `erase_source` returns `ErasureIncomplete` at `telemetry_redaction`; the durable audit and closure remain. Restoring the sink and retrying completes the closure, redacts the stable ID, and does not duplicate the audit. |

These are stronger interruption semantics than an ordinary success-path test:
the first checks rollback of every protected primary plane, and the second
checks truthful incomplete status and a recoverable postcommit obligation.
The second case checks database state through a separate SQLite connection;
the first also compares a SQLite snapshot directly. They do not include an
explicit close/reopen within these two named cases, a mid-commit process kill,
or an independent `PRAGMA integrity_check`. Those remain explicit matrix
limits rather than inferred coverage.

Raw-file SHA-256: `precommit.log`
`2772a5f5c612340082d661bf5d3a1f6fc3d59ba3a7c0daf9d28c41f9df18190b`,
`postcommit.log` `4146cb4156ae92b8d693413e4d85ee4d3a48b903cbf7eb49fe29464247e6e757`,
`precommit-resource.txt`
`68e9e0c018953c088f34ab230f7daf4169270603c6d018535e54f9006189b7eb`,
and `postcommit-resource.txt`
`0d8d486efc77650f36cf3150a7da0aea0ceec4ba52aff535e97fc8a59afbded7`.

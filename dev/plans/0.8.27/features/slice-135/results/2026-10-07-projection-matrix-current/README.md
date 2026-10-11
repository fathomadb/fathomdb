# Slice 135 current-source projection fault and capacity observations

**Status:** focused reuse of ten real-database regression cases. This is a
diagnostic matrix extension, not the complete Phase 1 robustness result.

## Identity and execution

- Candidate source: `102843780` on `llm/0.8.27-slice-135`, after the four
  confirmed search error-handling repairs. The two existing fixture sources
  are `src/rust/crates/fathomdb-engine/tests/slice90_projection_capacity.rs`
  (SHA-256 `88e994dc0d5944ddb546b8c383dd010dfa6ad202551644d49e991dfa09afd99b`)
  and `src/rust/crates/fathomdb-engine/tests/tc91_projection_commit_hardening.rs`
  (`49236d66b3c3c9d98bd076dea47711ed0e9a0b48503c40088e643c50d1f92cb0`).
  `Cargo.lock` is `9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`.
- `cargo test --offline --locked -p fathomdb-engine --test
  slice90_projection_capacity --test tc91_projection_commit_hardening --
  --nocapture --test-threads=1` passed. The resulting test binaries were run
  serially again with `timeout 30s`, `--nocapture --test-threads=1` and GNU
  Time. Both exits were 0. Test binary hashes were
  `8f0b148da6610225294f6b64eaaf2b15a10e8a3b85fda639051005d780b58f3c`
  (capacity) and
  `ac222570df742e5ffab7a83b1c5e4b5934cc12338d42c17941fd6ff33bcc6d95`
  (commit faults). Raw [capacity.log](capacity.log),
  [commit-faults.log](commit-faults.log), [capacity-resource.txt](capacity-resource.txt)
  and [commit-faults-resource.txt](commit-faults-resource.txt) are retained.
- Independent log inspection found the exact four and six expected test names,
  each with `ok`, zero failed/ignored tests and exit 0. The assertions are in
  the linked fixture sources; the logs do not export each intermediate state
  as a separate JSON record. GNU Time reported 1.49 s/28,224 KiB for the
  capacity binary and 0.60 s/35,688 KiB for the commit-fault binary, with
  zero measured swap events in both. These are test-process resources, not
  product latency samples.

## Exercised boundaries and asserted outcomes

| Boundary | Named real-database oracle |
| --- | --- |
| Provider call exceeds its deadline while holding the only physical slot | A second write stays durable and unprojected until the first call returns; a late timed-out vector is never committed; both rows eventually project. |
| Close while provider is held or retry is sleeping | Close refuses to claim completion while the provider remains held, cancels retry admission, and a fresh open recovers the pending vector row. |
| Provider error followed by wake notifications | Retry does not run before its 300 ms absolute delay and eventually commits once. |
| Worker projection commit reports SQLite busy or storage error | Durable canonical row is redispatched to ready state; failure is reported with the correct source/error code rather than recorded as exhausted embedding failure. |
| Provider or subscriber panics around a failed worker commit | Panic containment does not strand the worker or its in-flight row; redispatch reaches a ready vector. The raw log's two panic messages are intentional test stimuli. |
| Failed threshold-crossing mean-vector commit | Rollback avoids double-publishing the mean pin; exactly one pin event appears after retry. |
| Shutdown wins before failed-commit cleanup | Fresh open reconstructs pending work from durable state and reaches a ready vector. |

These fixtures use synthetic in-process embedder implementations to control
timing, error and panic behavior, while writes, projections and recovery use
real temporary SQLite databases. They are not a live external-provider test.
Only the named close/reopen cases verify reopened product state. Most cases
assert readiness through engine test hooks rather than an independent SQLite
query, and none add `PRAGMA integrity_check`; the separate
[four-case result](../2026-10-07-robustness-expanded/README.md) supplies those
checks for its own boundaries. Persistent provider failure, cancellation
under sustained mixed client load, crash inside a projection commit, and
interrupted erasure remain open matrix rows.

Raw-file SHA-256 digests: `capacity.log`
`32b70768a2bb0fe3b0b260245b761385622f45040a866812873cd694d99c8556`,
`commit-faults.log` `2bc14e67af06109a434c7314b5225f16a7eee18c0dac22c3f5293c3cf1854126`,
`capacity-resource.txt` `a09ce61c1ad005180a827c361c79c11c40cfbe07620bed2f12c1c10205401628`,
and `commit-faults-resource.txt` `6f075cf404284f875368b3f05552d9606accdaf17762682b390298da4a339909`.

---
title: Slice 135 exact-source crash and persistent SQLite fault probes
status: AUDITED_REAL_DATABASE_SUBSET
target_release: 0.8.27
---

# Crash and persistent SQLite fault probes at source 224e44c59

The isolated [probe source](probe.rs) and [manifest snapshot](Cargo.toml.snapshot)
depend on engine source
`224e44c593c13d86ece648adabe445723db04070` with test hooks and uses
fresh real SQLite databases. The offline Cargo test completed with **three
passed, zero failed**; its ignored fourth target is the controlled child
victim started by the crash test. The first build attempts stopped before
tests because the isolated lockfile requested an uncached crate. Resolving
from the exact candidate's pinned lockfile produced the retained
[lockfile](Cargo.lock) and passing run. No failed measured test was replaced.
The manifest snapshot can be restored as `Cargo.toml` in an isolated copy;
the snapshot name keeps repository override checks scoped to live packages.

| Fault point | Expected and observed post-state | Recovery |
| --- | --- | --- |
| Forced projection commit failure, then SIGKILL before worker queue cleanup | Durable canonical node remained; after fresh open and drain its vector became ready; no persisted projection failure; SQLite integrity `ok` | Reopen and drain completed; victim exit signal 9 recorded |
| Persistent writer-connection `PRAGMA query_only=ON` for three writes | Three typed `Storage` refusals; only preexisting row remained | Reenable writes, write a recovery row, close and reopen; exact five-ID state and integrity `ok` |
| Persistent external `BEGIN IMMEDIATE` SQLite lock for three writes | Three typed `Storage` refusals over 15,013 ms; no blocked row persisted | Release lock, write a recovery row, close and reopen; exact five-ID state and integrity `ok` |

The [test output](test.stdout) and [state records](test.stderr) retain each
fault point, typed error, observed state and integrity result. The
[independent audit](independent-audit.json) verifies all three records and
their source bytes. Its [four tests](test_audit.py) include four deliberate
mutations of the log: false vector recovery, false integrity, missing pass
and changed typed error; all were rejected. The product tests assert their
real reopened state directly; the independent audit checks retained logs.

The test temporary databases were deleted after their assertions and are
therefore unavailable for a second SQLite reader. Connection-level
`query_only` is a SQLite write-refusal proxy, not an OS file-permission test.
SIGKILL exercises process interruption, not power-loss storage reordering.
Other positions and fault types are linked in the four-area checkpoint.

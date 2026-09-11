---
title: Slice 76 — AC-020 experiment design
status: APPROVED_FOR_EXECUTION
---

# Slice 76 design

## Boundary

Slice 76 measures the safe statistics-enabled product and tests one reversible
statement-reuse prototype. It does not ship a correction. The registered
AC-020 test, its 8 workers, 1,600 searches per arm and its
`concurrent <= sequential * 1.5 / 8` oracle remain unchanged.

The final relevant product tree must match the safe checkpoint. Experimental
commits may exist in branch history long enough to provide reproducible SHAs,
but their product and test changes are removed after measurement. Durable
outputs are limited to this design, the plan, receipts and status.

## Current-path model

Each reader worker owns one SQLite connection. The common AC-020 search opens a
deferred reader transaction, reads projection/dependency state, executes the
two-stage vector path, hydrates candidates, runs the deferred-identity FTS path,
fuses results, hydrates surviving text identity and commits. SQL assembled only
from stable shape inputs is eligible for reuse; request-varying literal SQL is
not silently rewritten in this slice.

The earlier E.1 experiment covered four simpler pre-worker-pool statements and
is historical evidence, not a current control. The current census determines
the treatment membership and one cache capacity. Existing `prepare_cached`
sites remain unchanged and are counted separately.

## Measurement components

- A small runner invokes the exact registered command, validates one executed
  test and `AC020_NUMBERS`, preserves exit status and raw logs, and calculates
  median, min/max, nearest-rank quartiles and both reported speedup forms.
- The manifest binds source/relevant-tree/Cargo.lock/toolchain/binary hashes,
  SQLite versions and flags, fixture constants, host controls and run order.
- A positive statistics witness runs against each verdict binary: read
  `SQLITE_STATUS_MEMORY_USED`, allocate a controlled block through
  `sqlite3_malloc64`, prove the live counter rises, free it, and prove the live
  counter returns. Compile options and absence of the unsafe opt-in runtime
  configuration accompany, but do not replace, that behavioral witness.
- Current SQL evidence comes from code census plus a process-owned diagnostic.
  The diagnostic may count preparations and list normalized executed SQL, but
  it must not alter verdict scheduling or SQLite configuration.
- One profile compares the sequential and concurrent arms. Unresolved stacks
  remain unresolved; Slice 76 does not build a general telemetry framework.

Statistics-enabled means no `MEMSTATUS_OFF`, custom PCACHE2, PAGECACHE,
`LIBSQLITE3_FLAGS`, or inherited FathomDB performance overrides in verdict
processes. The manifest records the scrubbed names and effective compile flags.

## Prototype

The treatment changes only measured common-path `prepare` calls to
`prepare_cached` on their owning reader connection. Bindings, SQL text, result
iteration, transaction lifetime, ordering, eligibility and ranking are
unchanged. Capacity equals the measured resident shape set plus explicit
headroom for already-cached common-path statements. It is derived from the
per-reader cyclic distinct-SQL reuse distance and sealed as the smallest
no-eviction value; changing the connection-wide cache is counted as part of the
treatment and there is no capacity sweep.

Focused RED tests establish alternating binding freshness, transaction release,
error recovery, current/frozen visibility and supported schema reprepare. GREEN
is the smallest statement-site conversion. Existing human-authored dependency,
lifecycle, ranking and reader-pool tests provide the blast-radius checks.

## Decision

Seven baseline observations establish current behavior. Seven B and seven C
observations then run in the predeclared alternating order. Statement reuse is
eligible for Slice 77 only when it produces a statistically material concurrent
improvement under the shared rule, avoids more than 3% sequential regression,
keeps all semantic tests green and has bounded retained memory. Passing AC-020
in all seven treatment runs is strong recovery evidence, but formal release
acceptance remains Slice 85.

If baseline already passes all seven, Slice 76 records that result and performs
only the preparation census needed to explain whether the treatment is still
useful. If the treatment repeats E.1's sequential-only improvement, it is
rejected because it tightens AC-020 without reducing concurrent time.

## Failure and cleanup

Identity mismatch, a zero/skipped test, missing output marker, timeout or noisy
campaign stops interpretation. One environment correction is allowed. No broad
regression, package build, hosted run or global SQLite configuration experiment
is part of Slice 76.

After review, the prototype is removed, relevant product/test tree identity is
verified against the safe baseline, temporary binaries/profiles are removed or
their durable external hashes recorded, release state advances to Slice 77, and
the worktree is left clean.

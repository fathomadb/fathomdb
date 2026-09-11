---
title: Slice 76 AC-020 attribution result
status: COMPLETE
---

# Slice 76 result

Statement reuse is an eligible performance anchor for Slice 77, but it is not
an AC-020 correction. The hash-bound timing campaign is unchanged by the later
diagnostic-only census corrections.

| Metric | Matched baseline B2 | Statement reuse C |
| --- | ---: | ---: |
| Sequential median | 564 ms | 175 ms |
| Concurrent median | 137 ms | 80 ms |
| Speedup | 4.12x | 2.19x |
| AC-020 passes | 0/7 | 0/7 |

Statement reuse improved sequential elapsed time by 389 ms (69.0%) and
concurrent elapsed time by 57 ms (41.6%). Both exceed the shared protocol's
materiality screen and all focused semantic tests passed. The poorer ratio is
not an absolute performance regression and is not a reason to discard the
gain.

AC-020 requires at least 5.33x scaling. At the treatment's 175 ms sequential
median, the concurrent bound is 32.8 ms; the observed 80 ms would need another
approximately 59% reduction at unchanged sequential performance. All seven
treatment runs fail, so the remaining scaling problem is substantial and must
be attributed independently.

## Attribution

- The final treatment diagnostic observed 10 SQL shapes compiled during
  instrumented `prepare`/`prepare_cached` calls on each of 8 readers: 80
  prepare-time compilations total. It also recorded zero automatic reprepares
  through `SQLITE_STMTSTATUS_REPREPARE` in this fixture.
- Cached statements retained 57,976 bytes per reader by
  `SQLITE_STMTSTATUS_MEMUSED`, 463,808 bytes across eight readers. This is
  SQLite's statement-memory estimate, not process RSS.
- The older `baseline-prepare-census.log` and
  `treatment-prepare-census.log` classify requests by whether a statement had
  executed before. They are preserved as superseded evidence and are not
  directly compared with the authorizer-based compile count.
- Baseline-only gperftools profiles contain 54 sequential and 90 concurrent
  on-CPU samples. Concurrent cumulative stacks include `sqlite3RunParser`
  (44.4%), `sqlite3Parser` (40.0%), `yy_reduce` (37.8%), and
  `sqlite3VdbeExec` (35.6%), with active allocator and mutex/futex stacks also
  visible. These percentages overlap and are directional, not an additive
  time decomposition.
- The profiler binary links `libprofiler.so.0`; retained `ldd` output contains
  no tcmalloc. Profiling starts after fixture setup, and the concurrent arm
  performs the registered 1,600-search sequential warmup before its delimited
  1,600-search interval. The runner accepted all 8 registered reader workers.
- Kernel `perf` was unavailable because `perf_event_paranoid=4`; its two
  zero-byte `.data` attempts are retained only as blocker evidence.

CPU profiling cannot measure sleeping time or distinguish channel wait from
SQLite lock wait. Because the profile is pre-treatment, it also cannot identify
the post-cache residual. No dispatch, lock, or private-runtime treatment is
selected from it.

## Hypotheses and Slice 77 route

| Hypothesis | Result | Consequence |
| --- | --- | --- |
| Repeated prepare/finalize is material | Supported | Carry statement reuse forward as an eligible anchor. |
| Temporary b-tree/pager churn is material | Unresolved | No mandatory existing counter isolated this component. |
| Another source dominates the residual scaling failure | Residual confirmed; source unresolved | Attribute the post-cache residual before selecting another treatment. |

Slice 77 should retain statement reuse and obtain bounded post-cache residual
evidence. If permitted diagnostics cannot identify that residual, it should
report the uncertainty and propose one bounded follow-up rather than jumping to
SQLite isolation or another implementation by guess.

## Focused correctness

Treatment-enabled retained runs passed:

- statement mechanics: 3;
- reader pool: 6;
- Slice 75 feature interactions: 4;
- frozen reads: 5;
- positive MEMSTATUS witness: 1, with `before=0`, `during=1048576`, `after=0`.

No broad regression, package build, global SQLite configuration change,
allocator substitution, tcmalloc preload, schema change, or public API change
was performed. The experimental product and test diff is removed at slice
close; its commits, binary hashes, raw logs, profiles, and commands remain
identified by the manifest.

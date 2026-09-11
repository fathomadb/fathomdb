# Slice 79 result

Slice 79 ships the reviewed runtime-mode API and statement reuse, but does not
close AC-020. It closes as a bounded implementation result with that unchanged
release gate carried to Slice 80.

## AC-020

One release binary from product candidate `a6650c81` ran the registered fixture
in the sealed 14-process order. Both modes failed all seven unchanged oracles.

| Mode | Sequential median | Concurrent median | Median per-run speedup | Passes |
| --- | ---: | ---: | ---: | ---: |
| Diagnostics (`MEMSTATUS=1`) | 170.293082 ms | 76.731105 ms | 2.219349x | 0/7 |
| Performance (`MEMSTATUS=0`) | 174.632177 ms | 53.616722 ms | 3.135542x | 0/7 |

The performance-mode median bound was 32.743532 ms, so concurrent time still
needs about a 39% reduction at unchanged sequential performance. Across the
seven adjacent counterbalanced pairs, performance mode reduced concurrent time
by a median 19.376916 ms (25.23%) while sequential time differed by a median
3.216421 ms (1.95%). The mechanism is material, but insufficient.
The ratio of performance-mode medians is 3.257047x; 3.135542x in the table is
the median of the seven independently calculated per-run speedups.

Direct binary execution initially omitted the existing `AGENT_LONG=1` fixture
guard and produced no measurements; a second command had invalid `env` option
ordering and exited 127 before launching tests. The retained logs are the first
and only measured series after correcting those invocation defects. No result
was retried, tuned, or discarded.

## Protected guards

- AC-072 met its numerical limits in all three 10k/384d/1,000-query
  repetitions: p50 was 69 ms in every run and p99 was 76, 95, and 75 ms
  against 80/300 ms limits. All three runs had nonzero swap activity, however,
  so the retained Slice 71 environment policy makes them environment-invalid.
  They are qualified descriptive evidence, not an applicable AC-072 pass.
- Scale-02 acknowledgement was 1377.056350–1388.886163 ms and total was
  1381.492867–1393.327169 ms, within the 1543.539/1548.545 ms limits; spreads
  were 0.8591% and 0.8566%.
- Projection-active total was 1245.943056–1316.862285 ms, within the
  1442.198 ms limit with 5.6920% spread. Its 33.9289% asynchronous
  acknowledgement spread is descriptive; the authoritative Slice 71 recovery
  gates total time only. All six write-cell environments and applicable
  limits passed.

No historical baseline, broad regression, hosted CI, or package matrix was
rerun.

The populated schema-26 upgrade/reopen/projection/lifecycle witness passes 2/2
after its test setup selects the application-owned runtime before constructing
the old database. No migration or product behavior was changed.

## Disposition

Retain the runtime API, application-owned startup contract, performance default,
and statement reuse. They deliver a large absolute improvement and the
diagnostics choice remains explicit. Do not weaken AC-020 or discard statement
reuse to inflate its ratio. Slice 80 must attribute and correct the remaining
post-cache/performance-mode concurrency cost and obtain an environment-valid
AC-072 guard before Slice 85 final verification.

Exact observations and artifact identities are in `manifest.json`; raw timing,
environment, and write-cell records are retained beside it.

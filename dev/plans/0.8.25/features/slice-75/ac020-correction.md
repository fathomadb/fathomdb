# Slice 75 AC-020 correction

The unchanged final-candidate gate was RED at 537 ms sequential, 129 ms
concurrent, and a 100 ms bound.

A fixed-workload comparison used the repository's existing experiment seams:

- PCACHE2 alone remained RED at 573/121/107 ms.
- Memory-status accounting off passed four consecutive observations; the
  narrowest was 557/102/104 ms.
- Memory-status off plus PCACHE2 passed at 541/81/101 ms, but the custom
  allocator was rejected as unnecessary production blast radius.
- Page size and large reader-cache changes were not selected.

The correction disables SQLite's unused memory-allocation statistics before
FathomDB's first connection. It neither shuts down an already-initialized
SQLite runtime nor changes storage format, durability, cache limits, public
APIs, or the AC-020 threshold.

The exact acceptance command then passed at 520/91/97 ms with all experiment
environment variables absent. Focused post-change results also passed:

- AC-076: p50 1 ms, p99 2 ms.
- AC-072: p50 72 ms, p99 93 ms.
- AC-021: 60 seconds, 60 add/drop cycles, 60 rebuilds, zero `SQLITE_SCHEMA`.
- AC-059b: direct contract and 1,000-read race.
- AC-034a/b: 100/100 integrity checks, 4 ms p99 lost commit.
- Slice 75 feature interactions: 4/4.
- Populated schema-26 upgrade and reopen: 2/2.

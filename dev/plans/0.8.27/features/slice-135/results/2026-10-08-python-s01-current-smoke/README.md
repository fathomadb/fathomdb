---
title: Slice 135 current-wheel installed Python S01 functional smoke
status: FUNCTIONAL_SMOKE_NOT_TIMING_COMPARISON
target_release: 0.8.27
---

# Current-wheel installed Python S01 smoke — 2026-10-08

The installed wheel from source `d465cd56d2e863f900ea9a9da8bc372ca6a077c5`
has SHA-256 `33d37be7c80183b063788889954da698ea959997fc13b61e356006a3c26c8894`.
Its installed native module has SHA-256
`23a22d30b3ad700acd602000b0c26200ce3878a080895348546448ea79eb74d9`.
The engine source tree, Python package tree and `Cargo.lock` match the later
Slice 135 branch at the [current S01 protocol freeze](../../s01-python-current-comparison-protocol.json);
intervening changes are test and documentation work.

The [32-row raw result](slice135-python-s01-current-smoke-d465-32.json) and
[256-row raw result](slice135-python-s01-current-smoke-d465-256.json) each
exercise installed Python text, vector-bearing and hybrid calls with ten warm
samples plus one session-first and one warmup observation per cell. All 72
observations passed the seeded-result checks. The
[independent check](slice135-python-s01-current-smoke-d465-audit.json)
recomputed sample counts, source and wheel identity, corpus/model/query
settings and materialized IDs and branches. Changed ordered text IDs were
rejected at both corpus sizes. The native module bytes were checked against
the installed artifact path.

The two raw SHA-256 values are
`4998177f2f039a9ea7ece4abc9e56c540c6edbc749430b03c785d0302b77f7c8`
and `60ceae60f6a93ead65e19386ea4e3cf601c6ad1c09f8a7a4bcf6d86cec6c1fdf`;
the independent check is
`e2b6f832105920de788f5c4d845a18fc785f56d543972549099df5438d6f3604`.
The wheel remains locally retained with the
[current SDK capability receipt](../2026-10-08-current-sdk-capabilities/README.md)
pending end-of-phase archive retention.

Ten warm samples are functional feasibility only and do not estimate p99 or
compare version latency. The [frozen paired subset](../../s01-python-current-comparison-protocol.json)
sets 1,000 warm samples per cell and five alternating pairs at both sizes
before candidate timing.

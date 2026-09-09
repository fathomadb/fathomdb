---
title: Slice 71B — performance recovery
status: PASS
date: 2026-09-09
candidate: eda95b07a31f7c65cac8ac2a856a7f40de5cc72a
---

# Slice 71B performance recovery

Slice 71B recovers the write regression without changing its performance
limits. The exact correction is `eda95b07`; its release probe SHA-256 is
`df47663b32c572ffd8c995d7f7c0613de56d577ec1e78d1df92392ae1e101f18`.

The fix prepares governed write statements once, advances frozen-read
visibility once per canonical or projection transaction, caches invariant
projection checks, and raises the existing two-worker projection commit batch
from 16 to 64. Persistent triggers remain active for external SQLite clients;
custom main/TEMP triggers select the original row-trigger path. The final
review correction also keeps `drain(timeout)` bounded when the writer mutex is
busy and restores every capacity-dependent concurrency fixture.

## Acceptance result

Matched historical baselines are `b2bfb1f3` for Scale-02 and `4fc1b890` for
AC-013. Unchanged current is `c80e6909`. Times below are medians in
milliseconds. The 10k limits are historical +20%. A small result regresses only
when it is both more than 10% and more than 0.25 ms slower than historical.

| Fixture | Rows | Historical ack / total | Current ack / total | Corrected ack / total | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Scale-02 | 1 | 0.585 / 1.432 | 0.739 / 1.948 | 0.765 / 0.957 | PASS; ack delta 0.180 ms |
| Scale-02 | 10 | 1.769 / 2.568 | 2.349 / 3.560 | 1.505 / 1.699 | PASS |
| Scale-02 | 100 | 17.546 / 18.295 | 31.143 / 32.245 | 12.755 / 12.968 | PASS |
| Scale-02 | 1,000 | 181.376 / 182.214 | 420.398 / 422.726 | 122.168 / 122.873 | PASS |
| Scale-02 | 10,000 | 1,833.777 / 1,834.571 | 4,251.744 / 4,262.412 | 1,403.217 / 1,407.768 | PASS |
| AC-013 | 1 | 0.349 / 8.029 | 0.527 / 10.579 | 0.531 / 7.387 | PASS; ack delta 0.182 ms |
| AC-013 | 10 | 0.973 / 8.873 | 1.788 / 13.455 | 0.952 / 9.149 | PASS |
| AC-013 | 100 | 5.839 / 23.232 | 14.708 / 48.492 | 4.103 / 19.914 | PASS |
| AC-013 | 1,000 | 51.497 / 182.164 | 152.034 / 419.505 | 33.579 / 119.578 | PASS |
| AC-013 | 10,000 | 587.711 / 2,337.097 | 3,944.661 / 4,929.785 | 840.608 / 1,311.089 | PASS |

The corrected Scale-02 10k acknowledgement and total are respectively 23.5%
and 23.3% faster than historical. Corrected AC-013 10k total is 43.9% faster
than historical. The write regression is recovered rather than accepted or
deferred.

## Retained prospective repetitions

Each item is `ack/total` in milliseconds. Small workloads use one transaction;
10k uses 256-row Scale-02 and 1,024-row AC-013 foreground batches.

| Fixture | 1 | 10 | 100 | 1,000 | 10,000 |
| --- | --- | --- | --- | --- | --- |
| Scale-02 | 0.780/0.975, 0.765/0.957, 0.743/0.935 | 1.505/1.699, 1.436/1.626, 1.532/1.727 | 12.755/12.968, 13.345/13.554, 12.712/12.920 | 122.168/122.873, 121.914/122.564, 122.778/123.419 | 1,403.217/1,407.768, 1,421.565/1,426.151, 1,391.422/1,395.921 |
| AC-013 | 0.531/7.578, 0.512/7.197, 0.582/7.387 | 0.938/8.321, 0.957/9.149, 0.952/9.209 | 4.601/20.973, 4.103/19.914, 4.065/19.873 | 33.140/119.362, 33.732/119.578, 33.579/124.346 | 534.557/1,311.089, 840.608/1,312.575, 1,024.937/1,295.047 |

All observations are retained. Scale-02 10k acknowledgement and total spreads
are 2.17%; AC-013 10k total spread is 1.35%. AC-013 acknowledgement is retained
as the asynchronous work partition and is not used to hide completion cost.
Scale-02 used 40 foreground transactions with a 16-row final batch. AC-013 used
10 foreground transactions with a 784-row final batch and 157 projection
commits with a 16-row final batch. Its 1/10/100/1,000-row cells used 1/1/2/16
projection commits with final batches 1/10/36/40.

The command, exact raw JSON, host snapshots, timeouts, identities, and fresh
database path are retained in
[`raw.log`](../../../runs/0.8.25-slice-71/71b/recovery/eda95b07/raw.log), SHA-256
`0b90f76e647a3ab6896d380159b9602e12b3c139f77bdcdaf98ac0a55acb7fbc`.
The registered environment validator passes: load stayed below 0.44 across 24
CPUs, available memory stayed above 89%, swap I/O stayed zero, temperature
stayed below 79 °C, and no competing process was present. Earlier invalid runs
remain retained and were not reused after clarification `c1c12d5d`.

## Focused verification

The exact correction passes the visibility/coalescing and custom-trigger unit
tests, Slice 30 dependency/readiness tests, Slice 35 frozen-read tests, Slice 40
generation/completion/race tests, projection runtime tests, TC-91 projection
failure recovery, EU-5f mean pinning, and the five capacity-dependent fixtures
updated for batch 64/in-flight 128. The targeted drain-deadline RED/GREEN test,
affected-crate `cargo check`, and `cargo clippy -D warnings` pass. Independent
code review passes on `eda95b07`. No broad verification round, AC-072 rerun,
Windows/CUDA run, or full release regression was performed.

The exact commands, results, and wall durations are retained in
[`focused-verification.log`](../../../runs/0.8.25-slice-71/71b/recovery/eda95b07/focused-verification.log),
SHA-256
`5a06e1ffd911c51296572ad9cf3a2f3474409e29a22d1ade21f55b16711dbcc1`.
The exact drain-deadline regression invocation and result are retained in
[`drain-deadline-exact.log`](../../../runs/0.8.25-slice-71/71b/recovery/eda95b07/drain-deadline-exact.log),
SHA-256
`dc4604643ca6467be1c5738b48d4362eb258a038864471364b93d1957241209f`.
Selection was limited to the changed visibility/trigger path, projection
eligibility and completion invariants, frozen reads, rollback/failure recovery,
the capacity-coupled fixtures, the drain deadline, and affected-crate static
checks. The retained log records `BROAD_ROUNDS=0`.

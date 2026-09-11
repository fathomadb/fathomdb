# Slice 80.n revised AC-072 qualification audit

Verdict: **PASS — AC-072 ACCEPTANCE COMPLETE**

The owner approved a narrow qualification correction: host-wide Linux
`pswpin`/`pswpout` counters remain recorded diagnostics, but cannot alone reject
FathomDB because they do not attribute swap activity to the SUT. The exact
workload and latency limits remain unchanged: 10,000 rows, 384 dimensions,
1,000 warm-treatment queries, p50 <=80 ms and p99 <=300 ms. Load, memory,
temperature, competitor, affinity, quota, governor, identity, marker, count
and source/build controls remain binding.

The original `raw/ac072-slice80n-continuation-campaign/C*.verdict.json` files
are immutable historical verdicts. This audit writes separate reassessments in
`revised-qualification-audit/C1.json` through `C5.json` with the approved
policy. Each raw record binds source `38987930`, binary
`ff4b78f36898f8109eef2dcb64f91a99741f9619433d77de0cfef0e59d51895d`, and
protected product inputs
`95e15e3e4c089212431b7a173fca291539a072d98c87d3394c1fb9b4f3573274`.

| Cell | p50 / p99 ms | Revised qualification | Recorded machine-wide swap diagnostic |
| --- | ---: | --- | --- |
| C1 | 70 / 79 | pass | none |
| C2 | 70 / 77 | pass | `pswpin +2`, `pswpout +0` |
| C3 | 70 / 77 | pass | `pswpin +5`, `pswpout +0` |
| C4 | 70 / 76 | pass | none |
| C5 | 70 / 76 | pass | `pswpin +3`, `pswpout +0` |

All five retained fresh-process observations are numerically passing and pass
every remaining qualification control. C1–C3 alone establish the required
three consecutive passes; C4–C5 provide additional retained confirmation. No
benchmark was rerun, no observation was substituted, no host policy changed,
and no new threshold was introduced. AC-072 is complete and Slice 85 is
unblocked.

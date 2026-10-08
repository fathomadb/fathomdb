---
title: Slice 135 exact-source installed Python close and memory comparison
status: AUDITED_PHASE1_SUBSET
target_release: 0.8.27
---

# Installed Python S02-L lifecycle at source 224e44c59

The replacement [frozen protocol](../../s02-python-lifecycle-224e-v2-comparison-protocol.json),
SHA-256 `12ddf144fcd07ab52b8138a4cc98ff29fb24463d8b1cef04a6c7a8f00aeaa37e`,
was committed before timing. Its first revision failed preflight because its
campaign runner still pointed to an older wheel; that attempt created no
measured output. The replacement retains the baseline pilot and workload,
and binds the new runner and independent auditor to exact candidate source
`224e44c593c13d86ece648adabe445723db04070` and installed wheel SHA-256
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`.
Baseline source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`, wheel
SHA-256 `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.

All ten alternating blocks passed the [independent raw, order, wheel, resource
and reopened-state audit](independent-audit.json). There were 100 valid fresh
Python-process open/model-use/close cycles per version, no invalid measured
blocks and no host or child warnings. The auditor recomputed every close
sample and all before, open, close, 200 ms closed-idle and handle-drop
memory observations. [Four negative controls](negative-controls.json)
rejected a changed protocol, block order and two raw sample mutations.

| Exact installed Python `close()` boundary | Baseline | Candidate | Change |
| --- | ---: | ---: | ---: |
| Pooled nearest-rank p50 | 5.446 ms | 13.273 ms | +143.722% |
| Pooled nearest-rank p95 | 6.287 ms | 14.591 ms | +132.091% |
| Median of five pair p50 changes | — | — | +140.575% |
| Complete pair p50 range | — | — | +131.856% to +151.141% |
| Median opened-to-closed-idle PSS release | -33 KiB | +45,719 KiB | Different ownership timing |

The candidate releases about 45 MiB of PSS when `close()` returns with the
closed Python engine handle retained. The baseline retains the model until
handle drop. This matches the intended engine-owned embedder release change;
the extra synchronous close time is a genuine lifecycle tradeoff at this
boundary. `close()` remains functional and persisted state reopens correctly
in every measured cycle. These 100 samples support p50/p95 but not p99, and
five pairs do not prove general significance or establish a release gate.

The [copied raw archive](raw-archive/) retains all cycle JSON, source and
wheel identities, resource reports, environment and command records. Its
1,124-file `SHA256SUMS` manifest has SHA-256
`22dba02e0279caaa7112948593fe5f7b577cb473a449dc7f96eb13c081c9f85c`;
all copied bytes were verified. It remains untracked pending end-of-phase
retention.
The independent audit SHA-256 is
`7c3e9fec6692b886cf392f3e398084c44241b4571c54a61bac805748a66c219d`.

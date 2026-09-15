---
title: FathomDB 0.8.26 Slice 35 actuation characterization
status: COMPLETE
measured_on: 2026-09-15
---

# Slice 35 performance characterization

Command:

```text
cargo test -p fathomdb-engine --test slice35_actuation_performance -- --ignored --nocapture
```

Environment: x86_64 Linux, SQLite 3.53.2, debug cargo-test profile,
`fathomdb-noop` caller embedder. Each arm used one fresh database and ran the
same fixed workload sequence, intentionally retaining accumulated state. The
candidate is one current-V1 actuation call for derived node + dependency +
edge; the control is one pre-edge actuation call plus one ordinary edge write.
Latencies measure complete logical units in microseconds.

| Workload | Candidate | Control | Candidate calls | Control calls |
| --- | ---: | ---: | ---: | ---: |
| three-operation unit, 50 samples | p50 5,203; p95 6,665; 182.82/s | p50 4,376; p95 5,082; 218.48/s | 50 | 100 |
| 128 operations | 200,385 | 104,776 | 1 | 2 |
| 1,000 sequential IDs | p50 8,228; p95 11,475; 120.24/s | p50 6,322; p95 10,027; 148.68/s | 1,000 | 2,000 |
| eight concurrent unique IDs | p50 40,104; p95 68,260; E2E 77,920; spread 67,506 | p50 43,977; p95 54,413; E2E 56,752; spread 19,187 | 8 | 16 |
| eight exact replays | p50 1,557; p95 2,612; E2E 3,347; spread 2,608 | not equivalent: ordinary edge write has no operation-ID replay | 8 | n/a |

Response totals (candidate/control) were 23,409/23,916 bytes for the 50 units,
2,815/1,941 for the 128-operation batch, 482,234/490,849 for 1,000 sequential
IDs, and 3,872/3,944 for eight unique concurrent IDs. Eight exact replays
returned 3,872 candidate bytes. Each logical unit and exact replay named one
pending cursor before settling.

Both arms had a 557,059-byte checkpointed seeded baseline. After public drain,
close, and successful `wal_checkpoint(TRUNCATE)`, candidate storage was
6,414,339 bytes (5,857,280 growth across 1,123 stored logical units, 5,215.74
bytes/unit); control storage was 6,115,331 bytes (5,558,272 growth across 1,122
stored logical units, 4,953.90 bytes/unit). The one-unit difference is the
candidate's stored shared-ID setup receipt; exact replays add no domain unit.
All 24 measured concurrent calls returned successfully, so the observed public
writer/storage lock-failure count was zero. The candidate emitted one slow
event, attributable to the 200 ms 128-operation call; the control emitted none.

Disposition: retain the single-call prototype. It pays bounded simulation,
receipt, and source-reference overhead, but remains millisecond-scale for the
canonical unit, completes the maximum 128-operation request, converges exact
replay cheaply, and removes the control's unprotected crash boundary. No
performance threshold was exceeded because Slice 35 defines characterization,
not a compatibility gate. Optimization of ordinary write or transaction
machinery is not justified by this evidence.

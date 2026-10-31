# AC42-16 — `AGENT_LONG` perf-gate baseline on SQLite 3.54.1

This baseline is for Slices 50 to 90 to measure against. It is not a pass/fail
gate for Slice 42 (REQ42-B8). `perf_gates.rs` embeds with deterministic
synthetic embedders; no BGE model runs, so no GPU is involved.

- **Command:** `AGENT_LONG=1 cargo test --release -p fathomdb-engine --features
  test-hooks --test perf_gates -- --test-threads=1 --nocapture`. No `AC_*` or
  `FATHOMDB_TIER1_*` variables were set, so the default dev-host scales apply.
- **Tree and host:** tree `7d48766dd`; host windchill3, x86_64, nproc 24.
- **SQLite:** 3.54.1 from the crates.io `libsqlite3-sys` 0.39.1.
- **Load:** 2.25 at the start of the run and 5.68 at the end, on a shared host
  where other sessions were running.
- **Duration:** 742 s, from 13:03:54 to 13:16:34 on 2026-10-31.

## Full run (`7d48766dd`)

| Gate | Result | Numbers |
| --- | --- | --- |
| AC-011 protocol, AC-011a, AC-011b | pass | tier-1 write throughput not enabled (`FATHOMDB_TIER1_WRITE_PERF` unset) |
| AC-012 text query, n = 100,000 (reported tier, not binding) | pass | p50 12 ms, p99 29 ms (budget 20 / 150 ms, binding only at n ≤ 10,000) |
| AC-013 vector read-path smoke | pass | |
| **AC-013 vector retrieval latency, n = 10,000 (binding)** | **FAIL** | p50 128 ms, p99 155 ms against the 80 / 300 ms budget (`perf_gates.rs:808`) |
| AC-013b recall floor (reported) | pass | recall@10 0.0036 on the isotropic synthetic embedder (reported only; the real-corpus floor is eu7) |
| AC-017 vector projection freshness | pass | |
| AC-018 drain of 100 vectors | pass | 45 ms |
| AC-019 mixed retrieval stress, n = 10,000, 8 × 250 | pass | baseline p99 142 ms, stress p99 262 ms, bound 1,427 ms |
| AC-081 absolute read performance | pass | sequential 292.5 ms (warning), concurrent 53.4 ms, 1,600 searches each, ratio 5.47 |
| AC-081 oracle self-tests | pass | |
| AC-020 variants, A.3 diagnostics | ignored | retired (seq-277) or opt-in |

The run reported 16 passed, 1 failed and 7 ignored. `ac013_scale_treatment_rejects_invalid_value` is a `should_panic` test, and it passed.

## AC-013 and AC-012, base against final, same session

The AC-013 failure is the known regression that Slice 40 Unit I exposed, which
Slices 50 to 90 fix (master plan). To check that the SQLite bump does not
cause it, both gates were run in alternating rounds on the base `59079d5c5`
(vendored SQLite 3.53.2, already with per-connection caches) and on the final
`7d48766dd` (SQLite 3.54.1). Each round is the same command filtered to the
two gates.

| Round | Tree | AC-013 p50 / p99 (ms) | AC-012 p50 / p99 (ms) | Load (1 min) |
| --- | --- | --- | --- | --- |
| 1 | base | 125 / 164 | 12 / 29 | 4.26 |
| 1 | final | 129 / 236 | 11 / 29 | 3.08 |
| 2 | base | 125 / 142 | 12 / 30 | 5.75 |
| 2 | final | 129 / 153 | 12 / 30 | 4.51 |

AC-013 fails on both trees. Its p50 is about 3% higher on 3.54.1 (125 against
129 ms in both rounds), while its p99 varies with load on both. AC-012 shows no
change. Two rounds on a shared host cannot resolve a 3% difference; Slices 50
to 90 measure on 3.54.1 from here on.

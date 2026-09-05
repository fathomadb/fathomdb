# Slice 45 pagination performance result

Material means both >10% and >0.25 ms median paired p95 latency, or both
>5% and >8 MiB median peak RSS.

| Scale | Comparison | Baseline p95 ms | Treatment p95 ms | Delta ms | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.2311 | 0.3485 | 0.1167 | 50.51 | false |
| 10000 | frozen_page → mint_plus_page | 0.3485 | 0.4483 | 0.0994 | 28.53 | false |
| 10000 | frozen_page → continuation_page | 0.3485 | 0.3581 | 0.0097 | 2.79 | false |
| 10000 | current_state → frozen_state | 0.0260 | 0.1027 | 0.0769 | 295.55 | false |
| 50000 | exact_page → frozen_page | 0.2371 | 0.3542 | 0.1174 | 49.51 | false |
| 50000 | frozen_page → mint_plus_page | 0.3542 | 0.4613 | 0.1065 | 30.08 | false |
| 50000 | frozen_page → continuation_page | 0.3542 | 0.3636 | 0.0084 | 2.38 | false |
| 50000 | current_state → frozen_state | 0.0261 | 0.1040 | 0.0779 | 298.22 | false |

| Scale | RSS comparison | Baseline MiB | Treatment MiB | Delta MiB | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 18.03 | 19.01 | 0.98 | 5.42 | false |
| 10000 | frozen_page → mint_plus_page | 19.01 | 19.12 | 0.11 | 0.58 | false |
| 10000 | frozen_page → continuation_page | 19.01 | 19.20 | 0.20 | 1.03 | false |
| 10000 | current_state → frozen_state | 17.15 | 17.93 | 0.78 | 4.53 | false |
| 50000 | exact_page → frozen_page | 20.82 | 21.72 | 0.90 | 4.33 | false |
| 50000 | frozen_page → mint_plus_page | 21.72 | 21.88 | 0.16 | 0.72 | false |
| 50000 | frozen_page → continuation_page | 21.72 | 21.90 | 0.18 | 0.81 | false |
| 50000 | current_state → frozen_state | 19.77 | 20.62 | 0.85 | 4.29 | false |

| Scale | Cold open ms | Cold mint ms | Cold first page ms | Cold frozen state ms |
|---:|---:|---:|---:|---:|
| 10000 | 15.3285 | 0.1440 | 0.6034 | 0.2289 |
| 50000 | 19.3566 | 0.1528 | 0.6251 | 0.2341 |

| Scale | Token auth p95 ms | Snapshot binding p95 ms | Cursor auth p95 ms |
|---:|---:|---:|---:|
| 10000 | 0.0168 | 0.1466 | 0.0162 |
| 50000 | 0.0168 | 0.1471 | 0.0162 |

Overall material: **false**

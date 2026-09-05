# Slice 45 pagination performance result

Material means both >10% and >0.25 ms median paired p95 latency, or both
>5% and >8 MiB median peak RSS.

| Scale | Comparison | Baseline p95 ms | Treatment p95 ms | Delta ms | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.2326 | 0.3503 | 0.1169 | 50.27 | false |
| 10000 | frozen_page → mint_plus_page | 0.3503 | 0.4495 | 0.0996 | 28.42 | false |
| 10000 | frozen_page → continuation_page | 0.3503 | 0.3601 | 0.0092 | 2.62 | false |
| 10000 | current_state → frozen_state | 0.0260 | 0.1038 | 0.0778 | 299.76 | false |
| 50000 | exact_page → frozen_page | 0.2466 | 0.3654 | 0.1173 | 47.56 | false |
| 50000 | frozen_page → mint_plus_page | 0.3654 | 0.4663 | 0.1045 | 28.59 | false |
| 50000 | frozen_page → continuation_page | 0.3654 | 0.3752 | 0.0090 | 2.47 | false |
| 50000 | current_state → frozen_state | 0.0265 | 0.1063 | 0.0788 | 297.24 | false |

| Scale | RSS comparison | Baseline MiB | Treatment MiB | Delta MiB | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 18.09 | 19.01 | 0.92 | 5.07 | false |
| 10000 | frozen_page → mint_plus_page | 19.01 | 19.14 | 0.13 | 0.70 | false |
| 10000 | frozen_page → continuation_page | 19.01 | 19.12 | 0.11 | 0.60 | false |
| 10000 | current_state → frozen_state | 17.18 | 17.95 | 0.77 | 4.48 | false |
| 50000 | exact_page → frozen_page | 20.76 | 21.78 | 1.02 | 4.89 | false |
| 50000 | frozen_page → mint_plus_page | 21.78 | 21.79 | 0.01 | 0.05 | false |
| 50000 | frozen_page → continuation_page | 21.78 | 21.86 | 0.09 | 0.39 | false |
| 50000 | current_state → frozen_state | 19.77 | 20.53 | 0.77 | 3.87 | false |

| Scale | Cold open ms | Cold mint ms | Cold first page ms | Cold frozen state ms |
|---:|---:|---:|---:|---:|
| 10000 | 15.2429 | 0.1519 | 0.6064 | 0.2261 |
| 50000 | 19.5954 | 0.1535 | 0.6291 | 0.2297 |

| Scale | Token auth p95 ms | Snapshot binding p95 ms | Cursor auth p95 ms |
|---:|---:|---:|---:|
| 10000 | 0.0168 | 0.1459 | 0.0000 |
| 50000 | 0.0173 | 0.1508 | 0.0001 |

Overall material: **false**

# Slice 45 pagination performance result

Material means both >10% and >0.25 ms median paired p95 latency, or both
>10% and >0.25 ms median paired cold-operation latency, or both >5% and
>8 MiB median peak RSS.

| Scale | Comparison | Baseline p95 ms | Treatment p95 ms | Delta ms | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.2358 | 0.3519 | 0.1158 | 49.11 | false |
| 10000 | preminted_page → mint_plus_page | 0.3507 | 0.4551 | 0.1044 | 29.79 | false |
| 10000 | first_page → continuation_page | 0.3504 | 0.3604 | 0.0107 | 3.04 | false |
| 10000 | current_state → frozen_state | 0.0261 | 0.1057 | 0.0795 | 304.03 | false |
| 50000 | exact_page → frozen_page | 0.2383 | 0.3549 | 0.1164 | 48.84 | false |
| 50000 | preminted_page → mint_plus_page | 0.3547 | 0.4637 | 0.1084 | 30.56 | false |
| 50000 | first_page → continuation_page | 0.3540 | 0.3640 | 0.0106 | 2.99 | false |
| 50000 | current_state → frozen_state | 0.0257 | 0.1054 | 0.0789 | 307.50 | false |

| Scale | Comparison | Baseline ops/s | Treatment ops/s | Delta % |
|---:|---|---:|---:|---:|
| 10000 | exact_page → frozen_page | 4390.2 | 2932.6 | -33.20 |
| 10000 | preminted_page → mint_plus_page | 2934.2 | 2253.1 | -23.21 |
| 10000 | first_page → continuation_page | 2938.9 | 2848.7 | -3.07 |
| 10000 | current_state → frozen_state | 43372.4 | 10175.9 | -76.54 |
| 50000 | exact_page → frozen_page | 4325.6 | 2899.0 | -32.98 |
| 50000 | preminted_page → mint_plus_page | 2903.7 | 2209.5 | -23.91 |
| 50000 | first_page → continuation_page | 2909.8 | 2825.4 | -2.90 |
| 50000 | current_state → frozen_state | 43397.9 | 10175.1 | -76.55 |

| Scale | RSS comparison | Baseline MiB | Treatment MiB | Delta MiB | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 18.05 | 18.99 | 0.95 | 5.24 | false |
| 10000 | frozen_page → mint_plus_page | 18.99 | 19.11 | 0.12 | 0.64 | false |
| 10000 | frozen_page → continuation_page | 18.99 | 19.01 | 0.02 | 0.08 | false |
| 10000 | current_state → frozen_state | 16.92 | 17.83 | 0.91 | 5.38 | false |
| 50000 | exact_page → frozen_page | 19.01 | 20.18 | 1.17 | 6.17 | false |
| 50000 | frozen_page → mint_plus_page | 20.18 | 20.21 | 0.03 | 0.14 | false |
| 50000 | frozen_page → continuation_page | 20.18 | 20.13 | -0.05 | -0.25 | false |
| 50000 | current_state → frozen_state | 17.95 | 18.82 | 0.87 | 4.85 | false |

| Scale | Cold comparison | Baseline ms | Treatment ms | Delta ms | Delta % | Baseline open ms | Treatment open ms | Material |
|---:|---|---:|---:|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.4228 | 0.6341 | 0.2096 | 49.57 | 16.6422 | 16.7221 | false |
| 10000 | preminted_page → mint_plus_page | 0.6329 | 0.7826 | 0.1314 | 20.77 | 16.5007 | 16.7912 | false |
| 10000 | first_page → continuation_page | 0.6330 | 0.6495 | 0.0123 | 1.94 | 16.9100 | 16.6606 | false |
| 10000 | current_state → frozen_state | 0.0764 | 0.2505 | 0.1714 | 224.46 | 16.8912 | 16.7336 | false |
| 50000 | exact_page → frozen_page | 0.4376 | 0.6508 | 0.2178 | 49.77 | 20.3514 | 20.6777 | false |
| 50000 | preminted_page → mint_plus_page | 0.6384 | 0.7839 | 0.1530 | 23.97 | 20.5820 | 20.3609 | false |
| 50000 | first_page → continuation_page | 0.6490 | 0.6711 | 0.0197 | 3.03 | 20.7931 | 20.6940 | false |
| 50000 | current_state → frozen_state | 0.0772 | 0.2526 | 0.1805 | 233.69 | 20.8835 | 20.5951 | false |

| Scale | Mint context p95 ms | Mint snapshot p95 ms | Mint binding p95 ms | Mint codec p95 ms | Page token auth p95 ms | Page binding p95 ms | Cursor auth p95 ms |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 10000 | 0.0004 | 0.0081 | 0.0650 | 0.0103 | 0.0169 | 0.1548 | 0.0163 |
| 50000 | 0.0004 | 0.0130 | 0.0650 | 0.0102 | 0.0169 | 0.1539 | 0.0163 |

| Scale | Public list p95 ms | Public list ops/s | Full walk ms | Pages | Items | Walk items/s |
|---:|---:|---:|---:|---:|---:|---:|
| 10000 | 0.1840 | 5651.4 | 45.92 | 100 | 10000 | 217788.5 |
| 50000 | 0.1895 | 5489.1 | 208.92 | 500 | 50000 | 239330.2 |

Overall material: **false**

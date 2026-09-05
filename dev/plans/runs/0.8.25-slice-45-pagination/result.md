# Slice 45 pagination performance result

Material means both >10% and >0.25 ms median paired p95 latency, or both
>10% and >0.25 ms median paired cold-operation latency, or both >5% and
>8 MiB median peak RSS.

| Scale | Comparison | Baseline p95 ms | Treatment p95 ms | Delta ms | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.2345 | 0.3519 | 0.1177 | 50.17 | false |
| 10000 | preminted_page → mint_plus_page | 0.3515 | 0.4530 | 0.1020 | 29.02 | false |
| 10000 | first_page → continuation_page | 0.3509 | 0.3609 | 0.0100 | 2.86 | false |
| 10000 | current_state → frozen_state | 0.0260 | 0.1074 | 0.0811 | 311.83 | false |
| 50000 | exact_page → frozen_page | 0.2387 | 0.3563 | 0.1175 | 49.23 | false |
| 50000 | preminted_page → mint_plus_page | 0.3554 | 0.4644 | 0.1083 | 30.47 | false |
| 50000 | first_page → continuation_page | 0.3543 | 0.3652 | 0.0110 | 3.11 | false |
| 50000 | current_state → frozen_state | 0.0261 | 0.1079 | 0.0818 | 313.55 | false |

| Scale | Comparison | Baseline ops/s | Treatment ops/s | Delta % |
|---:|---|---:|---:|---:|
| 10000 | exact_page → frozen_page | 4420.7 | 2932.0 | -33.68 |
| 10000 | preminted_page → mint_plus_page | 2929.6 | 2259.0 | -22.89 |
| 10000 | first_page → continuation_page | 2933.1 | 2849.4 | -2.85 |
| 10000 | current_state → frozen_state | 42289.5 | 9942.1 | -76.49 |
| 50000 | exact_page → frozen_page | 4330.1 | 2892.9 | -33.19 |
| 50000 | preminted_page → mint_plus_page | 2896.0 | 2206.2 | -23.82 |
| 50000 | first_page → continuation_page | 2903.3 | 2822.1 | -2.80 |
| 50000 | current_state → frozen_state | 42239.3 | 9953.1 | -76.44 |

| Scale | RSS comparison | Baseline MiB | Treatment MiB | Delta MiB | Delta % | Material |
|---:|---|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 18.89 | 19.78 | 0.88 | 4.67 | false |
| 10000 | frozen_page → mint_plus_page | 19.78 | 19.84 | 0.07 | 0.34 | false |
| 10000 | frozen_page → continuation_page | 19.78 | 19.96 | 0.19 | 0.95 | false |
| 10000 | current_state → frozen_state | 17.97 | 18.73 | 0.76 | 4.24 | false |
| 50000 | exact_page → frozen_page | 21.60 | 22.52 | 0.92 | 4.25 | false |
| 50000 | frozen_page → mint_plus_page | 22.52 | 22.50 | -0.02 | -0.09 | false |
| 50000 | frozen_page → continuation_page | 22.52 | 22.53 | 0.02 | 0.07 | false |
| 50000 | current_state → frozen_state | 20.54 | 21.35 | 0.81 | 3.96 | false |

| Scale | Cold comparison | Baseline ms | Treatment ms | Delta ms | Delta % | Baseline open ms | Treatment open ms | Material |
|---:|---|---:|---:|---:|---:|---:|---:|---|
| 10000 | exact_page → frozen_page | 0.4172 | 0.6204 | 0.1999 | 47.91 | 16.3741 | 16.7551 | false |
| 10000 | preminted_page → mint_plus_page | 1.7805 | 0.7644 | -1.0182 | -57.18 | 15.4118 | 16.4427 | false |
| 10000 | first_page → continuation_page | 0.6221 | 0.6583 | 0.0360 | 5.79 | 16.4726 | 16.6039 | false |
| 10000 | current_state → frozen_state | 0.0744 | 0.2511 | 0.1767 | 237.46 | 16.5457 | 16.5615 | false |
| 50000 | exact_page → frozen_page | 0.4276 | 0.6432 | 0.2113 | 49.42 | 20.4756 | 20.7284 | false |
| 50000 | preminted_page → mint_plus_page | 0.6460 | 0.7693 | 0.1265 | 19.59 | 20.6882 | 20.6276 | false |
| 50000 | first_page → continuation_page | 0.6361 | 0.6643 | 0.0282 | 4.44 | 20.8187 | 20.7049 | false |
| 50000 | current_state → frozen_state | 0.0737 | 0.2519 | 0.1775 | 240.65 | 20.4774 | 20.6708 | false |

| Scale | Mint context p95 ms | Mint snapshot p95 ms | Mint binding p95 ms | Mint codec p95 ms | Page token auth p95 ms | Page binding p95 ms | Cursor auth p95 ms |
|---:|---:|---:|---:|---:|---:|---:|---:|
| 10000 | 0.0004 | 0.0082 | 0.0652 | 0.0103 | 0.0170 | 0.1554 | 0.0164 |
| 50000 | 0.0004 | 0.0133 | 0.0650 | 0.0103 | 0.0170 | 0.1546 | 0.0164 |

| Scale | Public list p95 ms | Public list ops/s | Full walk ms | Pages | Items | Walk items/s |
|---:|---:|---:|---:|---:|---:|---:|
| 10000 | 0.1828 | 5683.5 | 45.75 | 100 | 10000 | 218561.1 |
| 50000 | 0.1885 | 5528.0 | 208.51 | 500 | 50000 | 239797.5 |

Overall material: **false**

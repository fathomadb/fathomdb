# Installed TypeScript S02 baseline-only noise pilot — 2026-10-07

**Status:** controlled baseline-only pilot, not a 0.8.26/0.8.27 comparison.
The exact 0.8.26 source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`.
The installed npm main/platform archive SHA-256 values are
`90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336`
and `b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6`.
Node was v25.9.0, with CPU embedding and a fresh real database per sequence.

The [block runner](../../../../../../../scripts/slice135_ts_s02_block.py)
captured five separate blocks, each with one fresh-process warm-up and three
measured whole sequences. Each sequence opened an installed SDK engine, wrote
and projected the 32-row corpus and graph, executed text/vector/hybrid and
graph/evidence reads, erased the graph source, closed, reopened and checked SDK
state. The product timer ended after reopened close; an independent SQLite
canonical-row count ran afterward. Raw materialized observations, per-stage
durations, GNU Time child resources, command, host inventory and invalidators
are retained under `block-01` through `block-05`.

The [independent audit](independent-audit.json) rechecked all 20 attempted
sequences and accepted all 15 measured sequences. Block medians were 4,954.871,
4,883.622, 4,951.635, 4,839.937 and 4,944.749 ms. Their spread was
114.934 ms, or 2.324% of the median block median. All five blocks had no host
warning and no measured-child swap. The [negative controls](negative-controls.json)
were rejected for a retained canonical edge and an interleaved direct SQLite
read inside the product timer.

This small pilot supports the separately
[frozen TypeScript S02 paired subset](../../s02-ts-comparison-protocol.json):
five alternating version pairs with 20 measured sequences per block, 100 per
version. It does not support p95/p99 estimation, an equivalence threshold or a
candidate performance claim by itself. `SHA256SUMS` binds the local archive;
raw retention remains local pending the Slice 135 final evidence step.

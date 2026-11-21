---
title: Slice 70 source qualification commands
status: ACTIVE
---

# Slice 70 qualification

The [plan](../../plans/0.8.30/features/slice-70/plan.md) and
[design](../../plans/0.8.30/features/slice-70/design.md) preserve exact existing
canonical selectors, thresholds and evidence reuse. Raw execution logs remain
outside the checkout under `/tmp/fathomdb-0830-slice70-evidence`.

`run_orin_tc5.py CONFIG --output /tmp/fathomdb-0830-slice70/OUTPUT` selects only
the frozen real TC5 bridge through the maintained GPU runner. Use the configured
owned interpreter and wheel; it reuses the existing Orin exclusive lock and
observed-peer/resource helpers. Sampling at one-second cadence is a resource
check, not a100ms fault-protocol or latency claim. No reset is performed.

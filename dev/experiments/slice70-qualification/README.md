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

For an observation-only failure after a complete ingest, `--resume-from PRIOR`
reuses only its byte-pinned complete database with no uncheckpointed WAL.
The maintained runner validates the full first-query input/model/binary binding
and corpus counts, then copies to a new root and runs all100 queries afresh.
Record this as assembled proof and retain the original failed cell; no ingest
or already passing native qualification is repeated.
The original ingest's wheel and candidate-configuration identity must be retained
independently. The first-query manifest pins model/index/executor inputs; it is
not by itself an attestation of which Python wheel performed the ingest.

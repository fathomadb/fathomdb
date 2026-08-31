# GLOBAL-01 v2 recovery preflight result

**Status:** authorized for paid execution at a $12 hard cap.

The fresh v2 zero-spend preflight passed with configuration SHA-256
`483e11adaa302b75bda57f359894b4b01763ec65d63017cc112570977af08208`.

- All 1,397 corpus documents and 49 qualified questions matched.
- The development, witness, and held-out selection hashes matched.
- Isolated FathomDB Python and CLI versions were 0.8.23.
- Strict-current supersession, erasure, and temporal canaries passed.
- All 49 retrieval probes returned the registered depth of 50.
- Airlock exposed `deepseek-v4-pro` and `claude-haiku` through the virtual key.
- Checkpoint, missing-cell resume, backoff, completeness, and cost-cap controls
  remained enabled.
- Preflight spend was $0.

The measured retrieval steady p95 was 18.65 ms. The unchanged paid projection
is $9.50. Coreyt authorized the fresh v2 run on 2026-08-29 with a $12 hard cap.
The next gate is fresh A/A followed by the three-question witness; held-out
execution remains conditional on a valid witness.

## Records

- [Recovery contract](2026-08-29-global-01-v2-recovery-contract.md)
- [Authorized safe preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T2045Z-483e11ad/record.json)
- [Stopped v1 witness](../../experiments/runs/global-01-lazy-witness-20260829T1924Z-aa159044/record.json)

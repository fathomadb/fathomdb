# GLOBAL-01 v2 recovery preflight result

**Status:** ready for HITL; no model-completion call authorized.

The fresh v2 zero-spend preflight passed with configuration SHA-256
`15096cae2823d621954a640e185c8238e01ff8b46150c743c9c644a2f2af4cee`.

- All 1,397 corpus documents and 49 qualified questions matched.
- The development, witness, and held-out selection hashes matched.
- Isolated FathomDB Python and CLI versions were 0.8.23.
- Strict-current supersession, erasure, and temporal canaries passed.
- All 49 retrieval probes returned the registered depth of 50.
- Airlock exposed `deepseek-v4-pro` and `claude-haiku` through the virtual key.
- Checkpoint, missing-cell resume, backoff, completeness, and cost-cap controls
  remained enabled.
- Preflight spend was $0.

The measured retrieval steady p95 was 18.07 ms. The unchanged paid projection
is $9.50 with a proposed $12 hard cap. The v2 configuration remains
`pending_hitl`; the prior v1 authorization does not authorize this fresh run.

## Records

- [Recovery contract](2026-08-29-global-01-v2-recovery-contract.md)
- [Safe preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T2026Z-15096cae/record.json)
- [Stopped v1 witness](../../experiments/runs/global-01-lazy-witness-20260829T1924Z-aa159044/record.json)

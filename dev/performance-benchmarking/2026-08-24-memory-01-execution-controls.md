# MEMORY-01 execution controls

## Fixed comparison

- Profile: A0 turn-level FTS versus native Mem0 OSS.
- Corpus: LOCOMO SHA-256
  `79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4`.
- Scope: all 1,540 category 1–4 questions, top 10.
- Mem0 extraction: `gpt-4o-mini`; embedding:
  `text-embedding-3-small`.
- Scoring: identical `gpt-4o-mini` answerer and judge with official
  evidence-aware LOCOMO prompts.
- Spend ceiling: $20 across native ingestion and paired scoring.

## Rate and recovery controls

- Airlock admission: 120 requests per minute, concurrency 4.
- Native harness: 20 evenly spaced Mem0 requests per minute, concurrency 2.
  The remaining admission capacity covers Mem0's internal extraction and
  embedding fanout without front-loading a token-bucket burst.
- Breaker: four rate-limit responses in 60 seconds; 30-second client and
  provider cooldowns.
- Provider `Retry-After` takes precedence. Without that header, bounded
  exponential backoff applies.
- A failed ingestion chunk remains incomplete. Rerunning with `--resume`
  retries it rather than accepting a partial arm.

Airlock commit `7a1ed74` applies admission and breaker configuration inside
the spawned proxy process. Commit `00752e3` scopes the benchmark threat
exemption to the authenticated virtual-key identity. The derived official harness commit
`41d1e633c78dd9102f466b0255971a664f833fd3` is based on upstream
`4b61c5d31b9c668a12b4f5e78064248a02c82d2b`; its resilience patch is
`experiments/configs/mem0-oss/memory01-resilience.patch`.

## Current evidence

- [FathomDB arm](../../experiments/runs/fathomdb-locomo-official-seam-20260824T1309Z-3762b22a/record.json):
  complete, 1,540/1,540.
- [Native Mem0 arm](../../experiments/runs/mem0-oss-locomo-native-20260824T1325Z-9de95019/record.json):
  complete, 1,540/1,540 with no failed ingestions.
- Provider route: exact `gpt-4o-mini` and `text-embedding-3-small` aliases via
  OpenRouter and authenticated loopback Airlock.
- [Paired decision](../../experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be/record.json):
  pass; FathomDB 75.19%, Mem0 OSS 67.21%, overall delta +7.99 points, one-sided
  95% lower bound +5.78 points.
- Campaign spend: $6.52 of the approved $20 ceiling.

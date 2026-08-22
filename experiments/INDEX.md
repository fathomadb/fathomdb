# Experiment index

GENERATED FROM `index.jsonl` — do NOT hand-edit. Regenerate with `experiments/_lib.regen_index_md()`.

| ts | experiment | run_id | verdict | n | git_sha | cost_usd | headline | review |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-08-14T22:47:00Z | locomo-capability-preflight | locomo-capability-preflight-20260814T2247Z-1ff8b7bc | blocked_prerequisite | 1540 | 06a32c86edca5b3db02a5d016830a9e8ad835d69 | 0.0 | phase=A; provenance=ready |  |
| 2026-08-14T22:49:00Z | locomo-capability-gpu-wheel | locomo-capability-gpu-wheel-20260814T2249Z-5b1adaf8 | blocked_prerequisite |  | 289cae55044eb8cdd0cc03d275dcf610a1771096 | 0.0 | gpu_wheel=blocked_nvcc; phase=A |  |
| 2026-08-14T22:53:00Z | fathomdb-locomo-official-seam | fathomdb-locomo-official-seam-20260814T2253Z-aa0e73a8 | complete | 152 | e07381fc00db6daea7ee6a9ed26f387e77b11559 | 0.0 | retrieval_mode=fts_only |  |
| 2026-08-14T23:00:00Z | locomo-capability-retrieval-analysis | locomo-capability-retrieval-analysis-20260814T2300Z-c15dc62e | complete | 150 | e07381fc00db6daea7ee6a9ed26f387e77b11559 | 0.0 | r_at_10=0.6733333333333333; retrieval_mode=fts_only |  |
| 2026-08-14T23:03:00Z | fathomdb-locomo-official-seam | fathomdb-locomo-official-seam-20260814T2303Z-fb622897 | complete | 1540 | b7c91fdab25f7f770b9775504655952fe12009ec | 0.0 | retrieval_mode=fts_only |  |
| 2026-08-14T23:11:00Z | locomo-capability-a0-baseline | locomo-capability-a0-baseline-20260814T2311Z-d4a71071 | complete | 1536 | ac8882679d72c2d70974a6f91d24f546ef347b94 | 0.0 | delta=0.02405130733344985; ingest_unit=turn; r_at_10=0.6673177083333334; retrieval_mode=fts_only |  |
| 2026-08-22T12:22:00Z | answer-01-shortlist-dry-run | answer-01-shortlist-dry-run-20260822T1222Z-8a050808 | complete | 32 | f64e2bee8f9e5ec91f01ac0124631d32ba0459b3 | 0.0 | program_track=ANSWER-01; status=dry_run_proof |  |
| 2026-08-22T12:34:00Z | answer-01-shortlist-live | answer-01-shortlist-live-20260822T1234Z-8a050808 | complete | 32 | f64e2bee8f9e5ec91f01ac0124631d32ba0459b3 | 0.05709485 | decision=retain_a0; program_track=ANSWER-01; status=complete |  |
| 2026-08-22T14:46:00Z | tc5-gpu-smoke | tc5-gpu-smoke-20260822T1446Z-2d574205 | complete | 100 | cd72da1403f378e6ea004b5c7e56ecfa8e81b145 | 0.0 | arm=bridge; documents=7667; program_track=SCALE-01; recall_at_10=0.958; status=smoke_complete |  |
| 2026-08-22T16:05:00Z | tc5-gpu-primary | tc5-gpu-primary-20260822T1605Z-2d574205 | complete | 100 | ed3e0d2cc6b950c85d96812fabfdb29033c07583 | 0.0 | arm=primary; documents=17272; program_track=SCALE-01; recall_at_10=0.958; status=primary_complete | aggregate_fix_git_sha=46b38aa14a2000be72ac449d47ec67a335e027cf; status=validated |
| 2026-08-22T17:15:10Z | scale-02-a0-10000 | scale-02-a0-10000-20260822T1715Z-77c37c77 | advisory_limit_observed | 10000 | 636827179ce4fa421fd70ffd17bf0e2aed05ccaa | 0.0 | eligibility=fail; point=10000 |  |
| 2026-08-22T18:19:04Z | scale-02-input-pack | scale-02-input-pack-20260822T1819Z-221cf7ed | complete | 17376 | 4d2a4d65528f7a04a67ed3930ab02cb50aeae42d | 0.0 | file_count=17376; program_track=SCALE-02 |  |
| 2026-08-22T18:23:06Z | scale-02-fts-tuning | scale-02-fts-tuning-20260822T1823Z-27c3a7ae | complete | 10000 | 50869a70a0035cefff5b63ce57814003be7ff61b | 0.0 | cells=6; equivalence_mismatches=22; program_track=SCALE-02 |  |
| 2026-08-22T18:37:06Z | scale-02-fts-tuning | scale-02-fts-tuning-20260822T1837Z-d50ec2cf | complete | 10000 | d2ebdcd4526ad33e4b840eaf3d2c03dbc5f6681f | 0.0 | cells=6; equivalence_mismatches=22; program_track=SCALE-02 |  |
| 2026-08-22T18:51:14Z | scale-02-fts-tuning | scale-02-fts-tuning-20260822T1851Z-51e41245 | complete | 10000 | 6c24da79c09c17ec20ac4f9615c7f6229997e739 | 0.0 | cells=6; equivalence_mismatches=0; program_track=SCALE-02 |  |
| 2026-08-22T18:59:39Z | scale-02-fts-selection | scale-02-fts-selection-20260822T1859Z-946ebdc2 | recommendation_pending_hitl | 10000 | 6492c0879ac9db4b3285e3d9d3dd7d03fb18b6bf | 0.0 | quality_applicability=unchanged; recommended_cell=rank_default |  |

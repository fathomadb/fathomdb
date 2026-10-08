---
title: Slice 135 repaired-source Rust SDK S02 candidate-only timing
status: AUDITED_LOCAL_RAW_PENDING_RETENTION
target_release: 0.8.27
---

# Rust SDK S02 candidate-only timing — 2026-10-07

The [frozen protocol](../../rust-s02-candidate-timing-protocol.json) was
committed at `b38ba3bc5` before the measured campaign. Its SHA-256 is
`ee8113328a4290893beab91f8cb803f4333b5644d68b952bd991d63181318507`.
The clean product source was `3f29d649d0213e595c0dab251a449d92fd625792`
with Rust crate tree `c8eaaa19a0e0e876326b778733ace354cc7e21e8`.
The external Cargo consumer exercised the Rust SDK against a fresh real
database per process. The consumer source, manifest, lockfile, binary, block
runner and independent auditor matched all six frozen SHA-256 values.

The five separate baseline-free pilot blocks each had one warmup and three
measured sequences. All 15 measurements and 20 reopened-state audits passed.
Block medians ranged from 5,739.864 to 5,782.857 ms, a 42.993 ms spread.
Two pilot blocks recorded host-only swap-counter drift; child swap events were
zero. The pilot fixed five serial campaign blocks of 20 measured sequences,
one warmup per block, nearest-rank p50/p95 and no p99 claim.

## Measured campaign

| Block | Valid sequences | Whole p50 (ms) | Whole p95 (ms) | Host swap drift (pages) |
| --- | ---: | ---: | ---: | ---: |
| 1 | 20 | 5,755.730 | 5,820.384 | 2 |
| 2 | 20 | 5,770.103 | 5,814.951 | 0 |
| 3 | 20 | 5,765.189 | 5,802.598 | 4 |
| 4 | 20 | 5,737.423 | 5,825.584 | 0 |
| 5 | 20 | 5,735.493 | 5,786.536 | 0 |

All five blocks passed the independent raw, GNU Time, hash and reopened SQLite
audit. Its fresh connection found `integrity_check=ok`, 32 retained corpus
nodes, zero erased graph nodes and zero erased graph edges in every process.
The campaign has 100 valid measured sequences. Pooled nearest-rank whole
p50 was **5,754.390 ms** and p95 **5,815.786 ms**; the smallest/largest
observations were 5,689.145/5,834.316 ms. The five block p50 values spanned
34.610 ms. Peak measured-child RSS was 293,684 KiB. Two blocks had host-only
paging warnings; measured-child swap events were zero throughout.

The largest named marginal stage was reopened `Engine::open`: p50
4,767.522 ms, p95 4,821.228 ms. Projection drain followed at p50
681.502 ms, then initial open at 211.268 ms. Close and reopened close p50
were 15.478 and 21.139 ms. Stage percentiles cannot be added to reconstruct
the whole percentile. The result identifies the reopen path for investigation;
it does not establish a version regression because 0.8.26 has no Rust SDK
peer. The Python and TypeScript S02 paired results have different call
boundaries and must be interpreted separately. P99, a version delta and an
equivalence verdict are unsupported.

Two actual retained-block negative controls were rejected after their
intermediate hashes were updated: one falsified `evidence_resolved`, and one
changed a retained canonical corpus row while preserving SQLite structural
integrity. The auditor rejected the first semantic claim and the second
reopened-state mismatch. The focused Rust integration, block and audit tests
passed 6/6; Ruff and `rustfmt --check` passed. This is a scoped result, not a
full agent gate.

The local raw pilot directories are `/tmp/slice135-rust-s02-pilot-block-01`
through `-05`; the campaign is
`/tmp/slice135-rust-s02-candidate-campaign`. The campaign includes its
recomputation script, accepted block audits, both negative fixtures and a
verified `SHA256SUMS` covering 1,208 files. Its manifest hash is
`69370577edf9a917599a7251ed8f305ed2b0cfd96916f496890ba097c426adc4`.
The five pilot manifests also verified. Raw archive retention is deferred to
the end of Phase 1, so this local evidence is not yet published branch
evidence. Bounded S02 contention and the other Phase 1 matrices remain open.

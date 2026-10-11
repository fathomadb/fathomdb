---
title: Slice 135 C01 native Mem0 qualification failure
status: QUALIFICATION_FAILED_NO_TIMING
target_release: 0.8.27
---

# C01 native Mem0 qualification — 2026-10-07

**Result:** C01 cannot produce a defensible matched native Mem0 latency
sample from the retained August environment. No C01 timing was run, no index
was changed, and no paid re-ingest occurred. Report C01 as omitted for a
documented qualification failure, not as zero latency or a competitor verdict.

The prior [native Mem0 record](../../../../../../../experiments/runs/mem0-oss-locomo-native-20260824T1325Z-9de95019/record.json)
has SHA-256 `4637f6d4b722d6d1a49dd6e7328b16927314390f875efe7ccac145e4a1bf277b`.
Its pinned raw LOCOMO corpus is present and matches SHA-256
`79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4`.
The [required-artifact receipt](required-artifacts.json) checks every old
absolute path from that record. The exact external harness checkout and its
Python environment, external output root, base Compose file, Compose
override, and resilience patch are absent. The pinned Airlock configuration
path currently names a directory rather than the expected file. The old
record says its code checkout was dirty, so its Git SHA alone does not
reconstruct executable bytes.

The [volume metadata](volumes.json) and offline
[read-only inventory](volume-inventory.txt) show that named Mem0 history and
Qdrant volumes still exist, at about 7.1 MB and 310 MB. They contain a
`history.db` and Qdrant collection files. Both project
[containers](containers.txt) are stopped. Volume existence does not bind the
stored points to the pinned LOCOMO input, the old harness configuration, or
the exact index state at the previous measurement. The August record does
not contain a reusable volume digest or a current index validation receipt.

The volume inventory used a local `python:3.11-slim` image with `--pull never`,
`--network none`, `--read-only`, and both volumes mounted read-only. No
service started. A matched C01 run would need to restore and hash-verify the
external harness/configuration and question subset, validate index identity
and expected point coverage against the frozen LOCOMO input, then freeze the
same warmed client-to-materialized-top-10 boundary for Mem0 and FathomDB.
If the index cannot be validated, the plan's $20 paid re-ingest ceiling and
checkpoint/resume controls apply before any new ingestion.

The focused source-path and Docker inventory checks establish the missing
prerequisites. They do not prove the old index is corrupt or that Mem0 cannot
be measured in a future environment. The [SHA-256 manifest](SHA256SUMS)
binds the four retained raw receipts.

---
title: Slice 135 installed Python S01 paired campaign
status: CAMPAIGN_IN_PROGRESS
target_release: 0.8.27
---

# Installed Python S01 paired campaign

The [frozen subset protocol](../../s01-python-comparison-protocol.json) governs
the baseline and post-Slice-132 candidate comparison. No candidate timing was
examined before its first revision.

The first 32-row baseline block is retained as
[an invalidated attempt](invalid-pre-p99-fix/attempt.json). Its wrapper marked
the block valid and wrote 1,000 warm observations per query shape, but its
summary still labeled p99 unsupported. A failing focused test exposed that
inconsistency. Protocol SHA-256
`13188f0b303f87af1289371ee82dd4889c6e84a3abab7ecb7ba1f659ec370ec4`
and block runner SHA-256
`550e9729059a88c92e085d0a59b0b3614467088dc8e77c623cafafe4af748f3a`
identify that attempted revision. The raw samples are preserved, but this
block is excluded from all paired summaries. The corrected protocol pins a
new block runner hash and restarts pair 1 from a fresh database.

The campaign is in progress. Do not infer a candidate latency verdict from
this directory until all valid paired blocks, independent recomputation,
invalidators and limitations are reported here.

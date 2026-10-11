---
title: Slice 135 Phase 2 vector fidelity attempt 1 — invalid measurement seam
status: INVALID_DUPLICATE_BODY_FUSION
target_release: 0.8.27
---

# Phase 2 vector fidelity attempt 1 — invalid seam

The [first frozen protocol](../../phase2-vector-fidelity-protocol.json),
SHA-256 `535c4217838696d52bfdb27b885e4e19c9177c719cc13efa050e1e974dab1ac7`,
was committed at `a63a3b8c1` before the 1,000-document, 100-query installed
Python pair. The [full workspace gate](verification.stdout) passed 186/186
suites with no skipped or excluded suite and zero security findings. The
baseline and candidate ran on separate fresh databases. Their locally
retained raw JSON, intact and lexically isolated SQLite copies, and
[first audit](audit.json) agree byte-for-byte on document vectors, query
vectors and observed hits. The first audit computed `0.945` mean recall@10
for each version.

**Do not use that number as the Phase 2 vector-stage verdict.** The selected
corpus contained two duplicate canonical bodies: Enron selection positions
0 and 31, and CNN/DailyMail positions 118 and 247. Removing FTS rows left
one vector arm, but the production `fuse_three_arms` path adds both RRF
contributions when the same body appears twice in that arm. The duplicate
CNN body was promoted above a closer hit in three queries
(`cnn_dailymail:090`, `:180`, `:220`), producing exact-f32 rank inversions.
This invalidates the assumption that empty FTS alone exposes pre-fusion
vector order. The result is retained as an invalid attempt and a measurement
diagnostic, not a product correctness loss or a scored release comparison.

The [v2 protocol](../../phase2-vector-fidelity-protocol-v2.json) selects
1,000 globally unique canonical bodies and requires the independent audit to
reject any rank inversion. The raw attempt-1 databases and vectors contain
licensed source-derived material and remain local outside Git; preserve them
with the Slice 135 raw bundle. The first audit and this disposition are
inspectable without promoting its score.

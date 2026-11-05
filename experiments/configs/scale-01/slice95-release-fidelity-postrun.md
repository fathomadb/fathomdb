---
title: Slice 95 TC5 fidelity retrospective classification
status: COMPLETE
target_release: 0.8.28
---

# Slice 95 TC5 fidelity retrospective classification

This document classifies the already completed
`slice95-tc5-release-fidelity-20261105T0546Z-7b0bd356` receipt. It is post-run
classification documentation, not preregistration, a new measurement, or a
change to its immutable record, configuration, metrics or measured source
`5f668f33056d0a550c51810f6ebeeb1ef7d60695`.

Authority is the approved [Slice 95 qualification matrix](../../../dev/plans/0.8.28/features/slice-95/qualification.md),
AC075 fidelity row. The source-bound plan amendment supplies the missing plan
reference; the separate exact-record amendment documents its external summary
locators with omitted kinds. Neither amendment changes the record's bytes or
the existing classifier schema and validation rules.

The classification is **data plane fidelity and uncertainty only**. Real model
embeddings ran on CUDA; candidate selection and exact-f32 reranking ran through
CPU sqlite-vec. Recall uses exact-f32 same-model top-10 truth, with 100 completed
queries over 7,667 real documents and 1,000 bootstrap resamples. The plan binds
recall, bootstrap sigma, both confidence endpoints and the four recorded
coverage counts; every other payload leaf is an explicit identity or run
control exclusion.

The measured implementation executes the private `Engine.tc5_vector_stage`
route through the benchmark executable. It does not execute the classifier's
public `Engine.search`, `search_text_only` or `search_projected_text` operations.
Accordingly, the plan uses storage, corpus/gold and deterministic metric
components, with empty public call paths and execution witnesses. These empty
arrays do not mean that FathomDB was unexecuted: the exact measured runner,
benchmark and engine source blobs, immutable result payload and native artifact
identities document the private vector-stage execution.

The single bridge arm is not a new historical SCALE-01 primary run. This
classification makes no latency, planner, semantic-judge, whole-hybrid,
publication-readiness, durability or database-byte-immutability claim. The
recorded post-run database digest discrepancy remains unexplained. The
measured 0.8.28 candidate retains its recorded 0.8.27 package identity; neither
identity is rewritten by this documentation.

# COMP-01 — NFCorpus FTS comparison

**Status:** preparation; no measured result.

## Decision

How do FathomDB 0.8.27 and local LanceDB FTS compare in latency and retrieval
quality on the same NFCorpus test split?

## Inputs

Use the local BEIR NFCorpus 3,633-document corpus, exactly the 323 test query
IDs from `qrels/test.tsv`, their original graded judgments, and top-10 document
IDs. Freeze source and converted hashes, mappings, and duplicate-body groups.
The full experimental design is in the [campaign plan](../plans/2026-10-13-fathomdb-0.8.27-performance-testing.md).

## Run

First qualify one CPU-only FathomDB 0.8.27 cell with a fresh database and one
timed query pass that saves ordered IDs and latency together. Then add the
local LanceDB arm and use four fresh-database repetitions per arm in the
balanced order specified by the campaign plan. Score the original ID-level
qrels and report duplicate-body-affected queries separately.

## Stop

Do not report a paired result until both arms use the same frozen inputs and
their receipt and result validators pass. Preparation is not a benchmark run.

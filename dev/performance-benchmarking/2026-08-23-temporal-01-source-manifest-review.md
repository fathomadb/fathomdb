# TEMPORAL-01 source-manifest review

**Date:** 2026-08-23  
**Decision:** no external, source-derived validity-window manifest was found in
the upstream TimelineQA or LongMemEval releases. The first TEMPORAL-01
execution is therefore synthetic TRACE validity, not a corpus comparison.

## Review boundary

The required object maps each selected source record to canonical evidence
identities, a declared `[valid_from, valid_until)` window, exclusions, and
query denominators. It must be supplied by, or mechanically derived from, an
upstream source with a reviewed derivation. Timestamps alone are not a
validity-window manifest.

| Upstream | Available source fields | Missing for TEMPORAL-01 |
| --- | --- | --- |
| [LongMemEval](https://github.com/xiaowu0162/LongMemEval/blob/main/README.md) | question date, ordered session IDs/dates, answer-session IDs, and turn-level `has_answer` labels | Record validity intervals, supersession relation, and canonical FathomDB record-to-evidence mapping |
| [TimelineQA](https://github.com/facebookresearch/TimelineQA) and [paper](https://aclanthology.org/2023.findings-acl.6.pdf) | Generated timed episodes; paper-described ground-truth evidence for multi-hop QA | A released deterministic selected-record/evidence/validity manifest; the generator and paper do not publish one |

## Consequence

The external inputs remain registered and may be used only after the missing
mapping is supplied and reviewed. The first cell instead uses the existing
payload-free TRACE lifecycle fixture with explicit synthetic half-open validity
windows and `ReadView(valid_as_of=...)`. Its claim is limited to FathomDB
world-time validity boundary behavior. It cannot establish corpus retrieval,
answer quality, supersession, erasure, or historical-state correctness.

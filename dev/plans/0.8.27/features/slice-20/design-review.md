---
title: FathomDB 0.8.27 Slice 20 - independent design review
status: PASS
reviewed_on: 2026-09-21
---

# Slice 20 independent design review

Verdict: **PASS; implementation may proceed.**

The read-only reviewer found no blocking design defect. The plan is bounded,
matches the accepted one-source dependency and erasure contracts, explains the
actual receipt/closure failure mechanism, preserves requested-bucket report
semantics, and introduces no schema, API, wire, or error-taxonomy drift.

Two P2 clarifications were accepted before implementation:

1. freeze requested-bucket node/edge counts from the original deduplicated
   inventory before exact-row deletion; and
2. after receipt validation/redaction, delete completed soft closures for every
   erased requested source revision, including a revision whose current
   dependent plan is empty.

The first RED fixture then proved that the one-source validator forbids a
direct dependent outside its canonical source bucket. The design was narrowed
before production changes: cross-bucket ordering applies to original versus
replacement source revisions, while dependents remain in the original
requested inventory. This rejects unnecessary physical-set expansion.

The same independent reviewer re-reviewed that narrowing and returned PASS:
requested counts are frozen before mutation, receipt redaction precedes every
relevant completed-soft-closure deletion, and no cursor expansion, recursion,
schema change, or public-surface drift remains.

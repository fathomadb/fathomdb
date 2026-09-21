# GRAPH-RETRIEVAL-01 — Native expansion retrieval

**Status:** implemented; scored run blocked on GPU seed qualification

## Decision

Does native bounded graph expansion improve labelled MuSiQue evidence
retrieval over the identical fused control seeds and fixed candidate budget?

## Plan

Implement GPU-only top-20 seed qualification, immutable arm inputs, native
evidence-to-passage promotion, offline retrieval scoring, typed STaRK blockers,
safe receipts, and the optional gauntlet adapter.

## Stop

Stop on seed-manifest drift, failed historical top-10 parity, CPU encoding,
arm mismatch, budget drift, or an unqualified external dataset.

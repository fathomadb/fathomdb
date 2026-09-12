---
title: FathomDB 0.8.26 Slice 4 — architecture and code alignment review
status: DRAFT
---

# Slice 4 plan — architecture and code alignment review

## Purpose

Review the Slice 3 proposals against accepted architecture and the as-built
code. Propose corrections to architecture or code boundaries without changing
either.

## Method

1. Trace each proposed contract through ADRs, interfaces, schema, engine,
   query, CLI, bindings, packaging, and tests.
2. Identify whether the smallest change is a repair, additive surface,
   successor type, or persisted-format change.
3. Review transaction ownership, frozen-read authority, authorization,
   disclosure, replay, erasure, projection, and upgrade boundaries.
4. Reject designs that move Memex semantic policy into FathomDB.
5. Feed exact decisions and risk mitigations into Slice 6.

## Exit criteria

Every P0–P2 proposal has an as-built alignment finding, a recommended
architecture shape, known implementation seams, and explicit stop conditions.
No code or accepted architecture is changed.

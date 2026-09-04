---
title: 0.8.25 Slice 25 status
status: AWAITING_HITL_DESIGN_FIX_CAP
slice: 25
updated: 2026-09-04
---

# Slice 25 status

## Current state

Slice 25 is commissioned and remains `IN_PROGRESS`, but implementation has not
started. Five design FIX cycles have been consumed. The final independent
review at `0bf1575c` left two P2 ambiguities: refusal precedence under lifecycle
and dual exhaustion, and logical-ID normalization before request hashing.

## Required decision

Authorize one exceptional final documentation-only correction and independent
review using the exact recommendations in
[`design-review-cycle5.md`](design-review-cycle5.md), or postpone Slice 25.
No product code, schema migration, RED test, or next-slice work may begin until
that decision is recorded.

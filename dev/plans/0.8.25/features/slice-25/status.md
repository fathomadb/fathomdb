---
title: 0.8.25 Slice 25 status
status: DESIGN_FIX_6_AUTHORIZED_PENDING_REVIEW
slice: 25
updated: 2026-09-04
---

# Slice 25 status

## Current state

Slice 25 is commissioned and remains `IN_PROGRESS`; implementation has not
started. Five ordinary design FIX cycles were consumed. At `seq-275`, the
release owner authorized one exceptional documentation-only FIX-6 and final
independent review for the two P2 ambiguities left at cycle 5.

## Required decision

FIX-6 applies the exact recommendations in
[`design-review-cycle5.md`](design-review-cycle5.md): lifecycle refusal and
counter-exhaustion precedence, logical-ID normalization before hashing, and
the related keyed receipt validation clarification. Product code, schema
migration, and RED tests remain blocked until the exceptional independent
review passes.

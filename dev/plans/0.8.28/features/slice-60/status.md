---
title: FathomDB 0.8.28 Slice 60 — status
status: implementation in progress
target_release: 0.8.28
---

# Slice 60 status

Astra approved requirements, design and successor ADR on 2026-11-01 after
adding physical-span completeness, custom-trigger visibility protection and
independent embedder-api versioning. Implementation follows the reviewed
plan with behavior-specific RED/GREEN tests. Qualification is pending.

Implemented with separate RED/GREEN units: format admission and strict CS64
shape; FTS cursor identity; span schema; atomic span projection; independent
API 0.8.0 and BGE offsets; projection dispatch propagation; physical tuple
validation; trigger-safe cleanup and coalescing. Astra reviewed the deletion
refinement: edge supersession explicitly deletes spans when canonical
triggers are coalesced; the vec0-only deletion primitive preserves spans
through recentering.

Scoped checks passed: format/cutover/WAL/rebuild/quantization suites; span
schema properties; cached real-BGE empty/Unicode/trailing-whitespace and
510-token coverage; missing/mismatched/orphan span and trigger-admission
cases; custom-trigger visibility; both edge supersession paths; 257-row mean
pin/recenter span and f32 preservation, including batch dispatch. Independent
API release-version surfaces and benchmark support tests pass. The reviewed
bare-vec0 harness is committed under the experiment README. Full independent
verification, final code review, AC-013, eu7 and scale qualification remain
pending; no full-workspace-green claim is made here.

---
title: Slice 35 implementation review — FIX-2
status: RESOLVED_PENDING_REVIEW
reviewed_commit: e9fd6fd9
date: 2026-09-04
---

# Slice 35 implementation review — FIX-2

The second independent review returned `CHANGES_REQUIRED` with one P1 and
three P2 findings. FIX-2 closes the executable defects. The broader
verification-matrix finding remains closeout work and is not claimed by this
record.

## Disposition

- **Graph-frontier pre-truncation eligibility (P1): fixed.** The bounded BFS
  neighbor query joins each target canonical node and applies its frozen
  visibility and allowlisted eligibility predicates before `LIMIT 64`. A real
  database regression proves that an eligible sixty-fifth edge is no longer
  hidden behind sixty-four ineligible edges.
- **Authenticated-request validation (P2): fixed.** Once the token and frozen
  state are validated, the reader rejects NUL-bearing queries, non-finite
  alpha, and ranking depths outside the public `u32` range before compilation,
  embedding, or SQL execution. Python and TypeScript regressions prove invalid
  tokens still take precedence over these request errors.
- **Python response-schema validation (P2): fixed.** The Python response reader
  rejects unknown outer and echoed context schema versions with a typed
  `FrozenReadError` instead of relabeling them as v1.
- **Verification matrix (P2): pending closeout.** The implementation still
  needs the design's consolidated arm/plan matrix, mutation-race matrix,
  14-by-3 trigger proof, normative cross-SDK fixture disposition, and measured
  overhead evidence before Slice 35 can close.

Focused Rust and TypeScript regressions pass. A fresh, non-editable wheel built
from this worktree also passes the new Python validation and response-version
checks.

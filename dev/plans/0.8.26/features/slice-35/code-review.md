---
title: FathomDB 0.8.26 Slice 35 independent code review
status: PASS
reviewed_on: 2026-09-15
---

# Slice 35 independent code review

The first read-only review returned FAIL with seven findings:

1. Capacity refusal could mask earlier semantic failures and omitted the forced
   commit-failure hook on one terminal path.
2. Affected-revision corruption accepted syntactically valid nonexistent IDs.
3. Acceptance coverage lacked direct endpoint rollback, restart replay,
   provenance/collision, projection/commit failure, traversal, generation
   binding, edge-bearing corruption, and eight-unique-caller controls.
4. A fixed source-reference reserve weakened receipt-specific integrity.
5. PyO3 and N-API tested only schema precedence rather than every earlier
   top-level precedence family and the exact nested leaf path.
6. The public Python and TypeScript API references still listed four variants.
7. Performance evidence omitted sequential/concurrent response and cursor
   totals, computed lock-failure counts, seeded storage growth, per-unit bytes,
   and completion spread.

All findings were resolved. Capacity is enforced during rollback-only semantic
simulation in request order; affected IDs must resolve; source-reference limits
depend on the exact receipt; the missing tests and public docs were added; and
the deterministic harness/report now carries the complete accounting. The
design reviewer separately returned final PASS on the amended design.

The final read-only code rereview returned PASS with no material findings. It
ran the Slice 35 engine suite (13/13), both malformed-edge binding precedence
matrices, and `git diff --check`; all passed.

Independent verification then found two test-integration gaps. The Python
fixture adapter omitted `tValid`/`tInvalid`, and the changed Slice 25 fixture
violated a historical release-evidence digest seal. The adapter now maps the
two edge time fields. The sealed fixture is restored byte-identically, while a
new Slice 35 fixture and separate TypeScript test carry current edge
conformance without changing the retained Windows smoke inventory. Final
read-only rereview returned PASS; it verified both fixture digests and parsers,
the exclusive old/new TypeScript routing, the 5/5 Slice 73 structural tests,
and the Rust shared-fixture test.

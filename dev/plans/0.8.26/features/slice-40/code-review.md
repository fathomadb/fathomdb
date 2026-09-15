---
title: FathomDB 0.8.26 Slice 40 — independent code review
status: PASS
reviewed_on: 2026-09-15
---

# Slice 40 independent code review

The read-only reviewer inspected the approved requirements/design, RED commit
`943ac4f4`, and GREEN commit `cd7a4abe`. The first verdict was FAIL with four
findings:

1. Removing a newly created advisory-lock path after releasing its inode could
   split the lock namespace and admit concurrent writers.
2. The Slice 30 CLI mismatch matrix still treated the now-current schema 34 as
   incompatible.
3. Gating the complete provenance test target behind migration hooks hid two
   current-contract tests.
4. The custom migration helper was technically downstream-callable when its
   private feature was explicitly enabled, while the plan said only "no public
   migration."

The remediation keeps lock inodes persistent and performs every schema
classification under the held product lock. The plan/design now explicitly
allow an absent lock to become one empty persistent namespace file while all
database, WAL, SHM, journal, and existing-lock bytes remain exact. Tests cover
clean and WAL-bearing schema-33 refusal with lock and SHM both present and
absent. The CLI future fixture is schema 35; only the legacy provenance test is
feature-gated; and the integration-test migration seam is explicitly defined
as opt-in, non-forwarded, undocumented product surface.

The final rereview returned **PASS** with no residual actionable finding.

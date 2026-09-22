---
title: FathomDB 0.8.27 Slice 30 - design review
status: APPROVED
reviewed_commit: cf429bc4
rereviewed_commit: d6e5acd2
---

# Slice 30 design review

The independent review rejected the first design with one P1 and one P2.

- **P1:** the Rust matrix omitted the real combined `operator,test-hooks`
  engine surface, and the NAPI row did not name an exact generation command or
  features. The design now enumerates six Rust row IDs including the combined
  row, requires its own mutation/metadata coverage, and binds the production
  NAPI row to `npm run build:native` and `default-embedder`.
- **P2:** the entry SHA and eventual baseline provenance were ambiguous. The
  design now captures from the clean pre-baseline implementation commit,
  verifies clean status and `HEAD`, stores that source SHA in the manifest, and
  records the later baseline-tracking commit separately in status.

No scope was added beyond real surfaces already assigned to Slice 30.

Independent rereview approved `d6e5acd2` with no remaining material finding.

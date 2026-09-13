---
title: FathomDB 0.8.26 Slice 46 — technical design documentation design
status: DRAFT
---

# Slice 46 design — technical design documentation convergence

## Classification model

Every reviewed document receives one lifecycle class:

- **maintained topic design:** current subsystem implementation guidance;
- **reference:** still-valid bounded analysis or protocol input;
- **experiment:** evidence with an explicit result/disposition;
- **historical slice record:** immutable account of prior planned or executed
  work; or
- **superseded:** retained in place with a verified successor pointer.

The class, canonical owner, applicable release, and successor belong in the
design index. A document may be old without being disposable, and a new
document may still be non-authoritative.

## Reconciliation model

Start from the Slice 45 architecture map. For each maintained topic, compare
the design with accepted ADRs, public interfaces, requirements/acceptance, code
seams, and focused tests. Update design facts and navigation when authority and
implementation agree. If they disagree, record the conflict and return it to
the owning product slice; do not choose a winner inside documentation cleanup.

Prefer a small set of current topic owners with links to historical evidence.
Do not bulk rename or move `dev/design/`; stable paths and inbound links are
part of the documentation contract. Deletion is exceptional and requires
proof that the file has no authority, reference value, inbound link, or unique
rationale.

## Verification model

The disposition matrix is the review oracle. Automated checks cover index
membership, duplicate current ownership where mechanically identifiable,
paths, anchors, Markdown, and documentation build. Human review checks
semantic correctness, authority ordering, lifecycle classification, and loss
of rationale. The slice produces no generated “all current” assertion without
per-document evidence.

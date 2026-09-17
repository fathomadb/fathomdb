---
title: FathomDB 0.8.26 Slice 65 — semantic ownership gate design
status: DRAFT
target_release: 0.8.26
---

# Slice 65 design — enforceable authority topology

## Boundary of automation

The lifecycle checker can prove that every maintained owner declares a current
profile, points to existing authoritative inputs and implementation witnesses,
and does not route current semantics through a historical document. It cannot
prove that arbitrary prose accurately describes code. Semantic truth remains a
code-grounded review obligation, recorded through the authority-and-witness
metadata and Slice 60 matrix.

## Relationship classes

Maintained records distinguish:

- **semantic authority:** an accepted ADR/interface or maintained design that
  governs the current statement;
- **implementation witness:** code, test, or bounded enforcement path that
  demonstrates the current implementation seam; and
- **evidence only:** historical plans, experiments, release-local designs, or
  status records that explain provenance but do not own current semantics.

Semantic-authority edges must exist, be unambiguous, and terminate without
cycles at accepted external authority or a maintained owner. Evidence-only
edges may target historical records but cannot satisfy the authority or witness
requirement.

## Error-owner reconciliation

Stable error behavior is summarized in `errors.md` or the maintained subsystem
owner and linked to accepted/public contracts. Release-local slice designs stay
available as provenance. This removes the condition where a maintained current
owner requires a reader to treat a historical implementation plan as normative.

## Candidate model

Slice 50 remains valid historical evidence for the candidate it measured, but
it is no longer the final integration candidate once Slices 55–65 modify the
tree. Slice 65 creates the replacement exact candidate record. Expensive native
evidence is reused only when a diff-based input analysis proves it unaffected;
fresh SDK surface smokes and repository/security gates bind the new commit.

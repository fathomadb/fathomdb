---
title: FathomDB 0.8.26 Slice 45 — architecture documentation design
status: DRAFT
---

# Slice 45 design — architecture documentation convergence

## Documentation boundary

The inventory begins with `dev/architecture.md`, architecture-design entry
points, `dev/adr/README.md` and the decision index, and architectural claims in
maintained public concepts and interface navigation. The inventory—not filename
pattern alone—decides whether a document is architecture-bearing.

Accepted ADRs are decision authority. Current public interfaces,
requirements/acceptance criteria, and verified behavior are mutually checked
contract/evidence tiers; none silently wins a conflict by list position.
Maintained architecture synthesizes those authorities, while historical
plans/design records provide context only. Any disagreement among current
tiers is a blocking inconsistency routed to the owning slice or HITL.

## Organization model

- Keep one discoverable current architecture entry point.
- Link outward to focused subsystem designs and interfaces instead of copying
  their detailed contracts.
- Mark old release architecture as historical or superseded in place and link
  to the current entry point.
- Preserve stable paths where inbound links or unique rationale exist.
- Add a successor ADR rather than editing the substance of an accepted ADR.

## Correctness model

Create a review matrix mapping each maintained architectural claim to an ADR,
interface/requirement, and representative implementation/test witness. Resolve
documentation-only drift in this slice. Record product drift as a blocking
finding for its owning slice; do not make prose agree with incorrect code or
quietly change code to agree with prose.

Verification covers path/anchor resolution, current/historical labels,
duplicate-authority detection, Markdown and documentation builds, and focused
source checks for crate topology and named public boundaries. New automated
guards are justified only for stable, mechanically checkable invariants that
have already drifted.

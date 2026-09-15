---
title: FathomDB 0.8.26 Slice 45 — architecture documentation design
status: APPROVED
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

The reconciled inventory is
[`inventory.md`](inventory.md). Slice 45 changes only the architecture tier and
its immediate navigation/validation. Slice 46 owns maintained topic-design
classification and semantic updates; Slice 50 owns integrated release proof.

## Organization model

- Keep one discoverable current architecture entry point.
- Link outward to focused subsystem designs and interfaces instead of copying
  their detailed contracts.
- Mark old release architecture as historical or superseded in place and link
  to the current entry point.
- Preserve stable paths where inbound links or unique rationale exist.
- Add a successor ADR rather than editing the substance of an accepted ADR.

The concrete hierarchy after this slice is:

1. `dev/design/fathomdb-data-plane-architecture-v2.md` is the sole active
   system/data-plane architecture and carries the current 0.8.26 executable
   profile.
2. `dev/architecture.md` is the retained superseded 0.6.0 snapshot. Its stable
   path remains useful to inbound links, but its opening authority statements
   are reframed as historical and an exact banner names the active successor.
3. Accepted ADRs own decisions; `dev/adr/ADR-0.6.0-decision-index.md` locates
   them. The active architecture synthesizes but does not alter them.
4. `dev/interfaces/` owns maintained, section-status-qualified surface
   contracts. Architecture names boundaries and flows without copying
   field-by-field wire schemas.
5. `dev/design/` topic documents own implementation detail beneath the active
   architecture; their broader lifecycle reconciliation waits for Slice 46.
6. Release plans, slice records, notes, and tests are evidence/history, not
   competing architecture authorities.

## 0.8.26 profile delta

Architecture v2 remains the approved multi-release model. Version 2.2 is a
documentation profile, not a new decision. It adds the as-built changes since
v2.1:

- the current ten-member Rust workspace and existing Python/TypeScript binding
  roots;
- schema-34 fresh bootstrap plus locked refusal of every nonempty noncurrent
  database before product mutation;
- finalized frozen explanation identity;
- opt-in frozen graph evidence with separate exact target and terminal-edge
  references and intrinsic point resolution;
- immutable CLI-only data-plane integrity inspection; and
- one changed-in-place five-operation V1 actuation grammar whose derived edge
  shares the canonical writer transaction and complete prospective endpoint
  validation.

It preserves the v2 mechanism/policy boundary, one-writer/pooled-reader model,
same-file durable state, projection ownership, eligibility-before-truncation,
and deferred multi-source/lease/full-path work.

## Validation design

The recurrence guard reads front matter and exact navigation markers rather
than matching release prose heuristically. It must:

1. require `dev/architecture.md` to be `SUPERSEDED`, name an existing
   `superseded_by` path, and carry the exact supersession banner before its
   historical body;
2. require that successor to be the only `ACTIVE`
   `fathomdb-data-plane-architecture-v*.md` document;
3. require `dev/README.md` and `dev/design/README.md` to link the same active
   architecture; and
4. emit one explicit failure per violated authority invariant.

The checker does not parse arbitrary Markdown semantics, validate all designs,
infer whether prose is true, or select a release. Those would be brittle,
duplicate `scripts/release-current.py`, or enter Slice 46 scope. The active
architecture truthfully describes the most recently converged profile during
three valid lifecycle states: a newer release before its convergence slice,
that release after convergence, and no live release after publication. Slice
45 closeout invokes `release-current.py` separately and compares its 0.8.26
result with the profile; duplicate live states remain that canonical tool's
failure responsibility.

The focused test builds isolated document fixtures and proves the passing
hierarchy plus each failure mode. A source-contract assertion proves the same
checker is invoked by `agent-lint-md.sh` and by the Markdown-only CI job. The
guard therefore covers local/non-docs verification and docs-only changes
without duplicating its logic.

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

This guard is justified because both the top-level snapshot and the active
profile remained stale across multiple releases. Semantic source comparison
remains a human review obligation recorded in the inventory matrix.

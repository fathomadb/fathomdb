---
title: FathomDB Inspector — Astra-medium adversarial review
date: 2026-10-26
target_release: 0.8.30
desc: Review findings and dispositions for Inspector needs, requirements, acceptance criteria, and high-level design
status: ACTIVE
blast_radius: review record only; no implementation acceptance or release approval
---

# FathomDB Inspector — Astra-medium adversarial review

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Scope and method

An independent `gpt-6-astra` agent at medium reasoning reviewed the
[needs, requirements, and acceptance criteria](../needs-requirements-acceptance.md)
and [high-level design](../design/high-level-design.md). The agent was read-only and
checked current contracts and source as well as the proposal. This record
summarizes its findings and the author's remediation; it is not a new
requirements authority.

The owner clarified that the Inspector is an inspection tool. Documentation
and operation support users in building their mental model; teaching or
generating a conceptual explanation is not an Inspector responsibility.
The review explicitly applied this correction.

## Initial requirements review

The initial requirements at `53a7d2d91` had five material gaps:

| Severity | Finding | Remediation in the revised set |
| --- | --- | --- |
| High | Minimum conceptual distinctions were undefined. | Added a coverage table separating canonical record classes, kind/role, operational collections, source-derived records, projections, results, and lifecycle/read context. |
| High | Identity and revision rules lacked negative cases. | Added typed identity and session/class qualification, ambiguous content resolution, exact observed payload preservation, and equal-content/different-identity/revision fixtures. |
| Medium | Discovery traceability overstated coverage. | Added bounded field discovery and supported subset constraints; marked distribution analysis, exhaustive inference, and other broader needs partially covered or deferred. |
| Medium | Critical scope conflicted with the usability test. | Made storage/eligibility/readiness/result-presence distinction Critical; kept detailed outcome investigation High. |
| Medium | Evaluation could reward repetition of examples. | Defined the first participant audience, ordinary documentation access, an unseen transfer instance, misconception-based scoring, and advance rubric agreement. |

The reviewer later judged these findings substantially resolved at the
requirements level, subject to the operational-mode correction below.

## Combined requirements and design review

The agent reviewed commit `d4ed65687` and found two material issues. It confirmed
that the revised set complied with the owner's correction and that identity,
observation preservation, evaluation, and the existing-versus-new inventory
were substantially improved.

### High — incorrect operational history assumption

The proposal required current values and retained mutation history for the
same collection/key. Current writes instead branch between append-only-log
insertion and latest-state replacement. Latest-state writes do not append the
previous payload to the mutation log.

Evidence: [write commit implementation](../../../src/rust/crates/fathomdb-engine/src/write_commit.rs)
and [schema owner](../../../src/rust/crates/fathomdb-schema/src/lib.rs).

Remediation: distinguish the two registered collection modes throughout the
conceptual table, criteria, views, and fixtures. F-03 uses ordinary writes in
separate collections and verifies that overwritten latest-state values are
not presented as recoverable history. Equal keys across collections do not
establish a relationship.

### Medium — incomplete immutable-session prerequisite

Product-lock exclusion applies to cooperating writers. The immutable inspection
precedent also requires external SQLite writers stopped throughout inspection.
A one-time lock/sidecar check cannot prove that an external writer will not
change the database later.

Evidence: [operator data-plane inspection](../../../src/rust/crates/fathomdb-engine/src/operator/data_plane.rs).

Remediation: explicitly require all writers stopped for the supported quiescent
session. Distinguish enforced lock exclusion from this environmental support
condition. Define detectable-change invalidation and reattachment, and state
that change indicators do not prove absence of external modification.
F-05 includes missing-lock and file-identity-change cases.

### Additional clarifications

- Critical direct object links and revision distinction are separate from High dependency tracing and retained canonical revision-chain navigation.
- Admission means bounded header/schema validation, not an implicit full integrity audit.
- Query-runtime initialization, shared-engine conformance, preservation, and cancellation remain explicit new engineering work.

## Focused recheck and final disposition

The same Astra-medium agent rechecked the staged remediation over `d4ed65687`.
Its disposition was: both material findings resolved; no remaining material
issue found in the focused recheck; suitable to proceed to detailed design,
subject to proposed scope and implementation gaps.

It identified one minor traceability cleanup: canonical revision navigation
under INS-07.1 should cite F-01/F-02, not F-01/F-03. The author applied that
correction and removed unsupported INS-07 coverage from F-03 after the recheck.
No other semantic change followed that recheck.

The proposal is not implemented or accepted by this review. Quiescent local
attachment remains a proposed scope boundary; live inspection, supporting API
contracts, packaging, and feasibility of the read-only execution refactor
remain detailed-design work. Formative evaluation targets are proposals, not
approved release gates.

## Subsequent owner clarification — 2026-10-27

After the review above, the owner explicitly required interfaces appropriate
to FathomDB structures, including text/FTS/BM25, vectors, and graphs. A generic
SQLite table browser is insufficient. The author revised the proposal to add
Critical INS-09, a coordinated-view fixture, and structure-specific views,
including a required bounded interactive graph canvas. This supersedes the
earlier design's statement that a graph canvas was unnecessary. The review
above applies to its recorded revisions. Astra-medium subsequently reviewed
the staged additions over `ae917d1f6` and found no material issue. It confirmed
that a table-only implementation cannot pass INS-09, that F-07/T-03 exercise
coordinated navigation, and that vector distances, graph layouts, query
contributions, and stored origin relationships remain correctly distinguished.
The delta was judged suitable to proceed to detailed design.

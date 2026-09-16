---
title: FathomDB 0.8.26 Slice 46 — technical design documentation design
status: APPROVED
---

# Slice 46 design — technical design documentation convergence

## Scope decision

The repository contains 185 Markdown files under `dev/design/` at planning
baseline `636d96eb`. Treating each as a living subsystem specification would
promote obsolete plans and
experiments; rewriting all of them would destroy evidence. The design therefore
separates exact lifecycle coverage from semantic maintenance:

- one catalog covers every tracked design path exactly once;
- a short explicit set of maintained owners receives semantic review;
- existing maintained topic owners plus one new actuation owner reconcile the
  0.8.26 implementation detail beneath architecture v2.2; and
- historical, experimental, reference, and superseded bodies remain stable.

The catalog is navigation and lifecycle authority. Accepted ADRs remain
decision authority, `dev/interfaces/` remains public-contract authority, and
the active architecture remains system-shape authority. The catalog cannot
promote a document above those owners.

## Classification model

Every reviewed document receives one lifecycle class:

- **maintained topic design:** current subsystem implementation guidance;
- **reference:** still-valid bounded analysis or protocol input;
- **experiment:** evidence with an explicit result/disposition;
- **historical slice record:** immutable account of prior planned or executed
  work;
- **proposal:** unaccepted design offered for a future decision or campaign;
- **deferred:** accepted need or tracked design intentionally not active in the
  current implementation window; or
- **superseded:** retained in place with a verified successor pointer.

The class, canonical owner, applicable release, and successor belong in the
design lifecycle catalog. A document may be old without being disposable, and
a new document may still be non-authoritative. `dev/DOC-INDEX.md` stays the
thin repository cold-start map; it links to the lifecycle catalog instead of
duplicating 185 rows.

The catalog path is `dev/design/document-lifecycle.json`, encoded as one JSON
object with `schema_version: 1` and a `documents` array sorted by `path`. Every
document record has:

- `path`: exact repository-relative `dev/design/**/*.md` path;
- `class`: `maintained`, `reference`, `experiment`, `historical`, `proposal`,
  `deferred`, or `superseded`;
- `topic`: stable lower-kebab navigation key;
- `role`: lower-kebab role within the topic, such as `design`, `requirements`,
  `acceptance`, `evidence`, `policy`, `method`, `architecture`, or `index`;
- `owner`: existing repository path that owns the record's current
  interpretation; self is valid for a bounded reference/evidence/history;
- `release`: `current:<version>`, `historical:<version>`,
  `future:<version-or-window>`, or `cross-release`; and
- `successor`: an existing repository path or `null`.

For `superseded`, `successor` is mandatory and `owner` equals that immediate
successor. A successor cannot be self-referential. When the successor is another
cataloged design document, following successors must be acyclic and terminate
at a non-superseded catalog entry. An existing regular file outside the design
catalog (for example an accepted ADR, plan, or release-state file) is an
explicit terminal authority. Other classes may name a successor only when
verified. A maintained `(topic, role)` pair is unique; multi-record sets use
distinct roles, so a requirements/design/acceptance trio is valid.
Classification is based on substance and current authority links, never
target-release or status text alone.

## Reconciliation model

Start from the Slice 45 architecture map. For each maintained topic, compare
the design with accepted ADRs, public interfaces, requirements/acceptance, code
seams, and focused tests. The disposition matrix records both the governing
authority and at least one implementation/test witness. A policy or method
without a product-code seam instead records its exact bounded scope and
enforcement witness. Update design facts and navigation when authority and
implementation agree. If they disagree, record the conflict and return it to
the owning product slice; do not choose a winner inside documentation cleanup.

Prefer a small set of current topic owners with links to historical evidence.
No `dev/design/` path is deleted, moved, renamed, or archived in this slice;
stable paths, inbound links, and unique rationale are preserved.

## Verification model

The disposition matrix is the review oracle. Automated checks cover index
membership, duplicate current ownership where mechanically identifiable,
paths, anchors, Markdown, and documentation build. Human review checks
semantic correctness, authority ordering, lifecycle classification, and loss
of rationale. The slice produces no generated “all current” assertion without
per-document evidence.

## Current 0.8.26 topic owners

Existing topic owners remain authoritative where their substance is current,
even when their front matter names an older target. Slice 46 amends
`engine.md`, `migrations.md`, `retrieval.md`, `recovery.md`, `bindings.md`,
`errors.md`, `projections.md`, `scheduler.md`, and `release.md` only where the
semantic review finds a current gap. It creates `actuation.md` because no
maintained topic design currently owns the implemented actuation transaction
and receipt model.

`vector.md` is not accepted as a current owner while also calling itself a
stub. Slice 46 replaces that contradiction with a compact current design that
defines the schema-owned vec0/sidecar shape boundary, LE-f32 plus mean-centered
sign-bit representation, reader-snapshot query path, projection-generation
publication, erasure, and rebuild-from-canonical behavior. It links the schema,
engine, recovery, filter ADR, and focused vector/recovery tests rather than
repeating historical slice narratives.

`actuation.md` uniquely owns implementation-shape details below the accepted
ADR and interfaces:

- the sole actuation V1 grammar contains five operations and validates derived
  edge endpoints against the complete prospective batch;
- validation occurs before writes and all canonical/dependency/edge/receipt
  effects share one writer transaction;
- request identity and compact receipt integrity bind exact operation order and
  affected revision/source-reference summaries;
- replay returns only a receipt whose recomputed consequences match the stored
  request; and
- canonical lifecycle, erasure, and projection effects are reused rather than
  reimplemented in actuation.

Frozen/graph evidence remains in `retrieval.md`; immutable integrity inspection
in `recovery.md`; schema admission in `engine.md`/`migrations.md`; binding and
error conversion in `bindings.md`/`errors.md`; and publication mechanics in
`release.md`. Each 0.8.26 addition links to its ADR/interface, implementation
seam, and focused test/status witness. Multi-source dependency liveness,
persisted snapshot leases, graph continuation/full paths, generalized repair,
and publication remain explicit deferrals rather than current design.

## Mechanical contract

`scripts/check-design-lifecycle.py` parses the catalog as data and compares its
path set with tracked `dev/design/**/*.md` files. It rejects
duplicate/missing/extra paths, invalid or missing fields, unknown classes,
missing owner paths, missing declared successors, self/cyclic/nonterminating
supersession, a superseded entry without a successor, and duplicate maintained
`(topic, role)` ownership.
`scripts/tests/test_check_design_lifecycle.sh` uses isolated fixture
repositories so the RED oracle does not depend on the production catalog being
wrong.

The checker requires an active, non-comment `run_capped
check-design-lifecycle "$SCRIPT_DIR/check-design-lifecycle.py"` invocation in
`agent-lint-md.sh`. In CI it requires an active `run: python3
scripts/check-design-lifecycle.py` step inside the top-level `markdownlint` job,
whose job-level condition is `needs.changes.outputs.docs_only == 'true'`.
Commented occurrences and calls in other jobs do not satisfy the contract. This
is a small indentation-aware structural check of those owned call sites, not a
general shell or YAML interpreter. The checker does not lint document prose,
infer semantic truth, rewrite files, or enforce release-number heuristics.
Those are review responsibilities.

## Failure and preservation model

- Missing or ambiguous current authority fails the catalog gate; supersession
  must terminate without self-reference or cycles.
- A semantic contradiction stops the slice and returns to the owning product
  contract; documentation cleanup does not choose a new product behavior.
- No path is moved or deleted. Classification changes are ordinary reviewed
  edits, while historical bodies remain intact.
- Follow-up `636d96eb` is retained. Its A25-04 allocation correction removes a
  false 0.8.26 claim without changing an accepted architectural decision.

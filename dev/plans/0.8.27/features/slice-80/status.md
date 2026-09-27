---
title: FathomDB 0.8.27 Slice 80 - implementation status
status: COMPLETE
implemented_on: 2026-09-27
planning_commit: 9a31e979
implementation_candidate: 8e4499637e9d40ac6fcb9579f352b9f643e86709
reviewed_candidate: 3e60cc5dd37c8771d607285985337a7f35223aa1
closeout_commit: 4299511c7f725fab3fdad5add7a01e299076bd1b
---

# Slice 80 implementation status

## Completed scope

Slice 80 is complete on `release/0.8.27`. It moved the root-owned read side
into private semantic modules without changing behavior, schema 34, SQL,
statement ordering, snapshot or transaction scope, error mapping, feature
gates, public paths, or wire encodings:

- `fusion.rs` and `filter.rs` own ranking fusion and filtering;
- `search_types.rs`, `search.rs`, and `search_api.rs` own search carriers,
  execution, graph-arm integration, and the `Engine` search facades;
- `telemetry.rs` owns search observability and feedback;
- `read.rs` owns read verbs, canonical pages, and operational-state reads;
- `reader_pool.rs` owns the pool implementation and worker loop; and
- `graph_expand/` is split into types, codec, execution, and traversal.

Public and doc-hidden items keep their root paths through re-exports. Reader
data carriers remain at the root without field widening. The four search-owned
`ProjectionRuntimeShared` fields keep their shape and owner.

## TDD and implementation

Characterization commit `77439ba1` added three gap-filling tests on unchanged
production. Each passed on the baseline, failed against its specified
temporary mutant, and passed again after exact restoration:

- reader transactions release after an in-transaction refusal;
- the search graph arm applies the validity view; and
- projected-text search applies the validity view.

Fifteen mechanical extraction batches then moved the implementation. The final
structural batch was `f333926e`. Code-review fix `8e449963` returned the shared
reader-transaction primitive to root-private ownership and narrowed unnecessary
visibility; it changed no behavior.

Follow-up test commit `9700991f` closes the graph-result codec property gap.
Generated coherent results now prove typed decode→encode→decode equality across
linked fields, canonical integer strings, finite/null score, and optional valid
evidence. A second property proves generated nonzero first-entry evidence
positions are refused at the exact path. Both killed their specified temporary
production mutants and passed after exact restoration; production is unchanged
and no generated golden oracle was added.
Lint-only follow-up `31e78529` packages fixture inputs in one private test
carrier; test behavior and production remain unchanged.

## Review and verification

- **Design review:** PASS after three cycles plus an external code-grounded
  review (`design-review.md`).
- **Code review:** the independent `gpt-5.6-sol` high-reasoning review first
  returned FAIL with two P2 architectural findings. Both are closed in
  `8e449963` (`code-review.md`).
- **Independent verification:** the separate read-only `gpt-5.6-terra`
  verifier returned PASS at clean candidate `3e60cc5d`
  (`review-verification.md`).
  - canonical verification: 127/127;
  - workspace Clippy/check: PASS;
  - candidate-bound Python receipt: PASS;
  - public surface: 13 rows, exact;
  - hidden surface: 33 rows, additive only, with 261 additions, 0 changes,
    and 0 removals;
  - strict security: claimed 0 violations, 0 blockers, and 0 downgrades,
    including live AC-037. That claim at `3e60cc5d` is **UNEVIDENCED**: the pass
    lines and the grant/revert record were not captured. A HITL-granted re-run
    through `dev/release/ac-037-live-netns-hitl-runbook.md` then passed 0/0/0
    with both live layers at `66e27983`, and the grant was reverted
    (2026-09-27; `review-verification.md`). That run is historical only: it
    does not qualify later HEAD or the final candidate. By owner ruling, the
    next live AC-037 run is deferred until after Slice 130 and Slice 150 must
    bind the runbook receipt to the exact final candidate; and
  - feature-complete: 21 runs, 357 planned, 349 passed, 0 failed, and 8
    documented ignores.

## Slice 85 and Slice 90 handoff

Planning-only Slice 85 owns the carrier and dependency boundary work recorded
in the master plan and `design.md`:

- root reader data carriers and their private fields;
- the shared root-private `begin_attributed_reader_tx` primitive;
- the root `TelemetrySink` and `EvidenceCapture` carriers; and
- removal of the three facade-induced cycles (`read` ↔ `reader_pool`,
  `graph_expand` ↔ `reader_pool`, and `graph_expand` ↔ `search_api`) plus an
  enforceable exact or allowlisted remaining dependency graph.

Slice 90 consumes those settled boundaries, then owns the runtime facade,
reader-loop WAL/diagnostic arms, reader open-path helpers, remaining
search-index projectors, and final placement of the four search-owned
`ProjectionRuntimeShared` fields.

These are handoffs. Slice 85 is planned but uncommissioned; no Slice 85 feature
directory or implementation was created.

The frontmatter and release-state `sha`, `reviewed_candidate`, and
`closeout_sha` values remain the prior historical bindings. They must not be
read as binding follow-up commit `9700991f` or later HEAD; independent rereview
must complete before those fields are rebound.

## Cleanup

No new branch or worktree was created. The verifier removed its disposable
Python 3.12 environment and generated artifacts, restored the tracked
`src/python/fathomdb/_fathomdb.pyi`, and left the tree clean. It reported
using an already-active AC-037 capability without changing AppArmor state;
that claim is unevidenced (see above). Tags,
registries, publication, and later slices were not touched.

## Post-hoc design review fix-1 (2026-09-27)

A post-hoc adversarial design review returned PASS-WITH-FIXES:

- one P1: four struct fields had widened;
- three P2s: the dependency invariant was false at module level, the Slice
  140 handoff was missing, and the live AC-037 layer was unevidenced;
- seven P3s.

Fix-1 made the following changes, with no behavior change:

- restored the private carrier fields and narrowed visibility (`b8af4d86`);
- corrected the design, review, and plan records;
- marked live AC-037 UNEVIDENCED.

The per-finding record is in `design-review.md`.

---
title: FathomDB 0.8.27 Slice 40 - engine foundation design
status: APPROVED
target_release: 0.8.27
---

# Slice 40 design

Independent read-only design review passed after the master/test-plan hook
contract, clean-candidate comparator protocol, focused hook matrix, full-SHA
capture, and Slice-72 behavioral test were corrected. The review record is
`design-review.md`.

## Constraints

The engine stays one crate and `pub struct Engine` stays in `lib.rs`. New
modules are private implementation detail. Root `pub use` statements preserve
every public type and function path. Parent/child access uses the narrowest
`pub(super)` or `pub(crate)` visibility needed; no `Engine` field becomes more
visible outside the crate. Existing feature attributes move with their items
verbatim.

No public signature, discriminant, error string/code, serde/wire spelling,
SQL text, clock behavior, hook lifetime, synchronization ordering, schema,
transaction, lock, or package surface may change.

## Module ownership and batches

### Batch 1: `errors.rs`

Own `CorruptionDetail`, `CorruptionKind`, `CorruptionLocator`, `OpenStage`,
`RecoveryHint`, `EngineOpenError`, `EngineError`, their `From` conversions,
`Display`, `Error`, and stable-code mapping. Root re-exports preserve every
path; the stable-code helper remains crate-internal for lifecycle telemetry.
Imported domain errors remain owned by their existing modules.
`RuntimeConfigurationError` and `EmbedderChoice` stay in `lib.rs` for Slice 90:
they are open/runtime configuration, while the shared error carriers and
taxonomy belong in the foundation.

### Batch 2: `identity.rs`

Own `IdSpaceKind`, `IdSpace`, `SourceId`, `SourceVersionId`,
`SourceRevisionId`, `ArtifactRevisionId`, `DependencyId`, `CanonicalHash`, the
closed caller-ID validation/macro, and `derive_logical_id` /
`derive_stable_id`. These are shared identity primitives used across reads,
writes, lifecycle, provenance, evidence, bindings, and erasure. Dependency and
provenance request DTOs and operations remain for Slice 50.

The public types are re-exported at crate root. Derivation helpers are
`pub(super)` to the parent only. Existing sibling-module tuple construction is
preserved with crate-private fields or internal constructors, never public
fields. Constructor rejection order, reserved-source grammar, prefix parsing,
hashing input, and formatted representation are unchanged.

### Batch 3: `temporal.rs`

Own `ReadView`, `BoundaryCrossing`, private `FrozenView`, the process clock-read
meter, `current_epoch_seconds`, node/edge validity SQL generators, strict
ISO-8601/epoch conversion and shape validation, renderable epoch bounds, and
extractor timestamp normalization. Together these are the root-owned temporal
primitives shared by reads, graph validity, ingestion, and consolidation;
Slice 60 moves their call sites but does not duplicate temporal authority.

Root code imports the private frozen view and helper functions. Internal fields
and methods receive only parent visibility needed by existing query helpers.
The half-open node window, strict-future edge invalidation, eligibility
conjunction, single resolved instant, saturating pre-epoch behavior, and bound
parameter indices are copied without logic changes.

### Batch 4: `test_hooks.rs`

Own the debug-only projection transaction pause error/handle, the existing
Slice-72 forward rendezvous, and the root reader-search, frozen-validation,
evidence-linearization, erasure-lock, and explanation-finalization one-shot
hooks. The module groups mechanisms; semantic call sites stay in their owning
domains. Root re-exports retain the exact existing names and cfg gates.
Root-only fire/arm modules receive parent visibility, not public module paths.

This batch deliberately preserves two current categories:

- hooks already gated by `test-hooks` or `slice72-test-hooks` remain gated; and
- always-compiled, doc-hidden engine seams remain always compiled and outside
  the governed facade.

Changing that split would be a public/feature-contract decision, not a
mechanical extraction, and is rejected from this slice.

## Dependency direction

Foundation modules may import crate-root types with `use super::*` during this
mechanical slice. Domain modules and root code consume the root names or narrow
crate-private imports. This avoids duplicating dependency lists while the
monolith is still being decomposed. Slice 140 may tighten navigation/import
style after all domains have stable homes; Slice 40 does not manufacture an
intermediate dependency framework.

## Verification design

The immutable Slice 30 baseline is the public-surface oracle. Compare-only
receipts must have empty metadata and row diffs; a recapture is forbidden.
Focused behavior owners are:

- errors: lifecycle observability/error mapping plus compile/type checks;
- identity: `tc8_idspace_swap`, registration-identity inertness, and source-id
  validation/erasure suites;
- temporal: read-view, TC-33 timestamp/validity, graph temporal, and frozen
  read suites; and
- hooks: default targets `slice50_evidence`, `slice35_frozen_read_races`,
  `slice35_after_validation_races`, `slice45_pagination`,
  `slice15e_prekn_filterable`, and `slice55_explanation_hook_surface`;
  `test-hooks` targets `slice20_graph_evidence`, `slice55_explanation`, and
  `slice60_fix2_hooks`; the focused Slice-72 library rendezvous contract test
  under `slice72-test-hooks`; plus compile checks for `slice72-test-hooks` and
  `operator,test-hooks`.

The default, operator, test-hooks, and operator+test-hooks Rust rows are
compared independently. Conflicting accelerator feature combinations are not
collapsed into `--all-features`. The final broad gate is evidence against
cross-domain fallout, not a substitute for the per-batch checks.

Each batch ends at a clean commit because the comparator refuses dirty or
non-HEAD candidates. A disposable candidate is captured from that exact HEAD
and compared to the immutable tracked Slice 30 baseline. Empty metadata and
row diffs are required; the baseline is never regenerated by Slice 40.

---
title: FathomDB 0.8.27 Slice 50 - engine erasure and dependency design
status: APPROVED
target_release: 0.8.27
---

# Slice 50 design

Independent design review passed after correcting operation-specific ordering,
the Slice 20 nonterminal-closure carryover, final workspace gates, and all
existing dependency-kernel consumer edges. The durable record is
`design-review.md`.

## Boundary

Slice 50 first closes one already-allocated erasure defect, then changes Rust
source ownership without any further product behavior change. `Engine` remains
in `lib.rs`; all new modules are private; root `pub use` preserves existing
paths. The existing public `lifecycle` module remains the observability API and
is not repurposed for record state.

The move preserves all public signatures, derives, enum order, stable strings,
serde/wire spellings, SQL, transaction scopes, lock acquisition, atomics,
feature attributes, test-hook gates, and error precedence. Schema remains 34.

## Destination modules

### `record_lifecycle.rs`

Own record existence-state vocabulary and transition orchestration:
`LifecycleState`, `InitialState`, their conversions, lifecycle-target
resolution, and `Engine::transition`. It may call existing closure and
projection helpers through crate-private seams. It must not own or change
`lifecycle::{Event, Phase, Subscriber, ...}`, runtime lifecycle telemetry, or
actuation.

The transition transaction, source-revision closure admission, projection
effects, and write-cursor ordering move as one block. No SQL or call order is
rewritten during extraction.

### `provenance.rs`

Own the public provenance data contract and pure constructors/validation:
`ProvenanceCompleteness`, `SourceLocator`, the internal `ProvenanceRole`,
`WriteProvenanceV1`, `ProvenancedNodeV1`, `ProvenancedEdgeV1`,
`ProvenanceErrorReason`, and `ProvenanceError`.

`PreparedWrite` remains at root because Slice 60 owns write/ingest execution.
Where dependency validation must inspect provenance role, expose only the
minimum `pub(crate)` accessor or type visibility; do not make fields public or
add a second parser. Persistence logic embedded in write validation remains
with the write path unless it is already a self-contained provenance helper
whose move does not alter the transaction.

### `dependency.rs`

Own dependency registration rather than dependency closure or tracing:

- `SourceDependencyRegistrationV1`, source/derived lookups,
  `SourceDependencyV1`, `DependencyListV1`, and typed errors;
- prospective state and validated-registration carriers;
- generation parse/load/reserve/store;
- persisted canonical-source and dependency-chain validation; and
- `Engine::register_source_dependency`, `dependencies_for_source`, and
  `dependency_for_derived`.

`dependency_closure.rs` remains the durable closure state machine and
`dependency_trace.rs` remains the bounded read/codec owner. Registration and
closure are bounded peers: closure consumes the dependency generation and
persisted-chain validator, while registration consumes closure maintenance and
fencing. They collaborate through named crate-private seams exactly as current
behavior requires. The one-source model and 100-row lookup bound are unchanged.

Four existing call-only consumers follow helpers to the new owner without
moving their domain logic: `dependency_trace.rs` reads generation and validates
chains; `frozen_read.rs` captures generation in the frozen context;
`evidence.rs` validates authoritative chains; and `actuation.rs` consumes the
prospective registration and validation carriers. These edges are import/
visibility changes only.

### `erasure.rs`

Own the existing hard-erasure facade and its directly coupled implementation:

- `ExciseReport` and operator-gated `ExciseRecordReport`;
- `Engine::purge`, `erase_source`, `excise_source`, and operator-gated record
  excision;
- physical-closure finish/freeze coordination used only by hard erasure;
- at-rest WAL completion, durable pending-redaction discharge, telemetry
  redaction, and their erasure-owned helpers; and
- dependency/artifact/receipt cleanup helpers whose only authority is hard
  erasure.

The module calls existing closure, actuation, and projection helpers. Shared
projection registry/deletion machinery stays where it is for Slice 70. Shared
runtime/WAL machinery stays at root for Slice 90. If a helper has non-erasure
callers, it remains with its current owner and receives only a narrow
`pub(crate)` seam.

## Dependency direction and visibility

The root declares the four private modules and re-exports only the exact public
items that were previously rooted. Child `impl Engine` blocks can access root
state. Sibling modules communicate through named `pub(crate)` functions or
types only when current behavior requires it. No `Engine` field becomes public
or crate-visible solely for convenience.

Allowed collaboration is:

```text
lib/root state
  -> record_lifecycle -> dependency_closure / existing projection helpers
  -> provenance       -> identity
  -> dependency <-> dependency_closure
                -> provenance / identity
  -> dependency_trace / frozen_read / evidence / actuation
                -> dependency (call-only existing seams)
  -> erasure    -> dependency / dependency_closure / actuation /
                         existing projection and runtime helpers
```

The dependency/closure peer edge is intentional and limited to existing
generation, chain-validation, maintenance, and fencing seams. Any other cycle
is resolved by keeping a genuinely shared helper at root, not by duplicating
logic or widening a public module.

## Atomicity and ordering invariants

The current operations intentionally have different sequences. Extraction
preserves each sequence and its error precedence rather than normalizing them.

### Record transition

`transition` ensures the engine is open, resolves the lifecycle target, drains
outstanding work, acquires the existing writer/connection authority, maintains
closures, and performs its immediate transaction. It does not freeze the
projection scheduler or run physical at-rest completion. Cursor and closure
effects retain their current commit ordering.

### Dependency registration and lookup

`register_source_dependency` ensures the engine is open, validates the request
and current/prospective provenance chain under its existing connection and
transaction authority, reserves/stores the independent dependency generation,
and commits. It neither drains nor freezes projection work and does not advance
the canonical write cursor. Reciprocal reads remain bounded read operations.

### Normal hard erasure

`erase_source` preserves `SourceId` validation before `ensure_open`; changing
that order would change error precedence. The public erasure path then drains,
freezes projection work, and enters the existing shared mutation path. The
immediate transaction starts before `guard_no_pending_physical` fences a new
mutation. Inside it, the implementation computes affected rows, proof scope,
receipt references, revisions, and projection counts before deletion;
validates/redacts receipts before removing completed correction closures;
deletes identity, projection, and canonical rows; appends the exact audit and
durable redaction obligation; measures closure proof; and commits. The
canonical cursor advances only after a committed audit row.

After commit, physical closure validation and required WAL/telemetry at-rest
work occur in their current order; the closure becomes complete only after
that evidence succeeds. A discharge failure returns `ErasureIncomplete`
rather than a false success.

### Pending physical retry

An already-admitted physical closure follows its separate retry path: it uses
the existing freeze/wait coordination without substituting the ordinary drain,
revalidates the durable proof and pending obligation, completes required at-
rest work, and only then records completion. Already-complete closure and
retry remain idempotent.

`purge` retains its corrected handling for bare supersession and all erased
revision closures. Record excision retains its own validation, immediate
transaction, audit-cursor, and at-rest sequence. No helper extraction may move
a fallible step across a transaction or completion boundary.

## Nonterminal soft-closure carryover

`TC-6acb0013-bba8-4fee-ac18-27c64442908a` is accepted from Slice 20. A
nonphysical `superseded` or `soft_deleted` closure may be durably `proving` or
`incomplete` when its source revision is physically erased. Keeping that row
retains forbidden source-revision identity after a successful hard erasure.

The correction occurs before extraction and uses the existing erasure
transactions. For every erased source revision, after actuation receipts have
been validated and redacted, delete nonphysical closure rows whose cause is
`superseded` or `soft_deleted` without conditioning on phase. Preserve rows
whose cause is `purged` or `source_erased`, including the current operation's
physical proof. The cleanup remains inside the same immediate transaction as
identity and canonical deletion, so any earlier receipt, closure, or storage
failure rolls back the whole mutation. Public reports, audit rows, cursor
advancement, at-rest completion, retry behavior, and error precedence do not
change.

A bounded property/state-machine regression spans both `erase_source` and
`purge`, same/cross-bucket supported dependency shapes, and `proving`/
`incomplete` residue. The pre-fix implementation must fail because the soft
row survives. Exact assertions prove that all erased-revision soft rows are
gone, unrelated rows survive, physical proof rows survive, and refusal leaves
the complete pre-call snapshot unchanged.

## TDD and verification design

No layout assertion is added. The before/after oracles are existing public and
real-database tests. Each bounded move intentionally reaches RED through
unresolved ownership/import/privacy or a temporarily demonstrated behavioral
mutant, then returns GREEN without changing test assertions.

Deep owners:

- lifecycle: `opp12_existence_axis`, `opp12_lifecycle_verbs`,
  `slice20_dependency_lifecycle`, and lifecycle reliability/observability;
- provenance/dependency: `slice15_identity_provenance`,
  `provenance_mandatory`, `slice20_source_dependencies` (including property
  cases), `slice25_registration_identity_inert`, and
  `slice30_dependency_closure` (including its supported-shape state machine),
  plus the affected `slice55_dependency_trace`, frozen-read context,
  `slice50_evidence`, and Slice 25 actuation owners;
- erasure: the locked `correction_safe_erasure` matrix,
  `erasure_completeness`, `erasure_drain_ordering`,
  `erasure_projection_registry`, `excise_source`, and applicable operator
  record-excision/at-rest tests; and
- boundary evidence: facade re-exports/governed surface, immutable Slice 30
  comparator, hidden-surface comparator and release probe, and fast-tier test-
  target coverage.

Default, operator, test-hooks, slice72-test-hooks, migration-test-hooks, and
benchmark surfaces are checked only where affected. Conflicting accelerator
features are not collapsed into `--all-features`. Slice 50 does not require GPU
execution because it moves no accelerator path.

The final candidate runs focused owners first, then `agent-verify`, full-
workspace Clippy with warnings denied, and
`cargo check --workspace --all-targets`. Surface capture and comparison operate
only on clean commits. The public comparator must report empty metadata and row
diffs. Hidden structural rows and the release probe must compare equal; hidden
test-inventory rows may contain only exact reviewed additive tests, with no
removals or changes. The tracked baselines are never regenerated. The broader
registered/candidate gate is added only if the actual diff or a review finding
touches shared gate/tooling beyond the Rust domain work.

## Failure handling

- A public or hidden-surface difference is a product/gate failure, not an
  expected consequence of moving code.
- A semantic failure stops the structural batch; add a separate RED test
  before any fix and obtain code re-review.
- A gate defect may be corrected only with its own failing fixture and only if
  Slice 50 exposed it; do not weaken or repin the oracle.
- Unavailable platform or tool evidence is recorded as unavailable, never as a
  pass.

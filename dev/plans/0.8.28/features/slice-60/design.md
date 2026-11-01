---
title: FathomDB 0.8.28 Slice 60 — design
status: draft; awaiting Astra design review
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 60 design

## Format and vector identity

Bump `fathomdb-schema::SCHEMA_VERSION` to 35 and add a content-free fresh
schema step for the span table. Existing Engine admission remains the
version authority. Fresh databases replay the schema registry; non-current
populated databases never reach it. Change the fresh `_fathomdb_vector_rows`
definition to remove `UNIQUE(write_cursor)`, retaining `rowid INTEGER
PRIMARY KEY` and adding a nonunique record-cursor index. Production head
vector IDs remain equal to write cursors. Overflow allocation/grouping is
not enabled, but the physical schema can represent multiple vectors.

Create `_fathomdb_vector_spans` with `vector_rowid INTEGER PRIMARY KEY`,
`write_cursor INTEGER NOT NULL`, `window_number INTEGER NOT NULL`,
`byte_start INTEGER NOT NULL`, nullable `byte_end INTEGER`, and
`body_length INTEGER NOT NULL`. Require nonnegative window/start/length,
`byte_start <= body_length`, and when known,
`byte_start <= byte_end <= body_length`. Add `UNIQUE(write_cursor,
window_number)`. The head has window 0, start 0; NULL end means unknown.
Link vector identity to `_fathomdb_vector_rows.rowid`; transactional deletion
must cascade or explicitly remove the span even on connections where FK
checks are disabled. Prefer an `AFTER DELETE` trigger on vector mapping for
that invariant. Span deletion and insertion occur in the vector transaction.
Do not add a unique record constraint elsewhere.

The shared vec0 CREATE builder appends `chunk_size=64`, including attribute
reshapes and rebuilds. `ensure_vector_partition` accepts only the current
shape (dimension, required columns, chunk size); remove obsolete old-layout
migration branches instead of silently rewriting current-version stores.
Attribute reconciliation remains the existing same-format operator.
Recentring only rewrites binary codes and preserves original f32 bytes,
row IDs and spans.

## Text projection

`index_projector::project_canonical_node_row` binds `rowid` explicitly to
`write_cursor` in the shared `search_index` INSERT. Audit every other insert,
including rebuild and tests. `search_index_v2` and edge FTS keep their
existing identity contracts: this slice does not switch query plans or
change rank/order rules.

## Truncation reporting and projection

Add a documented default method to the public `Embedder` trait:
`truncation_offset(&self, input: &str) -> Result<Option<usize>, EmbedderError>`.
`None` means unknown; `Some(input.len())` means the full input is represented;
otherwise the value is the exclusive UTF-8 byte end of the head window.
The method must describe exactly the tokenizer/configuration used by embed,
including special-token overhead, and must not mutate provider state.
Custom implementations default to unknown. The pinned Candle BGE provider
uses its existing configured tokenizer offsets: when overflow exists use
the maximum end of retained non-special tokens, otherwise input byte length.
Do not infer a boundary from character counts or tokenize in the engine.
Delegating provider wrappers must forward the method. Candle BGE is the
production provider and reports exact coverage. Noop, experimental Nomic and
standalone ORT intentionally retain the default unknown in this slice;
unknown is truthful and never interpreted as full coverage.

Capture offset metadata in the existing fixed provider dispatch worker,
inside the same request deadline and panic boundary as embedding. Add
projection-specific single/batch requests and results so foreground query
embedding does not pay for unused metadata. Validate each known end against
input byte length and UTF-8 boundaries; invalid metadata is an invalid
provider result, following existing projection retry/failure semantics.
Do not call provider methods on the writer thread or outside the dispatcher.
The projection outcome carries head extent and body byte length. The writer
rechecks the existing generation/eligibility conditions before committing
mapping, vector, span and terminal in one transaction. Batch and scalar
paths obey the same contract; failures/deferred work leave no span.

All deletion paths already remove vector mappings; the mapping-delete
trigger removes associated spans. Rebuild resets them and projection
repopulates them. Attribute reshapes preserve mapping and spans. Test-only
vector insertion also records unknown/full coverage from its supplied text
so physical invariants remain usable by benchmark and regression fixtures.
Update `dev/interfaces/rust.md` for the public trait addition. The Axis E policy calls every trait change breaking: bump the independent
`fathomdb-embedder-api` version from 0.7.0 to 0.8.0 and update the workspace
dependency constraint, lockfile and independent-version contract references.
No binding surface changes are required because bindings do not implement
Embedder.

## Physical completeness and visibility

The span becomes the fourth member of the projection tuple (terminal,
vector mapping, vec0 row, span). Integrate it into
`projection_physical_tuple_is_empty`, scalar `member_completion`, bulk
completeness SQL, `empty_physical_fast_path`, restore/idempotence checks in
`index_projector`, current-store admission and integrity/readiness checks.
A completed head requires exactly one matching vector ID, record cursor,
window zero/start zero and body byte length, with a valid known end or NULL.
A failed/skipped/non-member tuple cannot hide orphan spans. Partial,
mismatched, duplicate or orphan tuples are corruption, not completed work
or an invitation to silently re-embed. UTF-8 validation uses the canonical
body when validating a known endpoint. Preserve bounded/indexed bulk scans;
no per-member query loop is added to normal readiness.

Add span INSERT/UPDATE/DELETE read-visibility triggers using the existing
nonce mechanism. Include `_fathomdb_vector_spans` in BOTH durable and TEMP
trigger queries in `projection_batch_has_no_custom_triggers`. The owned
mapping-delete cleanup trigger may be explicitly allowlisted only because
it cannot fire during projection's insert-only coalescing section; arbitrary
span triggers must prevent coalescing. A regression installs a span INSERT
trigger that changes canonical state and proves its visibility invalidates
frozen reads. Do not disable such custom triggers along with the built-in
visibility triggers.

Tests cover missing/mismatched/orphan spans through scalar, bulk and empty
fast paths, restore, reopen, readiness and integrity, in addition to healthy
round trips. Existing fixtures that manually fabricate complete physical
tuples must now include valid spans; this is fixture reconciliation, not
relaxation of corruption assertions.

## Verification and blast radius

Use real SQLite stores for shape/refusal, rowid gaps, rebuild/reopen,
recentring, erasure and transactional spans. Property tests generate valid
UTF-8 bodies and ranges and verify round trips and rejected boundaries.
Use small deterministic tokenizer fixtures for offset semantics and actual
BGE eu7 for numerical behavior. Exercise dispatch timeout, late result,
provider failure, batch cardinality and generation-change regressions.
Run affected schema, vector-shape, mean/recenter, projection, provider,
readiness, erasure and rebuild suites; the final gate checks shared crate
and binding compilation. A separate verification agent executes these.

The benchmark is a narrow real sqlite-vec harness, not a second engine.
Reuse the experiment's deterministic vector generator and two-stage query.
Document parameters in its README, test percentile/row-count correctness,
keep output outside git, and summarize results in status. Astra reviews the
design before code and the final implementation before slice closure.

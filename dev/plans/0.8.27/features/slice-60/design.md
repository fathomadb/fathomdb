---
title: FathomDB 0.8.27 Slice 60 - engine write, ingest, and consolidation design
status: APPROVED
target_release: 0.8.27
---

# Slice 60 design

Independent design review passed after three correction cycles. The durable
record is `design-review.md`.

## Boundary

Slice 60 changes only Rust source ownership inside `fathomdb-engine` and adds
characterization tests. `Engine` stays in `lib.rs`. All new modules are
private, and root `pub use` keeps every public path. The following are copied
verbatim:

- public signatures, derives, variant order, and doc comments;
- SQL text and statement order;
- `BEGIN IMMEDIATE` and the trigger-guard path choice;
- the `connection` mutex acquisition;
- atomics and their orderings;
- `#[cfg]` and test-hook gates; and
- error precedence and mapping.

Schema stays 34.

## Destination modules

Child modules see the root's private items. Moved `impl Engine` blocks
therefore keep private-field access without any field widening. A moved free
function, `impl Engine` method, type, or `ProviderSession` field that a
sibling or the root still uses gets `pub(crate)` (or `pub(super)`), and no
more. Every other moved item stays private. No item becomes `pub` unless it
already was.

### Seam table (grep-derived at `517e0545`, non-comment uses)

| Moved item | New owner | Consumers outside its owner | Visibility |
| --- | --- | --- | --- |
| `WriteReceipt`, `PreparedWrite` | `write` | root test hooks (`lib.rs:10871-11564`), root tests, and most sibling modules | already `pub`; root `pub use` |
| `storage_write_shape` | `write` | root `validate_nested_projection_sources_for_write` (`lib.rs:22852`) | `pub(crate)` |
| `vector_kind_needs_enrolment`, `enrol_and_unstrand` | `write` | root `write_canonical_row_with_kind_for_test` (`lib.rs:11305`, `lib.rs:11307`) | `pub(crate)` method |
| `batch_vector_kinds_needing_enrolment` | `write` | `actuation.rs:2116` | `pub(crate)` method |
| `validate_write`, `WritePlan` | `write_validation` | `actuation.rs:1519`, `1560`, `2154`, `2226`; `write_commit` (`lib.rs:25119`, `25240`) | `pub(crate)` |
| `apply_batch_in_transaction`, `CommitBatchError` | `write_commit` | `actuation.rs:1523-2276` | `pub(crate)` |
| `canonical_body_hash`, `CANONICAL_BODY_HASH_CALLS` (`#[cfg(test)]`) | `write_commit` | `evidence.rs:798`, `1204`, `1766`, `2685`, `2762`, `2775`; `dependency.rs:277`, `720` | `pub(crate) static` inside `thread_local!`; the `evidence.rs` test path follows it to `crate::write_commit::…` |
| `checked_locator_columns` | `write_commit` | `dependency.rs:280` | `pub(crate)` |
| `legacy_revision_id` (`#[allow(dead_code)]`) | `write_commit` | root `mod tests` (`lib.rs:26117`, `26292-26303`) | `pub(crate)`; the root `mod tests` imports it from `crate::write_commit`, with no ungated root `use` |
| `TriggerStateGuard` (type plus `disable`/`restore`), `advance_read_visibility` | `write_commit` | root projection-worker commit (`lib.rs:19427`, ~`19673`, `19671`; Slice 70) | `pub(crate)` |
| `validate_batch`, `collect_projection_jobs` | `write_validation` | `write` (`lib.rs:7957`, `7963`) | `pub(crate)` |
| `prior_node_cursors_by_logical_id`, `prior_edge_cursors_by_logical_id`, `prior_edge_cursors_by_triple` | `write_validation` | `write_commit` (`lib.rs:25288`, `25393`, `25429`) | `pub(crate)` |
| `commit_batch`, `CommitBatchError` variants | `write_commit` | `write` (`lib.rs:7985-8002`) | `pub(crate)` |
| `storage_write_shape` (additional) | `write` | `write_validation` (`22054`), `write_commit` (`24708`, `25258`, `25559`, `25570`) | `pub(crate)` |
| `Engine::provider_session`, `ProviderSession` and `ProviderSession::request` | `provider` | `ingest` (`8052`, `8172`), `consolidation` (`8531`, `8616`) | `pub(crate)` |
| `ProviderSession` fields `model`, `max_docs_per_request`; `ProviderTask` | `provider` | `ingest`, `consolidation` | `pub(crate)` |

The compiler is the final authority, and any seam not in this table is
recorded in `tdd-chronology.md`. `enforce_provenance_retention` moves into
`write_commit.rs` as a private helper of its only caller.

### `write.rs` — write facade and translation

Owns the public `WriteReceipt` and `PreparedWrite`. It also owns the
translation `storage_write_shape` (provenanced to storage shape),
`batch_is_admin`, `Engine::write` (event and counter wrapper), `write_inner`,
and late enrolment (`batch_vector_kinds_needing_enrolment`,
`vector_kind_needs_enrolment`, `enrol_and_unstrand`).

The `write_inner` order is preserved exactly:

1. `ensure_open`;
2. empty batch returns `WriteValidation`;
3. writer lock;
4. `maintain_before_writer`;
5. `validate_batch`;
6. nested projection-source validation;
7. read-only enrolment decision;
8. `collect_projection_jobs`;
9. the debug commit hook;
10. cursor base;
11. `commit_batch`;
12. publish `next_cursor`;
13. `notify_new_work`.

Late enrolment calls projection-registry helpers that remain at root for
Slice 70 (`kind_is_vector_indexed`, `vector_projection_declared`,
`register_vector_kind`, `reenqueue_stranded_vector_rows`,
`index_targets_for_row_kind`, `kind_is_vector_committable`). These are
call-only seams.

### `write_validation.rs` — pre-transaction validation

Owns `WritePlan`, `validate_batch`, `collect_projection_jobs`,
`validate_write`, `collection_metadata`, `validate_payload`, the external-ref
checks, and the `prior_*_cursors_*` lookups. Everything here is read-only
against the writer connection. It runs before `BEGIN IMMEDIATE`, and the
commit layer reuses the prior-cursor lookups inside the transaction.
`actuation.rs` keeps calling `validate_write`.
`validate_nested_projection_sources_for_write` is a projection-registry
validator, so it stays at root and is called as before.

### `write_commit.rs` — transactional execution

Owns `CommitBatchError` and its `From` impls, revision identity (hash fields,
runtime and legacy revision ids, `canonical_body_hash`,
`checked_locator_columns`, `revision_is_registered`,
`register_artifact_identity`, and the `#[cfg(test)]` counter
`CANONICAL_BODY_HASH_CALLS`), `TriggerStateGuard`,
`canonical_batch_has_no_custom_triggers`, `commit_batch`,
`advance_read_visibility`, and `apply_batch_in_transaction`.
`projection_batch_has_no_custom_triggers` has only a projection-worker caller
and stays at root.

The Slice 71B trigger-suppressed path and the row-trigger fallback remain one
function. They are not split. `apply_batch_in_transaction` remains the
shared seam for `commit_batch` and actuation.
`dependency.rs` and `evidence.rs` import `canonical_body_hash` and
`checked_locator_columns` from here. Projection row writers
(`project_canonical_*_row`) and dependency/closure helpers stay where they
are and are called as before.

### `provider.rs` — shared NDJSON transport

Owns `ProviderTask`, `ProviderSession` (with its `Drop` and handshake/request
behavior), `extractor_io_timeout`, `recv_extractor_line`, and
`Engine::provider_session`. There is one transport and no per-task copy. The
`Consolidate` caller's error remap stays in `consolidation.rs`.

### `ingest.rs` — extractor ingest

Owns `ExtractDocument`, `IngestWithExtractorReceipt`,
`Engine::ingest_with_extractor`, `run_extract_session`, and
`dedup_prepared_by_logical_id`. It writes through the public `Engine::write`
facade (node batch, then edge batch per request), exactly as today; the calls
are copied, not rerouted.

### `consolidation.rs` — consolidation provider

Owns the three `Consolidate*` DTOs, `Engine::consolidate_with_provider`,
`run_consolidate_session`, `assemble_consolidate_cluster`,
`apply_consolidate_verdicts`, `active_edge_write_cursor`, and
`prune_edge_projection_shadows`. Its own transaction and verdict ordering are
unchanged.

### Unchanged owners

- `actuation.rs`: imports only follow the helpers.
- Cursor primitives, `RowKind`, projection and registry code, and write test
  hooks: stay at root.

## Characterization design

### Existing deep owners (reused, not duplicated)

- **Same-batch ordering:**
  - `pr_g0_identity::s15_row_cursors_are_one_to_one_with_the_batch`
  - `batch_write_per_row_cursor`
  - `pr_g8_dangling_edges::s20_same_batch_*`
  - `slice15_byo_llm_ingest::ingest_dedups_duplicate_edges_within_batch`
- **Replay and idempotency:**
  - `pr_g0_identity::s15_*supersession_is_idempotent*`
  - `slice20_source_dependencies::registration_replay_*`
  - `slice25_actuation::mixed_batch_commits_atomically_and_exact_replay_is_idempotent`
- **Caller-owned identity and provenance:**
  - `slice15_identity_provenance` (restart persistence, runtime revisions)
  - `source_id_writes`
  - `provenance_mandatory`
- **Partial rollback owners, kept as they are:**
  - `slice15_identity_provenance::failing_provenance_write_cannot_commit_late_vector_enrolment_or_queue_changes`
  - `op_store::ac_060b_*`
  - `cursors::failed_commit_*`
  - root `visibility_exhaustion_rolls_back_and_restores_triggers`

### New suite: `tests/write_boundary_atomicity.rs`

**Snapshot oracle.** First call `engine.drain(timeout)` and require `Ok`, so
the background projection worker is idle before each snapshot. Do this for
both the pre-call and the post-call snapshot. Then open a separate `rusqlite`
connection on the database file (WAL-committed view). Enumerate the
`sqlite_master` rows with `type='table'`. Exclude virtual tables by
`sql LIKE 'CREATE VIRTUAL TABLE%'`; their shadow tables are ordinary tables
and are captured. Dump every row of every table using the typed cell encoding
of `correction_safe_erasure.rs` `database_plane_snapshot` (`ValueRef`-tagged,
real bits hex, text/blob hex, rows sorted). Also capture the `sqlite_master`
rows themselves. This oracle supersedes that hand-listed table set for write
boundaries. A refused write touches no nondeterministic column:
`state_nonce` changes only on a successful visibility advance.

Tables are discovered, not listed by hand. A new state plane therefore cannot
be silently omitted. Assert full equality between the pre-call and post-call
snapshots.

**Cursor oracle.** After each refusal, a valid write returns
`row_cursors == [pre_cursor + 1]`. This shows that the in-memory cursor was
not published and no cursor was consumed. Where setup altered a singleton to
force the fault, restore it through a separate connection before this probe. TEMP triggers
are dropped through `execute_for_test("DROP TRIGGER temp.<name>")` before the
probe. The probe writes a kind that no case's trigger matches. The
probe runs after the equality assertion.

**Closure precondition.** `maintain_before_writer` runs in `write_inner`
before validation, and in consolidation before `BEGIN`. It legitimately
commits the finalization of `proving`/`incomplete` closures. Each case
therefore asserts zero such `_fathomdb_dependency_closures` rows before the
pre-snapshot.

**Fixture.** Every case starts from a seeded database that holds:

- active nodes and an edge;
- a provenanced node with a source version;
- a declared vector projection; and
- `FixedEmbedder` (from the `slice15` pattern), so late enrolment is live.

Every refused `Engine::write` batch contains an unenrolled vector-eligible
kind (the provider cases are exempt), so a leaked
enrolment would show up in `_fathomdb_vector_kinds` or the queue and terminal
tables.

**Table cases.** Each case names its boundary, how the fault is produced, the
expected error, and the temporary mutant that must make it fail. Record each
mutant's failing assertion in `tdd-chronology.md`.

| Case | Boundary | Fault | Expected error | Required mutant |
| --- | --- | --- | --- | --- |
| structural | pre-transaction validation (after the writer lock) | an all-Node batch with an invalid node (for example, empty body or bad window) after valid ones | `WriteValidation` | Temporarily validate only `batch[..1]` and reuse its `WritePlan::Node` for every item, so the trailing invalid node commits. |
| db_dependent | validation against stored schema | a `PreparedWrite::OpStore { schema_id: Some(<collection name>) }` payload (the id equals the collection, so the check reaches `validate_payload`) failing a collection schema registered by a prior `AdminSchema` write (`validate_payload`) | `SchemaValidation` | Temporarily skip `validate_payload`. |
| enrolment_raise | auxiliary enrolment, first in the transaction, on the row-trigger path | a TEMP `BEFORE INSERT ON _fathomdb_vector_kinds WHEN NEW.kind = '<sentinel kind>'` trigger that raises, installed through `execute_for_test` | `Storage` | In `apply_batch_in_transaction`, temporarily replace `register_vector_kind(tx, kind)?;` with `let _ = register_vector_kind(tx, kind);`, so the batch commits without enrolment. |
| pre_tx_hook | commit hook before `BEGIN` | `force_next_commit_failure_for_test`; the case is `#[cfg(debug_assertions)]` | `Storage` | Temporarily hoist the `base_cursor`/`last_cursor` computation and the `next_cursor.store` above the hook. |
| late_provenance | apply, inside the transaction | a replayed `ProvenancedNode` (same `artifact_revision_id`) batched with a new-kind node, so `register_vector_kind` runs before `RevisionIdConflict` | `Provenance(RevisionIdConflict)` | Temporarily commit enrolment through `enrol_and_unstrand` before `commit_batch`. |
| execution_raise | apply, inside the transaction, on the row-trigger path | a TEMP `BEFORE INSERT` trigger on `canonical_edges`, installed through `execute_for_test`, raising on a sentinel edge placed after nodes | `Storage` | On the row-trigger path, temporarily apply and commit each item in its own `BEGIN IMMEDIATE` transaction. |
| visibility_last | last statement before `COMMIT` | `_fathomdb_read_visibility_state.generation` set to `i64::MAX` | `Storage` | Temporarily ignore the visibility result (`let _ = advance_read_visibility(&tx);`). |
| provider_handshake | before the first provider write | a harness replying with a wrong protocol in `ready` | `Extractor` | Temporarily write a sentinel row before the handshake. |
| provider_request_id | shared transport (`ProviderSession::request`), before the first batch write; `max_docs_per_request` at least the document count | mismatched `request_id` | `Extractor` | Temporarily write a sentinel row in `run_extract_session` before its `.request(..)` call. |
| consolidate_verdict | verdict validation | an out-of-cluster verdict that follows an applied `invalidate` verdict (which set `t_invalid`), inside the consolidation `BEGIN IMMEDIATE`; the cursor probe does not apply (plan item 8) | `Consolidator` | Temporarily run `tx.execute_batch("COMMIT; BEGIN IMMEDIATE")` after each applied verdict. |

For `execution_raise`: a TEMP trigger is a custom trigger, so this case
deliberately exercises the unchanged row-trigger fallback. `visibility_last`
exercises the trigger-suppressed path. Together they cover both branches of
`commit_batch`.

If a case's documented error differs from the one observed at baseline,
record the observed stable error. A differing error is not a defect by
itself. A snapshot difference is a defect.

**Profiles.** Integration tests link the library without `cfg(test)`. The
seams are therefore gated as follows:

- `pre_tx_hook` carries `#[cfg(debug_assertions)]`.
- `execution_raise` and `enrolment_raise` need a TEMP trigger on the engine's own connection
  through `execute_for_test`, so it carries
  `#[cfg(any(debug_assertions, feature = "test-hooks"))]`.

Helpers and imports used only by gated cases carry the same cfg. Otherwise
the release `--tests` typecheck with `-D warnings` reports them as dead code.
The file has no file-level `#![cfg]`, so no `[[test]]` or feature-matrix
registration is needed. The hidden inventory shows one new test target and
its tests as a reviewed additive diff. Every other fixture mutation, including setting or restoring the visibility
singleton, uses a separate `rusqlite` connection. Those cases compile in
every profile. The test-inventory count differs by profile only by these
gated cases.

**Property.** Use `proptest` with 32 cases and a fixed `failure_persistence`
of none. Generate 1-6 valid nodes and edges with unique logical ids and
bodies, and one invalid item from a closed set of structural faults inserted
at a generated index. Assert `WriteValidation` and full snapshot equality.
The invariant is human-defined: validation precedes mutation. No generated
oracle is stored.

**Non-vacuity.** Each mutant is applied locally, the targeted case fails, and
the mutant is reverted. Production is byte-identical at the test commit.

### Structural-move evidence

- **Per move batch:** focused owners, the new boundary suite, Slice 30 public
  comparison, and hidden comparison against
  `dev/plans/0.8.27/features/hidden-surface/baseline-*.json` (latest
  ancestor).
- **Expected hidden diff:** additive tests only. These are the new suite's
  tests and none else. The moved private items do not appear in the
  hidden-inclusive rustdoc rows, because private items are not captured. If
  they do appear, the batch stops and the reviewed-successor procedure
  applies.
- **Focused owners:**
  - `slice15_identity_provenance`, `source_id_writes`, `provenance_mandatory`
    (operator), `pr_g0_identity`, `pr_g8_dangling_edges`,
    `batch_write_per_row_cursor`, `cursors`, `op_store`
  - `slice15b_node_validity_write`, `slice45_nested_source_projections`,
    `tc57_*`, `error_taxonomy`
  - `slice15_byo_llm_ingest`, `slice5_provider_seam`,
    `multidoc_extractor_provenance`, `consolidate_provider`,
    `tc33_fix3_consolidation_timestamp_taxonomy`
  - `slice25_actuation*`, `slice35_actuation_spike`, `correction_safe_erasure`,
    `slice20_*`, `slice30_dependency_closure`
  - the crate's lib tests
- **Feature checks:** `cargo check -p fathomdb-engine --all-targets` with
  default, `operator`, `test-hooks`, `slice72-test-hooks`,
  `migration-test-hooks`, and `tc5-benchmark` features, plus
  `agent-typecheck.sh`'s release `--tests` checks.

## Risks

- **Doc intra-links** (for example, the `[`enforce_provenance_retention`]`
  link at `erasure.rs:1430`) may stop resolving after the move. No gate runs
  rustdoc link checks. Doc text stays verbatim, and the implementer records
  each newly unresolved link in `tdd-chronology.md` rather than editing
  public docs.
- **Source scrapers.** `tests/slice35_virtual_mutation_manifest.rs` and
  `experiments/slice35_virtual_mutation_audit.py` scrape
  `apply_batch_in_transaction` and `prune_edge_projection_shadows` by file.
  They receive the path-only amendment in plan step 3. The pinned
  `lib.rs:<line>` citations in `scripts/c1-conformance-pin.json`, the Windows
  WAL guard, and the facade `include_str!` tests cover no moved item.
- **`debug_assertions`-only hooks** in `write_inner` keep their `cfg` exactly.

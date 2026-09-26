---
title: FathomDB 0.8.27 Slice 60 - engine write, ingest, and consolidation domains
status: APPROVED
target_release: 0.8.27
baseline_sha: 517e0545
---

# Slice 60 plan

Independent design review passed after three correction cycles. The durable
record is `design-review.md`.

A post-closeout adversarial review found three contract gaps. Design FIX-1
amends the accepted inventory, adds coverage at both real writer-commit exits,
and requires the move not to add rustdoc-link warnings. Its durable record is
`adversarial-review.md`.

## Outcome

Move the root-owned write, provider-ingest, and consolidation implementation
out of `fathomdb-engine/src/lib.rs` into private semantic modules without
changing behavior, schema, public paths, feature gates, transaction scope,
lock acquisition, or call order. Before moving, add the missing write-boundary
characterization: a full-state, pre-call snapshot oracle for every failure
boundary on `Engine::write` and the two provider verbs.

This is a behavior-preserving structural slice. It fixes no known defect. If a
characterization test exposes a semantic defect, the move stops for a separate
RED/GREEN correction (test approach, "Separate behavior changes from
structural moves").

## Reconciliation since the draft

The master Slice 60 paragraph was written in prework at `a3e6cff6`. The
executable baseline is now `517e0545`. The material changes are:

1. Slice 40 moved errors, identity, temporal, and test hooks into private
   modules. The write path imports temporal validation helpers and keeps
   using them. Temporal authority does not move.
2. Slice 50 moved record lifecycle, provenance contracts, dependency
   registration, and erasure into private modules. It explicitly left
   `PreparedWrite` and write execution at the root for this slice.
   `dependency.rs` and `evidence.rs` now call the root write seams
   `canonical_body_hash` and `checked_locator_columns`. Those call sites
   follow the helpers. `record_lifecycle.rs` opens its own transaction and
   does not call the write seams.
3. `actuation.rs` already owns the actuation domain (DTOs, receipts,
   `Engine::actuate`, replay, redaction). It consumes the root write seams
   `validate_write`, `apply_batch_in_transaction`, `CommitBatchError`,
   `batch_vector_kinds_needing_enrolment`, and `load_next_cursor`. The draft's
   "organize actuation" is therefore narrowed to rewiring these imports. No
   actuation logic moves.
4. The post-Slice-40 hidden-surface oracle, test-inventory rows, target-
   coverage gate, and Slice 30 immutable real-surface baseline are required
   structural evidence. They are reused unchanged.
5. The write-path test inventory (recorded in `design.md`) found no test that
   compares every state plane against a pre-call snapshot after a failed
   write. It also found no fault seam inside the write transaction and no
   zero-mutation check for provider protocol failures. The draft's
   characterization list is approved for these gaps only. Same-batch
   ordering, replay/idempotency, and caller-owned identity already have deep
   success-path owners (listed in `design.md`). They are reused, not
   duplicated.
6. No new codec or translation logic is introduced. The draft's "property
   tests for new codec or translation logic" therefore applies to one
   property: validation-before-mutation over generated mixed batches. No
   codec property is invented.

7. Provider ingest commits each extracted node batch and edge batch through
   the atomic `Engine::write` facade, request by request. By existing
   contract, it is not atomic across requests or across a request's node and
   edge batches. Each committed batch's atomicity is owned by the new write-
   boundary suite. The slice characterizes provider failures before the first
   write only. It does not invent cross-request atomicity.
8. Consolidation's `supersede`/`merge` verdicts consume the in-memory cursor
   inside their transaction (`lib.rs:8785`). A later refusal rolls back the
   database but not that atomic. Cursors are monotonic, not dense, and reopen
   re-derives them from `MAX`. This existing behavior is outside the
   structural scope. The consolidation case therefore pins its preceding
   valid verdict to `invalidate` and does not apply the cursor probe.

Evaluation: approve the draft outcome with changes 3 and 5-8. Reject moving
the cursor primitives (`load_next_cursor`, `max_cursor`,
`reserved_write_cursor`), which open and read paths share (Slice 90). Also
reject moving projection enrolment and registry helpers (Slice 70) and the
lifecycle transition-move predicate (record lifecycle, already settled by Slice 50).

## Assigned inventory and disposition

Line numbers are at `517e0545`.

| Root-owned material | Disposition |
| --- | --- |
| `WriteReceipt`, `PreparedWrite`, `storage_write_shape`, `batch_is_admin` (12994), `Engine::write`, `write_inner`, and the late-enrolment trio (`batch_vector_kinds_needing_enrolment`, `vector_kind_needs_enrolment`, `enrol_and_unstrand`) | Move to private `write.rs`; root re-exports the public types. |
| `WritePlan`, `validate_batch`, `collect_projection_jobs`, `validate_write`, `collection_metadata`, `validate_payload`, the external-ref checks, and the `prior_*_cursors_*` lookups (22032-22341) | Move to private `write_validation.rs`. |
| `CommitBatchError` and its `From` impls, revision identity (`revision_hash_field`, `runtime_revision_id`, `canonical_body_hash`, `checked_locator_columns`, `revision_is_registered`, `register_artifact_identity`, and the `#[cfg(test)]` `CANONICAL_BODY_HASH_CALLS` counter), `TriggerStateGuard`, `canonical_batch_has_no_custom_triggers`, `commit_batch`, `advance_read_visibility`, `apply_batch_in_transaction` (24677-25600), and `enforce_provenance_retention` (19816, whose only caller is `apply_batch_in_transaction`) | Move to private `write_commit.rs`. `revision_hash_field` is the narrow crate-visible seam used by the root exception below. |
| `ProviderTask`, `ProviderSession`, `extractor_io_timeout`, `recv_extractor_line`, `Engine::provider_session` | Move to private `provider.rs` (shared NDJSON transport). |
| `ExtractDocument`, `IngestWithExtractorReceipt`, `Engine::ingest_with_extractor`, `run_extract_session`, `dedup_prepared_by_logical_id` | Move to private `ingest.rs`. |
| `ConsolidateAxis`, `ConsolidateCandidateEdge`, `ConsolidateReceipt`, `Engine::consolidate_with_provider`, `run_consolidate_session`, `assemble_consolidate_cluster`, `apply_consolidate_verdicts`, `active_edge_write_cursor`, `prune_edge_projection_shadows` | Move to private `consolidation.rs`. |
| `actuation.rs` | No move. Its imports follow the write seams. |
| `load_next_cursor`, `max_cursor`, `reserved_write_cursor`, `RowKind`, `is_legal_transition_move` (5214), `validate_nested_projection_sources_for_write` (22843), `projection_batch_has_no_custom_triggers` (25087, projection-worker only), projection enrolment/registry helpers, write-path test hooks (`force_next_commit_failure_for_test` and peers) | Stay at root for Slices 70 and 90. |
| `legacy_revision_id` (`#[allow(dead_code)]`) | Stay private at root with its root unit test. Moving it to a sibling would require `pub(crate)`, which AC-050a forbids for every `legacy_*` item. |

## Requirements and acceptance

The release-local R27-04/R27-05 and AC27-07/08 remain authoritative. Slice 60
adds:

| ID | Requirement | Acceptance |
| --- | --- | --- |
| R27-60A | Root ownership shrinks along the write, validation, commit, provider, ingest, and consolidation boundaries without adding a public path. | AC27-60A: the six private modules own exactly the approved inventory as amended above, with the explicit private-root `legacy_revision_id` exception required by AC-050a. The Slice 30 immutable public comparison has empty diffs. Every public root item keeps its path, signature, derives, and cfgs. |
| R27-60B | With no nonterminal dependency closure pending (`maintain_before_writer` legitimately finalizes those before validation), a write rejected at any boundary leaves every durable state plane byte-identical to its pre-call snapshot. It does not publish or consume a cursor. | AC27-60B: a table-driven characterization over a `sqlite_master`-derived full-database snapshot passes for these boundaries: pre-transaction structural validation, database-dependent schema validation, the pre-transaction hook, auxiliary enrolment failure, late in-transaction provenance refusal with a pending late vector enrolment, an execution-time `RAISE` inside the transaction, last-statement visibility exhaustion, and a one-shot commit abort at each of the trigger-suppressed and row-trigger `tx.commit()` exits. The commit cases are executed in both debug and release profiles with `test-hooks`. Translation (`storage_write_shape`) is an infallible `Cow` conversion, so it has no failure boundary. Each case also asserts that a following valid write receives the next unconsumed cursor. Each case is shown non-vacuous by a recorded temporary mutant. |
| R27-60C | Provider verbs that fail before their first write do not mutate state. A consolidation refusal after an applied verdict rolls back the whole transaction. | AC27-60C: a bad handshake and a mismatched `request_id` (with `max_docs_per_request` at least the document count) leave the full snapshot unchanged. An out-of-cluster verdict that follows an applied `invalidate` also leaves it unchanged. Each case has a recorded mutant. |
| R27-60D | Validation precedes every mutation for any batch composition. | AC27-60D: a bounded `proptest` inserts one invalid item at a generated position among generated valid nodes and edges. It asserts `WriteValidation` and an unchanged full snapshot. |
| R27-60E | The move creates no semantics and widens no authority. | AC27-60E: schema stays 34. No SQL text, statement order, transaction behavior, error mapping, wire, binding, package, or `dev/acceptance.md` change. Only the minimum crate-internal visibility needed across sibling modules is added. A moved-symbol intra-doc link may receive a link-only or plain-code-span repair. |
| R27-60F | Structural evidence covers public, hidden, test, and source-documentation surfaces. | AC27-60F: public and hidden structural rows compare equal. The release probe is equal. Test inventory shows only reviewed additive tests. Target coverage remains complete. The post-move rustdoc broken-link warning set adds no warning over the pre-move baseline. All `--all-targets` feature checks named in step 7 pass, and so does the release-profile typecheck. |
| R27-60G | The slice closes with independent review and receipts. | AC27-60G: design review, code review, independent verification, focused default and `test-hooks` debug/release tests, `agent-verify`, workspace Clippy with warnings denied, `cargo check --workspace --all-targets`, and the candidate SHA are recorded. |

No interface, ADR, or global acceptance change is needed.

## TDD implementation sequence

Use one writer in the durable release worktree. Read-only reviewers share it.

1. **Characterize (tests only).** Add `tests/write_boundary_atomicity.rs` per
   `design.md`. These are characterization tests of already-correct
   behavior, so they may pass at first. For each case, apply the named
   temporary mutant, record the failing assertion, restore production, and
   commit tests only. If a case fails against unmodified production, stop:
   that is a genuine defect and gets a separate RED/GREEN correction with
   review before any move. The post-closeout FIX-1 extension first adds two RED
   tests for a missing one-shot writer-commit-abort seam, then implements that
   private `test-hooks`-only TEMP-marker seam at both `tx.commit()` exits; it
   reuses `execute_for_test` and adds no public or hidden item.
2. **Pre-move receipt.** Run the focused owner suites (listed in `design.md`),
   the Slice 30 public comparison, and the hidden-surface comparison at the
   characterization commit.
3. **Move write validation, then commit.** Copy verbatim.
   `tests/slice35_virtual_mutation_manifest.rs` scrapes `apply_batch_in_transaction`
   and `prune_edge_projection_shadows` from `include_str!("../src/lib.rs")`.
   `experiments/slice35_virtual_mutation_audit.py` keys the same two sites
   to `lib.rs`. Amend them as reviewed path-only test-infrastructure changes.
   `SOURCE` becomes a `concat!` of the `include_str!`s of `lib.rs` and
   `write_commit.rs`, and step 5 adds `consolidation.rs`. Each audit key is
   re-keyed to its new file in the step that moves its function. That
   includes pulling the `prune_edge_projection_shadows` tuple out of
   `PRODUCTION_INVENTORY`'s `lib.rs` comprehension. Every needle, count, and coupling assertion stays
   byte-identical. Record this in `tdd-chronology.md`. The move is RED
   until imports and visibility compile. Restore GREEN without changing any
   assertion. Run the write owners, actuation, record-lifecycle, dependency,
   evidence, and the new boundary suite.
4. **Move the write facade.** Move the DTOs, translation, `Engine::write`, and
   late enrolment. Rerun the same owners and the surface comparisons.
5. **Move provider, ingest, and consolidation.** Run the provider-seam, BYO
   ingest, multidoc provenance, consolidation, and timestamp-taxonomy owners,
   plus the boundary suite.
6. **Refactor within the boundaries.** Change imports and visibility only. A
   behavioral change requires its own RED test.
7. **Gate.** Final public/hidden/test-inventory comparisons, a pre/post rustdoc
   broken-link warning-set comparison, feature checks
   (default, `operator`, `test-hooks`, `slice72-test-hooks`,
   `migration-test-hooks`, `tc5-benchmark`), the release typecheck
   (`agent-typecheck.sh`), the slice35 audit's Python test
   (`PYTHONPATH=. python -m pytest -q tests/experiments/test_slice35_virtual_mutation_audit.py`),
   `cargo fmt --check`, the boundary suite under `--features test-hooks` in
   debug and release profiles, `./scripts/agent-verify.sh`, workspace Clippy with
   `-D warnings`, and `cargo check --workspace --all-targets`.
8. **Review.** Independent code review of the actual diff (Opus 5.5, high).
   Resolve findings with RED/GREEN when behavioral. Run a focused re-check
   only, unless the fix's blast radius warrants more.
9. **Verify.** Independent read-only verification (Sonnet) at the clean
   candidate.
10. **Close.** Write `tdd-chronology.md`, `design-review.md`,
    `code-review.md`, `review-verification.md`, and `status.md`. Advance the
    release state to Slice 70 and regenerate its views.

## Non-goals

- no projection, embedding, vector-registry, read, search, open, or runtime
  moves (Slices 70-90);
- no binding or SDK change;
- no schema, SQL, wire, error, or public-interface change;
- no actuation redesign;
- no duplicate of existing success-path ordering, replay, or identity owners;
  and
- no full-regression rerun while resolving a narrow finding.

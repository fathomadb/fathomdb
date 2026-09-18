---
title: FathomDB 0.8.26 Slice 60 — authority and witness inventory
status: REVIEWED
---

# Slice 60 authority and witness inventory

Current planning baseline: `b770de01`. Draft-change baseline: `65f69ca1`.

## Post-draft changes

| Change after `65f69ca1` | Impact | Disposition |
| --- | --- | --- |
| Slice 55 canonical map and parity checker | Operation classification | Reuse 44 operations; no second list. |
| Python/TypeScript real-surface tests | SDK presence/absence | Executable witness for `rerank` and absent SDK operator roots. |
| TypeScript `rerank` and N-API bridge | Retrieval owner | Replace deferred claim; default search remains unchanged. |
| Binding design/interface amendments | Retrieval terminology | Preserve reviewed validation/device/identity contracts. |
| Slice 55 Astra remediation | Verification baseline | Preserve focused suites; no map change required. |

No later commit changed the target owner files. No unruled D26 decision or
Slice 3–8 allocation remains for Slice 60.

## Retrieval clusters

| Cluster | Authority | Implementation witness | Test witness | Disposition |
| --- | --- | --- | --- | --- |
| Typed lexical/vector/filter reads | `ADR-0.6.0-retrieval-pipeline-shape.md`; `interfaces/rust.md` | `fathomdb-engine/src/lib.rs`: `search_inner_with_stats`; `fathomdb-query/src/lib.rs` | `slice35_filter_grammar.rs`; `slice35_remaining_arm_matrix.rs` | Current; spellings delegated. |
| Text boundary and bounds | `ADR-0.6.0-text-query-latency-gates.md`; `retrieval-result-limits.md` | `lib.rs`: direct FTS collectors | `slice18_retrieval_result_limits.rs`; `slice23_text_limit_prefix_stability.rs` | Current; detail delegated. |
| Vector shortlist, exact rerank, RRF | `vector.md`; SDK interfaces | `lib.rs`: vector SQL builders and `fuse_rrf` | `pr_g9_rrf_fusion.rs::rrf_end_to_end_order_is_deterministic`; vector quantization suites | Current. |
| Optional CE and standalone `rerank` | `interfaces/python.md`; `interfaces/typescript.md`; `governed-operation-parity.json` | `lib.rs`: `rerank_passages`; binding wrappers | `pr_e2_rerank_passages.rs`; `test_ce_rerank_probe.py`; `standalone-rerank.test.ts` | Current; old deferral false. |
| Device and fallback | `ADR-0.8.23-dual-runtime-device-policy.md`; SDK error sections | `lib.rs`: `SoftFallbackBranch`; device resolution | `slice71_rerank_policy.rs`; CLI device tests | Current; four branch values. |
| Validity/filter/frozen eligibility | `ADR-0.8.25-eligibility-and-frozen-reads.md`; SDK interfaces | `frozen_read.rs`; eligibility SQL builders | `slice35_frozen_read.rs`; `slice35_remaining_arm_matrix.rs` | Before truncation. |
| Explanation/evidence/trace | evidence ADRs; SDK interfaces | result finalizer; `evidence.rs` | `slice50_evidence.rs::ordinary_search_is_equivalent_and_evidence_is_stateless`; Slice 55 explanation tests | Opt-in sidecars. |
| Graph expansion/evidence | graph interface sections; `ADR-0.8.26-exact-graph-artifact-evidence.md` | `graph_expand.rs`; `evidence.rs` | `slice60_graph_expand.rs`; graph-evidence tests | Current; continuation/full paths deferred. |

## Recovery clusters

| Cluster | Authority | Implementation witness | Test witness | Disposition |
| --- | --- | --- | --- | --- |
| Roots/flags | `ADR-0.6.0-cli-scope.md`; `interfaces/cli.md` | `fathomdb-cli/src/lib.rs`: `Command`, `DoctorCommand`, `RecoverArgs` | `fathomdb-cli/tests/parser.rs` | Current. |
| Doctor inventory/effects | REQ-036; `interfaces/cli.md` | `DoctorCommand`; `run_doctor` | `operator_cli.rs`; CLI unit process matrices | Enumerate and classify exactly. |
| Mutable-doctor authority conflict | CLI-scope ADR; REQ-036 | `Engine::recompute_mean`; `recompute_mean_in_tx_inner`; `run_pin_and_requantize_pass` | `pr2b_mean_recompute.rs::recompute_fault_rolls_back_fully`; `operator_cli.rs::t_pr2b_recompute_mean_happy_path_json_contract` | Authorized narrow successor ADR; no generalized exception. |
| Five recovery actions | `interfaces/cli.md` | `run_recover` | parser recovery-action cases; `operator_cli.rs` | Enumerate exactly. |
| Immutable integrity route | v2.2; `interfaces/cli.md` | `inspect_data_plane_integrity` | Slice 30 immutable process matrix | Uniquely immutable. |
| SDK/operator boundary | `governed-operation-parity.json`; SDK interfaces | facade `operator` feature boundary | `test_sdk_surface_parity_oracle.py`; `sdk-surface-parity.test.ts` | SDK erasure still exists. |
| One-object recovery output | `interfaces/cli.md` | `wire_recover`; recovery serializers | `operator_cli.rs` JSON assertions | Correct stale stream prose. |
| Path-scoped WAL recovery | `interfaces/rust.md`; `interfaces/cli.md`; `recovery.md` | `recover_truncate_wal`; `validate_recovery_schema_invariants`; `ShmSnapshot` | `truncate_wal.rs`; `recovery_cli.rs` | Operator-only; schema-34 invariants, byte-preserving refusal, SQLite-owned destructive checkpoint. |
| Projection repair | `recovery-0.8.25.md` | rebuild methods | projection-generation/rebuild suites | Delegate detail. |

## Engine clusters

| Cluster | Authority | Implementation witness | Test witness | Disposition |
| --- | --- | --- | --- | --- |
| Schema-34 fresh admission/open | breaking-boundary ADR; SDK interfaces | `Engine::open` | `slice40_fresh_database_cutover.rs::schema_34_bootstraps_fresh_and_reopens_without_migration` | Current. |
| Logical/write identity | `ADR-0.8.0-canonical-identity-substrate.md`; schema | writer supersession path | `pr_g0_identity.rs::s31_node_kind_change_reingest_supersedes`; `s15_row_cursors_are_one_to_one_with_the_batch` | `logical_id` alone; no caller `row_id`. |
| Caller/projection write lanes | v2.2; `projections.md` | `Engine.connection`; projection worker connections | projection concurrency/readiness suites | Current. |
| Projection commit ordering | v2.2; `projections.md` | `ProjectionRuntimeShared.commit_gate` | projection race and mean-pin suites | Current. |
| Reader transactions | async/read authority | `ReaderWorkerPool` | frozen/read concurrency suites | Current. |
| Atomic batches/receipt | typed-write and actuation ADRs | `Engine::write`; `WriteReceipt` | batch rollback suites; `batch_write_per_row_cursor.rs` | Three receipt fields. |
| Cursor/readiness distinction | SDK interfaces; v2.2 | `WriteReceipt`; `SearchResult`; projection status types | drain/visibility suites | Not revision identity. |
| Shared vector storage | `ADR-0.6.0-vector-index-location.md`; `vector.md`; schema | `vector_default`; `_fathomdb_vector_rows` | vector query/rebuild suites | Replace per-kind claim. |
| Runtime lifetime | engine/error interfaces | open/startup/close paths | startup-readiness, close, lock, and Windows suites | Exact errors delegated. |

## Review decision

The goal is complete after the bounded requirements/CLI corrections and owner
rewrites. Product repair, generalized recurrence tooling, broad error-owner
work, and candidate requalification remain outside this slice.

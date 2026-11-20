---
title: Slice 55 data integrity and recovery
status: COMPLETE
slice: 55
target_release: 0.8.30
---

# Slice 55 — Data integrity and recovery

Owner seq-407 allocates this scope from the prior 50 plan.
Owner seq-409 commissions the whole plan with `gpt-6.1-sol` high, after
integration of reviewed partial50 source.55 precedes60, which supplies the
remaining50 prerequisite; no completed slice is reopened.

## Entry and ownership

Reconcile the commissioned checkout against
`9538272e638d6bca4547a61519a0ce9c98c3872d` and current `origin/main`.
Inventory changes to the source, contracts, assigned Slice 3/4/5 and seq-406
drafts, and actual feature/test registrations. Record what applies and who owns
any prerequisite. Consume 10's approved setup, 30's relevant correctness proof,
and any 50 work needed by a selected lifecycle seam. Independent CPU design
and source work can proceed while 50 or 60 has unrelated open rows.
[Master](../../plan.md), [coverage audit](../../coverage-audit.md).

Users need correct record identity, visibility and durability as canonical data
and projections change. Operators need truthful diagnostic outcomes and exact
post-recovery state. 55 implements the selected R30-TEST integrity, recovery and
boundary gaps. 50 owns runtime admission, close, logging, Node resources and
conditional model release. 70 integrates executed behavior and source proof.
75 qualifies target-specific installed artifacts and Windows WAL. Protect the
retained40 proof and serialize shared source edits.

## Needs, requirements and acceptance

| Need and local requirement | Observable acceptance |
| --- | --- |
| R30-TEST / one committed object view | One independently authored real-store scenario and a bounded seeded operation model detect a wrong revision, stale authority or untruthful readiness. Cover write, committed lag, drain, update/supersession, rollback and fresh-process reopen across canonical, lexical, vector, graph and source/dependency observations. |
| R30-TEST / truthful temporal and graph retrieval | Assert node start-inclusive/end-exclusive visibility, expiry at fixed generation, applicable current/history flags, filtered bridges, direction/kinds and cap placement. At the edge boundary, `t_invalid == T` is excluded and `> T` eligible; future `t_valid` may still traverse under the shipped contract. Require exact relationship and provenance facts, with independently expected results for each modality. |
| R30-TEST / malformed-input resistance | Exercise at most three chosen production boundaries with nonzero input counts, reproducible seeds and a fixed budget. Assert no panic, hang, partial persisted change or unintended error class. Replay minimized counterexamples in the ordinary owning suite. |
| R30-TEST / committed state after failure | Existing CPU before/after-ack and projection-stop seams plus at most three missing CPU composition fault points. Show healthy control, actual delivery, exact survivors and absent uncommitted effects, pending/terminal authority, retained projection/dependency state, successful retry and teardown. Observe useful held-reader work, WAL growth, busy checkpoint, release/join and successful checkpoint/reopen. |
| R30-TEST / all doctor outcomes and effects | Account for all 17 current variants by command, contract, exact exit/semantic JSON fields and declared effects. Fill selected missing rows, including full-integrity findings and the paired WAL-only export/recovery witness below. Reuse existing runtime witnesses. |
| R30-TEST / thin boundary parity and oracle sensitivity | One authored expectation set exercises Rust/Python/TypeScript and MCP structured/compact at their actual boundaries. Record an independent oracle or a plausible wrong condition, ordering or cleanup that the test demonstrably rejects. Preserve execution evidence, skips and uncovered branches. |

## Architecture and draft design

- Use real format-37 stores and supported entry points. Name two sources,
  Unicode/equal-body distinct revisions, fixed known vectors, declared
  attributes, and one or two authored graph relations. Verify exact IDs,
  ranks where an independent oracle exists, source/revision evidence, and
  canonical/projection state. Observe durable committed lag using an existing
  rendezvous and then convergence after drain. Derive 12–24 model sequences
  of at most 12 operations from a small independent expected-state model.
  Keep FTS5, vector projection and canonical graph traversal distinct. Use
  fixed read instants and boundary crossings without a generation mutation.
  Preserve frozen/readiness semantics, scope and candidate ordering.
  [Architecture](../../../../design/fathomdb-data-plane-architecture-v2.md#read-and-eligibility-model),
  [eligibility ADR](../../../../adr/ADR-0.8.25-eligibility-and-frozen-reads.md),
  [graph control](../../../../../src/rust/crates/fathomdb-engine/tests/slice60_graph_expand.rs).
- Select actual parser or conversion targets from query/filter, ID/cursor/frozen
  wire, and MCP/native argument admission. Prefer existing Proptest/Hypothesis
  for a bounded starting method. Preselect byte/depth, time/iteration limits
  (at most 60 s per target and 5 min per smoke), valid and invalid seed corpus,
  shrink/replay and real state checks for admitted mutations. Record the
  method actually run. Evaluate and document the cargo-fuzz/libFuzzer
  coverage-guided target/toolchain route even if the selected smoke uses only
  the existing property infrastructure. Actual coverage-guided execution needs
  a selected real target and applicable toolchain setup; label an unexecuted
  route as such rather than claiming coverage-guided proof.
  Queue simulation and parser examples do not establish product fuzz.
  [Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz.html).
- Start with `slice135_process_kill_before_and_after_acknowledged_write`,
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts` and
  `tc91_projection_commit_failure_survives_stop_and_reopen`. Select a missing
  failure boundary only when the existing composition cannot distinguish
  wrong state. Compare durable records, profile and index/work state plus an
  unrelated survivor. Assert short watchdogs only as hang detection. A process
  interruption establishes process-crash scope. Separately, the inherited
  live checkpoint investigation selects at most one changed checkpoint boundary
  and remains held until concrete point, actor, venue/session/resource and
  fault authority are established. Reuse the maintained healthy and actually
  delivered-fault controls, preserve the pre-reopen database and its WAL/SHM
  companions, then assert exact durable outcomes after reopening. This single
  authorized live boundary is distinct from the at-most-three missing CPU
  composition points and the sacrificial WAL-only doctor witnesses below.
  Preserve critical erasure regressions and existing recover flags.
  [CPU fixtures](../../../../../src/rust/crates/fathomdb-engine/tests/slice135_robustness_first.rs),
  [checkpoint operator](../../../../experiments/0.8.29-slice100-checkpoint/README.md),
  [SQLite anomaly testing](https://www.sqlite.org/testing.html#anomaly_testing).
- Derive the doctor matrix from `DoctorCommand` and `run_doctor` at entry.
  Classify database-free inspection, ordinary engine-open diagnostics,
  immutable `data-plane-integrity`, export artifacts, cache/network work and
  transactional `recompute-mean`. Check success, findings, refusal, busy,
  malformed input and repeated effects where applicable. For `check-integrity
  --full`, produce deterministic damage that still reaches the findings path.
  Require exact CLI exit 65, `E_CORRUPT_INTEGRITY_CHECK` and its help anchor.
  Keep healthy and early-open refusal separate. The existing AC-043c test can
  return on either open refusal or physical Clean. Map ignored CLI placeholders
  to actual replacement or retained coverage before any removal. Keep
  `recover` separately gated by `--accept-data-loss`, and use SDK inspection
  for application reads after CLI export/recovery.
  [CLI contract](../../../../interfaces/cli.md),
  [doctor dispatch](../../../../../src/rust/crates/fathomdb-cli/src/lib.rs),
  [current integrity test](../../../../../src/rust/crates/fathomdb-engine/tests/check_integrity.rs).
- For one composed WAL-only CLI case, establish a stable, controlled copy
  boundary after the selected commit and before any checkpoint can put it in
  the main file. Prepare two isolated copies from the same main bytes and
  boundary, one with its matching WAL and one main-only. Before invoking the
  CLI, both opens must succeed and expose an independently chosen control
  record. The selected committed record must be visible with WAL and absent
  main-only. Use sacrificial witness copies so a witness open cannot
  checkpoint the CLI input. Fail setup if this precondition is false. Verify
  the output digest/byte count where applicable and exact canonical, profile,
  lexical and vector facts after fresh-process reopen. Check SQLite locking,
  checkpoint and backup behavior against the actual fixture before design
  approval. [SQLite WAL](https://sqlite.org/wal.html), [SQLite backup](https://sqlite.org/backup.html).
- Reuse the fixture for thin identity, Unicode, temporal/filter, provenance,
  error and cursor checks. Test actual MCP budgets, epoch/continuation and
  structured/compact outputs. Exercise mutations and exact source-evidence
  resolution through the SDKs where MCP lacks those operations. Keep one deep
  engine suite. Select a few wrong-state or missing-cleanup variants to show
  oracle sensitivity. Document branch coverage or the limits of a manual map.
  [Quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).

## Review, implementation and closure

Direct Astra-medium reviews design and any contract deltas. Direct Astra-medium
critically reviews the updated plan with at most one FIX-1 and re-review.
Commit only after PASS and proceed only when commissioned. For each demonstrated
defect require an actual failing assertion in its own RED, then separate GREEN
and refactor. A passing characterization can establish an oracle without
manufacturing RED. Direct Sol6.2-high reviews code and direct Terra verifies the
affected real executions. Use scoped checks locally and the full required gate
once on a stable coherent source phase, coordinated with 70.

Give 70 the selected commands/features, independent expected facts, actual
outcomes, minimized inputs, omissions and concrete wrong behavior each oracle
rejects. Give 75 the installed CLI and binding subset. Preserve original
failures and use owner disposition for unavailable mandatory proof. Update the
affected contracts, status and authorized state once, integrate the owning
branch and clean only owned inactive scratch and worktrees after retaining
evidence. Planning carries no new fault, download, GPU, paid or publish authority.

## Commissioned reconciliation

The exact entry is `72abe1043b0735ecaf79cde35fda227753c2ca2e` after reviewed
partial50, superseding the draft entry above. The six selected rows are retained.
[Composed design](design.md) enumerates changes, concrete fixtures, bounded
selectors, independent oracles, prerequisites and review/verification order.
Already-correct characterization need not manufacture RED; each demonstrated
behavior defect requires its own genuine failing RED then separate GREEN.

Completed selected implementation and default clean-source gate; see [status](status.md)
for actual independent counts, artifact subjects and qualification limits.

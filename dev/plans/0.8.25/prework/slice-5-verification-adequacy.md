---
title: 0.8.25 Slice 5 — verification adequacy review
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 5 — verification adequacy review

## Verdict

The repository has broad real-database, lifecycle, release, platform, and
cross-SDK coverage, but it cannot yet prove the new 0.8.25 contracts. That is
expected before feature implementation. Three existing verification debts do
need prework decisions: trivial property-test scaffolds, release-worktree
Python provenance, and stale canonical traceability/fixture prose.

No acceptance criterion, test, CI workflow, or source changed in this slice.

## Requirement-to-criterion adequacy

The locked 0.6.0 chain has 23 concrete needs, 79 requirement headings, and 124
acceptance headings, plus later amendments. Its coarse traceability is useful
but not fully self-consistent: `traceability.md` names NEED-026 although
`needs.md` does not, and REQ-067/AC-077 remain placeholders despite the
completed retrieval program. The Slice 3 local R25/AC25 packages close the
0.8.25 planning gap without altering global identifiers before approval.

| Draft package | Criterion status | Test status / owning slice |
| --- | --- | --- |
| R25-10 | AC25-10 is falsifiable | New receipt-schema fixtures and a native witness in Slice 10; reuse experiment runner tests. |
| R25-15 | AC25-15 covers positive/negative/restart/parity | New identity/provenance property and migration tests in Slice 15; reuse `tc8_idspace_swap`, `provenance_mandatory`, and source-id parity tests. |
| R25-20 | AC25-20 covers reference/cycle/liveness | New dependency graph property, query, and source-removal tests in Slice 20. |
| R25-25 | AC25-25 covers atomic failure/retry/receipts | New transaction fault-injection and SDK/wire fixtures in Slice 25; reuse batch-write and provider/consolidation tests as baselines only. |
| R25-30 | AC25-30 covers lifecycle/crash/restart/orphans | New dependency closure matrix in Slice 30; reuse transition, erasure completeness, projection registry, and race tests. |
| R25-35 | AC25-35 covers snapshot races and eligibility | New multi-operation snapshot and pre-truncation tests in Slice 35; reuse `ReadView`, filter grammar/unification, validity, and reader-pool tests. |
| R25-40 | AC25-40 covers generation/restart/readiness | New generation identity tests in Slice 40; reuse projection status/runtime/drain/rebuild suites. |
| R25-45 | AC25-45 covers pages/races/current state | New cursor model/property and `operational_state` point/page tests in Slice 45; mutation-log `after_id` tests are only a partial baseline. |
| R25-50 | AC25-50 covers exact resolution/non-disclosure | New evidence-reference codec/property, visibility, erasure, and default-hit non-regression tests in Slice 50. |
| R25-55 | AC25-55 covers reciprocal trace/exclusion/faults | New lineage/inclusion/exclusion and integrity-fault tests in Slice 55; reuse explanation and orphan-provenance checks. |
| R25-60 | AC25-60 covers constraints/pages/path lifecycle | New combined-expansion matrix in Slice 60; reuse direct BFS and search-expand wrappers only as compatibility baselines. |
| R25-65 | AC25-65 covers replay/quality/efficiency/promotion | New preregistered retrieval-only harness runs in Slice 65; prior performance receipts remain evidence, not oracles. |
| R25-70 | AC25-70 covers temporal/multi-hop/default regression | New held-out treatment receipts in Slice 70; reuse TEMPORAL/GRAPH/REASON results as hypothesis evidence. |
| R25-75 | AC25-75 covers installed parity/concurrency/performance | Integrated installed-artifact and workload campaign in Slice 75; feature-local correctness cannot be postponed here. |

## Critical-path matrix

| Critical path | Existing useful coverage | Missing proof required by 0.8.25 |
| --- | --- | --- |
| Identity/provenance | Stable `IdSpace`, source-id-on-every-hit, mandatory provenance, restart/reindex pins | Immutable revisions, source versions, byte locators/hashes, corrupt/mismatch cases, all-SDK wire evolution |
| Atomic mutation | Real SQLite batch writes and transaction behavior | Cross-domain semantic batch rollback, idempotent replay, complete consequence/refusal receipt |
| Lifecycle/erasure | Transition races, registry-driven erasure, WAL/telemetry completeness, source and nested-projection removal | Registered dependency propagation, multi-source liveness, crash/resume fencing, no-active-orphan closure |
| Read/snapshot/filter | `ReadView`, validity, typed filters, injection safety, reader pool | One reproducible cross-operation snapshot, eligibility-before-every-truncation, membership/existence native plans |
| Projection readiness | Registry/status/drain/rebuild and cursor tests | Durable generation identity and mutation-to-generation-ready correlation across restart |
| Pagination/current state | Mutation-log `after_id`, bounded list reads | Opaque bound cursors, duplicate/omission races, expiry/drift/mismatch, `operational_state` reads |
| Evidence/explanation | Compact hits, source IDs, CE/branch, explanation, telemetry, orphan doctor | Exact evidence resolver, stale-reference non-disclosure, forward/back dependencies, exclusion and graph-path reasons |
| Graph/retrieval | Direct bounded BFS, search expansion, graph arm, FTS/vector/RRF/CE, registered experiments | Constraint-before-cap expansion, continuation/path evidence, accepted deterministic selection profiles |
| Concurrency/performance | Reader/writer stress, latency/scale receipts, CUDA and Windows/Tegra routes | New-contract overhead and contention, cold/steady distributions, evidence/page/lifecycle/rebuild costs |
| Packaging/parity | Governed surface tests, native release workflows, registry smoke contracts | Release-worktree-built Python proof and installed 0.8.25 cross-SDK/wire conformance |

## Existing verification debt proposals

| ID | Finding | Proposal | Destination |
| --- | --- | --- | --- |
| V25-01 | `fathomdb-engine/tests/property_template.rs` and `src/python/tests/test_property_template.py` assert only identity; schema's property checks only migration max. This conflicts with the repository's property-test rule. | Replace the trivial properties with real existing-contract invariants before feature work: typed record/filter codec or write/reopen identity in Engine/Python, and schema registry contiguity/monotonicity in schema. Preserve a visible RED before GREEN. | Slice 7, if HITL includes |
| V25-02 | The release worktree's `.venv` shim imports the primary checkout, and the agent runner may skip/rebuild based on venv ownership. | Add a tested wheel-build + fresh external-venv verification verb that proves module/native-library source and never editable-installs from a worktree. | Slice 7, P25-ENV-01 |
| V25-03 | Canonical acceptance prose contains many “fixture pending” statements even where `test-plan.md` and tests now exist; NEED-026 is absent and REQ-067/AC-077 are stale placeholders. | Perform a bounded traceability reconciliation with a failing checker/fixture; update only provably stale mappings and successor pointers. Do not rewrite all historical AC prose. | Slice 7 documentation/test infrastructure |
| V25-04 | `agent-verify` intentionally excludes long, live-model, all-feature, operator, GPU, Windows, and registry-installed bodies. | Keep the fast gate bounded. Each feature plan must name its additional execution route; Slice 75 verifies receipt presence, not silently substitutes fast CI. | Slices 10–75 |
| V25-05 | Experiment results are plentiful, but end-to-end answer metrics can be mistaken for Engine quality and some runs bypassed `Engine.search`. | Make classification executable and require retrieval-only receipts for data-plane claims. | Slice 10 |

## Test-method requirements for every feature slice

1. Commit or stage a failing test before implementation; do not edit the oracle
   during GREEN.
2. Use a real temporary database. Codec, cursor, dependency, projection,
   recovery, and round-trip layers require property tests.
3. Cover positive, negative, failure/rollback, close/reopen, concurrent mutation,
   and lifecycle/erasure behavior in proportion to risk.
4. Prove unsupported/unknown inputs fail typed and that default compact search
   behavior remains unchanged when an opt-in feature is absent.
5. Add Rust, Python, TypeScript, and wire fixtures in the owning public-surface
   slice. Slice 75 audits; it does not backfill missing parity.
6. Record exact execution identity, feature flags, skipped/excluded suites,
   environment, and receipts. A skip is visible evidence, not a pass.

## Slice 7 boundary

Only V25-01 through V25-03 are repository-preparation candidates. All
feature-specific tests are directly allocated in the v2 feature plan. No
0.8.25 product behavior is implemented by Slice 7.

## Evidence

- 199 Rust integration-test files, 113 Python test files, 49 TypeScript test
  files, and 285 top-level test/support files.
- `dev/{needs,requirements,acceptance,test-plan,traceability}.md`.
- `scripts/agent-test.sh`, Cargo required-feature declarations, Python skip
  policy, CI route inventory, property-test scaffolds, and existing
  identity/lifecycle/retrieval/projection suites.

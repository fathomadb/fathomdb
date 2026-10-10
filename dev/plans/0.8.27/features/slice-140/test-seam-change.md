---
title: FathomDB 0.8.27 Slice 140 — test-seam change record
status: IMPLEMENTED_PENDING_REVIEW
target_release: 0.8.27
source_baseline: 815aa83386342d33729de16f5131e577fe6cb6a3
---

# Test-seam change record

The [inventory](test-seam-inventory.json) records 75 formerly default-dev
public paths, their current declaration owners, previous effective gates,
and direct integration-test references. Nested mean helpers inherit their
module gate. The intended new gate is `any(test, feature = "test-hooks")`.
Existing feature-only hooks retain their existing gate. The module/type
negative witnesses transitively cover the nested `AccumulatorHandle` type
and reader-guard methods; the Removed changelog lists these explicit names
alongside their parent paths. The public
comparator tool and immutable baseline remain unchanged.

## Removed fixture routes

Raw SQL, fixture constructors/writes, conversion and state witnesses, failure
injection, search tuning, and deterministic reader/projection pauses now
require an explicit test feature for external consumers. Debug assertions
provide no access. Their private carriers, fixture-only execution branches,
callback slots, and invocation sites carry the same gate. Clock instrumentation
also disappears from production, while production wall-clock reads continue.

Integration targets declare `test-hooks` alongside any existing required
features. This includes transitive corpus/IR support-file consumers. Behavioral test assertions and test bodies remain intact. Historical Slice 90
ownership guards now expect the approved explicit-test cfg instead of their
old default/debug cfg, preserving owner, attribute, and mutation checks. Binding unit tests enable the engine
feature through dev dependencies; shipped dependency features remain separate.
The source-derived feature matrix was regenerated, and its coverage check
still accounts for every test target. Tests move between feature inventories;
they are not deleted or weakened.

## Preserved routes

Binding authentication preflight, BM25F, fusion, reranking, runtime diagnostics,
and operator behavior still compile through their existing production routes.
`Engine::_graph_frontier_stats_for_test` is a visible public path in the
immutable Slice 30 comparator. Its current callers are recall/evaluation
fixtures (`pr_c1_graph_seeding` and `pr_g0_phase2_frontier`). It remains an explicit visible-surface
exception: withdrawing it requires its own public interface/ADR decision.
Slice 140 does not silently change that visible contract.

## RED and focused GREEN receipts

At RED commit `2ea164f09`, the explicit-feature consumer compiled and the
independent default-dev consumer incorrectly compiled all recorded fixtures.
`scripts/tests/test_slice140_test_seam_boundary.py` failed for that concrete
exposure. GREEN compilation rejects each inventory witness at its individual
source line in both default dev and release consumers and compiles both with
explicit `test-hooks`. Checking diagnostic source lines avoids a name appearing
only in an unrelated compiler suggestion being accepted as absence evidence.

Focused checks passed on the implementation worktree:

- Isolated consumer boundary controls (dev/release, negative/positive).
- Default engine library and integration-test compilation.
- Engine library/integration compilation with `test-hooks,operator`, dev and
  release profiles.
- Engine unit tests with `test-hooks,operator`: 98 passed.
- Engine Clippy with `test-hooks,operator` and `-D warnings`.
- Source-derived test-target coverage and feature-matrix consistency.

The full `./scripts/agent-verify.sh` attempt fails at preflight because this
worktree has no checkout-owned `.venv/bin/python`. No shared environment was
borrowed or repointed. The prepared release checkout must run the full gate
on the reviewed integrated source. The completed Slice 117 handoff is
integrated at `f1d4571b1`. Public install and compatibility guidance targets
0.8.27 publication; internal status records that publication remains owned
by Slice 160 and has not occurred. The integrated external boundary and
engine compilation passed again. The [public comparison](public-surface-comparison.md) records the intended
reader-guard removal and inherited earlier-slice differences. Independent
code review passed on `18ef03da9`; the [hidden successor record](../hidden-surface/baseline-01f4063f-diff.md)
now records all completed before/after rows and exact retained test union.
The independent hidden-diff review returned APPROVED with no findings.
Terra verification and the prepared-checkout full gate remain pending. The targeted AC-050c checker passed at `10eb0bd44`:
64 removals all documented.

## Full-gate default-proof correction

The first integrated full gate exposed default proof coverage lost by
whole-target `test-hooks` gating. `slice20_dense_readiness` and
`slice25_registration_identity_inert` are default targets again. Slice 20's
pinned P1/P2 tests use public caller-embedder open and governed vector
configuration; P1 pauses/releases the embedder deterministically with an
unpause-on-drop guard. Raw vector and terminal assertions are preserved.
Its six fixture-only sibling cases remain explicit item-level `test-hooks`.
Slice 25 uses public open and a real raw SQLite negative-control writer.
Its structural-writer proof moves, with the same name and oracle, to the
engine's top-level unit test in `write.rs`. C1's evidence location follows
that proof without changing any obligation/category or contract digest.
The [final successor](../hidden-surface/baseline-f327c28c-diff.md) records
that test path relocation and restored default-target/item-feature
distribution. Its independent exact artifact review returned APPROVE with
no P0/P1/P2; all Rust API rows remain equal to the previous approved
successor.

Historical root, WAL, projection-status, and Windows ownership assertions
now distinguish the explicit fixture gate from always-on production reads.
The managed-opener scanner removes real `#[test]` function spans from its
production inventory. A real opener without that attribute, and comment or
string decoys, stay visible to the strict production check.

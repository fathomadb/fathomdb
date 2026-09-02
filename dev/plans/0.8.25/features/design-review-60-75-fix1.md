---
title: 0.8.25 Slices 60/65/70/75 design review FIX-1 resolution
date: 2026-09-01
cycle: FIX-1
status: RESOLVED_PENDING_REREVIEW
review_source: dev/plans/0.8.25/features/design-review-60-75.md
---

# Slices 60/65/70/75 FIX-1 resolution

## Scope

This cycle resolves only the five P1 and four P2 findings from the independent
review. It does not reopen program scope, promote a historical treatment, move
semantic policy into FathomDB, or advance any design beyond `DRAFT_REVIEW`.
Slice 7 activation and slice dependencies still gate READY.

## Resolution

| Finding | Resolution | Added proof |
| --- | --- | --- |
| DR-60-01 (P1) | Successful paging now requires complete path-set materialization below the 10,000-state ceiling. State 10,001 returns `GraphExpansionBoundExceeded` with no page/cursor. Only a complete globally sorted set can keyset-page. | Over-10,000 same-hop shuffled graphs fail closed; below-cap insertion permutations/reopen prove no duplicate, omission, or reorder. |
| DR-60-02 (P2) | The request and executor now separate global hard eligibility, seed eligibility, intermediate node/edge traversal constraints, and terminal/output predicates. A node failing only `target_kinds` remains traversable. | `A(X) -> B(Y) -> C(X)` with terminal `{X}` must reach C; explicit intermediate `{X}` must prune B. |
| DR-65-01 (P1) | The 0.8.25 registry has no default-capable state or alias. Omitted profile resolves directly to immutable compiled A0. Registry entries named/shadowing `default` or `A0` reject. A future default change requires a successor design with decision/receipt/compatibility references. | Byte-identical omitted-profile A0 plus schema/builder negatives for default entries, aliases, and shadowing. |
| DR-65-02 (P2) | Context-changing is mechanical: any possible/observed ordered identity, body, evidence identity, or evidence-order difference from A0. Such selectors can be `qualified_retrieval_only`, but become `qualified_answer` only after correctness, groundedness, and attribution no-regression. Requests declare intended use; answer use rejects retrieval-only profiles. | Every selector is classified; answer-use rejection and the narrow identity/order-preserving telemetry exemption are tested. |
| DR-70-01 (P1) | PPR now uses total integer mass `10^15`, checked `u128` intermediates, deterministic largest-remainder allocation, canonical node/edge accumulation, exact direction mapping, a fully stated 15% restart recurrence, full-iteration integer L1 convergence, and `1e-12` rank buckets. | Hand-computable mass vectors, dangling/restart cases, near ties, parallel/self edges, shuffled insertion, reopen, and required-platform digests. |
| DR-70-02 (P1) | Canonical BFS/frontier/edge ordering is fixed. Exceeding node, edge, path-state, depth, or candidate caps fails before diffusion with no partial graph/ranking. A truncated treatment requires a new separately qualified revision. | Independent over-cap fixtures under permutations/reopen prove no partial digest or scoring. |
| DR-70-03 (P2) | Paths are labelled exact witness paths, not score provenance. Optional contribution accounting reports the top eight final-iteration incoming transfers plus omitted, restart, and dangling mass, iteration/convergence state, and complete-subgraph digest. | Contribution totals reconcile; witness terminology and Slice 55 omission/degradation signals are asserted. |
| DR-75-01 (P1) | Closure is split. Pre-publication uses locally packed registry-equivalent artifacts built once per target/profile from one commit, with per-target hashes and clean Rust/Python/npm/CLI consumers. Actual registry installs are a separately authorized post-publication close gate and cannot authorize publication. | Source/editable leakage, wrong commit/target/profile/hash, package skew, clean local-registry crate consumption, wheel/npm/native isolation, and post-publish state separation. |
| DR-75-02 (P2) | The sealed manifest now binds fixture/mutation digests, fresh-process rules, exact operation mix, mutation cadence/trace, snapshot/page span, warm-up, timeout/no-retry, seed derivation, SQLite/cache settings, CPU affinity, sampling, and complete failure accounting. Correctness/parity/lifecycle are zero tolerance; performance is prelabelled accepted-policy or advisory. | Schema removes each repeatability field in turn; workload fixtures validate cold/steady cardinality, deterministic scheduling, sampler calibration, and that advisory cells cannot decide release. |

## Replacement records

- `fix1-60-75/slice-60-design.md`
- `fix1-60-75/slice-65-design.md`
- `fix1-60-75/slice-70-design.md`
- `fix1-60-75/slice-75-design.md`

All four remain `DRAFT_REVIEW`. Re-review must confirm the nine corrections and
their RED/GREEN mappings. Any unresolved P1/P2 finding continues to block READY.

---
title: FathomDB 0.8.26 Slice 46 — design lifecycle inventory
status: REVIEWED
---

# Slice 46 design lifecycle inventory

## Coverage result

The exact catalog covers all 186 `dev/design/**/*.md` paths after adding the
missing actuation owner. No path was deleted, moved, renamed, or archived.

| Class | Count | Review disposition |
| --- | ---: | --- |
| maintained | 24 | Semantically reviewed against current ADR, interface, implementation, test, or bounded-owner authority. |
| reference | 26 | Retained as bounded input; not promoted to current product behavior. |
| experiment | 15 | Retained as evidence with its recorded outcome or scope. |
| historical | 93 | Preserved in place; no wholesale prose rewrite. |
| proposal | 22 | Kept allocated to future decision/work; not treated as shipped. |
| deferred | 2 | Kept explicit and outside the 0.8.26 implementation boundary. |
| superseded | 4 | Kept with an existing successor that is also the current owner. |

The baseline had 185 Markdown documents. The sole count increase is
`dev/design/actuation.md`; it closes a current-owner gap rather than duplicating
a historical Slice 25/35/40 record.

## Maintained-owner review

| Owner | Current authority or bounded scope | Implementation/test witness | Result |
| --- | --- | --- | --- |
| `README.md` | Lifecycle catalog navigation only; ADRs/interfaces remain higher authority. | `check-design-lifecycle.py`; focused fixture suite | Replaced the stale manual-current list with catalog-backed rules. |
| `actuation.md` | Breaking V1 boundary ADR and Rust/Python/TypeScript interfaces. | `actuation.rs`; Slice 25/35 actuation suites | Added the current batch, endpoint, receipt, replay, lifecycle, erasure, and projection design. |
| `bindings.md` | The three binding interface documents and governed SDK surface. | Rust/Python/TypeScript binding sources and Slice 35 conformance | Added the missing 0.8.26 cross-binding boundaries. |
| `embedder.md` | Embedder identity ADRs and dual-runtime device ADR. | embedder crates; `verify_embedder.rs` | Retained as the bounded embedder owner; no 0.8.26 correction needed. |
| `engine.md` | Breaking fresh-database ADR and Rust open interface. | engine `lib.rs`; `slice40_fresh_database_cutover.rs` | Corrected public open to fresh schema 34 only. |
| `errors.md` | Rust/Python/TypeScript error contracts. | engine/binding error conversion; `error_taxonomy.rs` | Redirected current semantics to maintained topic owners. |
| `fathomdb-data-plane-architecture-v2.md` | Accepted ADR/index hierarchy and maintained interfaces. | architecture authority checker; implementation seams in its profile | Retained active v2.2 plus the committed A25-04 deferral correction. |
| `gpu-eval-activities-policy.md` | Internal eval allocation only; product runtime is dual-runtime-ADR-owned. | CUDA preflight scripts and witness validators | Marked the still-applicable bounded policy active. |
| `lifecycle.md` | Lifecycle event contract and interface observability boundary. | `lifecycle.rs`; lifecycle observability/reliability suites | Retained current owner; no missing 0.8.26 fact. |
| `migrations.md` | Fresh-database ADR plus schema authorship/accretion rules. | schema `lib.rs`; `slice40_fresh_database_cutover.rs` | Bounded historical migration execution away from public open. |
| `nested-source-projections.md` | Accepted nested-source projection ADR. | projection engine path; `slice45_nested_source_projections.rs` | Corrected stale candidate-branch text to landed PR #195. |
| `op-store.md` | Rust operation-store/read interfaces. | engine op-store path; `op_store.rs` | Retained current operation-store owner; no 0.8.26 delta. |
| `orchestration.md` | Cross-release execution method only, subordinate to `AGENTS.md`. | `preflight.sh`; release-state and commission checks | Retained method without a release-local rewrite. |
| `perf-gates.md` | Accepted performance ADRs and release-gate policy. | `perf_gates*.rs`; recall predicate tests | Marked the current two-tier measurement owner active; no new claim. |
| `perf-regression-detection.md` | Append-only performance-history contract. | `perf-regression-check.rs`; CLI regression tests | Marked the landed/current detector active. |
| `pinned-override-rot-guard.md` | Governed pin/exception policy. | `check-pinned-override-rot.py`; shell wrapper and fixture tests | Replaced proposal language with implemented active-guard truth. |
| `projections.md` | Projection interfaces and generation/readiness decisions. | engine projection runtime/generation paths; Slice 40 generation suites | Added connection, commit-gate, generation, receipt, and derived-edge facts. |
| `recovery-0.8.25.md` | Current repair-authority successor under recovery ADRs. | engine recovery methods; rebuild/erasure suites | Retained; immutable inspection does not broaden repair authority. |
| `recovery.md` | CLI interface and accepted immutable-inspection boundary. | CLI/engine doctor paths; Slice 30/55 integrity suites | Added `data-plane-integrity` and its SDK exclusion. |
| `release.md` | Release ADRs and publication contracts. | `release.yml`; release smoke scripts | Separated Slice 50 candidate evidence from publish/post-publish smoke. |
| `retrieval-result-limits.md` | Accepted result-limit behavior and retrieval interfaces. | engine retrieval path; `slice18_retrieval_result_limits.rs` | Retained bounded result-limit owner unchanged. |
| `retrieval.md` | Retrieval interfaces and exact graph-evidence ADR. | frozen/evidence/graph engine paths; Slice 10/20 suites | Added finalizer, resolution, nondisclosure, and no-ranking-fiction facts. |
| `scheduler.md` | Projection scheduling/backpressure boundary. | engine scheduler/runtime paths; projection race/reliability suites | Added connection, commit-gate, generation, and common queue ownership. |
| `vector.md` | Filter ADR, schema-owned tables, and projection/retrieval/recovery boundaries. | schema/engine vector paths; quantization, snapshot, erasure, and rebuild suites | Replaced the self-declared stub with a compact current vector design. |

## Draft, deferred, and superseded allocations

The 22 proposal records remain proposals: the 0.9.0 readability proposal; the
three 1–2M scaling records; the CI/CD hypothesis; the five-document
FathomDB/Memex roadmap set; memory tiers/expiry; the temporary serial Rust gate;
and the worktree consolidation/consolidator requirements, design, acceptance,
execution-status, and retirement-proof sets. None is silently promoted by
target release or front-matter status.

The two deferred records are `0.8.x-after-0.8.25-design-notes.md` and
`ann-index-vec0.md`. The four superseded records are the 0.8.20 B5 verification
design, architecture v1, the old GPU allocation policy, and the 0.8.x
planner/router sequencing note; each catalog entry names its verified current
successor.

## Plan decision

The drafted Slice 46 purpose is approved with the scope adjustment recorded in
`plan.md`: exact mechanical coverage for the whole tree, semantic maintenance
for the 24 current owners, and preservation/classification rather than rewriting
for all other records. This is complete for the assigned documentation
convergence and does not absorb Slice 50 release verification or deferred
product work.

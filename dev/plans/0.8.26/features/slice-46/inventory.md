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

| Owner | Result |
| --- | --- |
| `README.md` | Replaced the stale June manual-current list with catalog-backed navigation and lifecycle rules. |
| `actuation.md` | Added the current five-operation atomic batch, prospective endpoint, receipt, replay, lifecycle, erasure, and projection design. |
| `bindings.md` | Added cross-binding 0.8.26 actuation, graph-evidence, fresh-schema, and CLI-only integrity boundaries. |
| `embedder.md` | Reviewed; current bounded embedder owner, with no Slice 46 change needed. |
| `engine.md` | Corrected public open to fresh schema 34 only and bounded the older migration sequence as historical. |
| `errors.md` | Redirected current actuation, projection-generation, frozen-read, evidence, and graph-expansion semantics to maintained owners. |
| `fathomdb-data-plane-architecture-v2.md` | Reviewed as active v2.2; retained the committed A25-04 post-0.8.26 deferral correction. |
| `gpu-eval-activities-policy.md` | Reviewed as a standing internal policy and marked active; product runtime policy remains ADR-owned. |
| `lifecycle.md` | Reviewed; current observability/lifecycle owner, with no missing 0.8.26 fact. |
| `migrations.md` | Distinguished public fresh-only admission from retained migration authorship/bootstrap mechanics. |
| `nested-source-projections.md` | Corrected the stale candidate-branch claim to the landed PR #195 state. |
| `op-store.md` | Reviewed as the current operation-store owner; no 0.8.26 delta belongs here. |
| `orchestration.md` | Reviewed as the current cross-release method; no release-local rewrite. |
| `perf-gates.md` | Reviewed as the current two-tier performance-gate owner and marked active; no new performance claim. |
| `perf-regression-detection.md` | Reviewed as landed/current and marked active; append-only evidence rules unchanged. |
| `pinned-override-rot-guard.md` | Corrected proposal language and marked the already-implemented guard active. |
| `projections.md` | Added worker-connection, commit-gate, generation, receipt-correlation, and derived-edge scheduling facts. |
| `recovery-0.8.25.md` | Reviewed as the current recovery successor; immutable 0.8.26 inspection does not alter repair authority. |
| `recovery.md` | Added the bounded immutable `data-plane-integrity` doctor route and SDK exclusion. |
| `release.md` | Separated Slice 50 non-publishing candidate verification from publish and post-publish smoke. |
| `retrieval-result-limits.md` | Reviewed as the bounded result-limit owner; no Slice 46 change needed. |
| `retrieval.md` | Added frozen-finalizer and exact graph-evidence resolution/nondisclosure/no-ranking-fiction facts. |
| `scheduler.md` | Added worker connection, commit-gate, generation revalidation, and common derived-edge queue ownership. |
| `vector.md` | Reviewed as the current vector-store owner; no 0.8.26 delta belongs here. |

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

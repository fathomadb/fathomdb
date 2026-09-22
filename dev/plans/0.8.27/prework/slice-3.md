---
title: FathomDB 0.8.27 prework Slice 3 - draft contracts and allocation
status: ACTIVE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 3 - draft contracts and architecture allocation

## Plan and delta reconciliation

Outcome: draft release-local needs, requirements, acceptance criteria, semantic
ownership, and later-slice allocation without changing a public interface or
the locked global acceptance register.

Since the 2026-09-02 intake, F27-01 has a stable Memex characterization and a
narrow Memex cutover exemption; the defect still blocks 0.8.27 publication but
not Memex 0.6.0 by itself. The refactor baseline is now schema 34 and includes
the semantic modules and cross-SDK parity work shipped in 0.8.25/0.8.26. The
draft is approved only after correcting report-count and retained-proof
assumptions described below.

## Needs, requirements, and acceptance criteria

Needs:

- **N27-01:** a supported correction/supersession must not make lawful source
  deletion impossible.
- **N27-02:** maintainers and coding agents must be able to change the five
  selected monolithic engine, binding, and SDK facades by semantic domain
  without changing public behavior.

| ID | Requirement | Acceptance criteria and owner |
| --- | --- | --- |
| R27-01 | `erase_source(bucket)` handles a bucket containing superseded revisions and already-complete direct dependency closure in same-bucket and both cross-bucket erase orders supported by the one-source model. | AC27-01 same-bucket; AC27-02 original-first; AC27-03 replacement-first. Slice 20. |
| R27-02 | Erasure remains truthful, fail-loud, idempotent, and complete at rest. Pre-commit blockers roll back. A post-commit scrub failure uses `ErasureIncomplete`, preserves the committed row deletion and durable retry obligation, and never falsely completes. | AC27-04 pre-commit rollback plus post-commit scrub-failure truth, retry, and idempotence; AC27-05 independent-reopen payload/live-authority absence after successful closure plus exact survivor and exact audit/closure identity exceptions. Slice 20. |
| R27-03 | No new public verb, report field, schema migration, or private consumer workaround is introduced. Requested-bucket `ExciseReport` count semantics remain unchanged unless separately ruled. | AC27-06 exact Memex characterization flips to supported success without relaxed setup; interface diff remains limited to clarified behavior. Slice 20. |
| R27-04 | Semantic decomposition preserves names, signatures, import/export paths, feature gates, error/wire encodings, ordering, locks, and package roots. | AC27-07 real-surface comparator rejects added/removed/changed Rust exports/re-exports, Python registration/stubs, Node declarations, and package exports. Slice 30; consumed by Slices 40-130. |
| R27-05 | Engine modules remain private inside the existing engine crate; root `Engine`, root re-exports, and the single-writer model remain. | AC27-08 focused owner suites, applicable feature routes, and candidate equivalence pass per structural slice and at Slice 150. |

`dev/acceptance.md` remains locked. AC27 identifiers are release-local until a
separate governed acceptance decision authorizes global changes.

## Draft architecture allocation

The vocabulary is conceptual, not a requirement for identical language trees:

- engine root remains the facade/`Engine`; root-owned types/errors become
  private foundation modules;
- write/ingest owns transaction, validation, provenance, extraction,
  consolidation, and actuation helpers;
- read/search owns filtering, reader pool, frozen/page reads, ranking, and
  query execution;
- graph/evidence extends existing `graph_expand.rs`, `dependency_trace.rs`, and
  `evidence.rs` rather than creating parallel implementations;
- projection/lifecycle extends existing `projection_generation.rs`,
  `lifecycle.rs`, `dependency_closure.rs`, `actuation.rs`, and
  `data_plane_integrity.rs`; and
- open/config/runtime/operator remains a late extraction after domains settle.

PyO3 and NAPI keep one registration root and separate conversion/DTO, engine
method, write/provenance, graph/evidence, and embedding/reranking concerns only
where their existing dependencies support it. TypeScript extends
`binding.ts`, `errors.ts`, `evidence-validation.ts`, `platform.ts`, `read.ts`,
and `validation.ts`; it must break the current `read.ts` -> `index.ts` type
dependency before inventing shared types. Python extends `graph.py`, `read.py`,
`types.py`, `errors.py`, `admin.py`, and `config.py`, consolidating duplicated
graph/evidence codecs while keeping the `Engine` facade stable.

Interface updates belong to Slice 20 only if F27 behavior needs clarification.
Refactor design and current-owner navigation converge in Slice 140. No accepted
ADR is changed by this prework draft.

## Implementation, review, verification, and status

Implementation is this release-local contract draft. Behavioral TDD and code
review are not applicable. A read-only code-grounded audit checked the current
engine and binding surfaces; package-level design review remains pending.

Status remains `REVIEW_PENDING`. No product, interface, ADR, acceptance file,
or workspace was changed. Next: Slice 4 code alignment.

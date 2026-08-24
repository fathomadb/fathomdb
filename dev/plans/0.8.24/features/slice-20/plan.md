---
title: 0.8.24 Slice 20 — benchmark-directed engine performance draft plan
status: READY
target_release: 0.8.24
---

# Slice 20 — benchmark-directed engine performance

## Planning boundary

This document plans Slice 20; it neither copies the performance branch nor
changes the engine. The retained SCALE-02 result is already the owner-approved
decision basis. Slice prep reviews the complete production delta and writes a
current-main design. It does **not** rerun or “confirm” the benchmark.

## Goal and outcome

Integrate the approved streamed BM25 boundary-tie completion into 0.8.24 if,
after a complete code-grounded review, the final delta remains compatible with
current main and its exact-result constraints. The slice must:

- preserve exact top-100 candidate and public top-10 ordering relative to the
  forced full-sort control;
- keep filters, edge-bearing databases, ineligible queries, and conversion or
  statement failures on a correct fallback path;
- retain shipped reader/cache/mmap/temp-store defaults;
- preserve public limit-prefix stability and existing retrieval contracts; and
- add no performance claim beyond the retained SCALE-02 decision record.

## Authority and inputs

- P24-11, R24-5, A24-6, and the Slice 6 approval.
- `dev/plans/0.8.24/prework/benchmark-evidence-index.md`.
- Branch `experiments/performance-0.8.23-plan-20260821`, result commit lineage
  through `9e507553` and production promotion `c7e83bfe`.
- Retained result
  `dev/performance-benchmarking/2026-08-22-scale-02-rank-boundary-result.md`
  and implementation note on that branch.
- Current `origin/main`, `dev/design/retrieval.md`, `dev/architecture.md`,
  REQ-010/AC-076, and current engine/tests.
- Memory `dont-price-a-confirming-run-for-a-settled-decision.md` and
  `perf-tuning-design-sweeps-not-adhoc.md`.

The whole experiment branch is not the integration unit. It contains thousands
of lines of unrelated program records and experiment infrastructure. The
reviewed final production behavior, its minimal retained provenance, and its
tests are the only candidate scope.

## Scope

### In scope

- Review the complete final engine/test delta against execution-time current
  main, including all prerequisites introduced before `c7e83bfe`.
- Write an implementation design that distinguishes existing current-main code
  from net-new behavior.
- Reconstruct the bounded production change under TDD on a fresh current-main
  worktree; do not opportunistically cherry-pick the entire experiment branch.
- Retain the owner-selected result and exact correctness/resource constraints.
- Update retrieval design or public behavior documentation only if the reviewed
  change makes an existing statement stale.

### Non-goals

- No new benchmark, reduced-N confirmation, mmap/cache tuning, general query
  rewrite, or new performance threshold.
- No import of unrelated experiment campaign docs, ledgers, runner programs,
  or external content-bearing artifacts.
- No change to hybrid/vector semantics, SDK shape, schema, or public score
  interpretation unless a reviewed prerequisite proves unavoidable and HITL
  explicitly expands scope.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, current-main SHA, retained evidence/provenance inventory;
- `draft-contracts.md` — slice-local need/requirement/acceptance drafts;
- `design.md` — current-main implementation and test design; and
- `research.md` — focused primary-source/code questions and conclusions.

### Prep tasks

1. Resolve the full commit graph and compare the final production files against
   current `origin/main`. Record exactly which earlier commits are semantic
   prerequisites to `c7e83bfe`.
2. Read the retained result and implementation note in full; verify their
   candidate SHA, artifact-manifest digest, 60-repetition design, owner ruling,
   and no-rerun clause.
3. Restate R24-5 and existing REQ-010/AC-076 without inventing a new general
   performance promise.
4. Propose slice-local drafts for review:
   - **N20-DRAFT:** direct FTS queries need bounded rank-boundary work that
     preserves exact ordered results at the approved operating envelope;
   - **R20-DRAFT-1:** eligible direct-text searches finish only the BM25 group
     crossing the 100-candidate boundary and restore stable cursor order;
   - **R20-DRAFT-2:** every ineligible or failed streamed path retains the exact
     existing full-sort behavior;
   - **AC20-DRAFT:** strict boundaries, ties beyond 100, all-equal scores,
     generated tie groups, conversion/statement failure, filters/edges, query
     plan, and public limit-prefix tests match the forced full-sort control.
5. Read retrieval architecture and the actual engine path. Write an
   exists-versus-net-new map before drafting the design.
6. Read every proposed test body in full. Do not infer fidelity from a test
   filename or import branch-generated expected values without review.

## Draft design and design review

### Required design content

The initial design must specify:

- eligibility predicate for the streamed path;
- ordered SQLite scan and the 100-candidate boundary;
- exact tie-group completion and stable `(BM25 score, write_cursor)` ordering;
- truncation and public-limit behavior;
- statement/row-conversion fallback semantics;
- filter, edge, and other ineligible-path preservation;
- test-only forced-full-sort control and production seam boundaries;
- no mmap/cache/temp-store/default changes; and
- telemetry or witness exposure, if any, kept private/test-only unless a
  separately approved public contract exists.

### Challenges and research plan

- Inspect SQLite's official FTS5 `bm25()` ordering and rank-column behavior to
  ensure the design does not assume an undocumented tie order.
- Inspect rusqlite iterator/error behavior for statement and row-conversion
  failures; the fallback must not duplicate, omit, or reorder results.
- Verify SQLite version/query-plan assumptions against the project's bundled
  version and current code, not a generic online example.
- Use primary SQLite and Rust/rusqlite documentation only for unresolved
  semantics. No competitive benchmark research is needed; the performance
  choice is already settled.

### Architectural-fit review and revision

Review the design against `dev/design/retrieval.md`, the current direct-FTS
prefix contract, the engine code, and the retained result. Remove experiment-
only surfaces, reconcile current-main drift, and explicitly list any public
behavior or design text that must change. A contradiction with accepted ADRs
halts for a successor decision; it is not silently resolved in code.

## Planned implementation sequence after prep approval

1. Cut a fresh worktree from current `origin/main`; serialize engine edits.
2. Commit meaningful failing tests first for the minimal production behavior.
   The retained branch supplies human-approved intent, but the integration
   still needs a demonstrable RED against current main.
3. Implement the minimal streamed-boundary path and fallback.
4. Refactor only after exact correctness tests are GREEN.
5. Update retrieval design, relevant changelog/release notes, and DOC-INDEX if
   the public behavior description changes.
6. Review the actual final diff against both current main and the retained
   production behavior; do not merge unrelated experiment files.

## Verification and evidence

Minimum targeted proof:

- `scale02_fts_rank_fast` production routing, exact equivalence, ties, and
  failure fallback;
- generated boundary-group property coverage;
- `slice23_text_limit_prefix_stability`;
- applicable filter/edge/direct-FTS engine tests;
- Rust format, targeted clippy/check/test; and
- full workspace clippy/check plus the normal agent verification before a
  green or slice-complete claim.

The retained SCALE-02 receipt remains the performance decision evidence. Test
execution after integration is a correctness gate, not another performance
experiment. No hosted CI or benchmark rerun is automatically required.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| Partial cherry-pick omits a prerequisite | Review the complete final delta and reconstruct the minimal current-main behavior. |
| A tie-order assumption changes results | Force equivalence across strict, crossing, all-equal, and generated tie groups. |
| Fallback hides conversion/statement errors incorrectly | Specify and test exact fallback entry, output, and error behavior. |
| Experiment-only controls leak into production | Keep controls private/test-only and review public interfaces. |
| Scope imports the entire benchmark program | Allowlist production engine, focused tests, minimal evidence/design docs only. |
| Integration no longer matches retained result | Halt for HITL; do not rerun to manufacture a new decision basis. |

The engine change is reversible by a normal follow-up commit restoring the
prior full-sort path. No history rewrite or destructive database operation is
part of the slice.

## Prerequisites and reviewer decisions

Before ready status, the reviewer must approve:

1. the exact minimal file/delta allowlist;
2. the current-main exists-versus-net-new design;
3. the RED tests and forced-control fidelity method;
4. the no-rerun interpretation of the retained decision; and
5. routing for any newly discovered prerequisite. A schema, SDK, or broad
   retrieval change requires a separate HITL-approved follow-on, not silent
   inclusion.

## Definition of done

Slice 20 closes only when the complete current-main delta has been reviewed,
the bounded production behavior is implemented under TDD, exact-result and
fallback tests pass, full workspace checks are green, retained evidence is
linked without a confirming benchmark, and no unrelated experiment program
material enters the release.

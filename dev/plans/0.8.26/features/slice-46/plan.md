---
title: FathomDB 0.8.26 Slice 46 — technical design documentation convergence
status: APPROVED
---

# Slice 46 plan — technical design documentation convergence

## Draft reconciliation and plan disposition

The draft was written at `7a5682796bc446ac38e5053cff5a18e9d33e9c00`.
The following material changes have landed on `release/0.8.26` since then:

1. Slice 9 activated the 0.8.26 release authority and corrected the bounded
   preparation items selected at `seq-286`/`seq-289`.
2. Slice 10 completed frozen-explanation correlation and public frozen-evidence
   guidance.
3. Slices 15 and 20 selected and implemented the opt-in graph-evidence V1
   sidecar, immutable target/terminal-edge resolution, nondisclosure, and
   cross-binding contract under accepted
   `ADR-0.8.26-exact-graph-artifact-evidence.md`.
4. Slice 30 qualified immutable, bounded, out-of-process
   `data-plane-integrity` inspection and its operator contract.
5. Slices 35 and 40 implemented the accepted breaking V1 actuation and fresh
   schema-34 database boundary, including prospective endpoint validation,
   compact receipt integrity, and cross-binding conformance.
6. Slice 45 made `fathomdb-data-plane-architecture-v2.md` v2.2 the sole active
   architecture, retained the 0.6.0 architecture as explicitly superseded,
   and added a recurrence guard. Its inventory deliberately left detailed
   design lifecycle and classification here.
7. The accepted interface and ADR owners now contain the 0.8.26 contract.
   The release-local Slice 10–40 plans/designs/statuses are execution records,
   not a replacement current topic-design hierarchy.
8. `dev/design/README.md` still carries a June manual list, while
   `dev/doc-index/design.md` and `dev/DOC-INDEX.md` combine or omit many of the
   185 Markdown files below `dev/design/`. Old target-release metadata alone
   therefore cannot identify current authority.
9. Follow-up `636d96eb` corrected A25-04's stale 0.8.26-successor allocation to
   explicit post-0.8.26 deferral. It is accepted as Slice 46 input because
   0.8.26 shipped only one-source dependency semantics, not the multi-source
   liveness grammar.
10. Slice 2 finding C26-07 allocates superseded-design successor navigation to
    this slice; include it through the catalog and focused banners. The
    `seq-281` evidence-preservation boundary rejects deletion, moves, archival,
    and historical prose rewrites. P26-09 and all integrated package/platform
    evidence stay with Slice 50.

**Disposition:** approve the purpose, lifecycle model, and no-delete default;
adjust the execution scope. Every `dev/design/**/*.md` path receives exact
lifecycle coverage, but semantic code/ADR/interface reconciliation is limited
to maintained owners and the 0.8.26 data-plane delta. Historical slice records
receive classification and successor navigation, not wholesale prose edits.
No bulk move, rename, archive, deletion, product change, or new architectural
decision is approved.

## Purpose and placement

Per placement ruling `seq-288`, make the existing technical design
documentation current, well organized, and correct after Slice 45 establishes
the architecture hierarchy and before Slice 50 verifies the integrated
release. This slice owns design-document lifecycle and navigation, not product
redesign or historical rewriting.

## Slice-complete workflow

1. Enumerate changes since this draft, Slice 45's architecture inventory,
   completed Slice 9–40 work, assigned functions, allocated draft items,
   accepted ADRs/interfaces, and the as-built implementation and tests.
2. Inventory every `dev/design/**/*.md` path in the machine-checked
   `dev/design/document-lifecycle.json` catalog. Record class, topic, role,
   owner, release relevance, and successor where applicable. Keep every
   reviewed file at its current path.
3. Evaluate the inventory and update/approve/reject/narrow this plan. Complete
   slice-local needs, requirements, and acceptance criteria without turning
   the slice into a historical-document rewrite.
4. Update the technical-documentation design, lifecycle catalog, navigation,
   successor pointers, and the exact maintained topic owners listed below.
   Obtain review from an independent read-only design-review subagent and
   resolve findings before implementation.
5. Use TDD RED/GREEN for any index, link, or documentation validator changes
   and obtain independent code review. Use explicit source-of-truth comparison
   for mechanical prose/metadata corrections.
6. Use an independent agent to verify organization, correctness, links,
   authority, Markdown, and the affected documentation build. Run
   `./scripts/agent-verify.sh` before completion; add broader matrices only when
   tooling changes justify them.
7. Write `status.md` with the disposition matrix, changed current documents,
   preserved historical records, reviews, checks, and unresolved findings.
   Merge and clean up any temporary branch/worktree used by the slice.

## Requirements and acceptance

### Need

**N26-06:** A maintainer can start from the active architecture and determine
which technical design is current for a topic without treating a historical
slice memo, experiment, or superseded proposal as present implementation
guidance.

### Requirements and criteria

| Requirement | Acceptance criterion |
| --- | --- |
| R26-46A: exact lifecycle coverage | AC26-46A: every tracked `dev/design/**/*.md` file is cataloged exactly once with valid class/topic/role/owner/release fields and an existing successor when one is declared. |
| R26-46B: current design authority | AC26-46B: `dev/design/README.md`, `dev/DOC-INDEX.md`, and `dev/doc-index/design.md` identify the active architecture and maintained technical-design owners without stale manual-current claims or duplicate current ownership. |
| R26-46C: maintained-owner truth | AC26-46C: every catalog entry classified `maintained` is semantically reviewed against its complete current authority or is narrowed by an explicit successor/reference boundary; no target-release/status heuristic substitutes for that review. |
| R26-46D: 0.8.26 design truth | AC26-46D: the owners in the topic matrix agree with accepted ADRs/interfaces and verified Slice 10/20/30/35/40 behavior for frozen/graph evidence, integrity inspection, changed-in-place actuation and persistence, lifecycle/erasure effects, bindings, schema-34 fresh open, performance, release, and explicit deferrals. |
| R26-46E: historical preservation | AC26-46E: historical, experimental, reference, proposal, deferred, and superseded records stay in place; superseded authorities have valid successor navigation; no unique rationale or evidence is deleted. |
| R26-46F: recurrence prevention | AC26-46F: a focused RED fixture proves missing/extra/duplicate paths, invalid fields/classes, absent owners/successors, superseded-without-successor, duplicate maintained topic/role ownership, or missing local/CI wiring fail; GREEN makes every arm pass. |
| R26-46G: proportional closure | AC26-46G: focused lifecycle, link, Markdown, and docs-build checks plus `agent-verify` pass; no product/ADR/interface/package/publication change and Slice 50 remains the integrated artifact/platform owner. |

### Topic-owner disposition

| Topic | Current owner(s) after Slice 46 | Disposition |
| --- | --- | --- |
| architecture | `fathomdb-data-plane-architecture-v2.md` | Review unchanged except committed A25-04 allocation correction. |
| engine open and schema admission | `engine.md`; `migrations.md` | Amend stale automatic-migration wording with the schema-34 fresh-only public boundary and internal-bootstrap distinction. |
| frozen and graph evidence | `retrieval.md` | Add current completion, sidecar, point-resolution, authority, nondisclosure, and no-ranking-fiction design beneath the accepted interfaces/ADR. |
| operator integrity and recovery | `recovery.md`; `recovery-0.8.25.md` | Add `data-plane-integrity` to the canonical doctor inventory and distinguish immutable inspection from recovery. |
| actuation, receipt persistence, lifecycle/erasure effects | new `actuation.md` | Distill the current five-operation V1 transaction, prospective validation, receipt integrity/replay, projection, supersession, and erasure design; historical slice designs remain evidence. |
| projections and scheduler | `projections.md`; `scheduler.md` | Review the derived-edge path and update only where current ordering/effect facts are missing. |
| bindings and errors | `bindings.md`; `errors.md` | Preserve their current-owner role despite old target metadata; add only missing 0.8.26 cross-binding/error rules. |
| vector, embedder, op-store, lifecycle observability | `vector.md`; `embedder.md`; `op-store.md`; `lifecycle.md` | Semantic review; retain or narrowly bound unchanged when 0.8.26 introduces no owned delta. |
| performance | `perf-gates.md`; `perf-regression-detection.md` | Review as current measurement owners; no new performance claim. |
| release | `release.md` | Preserve publish mechanics and point integrated non-publishing evidence to Slice 50. |
| cross-release method/policies | `orchestration.md`; `pinned-override-rot-guard.md`; `gpu-eval-activities-policy.md` | Review current applicability and catalog as maintained policy/method owners, not 0.8.26 feature designs. |
| nested projections and result limits | `nested-source-projections.md`; `retrieval-result-limits.md` | Review as bounded current owners; do not fold them into the 0.8.26 delta. |

## Implementation and TDD sequence

1. **RED:** add focused lifecycle-catalog checker tests for exact tree coverage,
   duplicate entries, invalid classes, absent owners/successors, and missing
   local/CI wiring. Commit the failing oracle before the checker/catalog.
2. **GREEN:** add the smallest checker and complete catalog; replace the stale
   manual-current list with catalog-backed navigation; add `actuation.md` and
   only the necessary topic-owner corrections and successor pointers.
3. Compare every maintained owner with its complete current authority. For the
   0.8.26 delta, verify the accepted ADR/interface, implementation seam, and
   focused Slice 10–40 tests. Correct documentation, not product code.
4. Run the focused checker/tests, Markdown/link/docs checks, and independent
   code review. Resolve findings with additional RED/GREEN only if behavior in
   the checker changes.
5. Use an independent verification subagent, run `agent-verify`, write the
   status/review evidence, and advance the release state to Slice 50.

## Stop gates

Stop and return to the owning slice or HITL on a product/design contradiction,
unclear canonical owner, required public-contract or ADR change, destructive
loss of historical evidence, broad path migration, or implementation work.

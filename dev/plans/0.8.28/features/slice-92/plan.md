---
title: FathomDB 0.8.28 Slice 92 — documentation audit and convergence
status: PLANNED
target_release: 0.8.28
planning_baseline: 896ff59e96caa3b98c98e4b246c407b088dbaf28
planned_on: 2026-11-04
---

# Slice 92 — documentation audit and convergence

Owner placement on 2026-11-04 (ledger seq-365): after completed Slice 90 and before Slice 95,
audit and update user, contributor and project documentation for the as-built
0.8.28 release. This plan defines the audit and correction work; it does not
claim that the documentation has already been reconciled. Slice 95 retains CI
repairs, assembled final qualification and non-publishing release rehearsal.

## Historical review and initial reconciliation

| Reviewed release work | Lesson applied here |
| --- | --- |
| [0.8.26 Slice 45](../../../0.8.26/features/slice-45/plan.md) established one active architecture; [Slice 46](../../../0.8.26/features/slice-46/plan.md) classified designs and repaired navigation. | Update current owners in place beneath accepted ADRs and interfaces. Reuse lifecycle/authority catalogs and preserve historical evidence and stable paths. |
| [0.8.26 Slice 60](../../../0.8.26/features/slice-60/plan.md) still found obsolete engine/retrieval/recovery prose after that catalog pass; [Slice 65](../../../0.8.26/features/slice-65/plan.md) distinguished topology validation from semantic truth. | Review maintained claims against implementation and tests. Green catalogs, old target metadata or an agent's assertion that a file is current cannot replace that review. Do not inherit the separately authorized product-recovery change. |
| [0.8.27 Slice 140](../../../0.8.27/features/slice-140/plan.md) converged user and contributor guidance, three-SDK examples, generated indexes and pending documentation targets before Slice 150 qualification. | Reuse its source-derived navigation and truthful publication model. Verify documented routes through real examples; distinguish candidate validation from registry/Pages availability. |
| Both ladders kept final candidate/platform qualification separate from documentation convergence. | Hand reviewed current documentation and narrowly tested examples to Slice 95; do not duplicate its five-platform matrix or release rehearsal. |

The initial review at the planning baseline enumerated these material inputs:

1. Completed Slices 30/31/40/41/42 change CUDA allocation/dependencies, SQLite
   packaging, lexical ranking, query compilation/stop words and context packing.
2. Slices 50/60/70 change bounded pending work, built-in-provider admission,
   storage/vector layout, head-span coverage and eligibility before KNN caps.
   Format 35 is an intermediate historical boundary; Slice 85 makes the current
   fresh-store format 36. Preserve old refusal and no-migration decisions.
3. Slices 80/85/90 establish search visibility/property filtering, explicit
   maintenance and filtered direct node/edge body FTS across three SDKs. Explain
   filter placement before truncation/ranking/limit, metadata/provider-free
   limits and body-only scope without promising future retrieval behavior.
4. Public README, package READMEs and site entry pages still target 0.8.27.
   The active architecture identifies a 0.8.27 profile; maintained engine and
   recovery prose still includes schema-34 current-state statements. These are
   audit targets, not instructions to replace every historical version string.
5. `scripts/check-public-doc-truth.py` already supports an explicit pending
   documentation target but hardcodes schema 34 for the selected target and
   compatibility guidance. Slice 92 owns the bounded format-36 documentation
   contract correction with RED/GREEN; retain validation of historical published
   receipts, false-publication assertions and platform limits.
6. [Slice 95](../slice-95/plan.md) now assigns seven CI repair areas and final
   qualification. Move substantive documentation convergence here; its S95-5
   remains a final consistency check and any correction caused by later repairs.
7. MCP server work, including exposing Slice 90 filtered body FTS, belongs to
   0.8.29. Other owner deferrals remain unchanged. Physical erasure is complete
   except for concrete defects; explaining the delivered contract adds no new
   campaign, guarantee or optimization work.

**Disposition:** add this bounded convergence slice. At execution, enumerate
commits and allocated items since this baseline, review the related slice
plans/statuses and assigned functions, then adjust the audit before editing.
Reject a new documentation system, whole-tree rewrite, fixed historical counts,
retired hidden-surface oracle, size gates and broad code decomposition.

## Needs, requirements and acceptance

Users need correct installation and usage guidance. Contributors need a
code-grounded architecture, decision/design hierarchy and source documentation.
Maintainers need discoverable current records without losing historical rationale.
The following IDs are slice-local; `dev/acceptance.md` stays locked.

| ID | Requirement | Decisive acceptance |
| --- | --- | --- |
| S92-1 audit coverage | Review user, contributor and project documentation with explicit coverage and dispositions. | A compact inventory covers the cohorts below and every current maintained design owner from the existing catalog. Record update, retain accurate, historical/superseded with successor, or defer with reason; identify concrete findings and affected paths. Historical cohorts may be grouped. No current release-relevant error is silently deferred to Slice 95. |
| S92-2 user truth | Publishable guidance describes actual 0.8.28 contracts and supported delivery routes. | README/package READMEs, install/compatibility, guides, SDK/CLI references, configuration/errors, examples and new 0.8.28 release notes agree with current interfaces/code. Cover the enumerated .28 deltas and their limits, format-36 refusal/no migration, built-in-provider policy and .29-only MCP boundary. Prior release notes remain historical. |
| S92-3 contributor truth | Architecture, decisions, designs and code documentation explain actual owners and invariants. | One active architecture and maintained topic owners agree with accepted decisions, interfaces, source and focused tests. Each corrected normative claim cluster has authority plus implementation/test witnesses in the inventory; policy/method owners name their bounded enforcement instead. Public Rust, top-level Python and exported TS documentation in affected API families states inputs, outputs, errors and relevant invariants. Stale internal comments are corrected or removed without restating code. |
| S92-4 authority and hygiene | Readers can find current authority without mistaking plans or evidence for contracts. | Existing lifecycle/current-owner catalogs, ADR index/status/successor links, dev/design/doc indexes, contributor navigation and current release records are coherent; affected links/source locators resolve. Inspect active needs/requirements/traceability/test/release guidance for .28 drift and reconcile within existing authority. Preserve unique rationale, append-only ledgers, frozen baselines and historical records; no bulk move/archive/delete. |
| S92-5 generated and release truth | Source-derived navigation and documentation targets remain accurate without a publication claim. | Existing generator --write/--check output and MkDocs navigation match current manifests, SDK map and developer entry points. At implementation select public_docs_target_release=0.8.28 in the active state alongside consistent guidance; keep published receipts/version manifests unchanged. The corrected truth checker accepts the pending format-36 target and historical published fallback, and rejects wrong schema/target, missing guidance, false publication and unsupported platform claims. No fixed operation/workspace count is copied from .27. |
| S92-6 executable guidance | Representative documented routes work through their stated surface. | Inventory selects and records Rust application-SDK compile/doc examples, Python and TS real-database quickstarts and a CLI example; cover changed filter/context/provider/maintenance guidance where applicable with deterministic small fixtures. Compile/run via documented candidate packages or external consumers as appropriate. Report source-only vs installed proof and unavailable routes explicitly; existing focused evidence may be reused only after comparing relevant source/example inputs. No mock database or claim of registry installation before publication. |
| S92-7 proportional repair and review | Documentation tooling corrections remain bounded and independently checked. | Astra reviews design additions/changes before implementation. Behavioral checker/example fixes have staged or committed observed RED tests, then minimal GREEN and code review; substantive oracles remain fixed. Prose corrections use code/authority comparisons rather than wording snapshots. Separate Astra verification checks coverage, semantic truth, runnable guidance and scoped gates; no unresolved material documentation finding remains at closeout. |
| S92-8 handoff | Slice 95 receives a clear current documentation baseline. | Status identifies changed current owners, preserved history, reviews/checks, source and evidence limitations, and any product/decision blockers. Update release JSON/views/indexes and advance to 95 only after acceptance; later code/gate changes trigger only affected documentation checks. No qualification, publication or .29-start claim follows from planning Slice 92. |

## Audit scope and update design

Use one short `inventory.md` during execution, with cohort/path, finding or claim,
accepted authority, code/test witness, disposition and verification. Reuse existing
catalogs as inputs; do not invent a second comprehensive lifecycle database.

| Cohort | Starting points and bounded audit |
| --- | --- |
| User-facing | Root and package READMEs; docs installation/getting-started/compatibility, concepts/guides/embedder/operations, all SDK/CLI/config/error references, examples, CHANGELOG and release notes. Follow current navigation and changed API families; check defaults, signatures, error/refusal behavior, platform/runtime/CUDA limits and measured claims. |
| Architecture and ADRs | dev/README.md, dev/architecture.md supersession, active data-plane architecture, accepted .28 ADRs and their index/relationship notes. Follow current authorities rather than selecting by filename date. Preserve accepted decision text; an actual policy conflict needs explicit resolution/successor, not silent prose reconciliation. |
| Designs and interfaces | Enumerate maintained owners in document-lifecycle.json and current-owner-authority.json. Review current prose, linked interfaces and source witnesses, with deeper updates for engine/open/migrations, retrieval/vector/projection/scheduler, bindings/errors and lifecycle/recovery/maintenance. Incorporate stable contract explanations now delegated only to slice records into their existing maintained owner. Keep experimental/deferred material visibly non-current. |
| Code and contributor docs | Crate/module/package docs, public Rust API docs, Python docstrings/stubs and TS TSDoc/declarations for changed .28 families; contributor landing/generated maps, build/test instructions, configuration and ownership locators. Review presence and correctness, not comment density; make no runtime or public API change. |
| Project hygiene | dev/DOC-INDEX.md and detail maps, architecture/design/interface entry points, active release plan/state/board, needs/requirements/traceability/test guidance, release/developer runbooks and applicable agent instructions. Correct stale next actions, duplicate current authority, dead links and unsupported claims; preserve closed slice/progress/experiment evidence in place. No whole-ledger rewrite or speculative backlog reallocation. |

Authority order is accepted owner rulings/ADRs and maintained interface contracts,
then verified implementation/tests, then explanatory design and guidance. A real
contract/code conflict is a named blocker for its owner; do not document a defect
as intended behavior or implement product repairs under this documentation scope.
Non-substantive decision summaries/links may be corrected without changing a ruling.

Update current owners in place, with brief historical/successor notes where useful.
Do not keep piling current addenda onto an obsolete normative body. Generate only
the existing navigation maps; detailed manual references and docstrings retain
human review. No new generator, universal prose linter or deployment is needed.

## Implementation and verification plan

1. Refresh the baseline and audit inventory, including all seven input categories
   above and any new allocated finding. Review needs/AC and adjust scope. Have
   Astra review the concrete claim/owner corrections and any checker design.
2. **RED:** for the known public-doc truth correction, add failing cases to the
   existing tests for a pending .28 format-36 target, historical published fallback
   and the retained negative controls. Stage/commit observed failures before code.
   Any further executable change needs its own behavior-specific failing test.
3. **GREEN:** make the smallest source-bound checker correction; update current
   prose, code documentation, catalogs/indexes, release target and generated maps.
   Correct examples to existing public behavior. Reuse existing tools and schemas;
   preserve publication receipts and current product/decision contracts.
4. Run focused example/compiler/doc tests and existing architecture/lifecycle,
   public-doc-truth, generator freshness and parity checks as applicable. Use
   scripts/agent-lint-md.sh, scripts/agent-lint-docs.sh, release-state views and
   the existing MkDocs site builder without deploying. Validate snippets that
   change; a syntax/link pass alone does not establish their runtime truth.
5. Obtain Astra code/content review and separate independent Astra verification.
   Scoped checks suffice for Markdown/state-only changes. Source, test or executable
   script changes require one final scripts/agent-verify.sh under repository rules
   after focused fixes; do not repeat broad runs for individual prose corrections.
   Slice 95 owns final feature/security/performance/platform/release qualification.
6. Write concise status, reconcile state/generated views and doc indexes, land the
   reviewed changes, and hand off to 95. Reuse the sole-writer development checkout;
   if isolation becomes necessary, land and remove only task-created worktrees and
   branches. No new worktree is needed for this plan.

The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
applies through real runnable examples, boundary/negative controls and one regression
per confirmed tooling defect. Defer fuzz/coverage frameworks and broad anomaly,
performance or erasure campaigns. New evidence is semantic drift despite existing
lifecycle gates and schema-34 assumptions in the otherwise reusable truth checker.

No version bump, tag, registry/Pages publication, source refactor, new public API,
changed compatibility policy or migration is allocated. Any release decision or
product defect found by the audit gets an explicit owner disposition; unresolved
release-relevant blockers prevent closure instead of being hidden in prose.

---
title: FathomDB 0.8.24 — release prework and portable CUDA distribution
status: PROPOSED
target_release: 0.8.24
---

# FathomDB 0.8.24 — plan of record

> **Planning posture.** This plan first performs bounded discovery and an
> owner-led decision review. Slices 1–5 write proposals only; they do not
> change product code, documentation contracts, dependencies, runners, registry
> settings, or release workflows. Slice 7 is the only prework implementation
> slice, and contains only work the owner accepts in Slice 6.

## Purpose

0.8.24 makes Tegra CUDA distribution practical without weakening the CPU
release contract:

- publish Tegra CUDA as exact `fathomdb==0.8.24+tegra` from a first-party PEP
  503 index while generic CPU `fathomdb==0.8.24` remains on PyPI;
- preserve current CPU artifacts and make release publishing idempotent; and
- prove each new target by installing the published package on that target.

**HITL scope ruling — 2026-08-30 (`seq-258`).** All Windows support is
postponed to **0.8.26**: Windows x64 CUDA distribution, its executor/VM/runner
work, Windows GPU smokes, and the Windows Python-SDK WAL review. The retained
Slice 40 and Slice 50 plans are input to 0.8.26, not blocked 0.8.24 work.

## Slice 0 setup record

- **Release branch:** `release/0.8.24`
- **Release worktree:** `/tmp/fathomdb-release-0.8.24`
- **Baseline:** `origin/main` at `ee9bb753e6847daa0e72902eed972a5498f4f556`
- **Preflight:** passed on 2026-08-23; the worktree was cut from current
  `origin/main`, not the shared local `main` checkout.
- **Shared checkout:** deliberately untouched. At setup it was three commits
  behind `origin/main` and had owner files untracked.
- **New CI work:** it is already merged on `main` and was assessed by Slice 10.
  No remaining integration change was found; the release branch does not
  silently substitute for `main` as its delivery branch.
- **Slice 30 identity correction:** D-80.6-3 already owns the same-name
  `+tegra` alternate-index topology. Slice 30 retired its stale
  distinct-distribution premise; GitHub Pages is the authorized interim PEP
  503 route while durable hosting/distribution remains deferred for review.

Slice 0 is complete. The overall plan remains proposed because no live
release-state JSON or generated status board has been deliberately created.
The completed Slice 6 decisions and Slice 10 assessment do not implicitly
create or activate that single-writer release state.

The detailed, read-only execution sequence is in
[Slice 0 — environment and project-infrastructure baseline](0.8.24/prework/slice-0-environment-and-infrastructure.md).

**Slice 0 completed 2026-08-23.** Its evidence and owner decision brief are in
`dev/plans/0.8.24/prework/`; remote GitHub metadata is deliberately recorded
as unknown while the authenticated API is rate-limited.

**Slice 1 completed 2026-08-23.** Its [design review](0.8.24/prework/slice-1-design-review.md)
and [library sweep](0.8.24/prework/slice-1-library-sweep.md) approve the
feature allocations, add draft N24/R24/AC24 items, and identify one contained
root-tooling remediation for later owner acceptance.

**Slice 5 completed 2026-08-23.** Its [verification-adequacy
review](0.8.24/prework/slice-5-verification-adequacy.md) separates the
existing local contract controls from the still-required target, registry, and
external-client evidence, and assigns every resulting proof gap to one feature
or contingent prework slice.

**Slice 10 completed 2026-08-24.** Its [status
record](0.8.24/features/slice-10/status.md) closes the current-main assessment
with no workflow or script change. The accepted [interface
design](0.8.24/features/slice-10/design.md) hands target-specific route work to
Slices 30/40 and final evidence assembly to Slice 70 without introducing a
future-design dependency or hosted ceremony run.

## Scope and non-goals

### In scope

- Same-distribution Tegra CUDA local-version metadata, first-party PEP 503
  publication route, and Jetson installed-package smoke.
- Existing CPU artifact preservation and publisher idempotency.
- The CI workflow already merged on `main` and assessed by Slice 10.
- Engine-level performance adjustments justified by benchmark-branch results.

### Not in scope

- Uploading `fathomdb==0.8.24+tegra` to PyPI.
- All Windows support (including CUDA distribution, VM/runner work, smokes,
  and Python-SDK WAL review), which is deferred to 0.8.26; Windows ARM64,
  macOS CUDA, Tegra SBSA, or changing 0.8.23 artifacts.
- Treating a benchmark, a Memex finding, or a CI workflow as a product decision
  before it has been inspected and accepted through Slice 6.

## Decisions Slice 0 must prepare for HITL

1. **Tegra package transport.** D-80.6-3 already selects the distribution and
   installer shape: generic CPU `fathomdb==0.8.24` stays on PyPI; Tegra uses
   exact `fathomdb==0.8.24+tegra` on a first-party PEP 503 index, detection-
   gated on classic Tegra. Slice 30 uses the authorized interim GitHub Pages
   endpoint and must re-review durable hosting before a later Tegra release.
2. **Main CI relationship.** Establish whether the new CI workflow on `main`
   already covers the relevant contract, needs a narrow extension, or should
   remain independent. No release branch change may overwrite newer main CI
   work.
3. **Benchmark evidence source.** Identify the benchmark branch, committed
   result files, baseline, decision rule, and owner-approved performance target
   before Slice 20 alters the engine.

## Prework slices — discovery before implementation

| Slice | Title | Deliverable | Permitted effect |
| ---: | --- | --- | --- |
| **0** | Environment and project-infrastructure baseline | Setup record; runner/registry/CI/benchmark inventory; decision brief for the five items above. | Create isolated branch/worktree and this draft only. |
| **1** | Dependabot and library sweep | `slice-1-library-sweep.md`: every upgrade candidate, source, current/available version, security/compatibility signal, and pin/source rationale. | Write findings only. No dependency, lockfile, or Dependabot change. |
| **2** | Repository cruft review | `slice-2-cruft-review.md`: program docs, `dev/`, developer notes/intermediate docs, public docs, and code/test artifacts categorized keep / deprecate-in-place / archive / delete. | Write proposal only. No deletion, archival, or doc rewrite. |
| **3** | Product-document and architecture draft deltas | `slice-3-product-doc-drafts.md`: draft CRUD changes for User Needs, Requirements, and Acceptance Criteria; draft architecture CRUD changes; a slice allocation for every draft. | Drafts only. No contract, ADR, requirement, AC, or architecture edit. |
| **4** | Architecture and code-alignment review | `slice-4-architecture-alignment.md`: implications from Slices 0–3 plus a high-level code-versus-architecture alignment review and proposed code/architecture changes. | Write proposal only. No implementation. |
| **5** | Verification-adequacy review | `slice-5-verification-adequacy.md`: requirement → acceptance criterion → test traceability; product-goal/critical-path coverage; gaps and proposed proof. | Write proposal only. No test or workflow change. |
| **6** | Consolidation, independent plan review, and HITL | Ranked proposal register, an owner decision record, Slice 7 implementation plan, independent review output, up to two FIX-n updates, and final owner disposition. | Decisions and plan updates only. |
| **7** | Accepted prework implementation | The Slice 6-approved plan executed with normal tests/review. | Only owner-accepted prework items. |

Draft execution plans now exist for
[Slice 3](0.8.24/prework/slice-3-product-document-drafts-plan.md),
[Slice 4](0.8.24/prework/slice-4-architecture-alignment-plan.md),
[Slice 5](0.8.24/prework/slice-5-verification-adequacy-plan.md),
[Slice 6](0.8.24/prework/slice-6-hitl-consolidation-plan.md), and
[Slice 7](0.8.24/prework/slice-7-accepted-prework-implementation-plan.md).
They define execution and evidence boundaries but do not mark those slices
started or authorize implementation.

### Slices 1–5 method and durable locations

Slice 0 creates `dev/plans/0.8.24/prework/` and the five named records above.
Each proposal must cite its concrete source files, distinguish evidence from
inference, name its suggested destination slice, and explain why no immediate
action was taken. For Slice 1, every pin is classified as intentional
compatibility/dependency/security/toolchain pin, unknown, or stale; unknown is
never silently unpinned. For Slice 2, “archive” names the destination and
retention rationale; “delete” names the replacement or why none is needed.

Slice 3 treats User Needs, Requirements, Acceptance Criteria, and architecture
as draft changes under review—not as generated text to land. It explicitly
allocates each draft to Slice 7 or a numbered feature slice below, or marks it
postponed.

Slice 5 must answer all three questions separately:

1. Does every in-scope requirement have falsifiable acceptance criteria?
2. Does each acceptance criterion have an appropriate test or other named
   verification mechanism?
3. Are the in-scope product goals and critical paths—especially CPU install,
   Tegra install, and publication idempotency—known and adequately exercised?

### Slice 6 HITL protocol

Slice 6 consolidates every proposal from Slices 0–5 into one compact register:

| Field | Meaning |
| --- | --- |
| Proposal | Exact requested change and source record |
| Understanding | Clear / needs evidence / unclear |
| Risk | Low / medium / high with concrete failure mode |
| Effort | S / M / L with dependency notes |
| Recommendation | Include / postpone / reject |
| HITL decision | Accepted / postponed / rejected / needs clarification |
| Destination | Slice 7 or a feature-slice number |

The owner receives the register interactively and decides each item. Only then
does Slice 6 write the detailed Slice 7 implementation plan. A separate
read-only subagent reviews that plan for scope, missing tests, worktree/main
boundary errors, and unmet decisions. The author may perform at most two
documented FIX-n cycles, then presents the revised plan to the owner and
records the final decision. No subagent starts until this Slice 6 review point.

## Merged-main CI integration assignment

`5e2a05e2` (`ci: streamline long-running job setup`) is already merged on
`origin/main`. It adds cache-aware setup to the long-running CI jobs, separates
heavy-only dependency bootstrap into `scripts/bootstrap-heavy.sh`, and adds the
corresponding local CI-contract tests. It is not release-branch work and does
not create a new prework gate.

**Slice 10 owns its integration.** After Slice 6 authorizes feature planning,
its draft must begin from the then-current `origin/main`, inspect this landing
alongside the prior proportional-routing workflow changes, and determine the
smallest release-interface action, if any. It must preserve the fast/heavy job
ownership split and the local contract tests; it must not recreate, backport,
or overwrite the merged main CI work from `release/0.8.24`. Its ready plan
must state the narrow local structural checks used for any CI edit (including
the relevant CI-contract tests and `actionlint`) and must not manually start a
full hosted CI run merely to integrate this already-merged work.

## Proposed feature slices

Feature slices use multiples of ten. Their contents follow the completed Slice
6 decisions plus each slice's own draft-to-ready review; this ladder remains a
proposed release-level allocation rather than an implicit release-state record.

### Prework-to-feature allocation and planning protocol

Slice 7 implements **only** owner-accepted repository/version-preparation work
whose primary destination is Slice 7. It does not plan, make ready, implement,
or provide acceptance evidence for Slices 10–70. Each feature slice opens its
own draft plan after the applicable Slice 6 decisions, incorporates its rows
from the register below, completes its own design/ready review, and then
implements only that ready plan. A new finding from Slices 3–5 must be added
both to this register and to the appropriate feature-slice draft; it must not
be silently folded into Slice 7.

| Source finding | Primary destination and mandatory initial-plan content |
| --- | --- |
| Slice 0 main-CI relationship and new-main workflow inventory; merged-main `5e2a05e2` cache/heavy-bootstrap CI landing | **10:** assess the exact current main CI/release interface, preserve proportional routing plus the fast/heavy ownership split, and make no release-branch recreation, backport, or overwrite of main CI. |
| Slice 0 retained performance branch/evidence and owner-selected streamed boundary-tie result | **20:** inspect the named evidence and unmerged engine delta, declare the integration decision rule and targeted correctness proof, and do not require a confirming benchmark run. |
| Slice 0 Tegra package-identity decision; Slice 1 N24-1/R24-8 | **30:** apply D-80.6-3's same-`fathomdb`, exact-`+tegra` first-party-index topology through the interim GitHub Pages route; preserve explicit CPU-versus-CUDA selection and Jetson proof. |
| Slice 0 Windows SDK/executor decision; Slice 1 N24-2/R24-9 | **40:** **POSTPONED TO 0.8.26 (`seq-258`).** Preserve the reviewed draft as an input; do not select a SDK matrix, executor, artifact, or Windows proof route in 0.8.24. |
| Slice 1 R24-12 and the unresolved external Memex evidence boundary | **50:** **POSTPONED TO 0.8.26 (`seq-258`).** Preserve the read-only evidence plan; do not inspect, reproduce, or disposition Windows WAL behavior in 0.8.24. |
| Slice 1 N24-3/N24-4/R24-10 and Slice 0 publication topology | **60:** plan Tegra target smokes, CPU artifact preservation, and immutable-existing-artifact publisher retry/idempotency evidence. |
| Slice 0 release-control/branch-boundary evidence and the completed feature-slice outputs | **70:** plan evidence assembly, release-branch/main integration, and owner-ready publication preparation; publication itself remains separately authorized. |
| Slice 3 conditional target-artifact CRUD (`REQ-TARGET-TEGRA` / `AC-TARGET-TEGRA`) | **30:** carry the proposed need/requirement/criterion into Slice 30's own draft-to-ready plan; do not edit canonical contracts until owner approval. |
| Slice 3 conditional Windows CUDA CRUD (`REQ-TARGET-WINDOWS-CUDA` / `AC-TARGET-WINDOWS-CUDA`) | **40:** **POSTPONED TO 0.8.26 (`seq-258`).** Do not edit canonical contracts in 0.8.24. |
| Slice 3 release retry and installed-target CRUD (updates to REQ-050/AC-054 and REQ-052/AC-056) | **60:** make the canonical/documentation change only if the owner accepts it, then prove retry-safe completion and target-installed smoke. |
| Slice 4 code alignment: current CI is proportional and already on main | **10:** retain a no-change presumption; add a route only when the feature ready plan proves a concrete selector/transfer/smoke gap. |
| Slice 4 code alignment: retained 811-line SCALE-02 branch delta | **20:** review the complete delta and write a bounded implementation design with the retained fidelity constraints; no opportunistic commit copy or benchmark rerun. |
| Slice 4 code alignment: Python retains `fathomdb-tegra` only as a local sibling-package tripwire | **30:** do not publish that name; extend the existing `+tegra` displaced-build guidance for the interim Pages endpoint only after its install contract is implemented. |
| Slice 4 code alignment: Windows loader/package and hosted jobs are CPU-only | **40:** **POSTPONED TO 0.8.26 (`seq-258`).** Retain the finding without 0.8.24 implementation. |
| Slice 4 code alignment: existing WAL controls do not attribute external client behavior | **50:** **POSTPONED TO 0.8.26 (`seq-258`).** Retain the attribution finding without 0.8.24 evidence work. |
| Slice 4 code alignment: retry-safe mechanics conflict with atomic-publish wording | **60:** implement the owner-approved release semantics/documentation change and targeted publisher/installed-smoke proof. |

The register does not make any feature ready or authorize its work. It is the
minimum input each feature's separate draft plan must carry forward; later
Slices 3–5 may refine, reject, or add rows through the same mechanism.
Slice 7 is not a ladder gate for feature planning or implementation. A feature
may name a specific owner-accepted Slice 7 maintenance change as a narrow
dependency only when its own ready plan proves that dependency; absent that
explicit proof, Slices 7 and 10–70 proceed independently.

| Slice | Title | Primary outcome | Depends on |
| ---: | --- | --- | --- |
| **10** | Main CI workflow assessment and integration | **Complete:** current `main` supplies the existing proportional CI/release interface; no workflow or script change was required. | 0, 6 |
| **20** | Benchmark-directed engine performance | **Complete:** owner-selected streamed BM25 boundary behavior was reconstructed and independently verified without rerunning the settled benchmark. | 0, 5, 6 |
| **30** | Public Tegra CUDA distribution | **Complete:** exact `fathomdb==0.8.24+tegra` is served from the authorized interim GitHub Pages PEP 503 index with Jetson proof; never upload the host-bound wheel to PyPI. | 0, 6, 10 |
| **40** | Windows x64 CUDA distribution | **POSTPONED TO 0.8.26 (`seq-258`).** No executor, VM, runner, build, smoke, workflow, or publication work remains in 0.8.24. | — |
| **50** | Windows Python SDK WAL behavior review | **POSTPONED TO 0.8.26 (`seq-258`).** No external-evidence review or native-Windows reproduction remains in 0.8.24. | — |
| **60** | Installed-package smokes and CPU/publisher preservation | **Complete:** exact public Pages Tegra install passed on Jetson with retained digest, lifecycle, and CUDA witness; CPU publisher preservation remains covered by existing fixtures. | 30 |
| **70** | Release integration and owner-ready evidence | Assemble the non-Windows scoped evidence, resolve release-branch/main integration, and prepare—not perform without direction—the 0.8.24 publication decision. | 10, 20, 30, 60 |

### Feature-slice draft plans

Each feature has a separate plan. Slices 10, 20, and 30 are complete. Slice 30 published its verified Tegra wheel
to the authorized interim Pages route; durable hosting/distribution remains a
later review. Slices 40 and 50 are postponed to 0.8.26. Slices 60 and 70 retain
their own slice-preparation, draft contract/design review, implementation
boundary, verification, prerequisites, and handoff. A plan's existence does not
make its slice ready or authorize implementation or an external release action.

- [Slice 10 — main CI assessment and integration](0.8.24/features/slice-10/plan.md)
- [Slice 20 — benchmark-directed engine performance](0.8.24/features/slice-20/plan.md)
- [Slice 30 — public Tegra CUDA distribution](0.8.24/features/slice-30/plan.md)
- [Slice 40 — Windows x64 CUDA distribution](0.8.24/features/slice-40/plan.md)
- [Slice 50 — Windows Python SDK WAL review](0.8.24/features/slice-50/plan.md)
- [Slice 60 — installed smokes and publisher preservation](0.8.24/features/slice-60/plan.md)
- [Slice 70 — release integration and owner-ready evidence](0.8.24/features/slice-70/plan.md)

### Feature acceptance signals

| ID | Requirement | Evidence required before Slice 70 |
| --- | --- | --- |
| R24-1 | Tegra’s public identity is registry-valid and cannot be confused with generic ARM64 CPU. | Metadata/selection test, registry query, and clean Jetson installed-package open/write/search/close/exit smoke. |
| R24-2 | Windows x64 CUDA has one explicit supported SDK matrix and remote proof route. | **POSTPONED TO 0.8.26 (`seq-258`)**; not a 0.8.24 release condition. |
| R24-3 | Existing CPU artifacts are preserved. | CPU package-name/version/install checks and ABI-floor contract checks for the affected lanes. |
| R24-4 | Publishing remains retry-safe. | Contract test or recorded dry-run demonstrating skip-if-existing/idempotent publisher behavior without replacing valid existing artifacts. |
| R24-5 | Engine changes improve the nominated benchmark according to its pre-declared decision rule and do not regress applicable critical paths. | Retained baseline/comparison evidence plus targeted regression tests. |
| R24-6 | Python-SDK Windows WAL behavior has an evidence-based disposition. | **POSTPONED TO 0.8.26 (`seq-258`)**; not a 0.8.24 release condition. |
| R24-7 | New CI work is compatible with the release topology. | Main-branch workflow review, actionlint/contract proof as applicable, and documented branch/interface ownership. |

## Branch, worktree, and publication rules

- The shared `main` checkout is not a 0.8.24 editing surface. Every writer uses
  a fresh worktree off current `origin/main`; shared mutable workflow files are
  serialized.
- Any later shared-CI work demonstrated by Slice 30 or 40 belongs on current
  `main` through that slice's explicitly revised plan. The release branch is
  integrated only after such work is verified from git; it must not recreate
  or overwrite it.
- No `maturin develop` or `pip install -e` runs from a worktree. Targeted Python
  build operations use the canonical main checkout or an approved target host.
- No release tag is created or pushed by this plan. A `v*` push is a real,
  multi-registry publication event and requires a separate owner decision.

## Immediate next action

Slices 0–7, 10, 20, 30, and 60 are complete. Slices 40 and 50 are postponed to
0.8.26 by `seq-258`; they no longer block this release. Slice 70 remains final
integration only. No additional external release action is authorized by these
planning records.

---
title: Inspector 0.2.0 release plan — draft
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Prework, human disposition and six feature slices for connected exploration
---

# Inspector 0.2.0 release plan

## Current execution

Slices0–7,9,10,20–60 and75 are complete;8 remains reserved.70 technical
qualification is retained. The owner explicitly removed the five-person study
requirement in [INS-020-LOCAL-RELEASE](release-decisions.md) (Inspector ledger
seq-1); no sessions occurred and none is reported as passed.80 reconciled current
requirements/docs and70 disposition without reopening this ruling.

[80 readiness is COMPLETE](slice-80/status.md): current docs, local CI,
compatible candidate and safe promotion are tested and independently verified.
[90 local release](slice-90/status.md) created the final local distribution at
`dist/inspector/0.2.0/`; no registry publication occurred.

The owner commissioned [80 release readiness](slice-80/plan.md), then
[90 local release](slice-90/plan.md).80 establishes tested documentation, local CI,
packaging and distribution methods;90 releases exact qualified Inspector0.2.0
artifacts into ignored `dist/inspector/0.2.0/`. No registry upload, release tag,
remote push or Engine publication is authorized. The existing dedicated branch/
worktree `inspector/dev/0.2.0` is reused sequentially.90 followed80's verified COMPLETE handoff and material review remediation.

## Original planning baseline — before the later execution commission

### Original authority, baseline and planning state

The owner [adopted the scope](scope.md) on 2026-11-19 and then requested this
release/slice planning. Scope is adopted; this execution plan and every slice
plan/design are DRAFT. Creating these documents executes no slice, creates no
release checkout, installs nothing and applies no prework proposal. Actual
Slice 9 review/decisions cannot occur before its evidence exists.

The current planning checkout is branch `llm/0.8.29-inspector-additional`, rooted
at `dev/0.8.29` source `6edc1155f`; adoption is `609dd85c41`. The product is an
unpublished 0.1.0 record-browser foundation. Text/vector/interactive graph and
new two-observation comparison remain unimplemented. [Port evidence](../0.1/compatibility-0.8.29-port.md)
and historical statuses retain their real limits: no full-workspace green,
missing checkout-owned Python environment, and unrun explicit long/native cases.

The future release branch is `inspector/dev/0.2.0`, with a dedicated linked
worktree selected at `/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`.
Its base must include the final approved HEAD of this planning branch and its
Inspector port/scope/plans, retaining ancestry to the selected 0.8.29 baseline.
Do not cut from bare 0.8.29 or main and lose Inspector work. Future Slice 0
creates and enters the release checkout before its reports; verify actual
branch/HEAD and working directory. Never execute in the primary checkout.
The double slash in the user's illustrative branch placeholder is normalized
to the valid single-slash ref above.

The [owner checkout decision](slice-0/checkout-decision.md), 2026-11-19, requires
future Slice 0 to create and enter this release branch/worktree from the approved
final planning HEAD. Reports for 0–9 then use it sequentially. Only that initial
checkout action is exceptional: other environment/infrastructure changes remain
proposals for actual Slice 9 disposition and conditional Slice 10 implementation.
Slice 10 reuses the existing release checkout and performs no initial bootstrap.
Recording the decision executes no slice and creates no branch/worktree now.

## Product result and unchanged boundary

Deliver coordinated canonical record/index/vector/stored-relationship/provenance
inspection, actual lexical and compatible cached CPU semantic/hybrid queries,
metric-faithful stored neighborhoods and two bounded observations compared
in-session. The person fixture must traverse the real representations and
stored relations while preserving exact identity/revision and original context.

[Requirements](../../requirements.md), [acceptance](../../acceptance.md) and
[the single verification matrix](../0.1/verification-plan.md) remain canonical.
All mandatory Need/Should, INS-01–09, INS-X01–X11, V01, F-01–07, T-01–05 and
High investigation obligations carry forward, plus INS-05.7–05.10/M40. The
[backlog](../../BACKLOG.md) B01–B04 are scoped; B05–B10 remain outside.

Existing Engine/SDK/MCP/schema/ranker/public API and pin changes, administration,
repair/migration, active-external-writer/live/remote/multiuser attachment,
historical formats/extra platforms, persistent workspaces/general export,
advanced maps/ANN/relevance campaigns/graph analytics, GPU expansion and
teaching/generated narratives remain outside the adopted scope. Repo-wide
prework can inspect those areas and report dependencies; it does not commission
changes to them. A required missing dependency needs an explicit separately
scoped decision, not a copied implementation or an unavailable-mode waiver.

D-01 remains fixed: independent inactive, erasure-content and telemetry gates,
physical deletion correct and unchanged, conditional legitimate retained-content
export only, no reconstruction/retention/erasure delay. Normal qualified owned
Engine lifecycle effects remain allowed; inspection controls do not imply
immutable bytes. Numeric/field limits, initial platform/distribution targets and
all existing qualification requirements are inherited unchanged from scope.

## Ladder and stage gates

Default execution is sequential, each slice by its own directly commissioned
agent. Evidence-only fan-out is possible if repo writes remain serialized or
isolated; it is not a reason to add worktrees per slice by default.

| Slice | Scope / work type | Output and dependency |
| --- | --- | --- |
| [0](slice-0/plan.md) | Environment/project infrastructure and release-checkout decisions | Inventory/proposals and an overall-plan check; first creates/enters release checkout under the owner ruling, then reports. |
| [1](slice-1/plan.md) | Library/Dependabot/pinning review plan | Required/optional/external-owned candidates with evidence and pin reasons; follows 0. |
| [2](slice-2/plan.md) | Repo-wide cruft review | Keep/deprecate-in-place/archive/delete proposals, reference/retention dependencies; follows 1. |
| [3](slice-3/plan.md) | Local draft needs/requirements/AC and architecture CRUD | Draft items allocated to implementation owners, no canonical changes applied; follows 2. |
| [4](slice-4/plan.md) | Architecture and high-level code alignment | Source-backed gaps/proposals and architectural fit, no fixes applied; follows 3. |
| [5](slice-5/plan.md) | Verification adequacy/critical paths | Req→AC→meaningful test/executed evidence gaps and proposals; follows 4. |
| [6](slice-6/plan.md) | Stale documentation evidence | User, consuming developer and contributor truthfulness proposals; follows 5. |
| [7](slice-7/plan.md) | Build/preflight/verify/CI/CD release-delay evidence | Pragmatic proposed remedies, cause/ownership and relevant gate impact; follows 6. |
| [8](slice-8/plan.md) | Reserved placeholder | No invented review, feature or implementation; follows 7. |
| [9](slice-9/plan.md) | Consolidation and interactive HITL review | Score/disposition 0–8 items, review/refine conditional 10 up to four FIX-n cycles, actual human ratification; requires all 0–8 outputs. |
| [10](slice-10/plan.md) | Approved prework implementation | Conditional until item-level Slice 9 approval; apply only the approved set, verify and report; follows ratified 9. |
| [20](slice-20/plan.md) | Coordinated record and representation foundation | Usable identity/context/readiness/association navigation and carried foundation corrections; requires 9/10. |
| [30](slice-30/plan.md) | Indexed text and FTS/BM25 | Actual lexical query/evidence/origin UI; follows 20. |
| [40](slice-40/plan.md) | Vectors and supported cached semantic/hybrid retrieval | Stored representations/windows, measured neighborhoods, runtime ownership and actual serving evidence; follows 20/30. |
| [50](slice-50/plan.md) | Graph, provenance and retained lifecycle | Directed interactive graph with full edge detail, bounded traces/history; follows 20/30/40. |
| [60](slice-60/plan.md) | Two-observation comparison and coordinated person workflow | Pin/compare real scoped observations and complete cross-view interactions; follows 20–50. |
| [70](slice-70/plan.md) | Installed technical qualification | Qualified package/browser/Engine pair and technical evidence retained; former study removed by owner decision; final release now90. |
| [75](slice-75/plan.md) | Expert UX remediation and refreshed installed candidate | COMPLETE: seven findings resolved and updated installed candidate verified; readiness80 and local release90 follow. |
| [80](slice-80/plan.md) | Release readiness | Reconcile study decision/current docs, execute local CI, qualify packaging/distribution and concrete90 handoff; follows75. |
| [90](slice-90/plan.md) | Local release | Promote exact qualified bytes to ignored dist, verify final installation/integrity and record local release; follows COMPLETE80. |

Prework 0–8 produce reports, local drafts and proposals; only future Slice 0
creates and enters the release checkout under the recorded owner ruling. They do not execute
environment/library upgrades, cleanup or product changes. Slice 8 is reserved.
The generic execution template's implementation sentence means producing the
assigned prework deliverable, not implementing its proposals.

Slice 9 uses one lean item list, retaining source IDs to avoid duplicate
findings from 2/6 or 4/5/7. Each item records understanding (unknown/partial/
source-established), risk (low/medium/high/unknown), effort (small/medium/large/
unknown), recommendation (include/postpone/needs evidence), dependency, owner
and acceptance/verification consequence. Present concise batches interactively;
record actual include/postpone decisions and rationale, never default consent.
Prepare/update Slice 10 from that selected set, commission appropriate direct
review, allow at most FIX-1 through FIX-4, and present changed scope/residual
findings for final human ratification. If unresolved after four, stop and expose
remaining issues; do not self-approve or loop indefinitely. Scope adoption is
already settled and is not re-asked unless a new change genuinely alters it.

Slice 10 is a conditional draft now, not an invented approved implementation
bundle. Every implementation item requires its Slice 9 ruling and concrete
acceptance/rollback or failure handling. If no items are approved, record the
ratified no-change result; unresolved mandatory prerequisites still block later
features. A risk score or review recommendation is not approval.

## Feature coverage and carried work

| Outcome / evidence group | First delivery responsibility | Final qualification |
| --- | --- | --- |
| INS-01/02 and record/operational parts of INS-03; M01–07/09–11/17 and scoped metadata | 20, with feature extensions | 70 |
| Lexical INS-04/05/06/08/09.2, V01-N05 and lexical evidence | 30 | 70 |
| Vector INS-04/05/06/08/09.3, V01-N04/N07, source windows and population/metric facts | 40 | 70 |
| Graph/provenance/history INS-03.3/07/09.4, V01-N03 and complete edge/lifecycle facts | 50 | 70 |
| Coordinated INS-09.6 and two-observation INS-05.7–05.10/M40 | 20 foundations; 30–50 local navigation; 60 integrated comparison | 70 |
| INS-X01–X11, V01-S01–S04, policy/cancel/security/accessibility/resources | Every affected feature owns a complete local result | 70 adds integrated/installed proof |
| T-01–T-05, High investigation and installed/default artifact support | 60 supplies person interaction;70 supplies installed technical checks;80 readiness;90 local release | 70 |

The following unique accountable mapping is DRAFT release allocation, not an
executed-evidence or criterion change. Every M01–M40 appears once. Historical
owner columns are reconciled in the single original matrix under S9-006;
its criteria and actual test evidence are unchanged. Contributions do not
transfer away an earlier feature's local completion duty.

| Unique accountable owner | Matrix rows (each M01–M40 appears once) | Canonical responsibility / mandatory contributions |
| --- | --- | --- |
| 20 | M01–M07, M09–M13, M15, M17–M18, M20, M24, M29, M31–M33, M36, M39 | Record/catalog/admission/references/readiness/policy/queue/security/exact values/export foundation. Text M06→30; origins, readiness, observations M09–13/15/17→30/40/50; edge M18→50; every new view extends common contracts. |
| 30 | M21–M22, M25 | Lexical query, actual route/tokenizer/match/score and High per-result evidence; M21 vector contribution 40, graph contribution 50, combined investigation 60. |
| 40 | M16, M23, M26, M30 | Stored metric-faithful vector UI and cache-only CPU semantic/hybrid; 30 supplies lexical M16 family values, 20 common startup refusal, every serving score retains own meaning. |
| 50 | M08, M19, M27, M38 | Exact edges/directed graph, High source/dependency/revision and functional investigation; 30/40 provide outcome evidence, 70 adds installed High functional verification; human study removed. |
| 60 | M28, M40 | Person cross-view workflow and two actual pinned observations; 20–50 provide correct individual views, references and facts. |
| 70 | M14, M34–M35, M37 | All-view keyboard integration, installed lifecycle and core T-01–05/High functional goals; each feature owns keyboard basics. The unperformed human contribution is removed by INS-020-LOCAL-RELEASE. |

Use the existing matrix once, with current ownership reconciled to this ladder.
Legacy 0.1 paths/history remain evidence inputs. Old Slices 15/16 do not replace
new prework; old Slice 20 ACTIVE is not new 20 COMPLETE. Old 30/40/50 allocation
feeds the same-numbered new drafts. Old final integration/qualification 60 feeds
new 60 interaction and 70 final qualification; M40 is owned by new 60, qualified
in 70. Do not create a second acceptance/matrix register or relabel old tests as
new execution. Each feature is complete locally; 70 cannot absorb unfinished
basic views, identity, error handling or accessibility.

## Working method, prerequisites and evidence

[The shared workflow](slice-workflow.md) binds every draft. Each includes the
user's filled future execution prompt, local draft needs/req/AC/design changes,
delta reconciliation, source reuse versus new work, challenges and primary-source
web research questions, architectural-fit checks, dependencies for the next
reviewer, meaningful verification and concise status/cleanup. Research planned
for execution is not claimed performed by drafting.

For feature behavior: real failing RED run/commit, separate minimal GREEN,
refactor without weakening the oracle, independent Sol-high design/code reviews
and direct Terra verification. No DB mocks or regenerated golden answers.
Prose-only prework uses document/traceability/source checks, no artificial TDD.
Every slice is assigned its own executing agent; reviewers/verifiers are read-only.
Use the release checkout sequentially. Concurrent mutators require isolation;
extra slice-owned branches/worktrees integrate back to the release branch and
clean only their own fully integrated resources. Preserve the release checkout.
No main merge, force push, hook bypass, tag or publication is implied.

Do not start feature execution without the adopted scope, ratified 9/10 output,
qualified source/capability evidence and a checkout-owned tool environment.
A missing .venv was a real port full-gate blocker; system Python or another
checkout's environment cannot convert it to a pass. Inspect scripts before
choosing setup: editable native installs from worktrees are forbidden, so use
an approved isolated packaging/test setup, not a conflicting convenience recipe.
Match Rust/Python/Node/TS/browser versions to source contracts, not memory.

A source-backed prerequisite for 4/9/40 is the stale legacy helper→Caller
embedding design: production `EmbedderChoice` has Default/None; Caller belongs
to test/evaluation containment under the accepted 0.8.28 open-policy ADR.
Investigate supported existing cache-only public routes rather than presuming
all semantic operations impossible. Missing mandatory serving support stays a
blocker or separately ruled dependency; it cannot authorize an Engine change,
test-only reuse, copied ranker or removal of the adopted positive case. Current
public snippet APIs also require correcting stale HLD absence claims as a draft.
Current vector producers/admission are head-only. Under owner S9-010, 0.2.0
qualifies that supported single head-window positive; within-source multiple-window
positive is deferred. Preserve exact row/profile/cursor/window/span/origin facts
and bounded metric/Engine-serving distinctions. No synthetic extra-window fixture
or Engine change. The separate cache-only CPU Engine serving positive remains
mandatory and unqualified until 40 proves it. Slice
30 also qualifies route-specific BM25F controls, bounds and cancellation rather
than assuming public snippets provide every scoring or snapshot contract.

Plans have no executed status.md now. Future statuses record only actual scope,
changes/review dispositions, RED/GREEN or report evidence, checks/skips/blockers,
remaining work and owned cleanup. Derive completion from the implemented result
and executed evidence. Specific findings get focused reproducer/blast-radius
checks; full required source/release gates still run at appropriate final stages.

## Quality strategy and release boundary

The [quality roadmap](../../../notes/fathomdb-quality-testing-roadmap.md) was
read during planning. Adopt real databases, independent person/adjacency/lexical/
numeric oracles, meaningful reference/codec property tests, targeted fault/cancel/
late-result and handle/resource tests, browser+keyboard and functional investigation tasks,
all D-01 overlaps, and exact installed artifact checks. New evidence includes
the working port, format37 semantic drift and the real 8,192-queued-record budget
failure fixed by per-transaction caching. Retain that regression. Defer global
MC/DC, general SQLite fuzzing, exhaustive allocation/VFS/power-loss campaigns and
unapproved platform matrices; they are outside this Inspector-owned release.

This is a CPU-only Inspector release with unchanged Engine/dependency support
policy. [CUDA review guidance](../../../design/cuda-version-support-review.md)
is an input to Slice 0's future applicability/policy review; no usage collection,
importance-table approval, driver/artifact change or GPU qualification is claimed
by drafting. Likewise overflow/chunking/runtime changes in Engine are not
commissioned; Inspector must truthfully inspect the format37 windows it supports.

Proposed product changes are the new lexical/vector/graph operations, coordinated
record navigation, and in-session comparison. There are no adopted default-policy,
Engine schema/public-pin changes. Feature 70 plans the eventual Inspector-only
0.2.0 package/version/help/build metadata update and local artifact qualification;
no manifest bump or registry publication occurs in this planning turn. A required
new upstream capability or scope expansion is presented explicitly at the proper
human decision boundary rather than hidden as implementation detail.

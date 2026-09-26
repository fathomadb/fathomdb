---
title: FathomDB 0.8.27 — correction-safe erasure and semantic refactoring
status: ACTIVE
target_release: 0.8.27
execution_model: direct; no Steward or Orchestrator
---

# FathomDB 0.8.27 — correction-safe erasure and semantic refactoring

## Goal and scope

FathomDB 0.8.27 fixes Memex's correction/supersession erasure blocker, then
refactors five selected monolithic engine, binding, and SDK facades into
semantic domains that are easier for humans and coding agents to understand,
navigate, change, and test.
The work executes directly in the dedicated `release/0.8.27` worktree. It does
not use the FathomDB Steward or Release Orchestrator roles.

The release covers:

- correction-safe source erasure after supersession;
- `fathomdb-engine/src/lib.rs`;
- `fathomdb-py/src/lib.rs`;
- `fathomdb-napi/src/lib.rs`;
- `src/ts/src/index.ts`; and
- `src/python/fathomdb/engine.py`.

The shared structural vocabulary is:

```text
core / open / configuration
types / errors
write / ingest
read / search
graph / evidence
projection / lifecycle
embedding / reranking
admin / operator
```

Trees need not be identical across languages. The same term must represent the
same product concept wherever it appears.

Unrelated cleanup is postponed unless it becomes a direct prerequisite for the
five-file refactor. The release stops at a qualified candidate. Pushes, tags,
registry changes, and publication require separate authorization.

## Scope disposition

F27-01 remains release-blocking and is the first product slice.

The former D27 candidates move to 0.8.28:

| Former ID | 0.8.28 placement |
| --- | --- |
| D27-01 | D28-05 — opt-in cross-operation frozen-snapshot leases |
| D27-02 | D28-06 — request/snapshot/projection/ordering-bound cursors |
| D27-03 | Merge into D28-03; retain D27-03 as provenance for generalized graph/state continuation |
| D27-04 | D28-07 — persisted source-complete evidence receipts |

Slice 9 must reconcile this placement across the two draft scopes, roadmap,
program sequencing, design allocation notes, and maintained indexes.

## Release authority and workspace

- **Baseline:** `2332242848d6135a1737d1abcc767c99620a9f53`, equal to
  `origin/main` when planning opened.
- **Branch:** `release/0.8.27`.
- **Worktree:**
  `/home/coreyt/projects/fathomdb-worktrees/release-0.8.27`.
- **Primary checkout:** remains untouched.
- **Schema baseline:** 34.
- **Publication:** not authorized by this plan.

Slice 0 records and verifies the already-created workspace without mutating
release state. Slice 9 activates the release-state file and board together with
their generated views, names `refs/heads/release/0.8.27` as the active ref, and
records Slice 10 as the immediate next action. Branch/worktree creation is
already satisfied and must not be repeated.

## Prework reconciliation

The 2026-09-02 draft intake preceded both shipped 0.8.25 and 0.8.26. The
executable baseline is now schema 34 at `23322428`, with 0.8.26's fresh-database
boundary, exact SDK-operation parity, current design-owner authority, graph
evidence, integrity, and derived-edge contracts. After the 0.8.26 tag, main also
added the performance-gauntlet/graph-benchmark program and F27-01 intake.

F27-01 is reproduced by Memex's exact characterization. A later narrow Memex
exemption means the defect no longer blocks Memex 0.6.0 cutover by itself; it
remains a privacy defect and blocks 0.8.27 publication. Existing tests cover
supersession and erasure independently but not their composition, so the plan
does not pre-authorize a root cause or production fix.

The readability proposal was written against a 17,910-line engine. The current
five targets total 52,818 lines: engine 34,184; PyO3 5,943; NAPI 5,406;
TypeScript 4,439; Python 2,846. Existing semantic modules and 0.8.26 parity
instruments are reused; stale line maps and the experimental branches are not
merged. Line count remains an advisory attention signal, never a correctness
gate.

## Release requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-01 | Supported correction/supersession cannot make a source bucket unerasable in the shipped one-source dependency model. | AC27-01/02/03: same-bucket, cross-bucket original-first, and cross-bucket replacement-first cases pass through public `erase_source`. |
| R27-02 | Erasure remains truthful, fail-loud, idempotent, and complete at rest. Pre-commit blockers roll back. A post-commit scrub failure returns `ErasureIncomplete`, preserves its durable retry obligation, and never falsely reports closure. | AC27-04/05: real pre-commit rollback and post-commit scrub-failure/retry cases pass; independent-reopen physical payload and live-authority absence passes after successful closure with exact survivors and exact audit/closure identity exceptions. |
| R27-03 | The correction introduces no new verb, report field, schema migration, or private consumer workaround. | AC27-06: the unchanged Memex characterization flips to supported success; requested-bucket report counts and error mappings remain exact. |
| R27-04 | Semantic decomposition preserves all observable Rust, Python, Node, TypeScript, wire, feature, transaction, locking, and package-root contracts. | AC27-07: a real-surface comparator rejects synthetic extra/missing/changed exports, re-exports, registrations, declarations, and package exports. |
| R27-05 | The engine remains one crate with root `Engine`, private internal modules, root re-exports, and the accepted single-writer model. | AC27-08: focused owning suites and affected feature/artifact routes pass per move and at candidate qualification. |

`dev/acceptance.md` remains locked. AC27 identifiers are release-local until a
separate governed acceptance decision authorizes a global addition.

## Cross-cutting DoD and lean slice execution contract

Every slice applies this workflow proportionately.

1. **Reconcile change since drafting.** Enumerate changes to the draft, related
   work landed since drafting, assigned features/functions/surfaces, and items
   allocated by prior slices. Approve, narrow, adjust, or reject the draft
   without absorbing adjacent work.
2. **Complete contracts and design.** Add or update applicable user needs,
   requirements, acceptance criteria, interfaces, and design. Material design
   changes receive independent read-only design review before implementation.
3. **Implement with focused TDD.** Behavioral changes use RED → GREEN →
   REFACTOR. Structural moves preserve behavior and follow the testing contract
   below. Material implementation receives independent code review.
4. **Verify proportionately.** Use focused and blast-radius tests during work,
   an independent read-only verification subagent at completion, and the
   repository-required verification gates before declaring the slice green.
5. **Close and clean up.** Record scope decisions, exact commits, tests,
   reviews, unresolved evidence, and the next dependency. A slice normally
   uses the release worktree directly. Any temporary worktree must be merged,
   verified from the release worktree, and removed when safe.

The release worktree has one writer. Read-only design, code, and verification
reviewers may share it.

## Reserved-gap policy

The numbered ladder intentionally reserves Slice 8 and the unused decimal
positions between planned feature slices. A reserved slot remains empty unless
new in-scope evidence requires a separately reviewed insertion; it is never an
implicit patch lane or authority to absorb unrelated work. If a planned slice
cannot remain bounded without consuming its reserved band, stop and revise the
plan rather than overflowing into the next feature number.

## Testing contract

The central rule is: **tests remain more stable than the code being
reorganized**. Tests describe observable contracts and invariants, never the
new file layout, private helper names, or internal call graph.

The detailed
[`0.8.27 refactor test approach`](0.8.27/prework/0.8.27-refactor-test-approach.md)
is incorporated by reference. This plan owns release scope and sequencing; the
test-approach record owns the expanded invariant, anti-vacuity, fault-injection,
feature-matrix, artifact, and slice-specific testing instructions.

### Test discipline

- Slice 20 is fix-to-spec: commit genuine failing tests, preserve the RED
  oracle, and fix production code without changing those tests.
- Slices 40–130 are normally behavior-preserving: establish the pre-move
  result, move code mechanically, regain green, and only then simplify it.
- A new characterization test may pass initially. Prove it is non-vacuous by
  temporarily injecting a plausible defect, observing failure, and reverting
  the defect.
- New logic introduced during extraction requires a genuine RED test.
- If a refactor reveals a semantic defect, stop and create a separate
  test-first correction. Do not hide it inside structural cleanup.
- Keep behavioral assertions unchanged through moves. Isolate required import
  or test-location edits from assertion changes.
- Name tests for behavior, not slices or modules.
- Test through public or deliberately narrow feature-gated seams. Do not widen
  engine fields merely to make tests compile.
- Database contracts use a real database. Physical absence and atomicity tests
  inspect canonical, provenance, projection, registry, queue, terminal,
  telemetry, vector, and WAL state as applicable.
- Close and independently reopen the database before making at-rest claims.
- Derive exact expectations from seeded physical state. Avoid `nonzero`,
  `eventually zero`, or copied-literal parity oracles.
- Multi-stage operations receive failure injection before and after important
  side effects, with every affected state plane compared with the pre-call
  snapshot.
- Codec, projection, recovery, and round-trip layers receive property tests
  with human-defined invariants. Generated snapshots are not accepted oracles.
- Concurrency uses barriers, hooks, and explicit rendezvous points, never
  sleeps. Global runtime, loader, memory, and artifact tests use fresh
  processes.
- Test default, operator, test-hooks, and other applicable feature combinations
  separately. Do not substitute `--all-features` where CUDA, Metal, or other
  configurations conflict.
- Source-linked extensions are not artifact proof. Binding and release slices
  build fresh packages and install them into isolated consumers.
- Record candidate SHA, features, platform, artifact hashes, command, true exit
  status, and test counts. Pipes and wrappers must not mask the tested
  command's exit.

### Invariant ownership

| Invariant | Deep owner | Thin parity checks |
| --- | --- | --- |
| Erasure completeness | Rust engine integration suite | Python and TypeScript public API |
| Public surface | Slice 30 surface comparator | Installed-package smokes |
| Transaction atomicity | Engine fault-injection tests | Binding error mapping |
| Wire validation | One shared fixture corpus | Python, Node, and TypeScript |
| Projection readiness | Engine state-machine and physical-row tests | Binding status decoding |
| Runtime/loading | Fresh-process native tests | Installed wheel/npm consumers |

Bindings do not duplicate the complete engine matrix.

## Prework: Slices 0-9

Slices 0–8 are investigation and durable proposals only. They do not modify
product code, dependencies, environments, CI, or accepted contracts.

### Slice 0 — environment and project infrastructure

Record the existing branch, worktree, clean base, and intended release
authority. Draft the plan inputs without creating or changing release state;
Slice 9 owns activation after review.

Inventory and allocate decisions for:

- Rust 1.95, installed targets, clippy/rustfmt, linker, Cargo caches, SQLite,
  ptrace, and build roots;
- Python 3.12 development and abi3-py310 release compatibility;
- Node 25.9.0 and npm 11.12.1;
- isolated tooling and artifact environments;
- Linux x64/ARM64, macOS x64/ARM64, Windows x64, CUDA x64, Jetson, and the
  local Windows VM;
- storage, disk, credentials, network/registry access, model caches, and
  release runners; and
- documentation, package rehearsal, and installed-artifact needs.

The host Node 26/npm 12 combination is not admissible evidence. Use a
tooling-only Python venv and a separate wheel-installed artifact venv; never
editable-install the binding from this worktree. Browsers are N/A unless a
browser-owned surface is discovered. Write findings durably and take no setup
action.

### Slice 1 — dependency and pinning sweep

Run a live, read-only dependency, advisory, Dependabot, and source-pin sweep.
The planning baseline proves no mandatory direct product-library upgrade.
Refresh and classify these candidates:

- Markdownlint/smol-toml advisory status;
- Mermaid/DOMPurify;
- unmaintained `paste` through Candle/tokenizers;
- Ruff and Pyright pins;
- TypeScript, Node types, and NAPI tooling; and
- GitHub Actions SHA/tag mappings and comment drift.

Retain protected Candle, ORT, sqlite-vec/rusqlite, PyO3/N-API, toolchain, and
full-SHA Action pins absent dedicated evidence. Classify every candidate as
update, retain, investigate, or postpone, with its dependency path, driver,
blast radius, and proof. Take no dependency action.

### Slice 2 — repository and documentation cruft

Review program documentation, engineering design and plans, source/tests,
developer notes, contributor documentation, and public documentation.
Classify each item as keep, deprecate-in-place, archive, or delete.

Treat the old 17,910-line refactor inventory as stale historical evidence. Do
not merge the large experimental refactor branches. Propose preserving concise
syntheses while retiring disposable generated maps and JSON after their useful
findings have been incorporated. Take no cleanup action.

### Slice 3 — draft contracts and architecture allocation

Draft CRUD changes for user needs, requirements, acceptance criteria,
interfaces, ADRs, and architecture. Allocate each draft to one later slice.
Use release-local R27/AC27 identifiers until HITL authorizes amendments to
locked global registers.

Produce the shared semantic taxonomy, current symbol inventory, draft
symbol-to-module ownership map, public-surface constraints, and proposed slice
allocation. Take no acceptance or implementation action.

### Slice 4 — architecture and code alignment

Review the Slice 3 architecture against accepted ADRs and current code.
Preserve the monolithic engine crate, private internal modules, root `Engine`,
root re-exports, and single-writer architecture.

Record what exists today versus what is net-new. Review high-level code
alignment and propose corrections to either code or architecture without
implementing them.

### Slice 5 — verification adequacy

Trace applicable needs to requirements, requirements to acceptance criteria,
and acceptance criteria to real tests. Review whether critical paths are known
and adequately tested, including:

- erasure and dependency closure;
- transaction atomicity and replay;
- public surfaces and feature gates;
- FFI conversion and hostile inputs;
- declarations, stubs, and shared wire fixtures;
- persistence and independent reopen;
- package installation and native loading; and
- platform-specific runtime behavior.

Identify vacuous tests and duplicated binding matrices. Allocate missing work
without writing tests.

### Slice 6 — stale documentation evidence

Review available repository evidence and transcripts for stale user-facing,
SDK-consumer, developer, and contributor documentation. Include stale release
claims, source-line citations, testing instructions, architecture ownership,
and refactor-experiment records. Propose exact dispositions without editing the
documents.

### Slice 7 — build and delivery failure evidence

Review available evidence for:

- environment and build failures;
- preflight and `agent-verify` failures or misuse;
- CI and gitleaks failures;
- packaging and installed-artifact failures;
- registry and publication failures; and
- runner, cache, disk, credential, and platform failures that delayed delivery
  after product code was working.

Separate pre-build failures from post-build delivery failures. Prefer focused
recurring-root-cause corrections and preserve the repository's intentionally
lightweight CI posture. Propose work and placement without implementing it.

### Slice 8 — reserved

Record no work and infer no authority.

### Slice 9 — proposal review and direct decisions

Consolidate Slices 0–8 into one proposal register. Score every item for:

- understanding: high, medium, or low;
- stability risk: critical, high, medium, or low;
- effort: XS, S, M, L, or XL; and
- disposition: include, postpone, reject, or needs more information.

Record every direct user-authorized ruling, rationale, and delivery slice.
Draft Slice 10, obtain an independent read-only subagent review, and perform at
most four documented FIX-n cycles. Update the release plan/state only after the
review findings are closed. Prework approval does not authorize tags,
publication, registry mutation, or feature implementation beyond the slices the
user actually commissioned.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.27:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 80** (`ENGINE-READ`) — engine read, search, graph, and evidence domains

**Remaining ladder:** 80 → 90 → 100 → 110 → 120 → 130 → 140 → 150.<!-- END GENERATED release-state:0.8.27:plan-immediate-next -->

## Slice ladder: features and refactoring

### Slice 20 — correction-safe source erasure

Reproduce and fix F27-01 without structural moves.

Create one table-driven Rust integration matrix covering:

- same-bucket replacement;
- cross-bucket original-first erasure;
- cross-bucket replacement-first erasure;
- already-complete closure;
- unaffected-source control; and
- a real incomplete blocker.

Give every content-bearing seeded record/store a unique sentinel; use keyed
identity and exact counts for content-free metadata. Derive exact
`ExciseReport` counts from the requested bucket's pre-erasure canonical and
row-owned projection inventory. Closed dependent deletion does not silently
change this published report meaning. After close and independent reopen, prove
absence of erased payload, logical/artifact/source-revision identities, and
live authority from canonical rows, both provenance axes, every
content-bearing projection, vector sidecars and physical vectors, telemetry,
and WAL-backed state. The raw non-PII `source_id` may remain only in the exact
accepted proof fields: one new `excise_source_audit` row for the committed
deletion (`record_key` and `payload_json.source_id`, with exact report counts),
no additional audit row on a scrub-only retry, and—when affected physical
dependents exist—one `source_bucket` closure row whose `root_value` is that
`source_id` (otherwise zero). Assert those exact row counts and content plus
exact unrelated survivors; no broad identity allowlist is permitted.

Pre-commit blockers remain atomic and roll back. Post-commit scrub failures
retain the accepted `ErasureIncomplete` contract: row deletion is already
durable, the pending at-rest obligation survives, retry completes it
idempotently, and no incomplete attempt reports successful closure. Memex's
exact characterization scenario and setup stay unchanged, but its old
`storage_failed` oracle is replaced by exact success receipt/count,
restart/visibility, physical-absence, and survivor assertions. Freeze that
revised consumer test through Slice 150. Python and TypeScript add only one
corrected-success smoke and one correctly mapped incomplete-result smoke.
Commit RED before production changes.

### Slice 30 — current inventory and comparison guardrails

Rebaseline at the release SHA:

- paths and line counts;
- real exported surfaces;
- top-level symbols;
- direct dependencies and co-change neighbors;
- focused test ownership;
- boundary maps; and
- the exception register.

Build a reusable surface comparator and self-test it against synthetic added
and removed Rust symbols, a changed re-export, a missing Python registration or
stub entry, a changed Node declaration, and a changed package export.

Derive surfaces from compiler or artifact introspection, not duplicated
hand-written lists. Bind the approved baseline to the release SHA and exact
feature sets; keep it immutable until Slice 150. File-size findings remain
advisory.

### Slice 40 — engine foundation and test seams

Keep `struct Engine` at crate root. Extract shared errors, identity, temporal
primitives, and subsystem-scoped test hooks in 300–1,200-line mechanical moves.

After each extraction batch:

- run root-path and feature-surface comparisons;
- test error conversion, identity normalization, and temporal boundaries
  through public seams;
- preserve every existing hook gate and root path exactly;
- prove feature-only hooks remain absent from default surface rows; and
- prove `test-hooks` and `slice72-test-hooks` builds expose only their existing
  narrow ordering/fault seams.

Do not test filenames or widen engine fields.

### Slice 50 — erasure, lifecycle, dependency, and provenance

Execute the independently reviewed
[`Slice 50 plan`](0.8.27/features/slice-50/plan.md) and
[`design`](0.8.27/features/slice-50/design.md). First close Slice 20 carryover
`TC-6acb0013-bba8-4fee-ac18-27c64442908a`: physical erasure must remove
`proving`/`incomplete` nonphysical soft-closure identity for every erased
revision after receipt validation while preserving physical proof rows. Then
move the corrected implementation and its record-lifecycle, provenance-
contract, and dependency-registration peers into private semantic modules.
Keep existing regression assertions locked. Preserve the published
observability `lifecycle` path and each operation's distinct validation,
transaction, drain/freeze, cursor, retry, and at-rest ordering.

Add focused coverage only for newly isolated dependency discovery,
already-complete closure, exact physical deletion counts, rollback, ordering,
and supported dependency-shape property/state-machine tests, including the
nonterminal carryover across both hard-erasure verbs. Do not invent deferred
multi-source semantics.

### Slice 60 — write, ingest, consolidation, and actuation

Execute the independently reviewed
[`Slice 60 plan`](0.8.27/features/slice-60/plan.md) and
[`design`](0.8.27/features/slice-60/design.md). They narrow actuation to
import rewiring (`actuation.rs` already owns that domain) and add a full-state
write-boundary characterization before the moves.

Organize validation, translation, execution, commit, provenance, extractor,
consolidator, and actuation boundaries while preserving transaction boundaries
and call order.

Inject failures at validation, translation, execution, auxiliary enrollment,
and commit boundaries. Compare canonical, provenance, registry, queue,
terminal, and projection state with the pre-call snapshot. Cover same-batch
ordering, replay/idempotency, caller-owned source identity, invalid input
causing zero mutation, and property tests for new codec or translation logic.

### Slice 70 — projection, embedding, and reranking

Organize projection runtime/generation/registry/maintenance, vector
partition/mean/equivalence, embedding, and reranking. Preserve commit ordering
and all feature gates.

Test projection and registry transitions as state machines: canonical commit
before publication, exact generation changes, recovery after interruption,
rollback without false readiness, and physical vec0 rows when publication is
claimed. Exercise applicable features separately. CPU/CUDA comparison uses a
justified numerical tolerance, not byte identity. CUDA evidence must prove
CUDA selection and computation.

Because the feature-gated embedding and reranking targets exercise this code,
Slice 70 also runs the feature-complete test gate,
`scripts/test-feature-complete.sh`, and records its summary.

### Slice 80 — read, search, graph, and evidence

Organize reader pool, frozen/page reads, filtering, search, ranking, graph, and
evidence. Split `graph_expand.rs` into types/validation, codec, and execution
only where current dependencies support the boundary.

Separate tests into codec round-trip/corruption properties, validation tables,
and real-database execution. Cover snapshot authority during concurrent
mutation, transaction lifetime, eligibility before every bounded cap, and
deterministic tie ordering. Use controlled physical fixtures and justified
SQL/plan assertions where result-only tests could miss an ineligible row
consuming a candidate window. Keep graph and evidence tests together until
behavior proves independent seams.

Carried from Slice 70 (`features/slice-70/status.md`):

- **Search-owned runtime fields.** Decide the owner of the four search-owned
  fields that Slice 70 left unchanged on `ProjectionRuntimeShared` (now in
  `projection_runtime.rs`): `search_limit_override`,
  `recency_reweight_enabled`, `importance_reweight_enabled`, and
  `vector_stage_only_for_test`. Moving them changes the struct's shape, so
  design review must approve it.
- **Slice 80 items.** These items are Slice 80's. Slice 70 kept them at
  root:
  - `fuse_rrf`, `fuse_three_arms`, `RRF_*`, and `RECENCY_WEIGHT`;
  - `apply_recency_reweight`, `apply_importance_reweight`, and their maps;
  - `branch_str` and `append_jsonl`;
  - the search-limit constants;
  - `vector_filter_*` and `read_search_in_tx`.

  Search keeps calling `rerank::try_rerank_fused`.
- **Per-batch check.** Every gate that scrapes engine source by file must run
  in each batch, not only at closeout. Slice 70 missed two:
  - `scripts/tests/test_windows_wal_attribution_ci_job.sh`, whose
    `function_body` matches only indented methods;
  - the slice35 manifest and audit.

  Before moving code, grep `scripts/tests` and `tests/` for `src/lib.rs`
  readers.

### Slice 90 — open, configuration, runtime, operator, and facade closure

Move open/probes, runtime configuration, WAL ownership, and operator
diagnostics after domain paths stabilize. Finish engine `lib.rs` as module
declarations, `Engine` state, core controls, and root re-exports, targeting
roughly 300–800 lines.

Use fresh-process tests for defaults and overrides, malformed configuration,
open/close/reopen, WAL ownership and busy/locking behavior, probe side effects,
error precedence, and faulted open/shutdown without orphaned runtime or WAL
state. Re-run root re-export and operator-feature comparisons after each
extraction.

Carried from Slice 70 (`features/slice-70/status.md`):

- **Configuration contract gap (ledger seq 258, `TC-b602d87a…`).**
  - **Code:** `PROJECTION_WORKERS = 2` and `DEFAULT_EMBED_TIMEOUT_MS` are
    fixed in `fathomdb-engine`.
  - **Bindings:** Python and TypeScript store `EngineConfig` without
    forwarding it, and NAPI `open` ignores `engine_config`.
  - **Contract:** the accepted `ADR-0.6.0-embedder-protocol.md` and the
    locked `dev/design/bindings.md` require a configurable embedder pool size
    and call timeout.
  - **Slice 90's job:** resolve the gap by implementing the forwarding or by
    proposing a successor ADR. Do not describe the fixed constants as
    satisfying the contract.
- **Open-path and runtime items.** These are Slice 90's. Slice 70 kept them
  at root:
  - `check_embedder_profile`, `default_embedder_identity`,
    `edge_vector_prune_complete`, and `prune_orphaned_edge_vectors`;
  - the embedder and reranker open gates;
  - `open_managed_connection` and `open_runtime_connection`;
  - `verify_embedder` and the other operator diagnostics;
  - `drain_embedder_events`;
  - the runtime constants.
- **Per-batch check.** The Slice 80 note on running source-scraping guards in
  every batch applies here too.

### Slice 100 — PyO3 binding decomposition

Decompose the binding into errors, types, facade, write translation,
read/search, graph/evidence, projection, and embedding/admin concerns.

Build and import an actual artifact. Compare registrations, names, signatures,
exception identity/mapping, stub alignment, and supported imports with the
Slice 30 baseline. Test blocking/GIL behavior with deterministic barriers.
Preserve one `_fathomdb` module and update `_fathomdb.pyi` in the same change.

### Slice 110 — napi-rs binding decomposition

Apply the same vocabulary where responsibilities match while retaining
language-specific conversion and async plumbing.

Build and pack/install the artifact in a clean Node consumer. Compare generated
exports/declarations, package exports, error envelopes, panic containment,
synchronous throw versus Promise rejection, hostile-string fixtures, wrong
types, overflow, and async responsiveness. Do not assert Rust file placement.

### Slice 120 — TypeScript SDK decomposition

Reduce `index.ts` to exports and thin `Engine` wiring, with domain logic under
read, write, search, graph, evidence, projection, and admin modules.

Add consumer compile fixtures importing from the package root and every
supported subpath. Compare declarations and runtime exports with the immutable
baseline. Use the shared wire/error fixture corpus and test exact names,
overloads, error precedence, wire shapes, and delegation. Tests must not import
new private modules.

### Slice 130 — Python SDK decomposition

Reduce `engine.py` to facade delegation and lifecycle wiring, using existing
public-domain modules and narrowly scoped helpers.

Test root and documented imports, contractual `__all__`, callable signatures,
exception identities, stub/type-checker agreement, and public examples or
doctests. Keep deep database semantics in Rust and use thin Python parity
checks. No test may depend on a particular helper file.

### Slice 140 — documentation and structural convergence

Converge architecture, design, interfaces, examples, and source citations on
the final taxonomy.

Validate that source citations resolve, documented public paths import,
examples compile or run, every file above 3,500 lines has an approved exception
or follow-up, every 2,000–3,500-line review has a recorded human verdict, and
the exception register covers every trigger. Do not make file size itself a
blocking automated test. The five named outliers must no longer remain
unexplained monoliths.

Carried from Slice 70 (`features/slice-70/status.md`):

- **Dual-runtime ADR status (ledger seq 259, `TC-7fed8d8d…`).**
  `ADR-0.8.23-dual-runtime-device-policy.md` and decision-index row 43 still
  read `proposed`. HITL `seq-250` and `seq-252` (0.8.23) required the shipped
  CPU/GPU runtime. Present a HITL status ruling; do not edit the status
  without one.
- **Unreviewed design memos.** `dev/design/embedder-decision.md` and
  `dev/design/0.8.1-slice-10-reranker-design.md` are still `UNREVIEWED`.
  Slice 70 treated them as informative history. Review them or mark them
  historical.
- **Slice 40 precedence edit.** The 0.8.25 Slice 40 dense-state table
  (`dev/plans/0.8.25/features/slice-40/design.md`) lists both "`failed` + no
  sidecar + no vec0 → failed" and "edge member not enrolled → corrupt".
  Production and the in-crate classifier test apply the `failed` row to an
  unenrolled edge. State that precedence in the table.
- **Test seams.** The `*_for_test` Engine seams that Slice 70 left at root
  (including the projection, vector, and embed seams, `projection_status`,
  and `mean_centering_internals_for_test`) fall under the existing Slice 140
  test-seam gating ruling.

Slice 140's intended hidden-surface differences, including the test-seam
gating, are captured at its landing commit as a successor hidden baseline with
a reviewed diff record against the previous baseline.

### Slice 150 — integrated release qualification

Write little or no new product test code. Re-run:

- the immutable public-surface comparator, together with the hidden-surface
  and test-inventory comparison against the current hidden baseline;
- the feature-complete test gate, `scripts/test-feature-complete.sh`;
- full `agent-verify`;
- full-workspace clippy and check;
- applicable feature combinations;
- package builds;
- isolated installed-artifact smokes;
- required native CPU/platform routes;
- warranted CUDA routes; and
- Memex's unchanged exact test against the candidate artifact.

Bind the candidate manifest to the reviewed commit, platforms, feature sets,
commands, test counts, and artifact hashes. A qualification defect returns to
the owning slice with a new RED test; never weaken a qualification test.

As its last step, after qualification fully passes, Slice 150 retires the
0.8.27 hidden-surface oracle (RH-12 in
`dev/plans/0.8.27/features/hidden-surface/plan.md`): record the final hidden and
test-inventory comparisons with manifest digests in that unit's `status.md`,
run `hidden_surface.py prune`, then delete `dev/tools/hidden_surface.py`, its
self-tests, fixtures, fast-tier line, and committed `baseline-*.json` files.
The feature-complete gate, the test-target coverage check, their shared
`scripts/lib/test_targets.py`, the committed feature matrix and skip
allowlist, the warning-free test-build gates, the dead-code lint, and the
removal-changelog gate stay. The retirement commit rewrites cadence step 4
and this slice's text to say the oracle ran and was retired, citing the
`status.md` digests. If the decomposition is abandoned or deferred, the same
unwind runs at that decision.

## Structural slice cadence

Slices 40–130 use this cadence:

1. Run focused tests at the pre-move SHA.
2. Add missing characterization coverage and prove it non-vacuous through
   temporary defect injection.
3. Make one mechanical move.
4. Run focused tests, the surface comparator, and the hidden-surface and
   test-inventory comparison against the current hidden baseline.
5. Refactor inside the new boundary, adding RED tests for new logic.
6. Run `agent-verify` and full workspace checks before closing the slice.
7. Preserve exact evidence and exclude unrelated cleanup.

The hidden-surface and test-inventory comparison is
`python3 dev/tools/hidden_surface.py compare`, run on a fresh capture of the
slice head against the `baseline-<sha>.json` under
`dev/plans/0.8.27/features/hidden-surface/` whose capture commit is the most
recent ancestor of `HEAD`. An unexpected hidden-surface difference blocks the
batch. An intended one is recorded as a successor `baseline-<sha>.json`
beside a reviewed `baseline-<sha>-diff.md`; no baseline file is rewritten.

Test-inventory policy: a removed or newly ignored test, or a test target that
no longer builds, blocks the batch unless the slice status names it with its
reason. An added test is expected when characterization coverage is added.

## Public interfaces and acceptance

Only Slice 20 intentionally changes behavior:

- `erase_source` includes already-closed dependents required for complete
  physical erasure;
- its report remains truthful and names the requested source bucket; and
- Memex needs no private database access.

A new method or report field is prohibited unless RED evidence proves the
existing report cannot represent the outcome. That decision returns to HITL
before implementation.

All structural slices preserve:

- schema version 34 and persisted/wire encodings;
- Rust root exports, `pub mod lifecycle`, and feature-gated benchmark paths;
- Python import paths, extension registrations, signatures, and typed errors;
- TypeScript exports, declarations, error codes, and JSON shapes; and
- default, operator, test-hooks, embedder, reranker, and platform boundaries.

Completion requires behavioral parity outside F27-01, shared semantic
vocabulary, reviewable module sizes, non-vacuous invariant testing, passing
source and installed-artifact verification, and a successful Memex
candidate-artifact witness.

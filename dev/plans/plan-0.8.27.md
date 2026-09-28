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
**IMMEDIATE NEXT: Slice 85** (`ENGINE-BOUNDARIES`) — engine carrier ownership and dependency-boundary enforcement

**Remaining ladder:** 85 → 90 → 100 → 110 → 114 → 115 → 120 → 130 → 135 → 140 → 150.<!-- END GENERATED release-state:0.8.27:plan-immediate-next -->

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

The post-closeout result-codec follow-up adds generated coherent
decode→encode→decode typed equality and positional evidence-corruption
properties. Both use human-defined invariants, kill their specified temporary
production mutants, and create no generated golden oracle.

### Slice 85 — engine carrier ownership and dependency-boundary enforcement

Settle the read-side carrier ownership and module dependency graph before the
runtime facade is reduced. This is a planned slice only: this section does not
commission implementation. The Slice 85 feature directory contains only the
durable architecture recommendation and prospective design-review records;
no implementation package exists yet.

**Design status.** A code-grounded review on 2026-09-27 found that the earlier
revision re-created prohibited cycles through handler-returned types, left
`Engine`-method calls, crate-root edges, and `Engine`-field aliases invisible
to the gate, and overstated compiler enforcement. This revision corrects
those defects. It must pass an independent design review before Slice 85 is
commissioned.

#### Final homes

Slice 85 establishes these final homes once; Slice 90 consumes them without
moving them again:

- new top-level `read_api.rs`: the current `read.rs` `impl Engine` facade;
- new top-level `graph_api.rs`: graph-neighbor, graph-expand, boundary, and
  every graph `Engine` test facade, including the root
  `explain_graph_neighbors_for_test` seam;
- existing `search_api.rs`: additionally owns `search_expand` and
  `search_expand_with_limit`;
- new leaf `structural_state.rs`: `structural_dependency_state`, so the
  remaining search-to-graph dependency is one-way;
- new leaf `reader_transaction.rs`: `begin_attributed_reader_tx`, below the
  pool and every read/search/graph handler, with its exact borrow lifetime,
  deferred-transaction behavior, attribution ordering, table-backed snapshot
  probe, and pause-hook order preserved;
- new top-level `wal_attribution.rs`: `WalAttributionCollector`, every
  `Reader*Pause` alias and reader pause carrier, and the related attribution
  roles, activity, phases, snapshots, and private helpers. `reader_transaction`
  depends on this owner; the collector and aliases move together, and Slice 90
  consumes rather than redesigns this boundary;
- `reader_pool.rs`: pool implementation, the request protocol and boxed
  envelopes, the pool-level response aliases (`ReaderResponse`,
  `EvidenceReaderResponse`), reader constants, and `CacheStatusReply` with its
  exact root re-export;
- `read.rs`: `PageReaderError` and its `From` impls, because the read
  handlers return and construct it;
- `search.rs`: `SearchReaderWork`, `FrozenQueryRuntime`, `EvidenceCapture`,
  `NoEvidenceCapture`, the search-only `SearchReaderError`, and the
  projected-text handler result. `read_projected_text_in_tx` returns
  `Result<SearchResult, SearchReaderError>` directly; no pool-owned response
  alias crosses into the handler. Before that ownership is final, filter
  validation, frozen-read validation, read handlers, and graph handlers stop
  consuming `SearchReaderError`;
- `filter.rs`: a narrow private snapshot-filter validation error containing
  only storage and invalid-filter outcomes. Each caller maps it into its local
  error without changing public error mapping or refusal precedence;
- `graph_expand/`: a narrow private search-expansion handler error containing
  only the cases its handlers actually produce. Its exact variants are fixed
  by characterization before the move; it must not depend on
  `search::SearchReaderError`; and
- `telemetry.rs`: `TelemetrySink`.

**Handler-result ownership invariant.** Handler modules never depend on
`reader_pool`. A private result or error carrier crossing from a handler to the
pool belongs to that handler or to an explicitly verified lower shared owner.
Pool protocol types may depend on those carriers. A newly named module is not
assumed to be a leaf: its transitive type and callable dependencies must prove
that placement.

All existing private fields remain private. Construction crosses boundaries
only through `EvidenceCapture::new`, `FrozenQueryRuntime::new`, named
`SearchReaderWork` constructors for hybrid, text-only, and evidence paths, and
pool-owned request factories. Those factories cover every direct facade/path
construction today: get-by-id, collection/list, canonical/operational page,
projected-text, hybrid/text/evidence search, graph-neighbor/search-expand,
crossed-boundary, `GraphExpandReaderRequest`, the cfg-gated baseline page, the
`tc5-benchmark`-gated `VectorStage` request, and the
`#[doc(hidden)]` `explain_graph_neighbors_for_test` request. The last two are
private typed pool capabilities/factories, not sender or `ReaderRequest`
exposure; their existing `tc5-benchmark` feature gate and `#[doc(hidden)]`
surface remain exact. Focused verification includes `tc5_vector_stage` under
`tc5-benchmark` and `slice20_graph_traversal::explain_plan_uses_indexes`
under its existing gate. The WAL seam receives typed cfg-gated worker-zero
controls for `HoldWalSnapshot`, `HoldWalSnapshotBounded`,
`HoldWalSnapshotWithCommitAck`, and the two inventory requests. No pool sender
accessor or generic sender exposure is permitted. Root ownership is allowed
only through an item-specific design-review exception proving durable ownership
and a stronger invariant that moving the item would violate.

The intended governed boundary is
`read_api/search_api/graph_api → reader_pool → read/search/graph_expand`,
`search → graph_expand`, `read/search/graph_expand → filter`,
`read/search/graph_expand → reader_transaction`,
`reader_pool/reader_transaction → wal_attribution`, and
`search/graph_expand → structural_state`. It eliminates all four Slice
80 cycles: `search` ↔ `graph_expand`, `read` ↔ `reader_pool`,
`graph_expand` ↔ `reader_pool`, and `graph_expand` ↔ `search_api`, and it
introduces no `search` ↔ `reader_pool` cycle. This is a boundary policy, not a
claim that this prose enumerates every permitted dependency or cycle in the
crate. Facade-to-handler type edges such as `read_api → read` and
`search_api → search/search_types/frozen_read/graph_expand` are expected.
The report-only phase freezes their exact item-level forms in the committed
policy file; that file, not this illustrative list, defines the expected edge
set used by AC27-85C. In particular, graph and read handlers do not depend on
`search::SearchReaderError`.

Slice 85 must not expand into whole-crate import normalization or force a type
or helper into an unnatural owner merely to make the graph visually simpler.
Earlier slices deliberately retained shared root helpers, wildcard navigation,
and subsystem-local cycles. The gate reports those facts, but enforcement is
bounded to the reviewed read/search/graph ownership boundary. Broad import and
navigation tightening remains Slice 140 work.

#### What enforces what

Rust privacy enforces field privacy and construction boundaries. Explicit
imports make most edges visible. Dependency direction is enforced by a small
`syn`-based AST gate in the normal `agent-lint` path; the compiler does not
prove the module graph.

The gate lives in `dev/tools/module-boundary-gate/` as a locked standalone
developer-tool crate with its own `[workspace]`; it is not a shipping crate or
an eleventh member of the root Cargo workspace. Its `syn` dependency disables
default features and enables exactly `full`, `parsing`, and `visit`; its direct
`proc-macro2` dependency enables `span-locations` for diagnostics. The crate
checks in its own lockfile, is covered by the repository dependency/license
gates, and is invoked by `scripts/agent-lint.sh` through a thin script that
reuses a cached binary only when its manifest, lockfile, or Rust sources are
unchanged. The implementation batch always updates the `AGENTS.md` §3 lint row
when the boundary gate joins `scripts/agent-lint.sh`. It updates the §2 root
workspace crate count only if a reviewed change makes the tool a root-workspace
member; no stale lint-content or crate-count exception is permitted.

Governed modules:

- forbid wildcard imports/re-exports and root-re-export indirection; where an
  item has a semantic owner (for example `errors`), internal code imports that
  owner rather than its root re-export;
- name semantic owners in internal imports;
- colocate inherent impls with their owned types, except the explicitly named
  `Engine` facade homes above;
- express cross-module behavior as module-qualified free functions by
  default. An owner trait is permitted only where receiver-based polymorphism
  provides a real benefit, never only to make the gate see an edge; on
  read/search hot paths it uses static dispatch;
- permit cross-module inherent methods only from a committed, reviewed list
  of owner-qualified constructors and true receiver operations (for example
  `EvidenceCapture::new`, `FrozenQueryRuntime::new`, and the pool request
  factories); and
- fail on macro-hidden boundaries unless an item-specific review records and
  tests the expansion boundary.

#### Gate semantics

The gate discovers the complete crate module inventory from source, then
enforces a bounded ownership policy. It does not infer that every discovered
module is Slice 85 work.

Extraction is whole-crate, enforcement is scoped. Extract edges from every
discovered module, including admitted and report-only modules, before
computing reachability. Follow outgoing and incoming executable root-item
references to a fixed point, including helper-to-helper chains through
non-governed modules. Never stop at the first out-of-scope endpoint. A
governed → outside → governed path must be visible even when neither outside
module is otherwise subject to import normalization. Unknown relevant edges
remain conservative candidates and block a claimed absence of a return path.

- **Module inventory and classification.** Recursively discover every inline
  and out-of-line `mod` declaration from `lib.rs`, including all declared
  `graph_expand` submodules, with source locations and `cfg` conditions. A
  committed classification accounts for every discovered module as governed,
  an admitted contract/external dependency, or out-of-scope but reported.
  Missing, duplicate, and stale classifications fail. The initial governed
  seed is `read`, `read_api`, `search`, `search_api`, `search_types`,
  `reader_pool`, `graph_api`, `graph_expand` and each of its declared
  submodules, `filter`, `frozen_read`, `reader_transaction`,
  `wal_attribution`, `structural_state`, and `telemetry`, plus the specific
  `dependency_closure` and `evidence` boundary edges named in the inherited
  allowlist and every root executable item reached from or calling that seed.
  The classification contains only modules actually discovered at that
  candidate; each planned module enters it in the same batch that declares its
  file. The report-only phase may not expand the seed automatically: any other
  new endpoint needs an item-specific contract admission or a reviewed plan
  change.
- **`Engine` field set.** Derive the complete `Engine` struct field inventory.
  Every discovered field has exactly one committed owner or named reviewed
  out-of-scope exemption; relevance is never inferred by omitting a field.
- **`Engine` method set.** Derive every method and defining module from every
  `impl Engine` block, including overlapping `cfg` forms. This map is
  source-authoritative and has no duplicate hand-kept manifest. Empty maps and
  multiple definitions active in the same checked configuration fail;
  mutually exclusive platform/feature definitions remain separate entries.
- **Cross-module inherent methods.** Derive every inherent method on a governed
  type whose visibility exceeds its owner module. The declarations must equal
  a reviewed owner-qualified list; stale and unlisted entries fail. `Engine`
  methods use the separate source-derived rule above.
- **Root contracts and exclusions.** A reviewed item list admits only the root
  contracts and narrowly justified contract edges needed by the governed
  boundary. Stale entries fail. Expected boundary edges are reviewed policy;
  the tool never rewrites them from a failing candidate.

The gate reports composition edges, contract/type references, ordinary owner
dependencies, `Engine`-field capability accesses, and callable references as
different edge kinds. Ordinary governed type-owner dependencies participate in
SCC calculation unless an item-specific contract admission says otherwise.
Admissions are limited to named type-only relationships, never a callable,
field access, or any edge needed to recreate one of the four forbidden cycles.
The initial named admission is the payload reference
`errors::EngineError::GraphExpansion → graph_expand::types::GraphExpansionErrorV1`:
classify `errors` as an admitted contract dependency, not a governed module or
an unexamined external leaf. Resolve the graph facade re-export to `types`.
Only that data reference is omitted from SCC edges; constructors, conversions,
Display/Error impl bodies and other executable references remain extracted.
Colocate the graph error's inherent/trait impls currently in `execution` with
its `types` definition; no admission can hide an execution dependency.
An added executable `errors → graph_expand::execution` edge must fail if it
creates a governed return path. The admission is stale-checked by item and
edge kind, not a blanket exemption for the errors module.
Re-export facades resolve to the defining item; their composition links must
not manufacture execution edges between graph submodules. Every admitted or
out-of-scope endpoint is reported. A path that leaves the governed set and
returns is reported as an unresolved boundary path and blocks closeout until
its intervening items are analysed or a reviewed type-only admission proves
there is no executable return path. This requires focused item inspection,
not import normalization of the intervening subsystem.

**Crate-root semantics.** `lib.rs` is the composition root, but root is not one
aggregate executable node:

- `mod` declarations, public re-exports, and `Engine` storage layout are
  composition metadata and do not participate in SCCs;
- each admitted root contract is an item identity, reported separately from
  executable dependencies; and
- each root free function or root-defined method body is its own executable
  item node. Calls and callable references to it participate in SCCs, so a
  real cycle through a root helper fails without inventing paths between
  unrelated root functions.

The enforced source grammar is deliberately smaller than Rust's semantic
model:

- grouped, aliased, and re-exported imports and module-qualified paths resolve
  to their defining owner; globs fail in governed modules. Outside that set,
  conservatively expand a glob to every cfg-visible candidate item in its
  imported namespace for name lookup, preserving edge kinds and re-export
  origins. Report the glob import itself, but derive possible executable
  edges from actual names/paths used in bodies, not a fabricated call to
  every unused imported function. Ambiguous
  glob uses retain all candidate edges; unresolved namespaces or macro-hidden
  edges on a possible governed return path block closeout, never disappear.
  Unrelated outside ambiguity is reported without forcing its cleanup;
- every governed `Engine` field access is an edge, including non-call access,
  references, dereferences, simple aliases, and closure/thread capture. An
  unresolved matching field name is a conservative edge and its diagnostic is
  labelled as such;
- `self.m(...)`, `Self::m(...)`, `Engine::m(self, ...)`, typed `Engine`
  bindings, and simple reference/dereference/parenthesized aliases resolve
  through the source-derived method map with lexical shadowing respected;
- callable references such as `let f = Engine::search_inner` and
  `let f = root_helper` create the edge when the item is referenced, even if
  invocation occurs later; and
- a governed module may not define an unlisted local free function whose name
  shadows a governed owner-qualified callable. The gate resolves declarations
  before calls, reports both locations, and requires an owner-qualified call
  or a reviewed local adapter name; this catches wrappers such as the current
  graph-local `validate_filter_attributes_on_snapshot` instead of attributing
  them silently to `filter`; and
- an unresolved dot call whose name belongs to a governed cross-boundary
  method fails with a request for an owner-qualified call or a syntactically
  resolvable receiver. External same-name methods are never silently assigned
  to a governed owner.

The gate therefore emits an exact graph **under the enforced source grammar**,
not an exact Rust semantic call graph. Configuration inputs are explicit
compiler-reported target cfg values and Cargo feature closures, not guessed
Rust type information. Evaluate `cfg`/`cfg_attr` against each recorded input;
unsupported predicates, generated source, and unresolved includes fail with a
named resolution requirement. Compute separately labelled default,
`test-hooks`, `tc5-benchmark`, and test-build graphs, including applicable
feature combinations and the Linux/non-Linux arms. A union report does not
replace these results. Production call sites in test builds are still checked;
test bodies have a separately reported test policy preserving existing test
identities rather than forcing domain ownership on the harness.

Within the governed boundary, the allowlist is shrink-only. None of the four
Slice 80 cycles is eligible for retention, exception, or allowlisting. Only
the three inherited boundary cycles are initial narrow candidates:
`search` ↔ `dependency_closure`, `search` ↔ `evidence`, and
`graph_expand` ↔ `evidence`. Other whole-crate cycles remain visible in the
inventory report but are not silently added to this allowlist or pulled into
Slice 85.

#### Why not rust-analyzer SCIP

A rust-analyzer SCIP trial is rejected as the gate, primarily because it was
incomplete. It resolved a same-line dispatch but missed formatting-equivalent
multiline dispatches and the `search_inner` traversal edge, and its indexing
output was operationally noisy. It therefore could not provide a stable,
authoritative graph.

Cost was secondary and would have been tolerable had it been correct. On the
measured host:

- engine indexing took about 6.8–6.9 seconds, 1.37 GB peak RSS, and an 11 MB
  index;
- workspace indexing took about 8 seconds, 1.386 GB, and a 14 MB index.

The tooling is also absent by default.

#### Implementation order

Once commissioned, Slice 85 runs in ordered batches. Each move batch contains
one semantic ownership cluster and runs the working graph report plus its
focused behavior tests.

1. Capture the exact baseline required by R27-85F.
2. Build the gate in four independently reviewable RED/GREEN sub-batches:
   source discovery/classification; imports and item-level root graph; Engine
   fields/methods and callable references; inherent-method policy plus negative
   fixtures. The first sub-batch creates the standalone tool crate, lockfile,
   lint hook, dependency/license coverage and cache-staleness test. Run it
   report-only against production.
3. Move one lower owner per batch: `structural_state`, then
   `wal_attribution`, then its consumer `reader_transaction`.
4. Characterize and separate the narrow handler errors first, preserving exact
   mappings and precedence; then add search constructors; then move one pool
   request family per batch.
5. Move one facade family per batch: read, then graph, then search expansion,
   eliminating the four prohibited cycles.
6. Switch the production gate from report-only to enforcement, and perform
   closeout verification.

Import normalization is explicit work inside stages 3–5: each moved owner
lands with explicit imports, and stage 5 includes separate batches for every
remaining governed file (`read`, `search`, `search_api`, `search_types`,
`reader_pool`, `filter`, `frozen_read`, `telemetry`, and every `graph_expand`
file). Remove all `use super::*` and other globs, including their test imports
unless the separately declared test policy admits that exact harness import.
Use one module per batch, normally 300–1,200 mechanical changed lines; split
larger modules by import/owner cluster without mixing behavior changes.
Run compile, focused tests and the report after each batch; stage 6 requires
zero unapproved governed globs. The `dependency_closure`/`evidence` inherited
edges and `errors` payload admission govern named edges, not their entire
modules: those modules retain report-only/conservative extraction outside the
named edges and need no blanket `use super::*` rewrite in Slice 85.

AC27-85D additionally requires whole-crate edge extraction and fixed-point
root reach as specified above. AC27-85E additionally requires a two-helper
root chain, a return path through two report-only modules (one glob-only),
and a path through an admitted module, each with the tested edge as the sole
reverse edge. It also requires a local free-function shadow fixture whose only
reverse edge passes through the shadowing wrapper. Positive fixtures retain an
unrelated outside glob and the named `EngineError` payload admission; negative
mutants add executable graph calls behind that admission, ambiguous-glob
return paths, and an unlisted shadowing adapter. No fixture may pass merely
because its intermediate module is outside the governed seed.

The gate follows characterization-first TDD; carrier and facade moves follow
mechanical RED/GREEN compile cycles. Gate and error batches target roughly
300–600 non-mechanical changed lines and split above 800 unless a specific
cohesion reason is recorded. Pure moves may be larger, but each contains one
ownership cluster and no behavior change. The narrow error work is not called
a verbatim move; its RED tests cover every existing mapping and refusal order.
Resolve report-only edges before freezing the enforcement policy. A named
inherited cycle allowance authorizes only its reviewed directed edges, not an
entire SCC or a new path through an allowed module. Unknown paths and unlisted
edges fail. Normal lint uses the cached gate binary and fast fixtures; record
warm runtime/RSS and keep the gate free of whole-workspace indexing and
production mutation runs. Independent code review and independent read-only
verification bind the closeout to the exact candidate, as for prior slices.

#### Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-85A | Every root-kept carrier receives durable non-root semantic ownership without field widening, rooted-contract drift, or invented leaf ownership. | AC27-85A: `ReaderWorkerPool`, `SearchReaderWork`, every reader request/response carrier, `FrozenQueryRuntime`, reader errors/constants, `WalAttributionCollector`, every `Reader*Pause` alias and related attribution helper, `TelemetrySink`, `EvidenceCapture`, and `begin_attributed_reader_tx` each move to a named non-root semantic owner. `PageReaderError` is read-owned; projected text uses the direct search result; filter validation and graph search expansion use characterized narrow errors rather than `SearchReaderError`. Handler modules never depend on `reader_pool`, and every proposed shared leaf has its transitive dependencies verified. All previously private fields remain private and every rooted public/re-export path is exact. Any item retained at root has an item-specific design-review exception proving durable ownership and the stronger invariant that moving it would violate. |
| R27-85B | Facades and handler results do not create reverse handler dependencies. | AC27-85B: all four Slice 80 cycles—`search` ↔ `graph_expand`, `read` ↔ `reader_pool`, `graph_expand` ↔ `reader_pool`, and `graph_expand` ↔ `search_api`—are absent from every applicable governed configuration graph; no `search` ↔ `reader_pool` or other new governed cycle appears; and graph/read/filter/frozen-read code does not depend on `search::SearchReaderError`. |
| R27-85C | Remaining governed dependencies are explicit and minimal without expanding Slice 85 into whole-crate normalization. | AC27-85C: source discovery classifies every crate module, while a committed policy file frozen after report-only accounts for every expected item-level edge in the reviewed read/search/graph boundary under the enforced source grammar; prose examples are non-authoritative. Composition, contract, ordinary owner, capability, and callable-reference edges remain distinguishable. Root executable items participate individually in SCCs; admitted root contracts do not invent executable paths. None of the four Slice 80 cycles is eligible for retention, an exception, or an allowlist. Only `search` ↔ `dependency_closure`, `search` ↔ `evidence`, and `graph_expand` ↔ `evidence` are initial candidates for the narrow, shrink-only boundary allowlist. Other crate cycles are reported without automatic scope expansion. An edge absent from the frozen policy fails with source, destination, item identities, edge kind, configuration, and location. |
| R27-85D | Dependency direction is enforced by a normal-lint AST gate over explicit ownership; Rust privacy enforces field and construction boundaries. | AC27-85D: normal `agent-lint` runs the locked standalone `dev/tools/module-boundary-gate` crate with the specified minimal parser features and a tested stale-cache rebuild rule. The source-derived module inventory has a complete reviewed classification; the source-derived Engine field inventory has an exact owner/exemption map; the Engine method map is derived only from source and is nonempty and cfg-consistent; externally visible inherent methods equal their reviewed owner-qualified list; and root contracts/exclusions are exact and stale-safe. The gate extracts whole-crate edges, records field access and callable references as well as calls, follows item-level executable root reach to a fixed point, rejects governed globs and conservatively handles outside globs, rejects unresolvable relevant receivers, external-name ambiguity, root-re-export indirection, and unreviewed relevant macros, and emits separately labelled default, test-hooks, tc5-benchmark, and cfg(test) graphs plus SCC diagnostics. |
| R27-85E | The structural gate is non-vacuous across its enforced source grammar. | AC27-85E: compiled fixtures cover grouped/aliased/re-exported imports, glob rejection, same-line/multiline parity; direct, non-call, dereferenced, typed-binding, simple-alias, closure-captured, and conservative-unknown-base Engine fields; `self`/`Self`/`Engine` method calls, typed bindings, simple aliases, callable references, unresolved-alias rejection, and a local free-function shadow; listed qualified constructors/receiver operations, unlisted declarations, duplicate names, external same-name calls, and stale list entries; admitted root contracts, composition-only root metadata, a sole reverse edge through one executable root helper, a transitive leave-and-return path, and unrelated root helpers; nested and cfg-gated modules plus missing/stale/empty classifications and maps. Each edge family has a compiled negative fixture whose tested edge is the only reverse edge. Independent production mutants exercise each extractor and the four former cycles, fail with the exact edge/policy diagnostic, and restore the source exactly. Fast fixtures run in normal lint; production mutation qualification runs when the gate changes and at closeout. |
| R27-85F | The boundary change preserves behavior and surfaces from a verified exact baseline. | AC27-85F: before any move, a capable host with the required free space, native module, and GPU tooling must pass exact-baseline `agent-verify`, the candidate-bound native receipt, and official public and hidden captures. After the move, focused read/search/graph/evidence/reader/WAL routes, applicable feature builds, source-scraping gates, exact public surface, additive-only hidden surface, and runtime receipts match that baseline except for reviewed structural inventory additions. The request envelope remains within its size bound; transaction lifetime, attribution finish order, pool-before-profile-context drop order, worker-zero WAL pinning, cfg gates, and qualified test identities are unchanged. |
| R27-85G | Security evidence is not overstated. | AC27-85G: Slice 85 makes no current-HEAD or final-candidate AC-037 claim. The `66e27983` and `24813b8e` runs remain historical or diagnostic only; Slice 150 alone owns exact-final-candidate live qualification. |

Slice 90 may start only after these boundaries are reviewed, verified, and
recorded as settled.

### Slice 90 — open, configuration, runtime, operator, and facade closure

**PLANNED; uncommissioned.** The complete prospective ownership, requirements,
acceptance, and ordered batch contract is
[`Slice 90 design`](0.8.27/features/slice-90/design.md). It supersedes the
optional placement wording in earlier handoffs. All work promised below must
be completed and independently verified within Slice 90 before Slice 100;
there is no Slice 91 allocation. Internal batches provide sufficient isolation
without a new ladder dependency. A later discovery requiring a scope change
blocks closeout and requires a reviewed plan change, not silent deferral.

Move open/probes, runtime configuration, WAL ownership, and operator
diagnostics after domain paths stabilize. Finish engine `lib.rs` as module
declarations, `Engine` state, core controls, and root re-exports, targeting
roughly 300–800 lines. That range is advisory, not a correctness gate: Slice
90 must not move a helper, carrier, error, or runtime field into an awkward
owner merely to reduce root line count or make the dependency diagram tidier.

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
  - **Contract:** accepted `ADR-0.6.0-scheduler-shape.md` additionally
    requires distinct Tokio orchestration and embedder pools, bounded tasks
    and writer-channel handoff. `ADR-0.6.0-embedder-protocol.md` requires
    engine-owned dispatch and a timeout for every embed; the current
    mutex-serialized projection/watchdog threads and direct query calls do
    not implement that topology. The bindings symmetry contract does not
    turn NAPI's host executor into an engine-owned pool.
  - **Slice 90's job:** implement and verify the accepted runtime contract
    end-to-end, in a separate RED/GREEN batch before mechanical moves. A
    merely proposed successor ADR does not close the gap. Only an accepted
    successor with its implementation and tests completed within Slice 90
    can replace that obligation. HITL decision `seq-293` rules
    `D27-runtime-topology` as Option B: preserve the synchronous primary-writer,
    projection-worker and `commit_gate` ownership model, and codify a narrow
    successor with real engine-owned orchestration/embed-dispatch capacities,
    universal deadlines, operation-specific timeout outcomes and bounded
    shutdown. The literal historical Tokio/task/dedicated-writer topology is
    not selected.
    The dedicated design records current substrates/widths, exact consumers,
    item-specific owners and required effect tests. The revised
    `features/slice-90/option-b-successor-adr-scaffold.md` is the reviewed
    minimum ruled-direction contract for B, pending independent design approval
    and formal successor-ADR codification: it covers open-time equivalence,
    projection/query/direct dispatch, operation-specific fallback, bounded
    admission, recovering hung slots, safe database quiescence followed by an
    independent embed-runtime drain budget, stage-specific projection retry
    accounting, same-snapshot frozen fallback, exact projection-row capacity,
    current binding input shapes and clause-level ADR supersession. This is
    cancellation-before-join, not deferred cancellation after database drain.
    Failed-open cleanup uses the same provider-only retention exception;
    successful degraded open retains a live engine. No reaper thread is added,
    repeated close does not restart the shared drain budget, and historical
    PR-9 terminal-failure expectations are explicitly amended where fixed
    hung-slot accounting instead leaves durable pending work. This is
    architecture correction plus forwarding, not
    forwarding alone. No Slice 91 technical boundary has been demonstrated;
    runtime qualification precedes moves within 90 unless an explicitly
    reviewed dependency requires a ladder change.
  - **Configuration documentation action:** in the functional configuration
    batch, update the accepted successor and decision index, internal engine/
    scheduler/embedder/bindings designs, Rust/Python/TypeScript interfaces,
    `docs/reference/config.md`, and affected error guidance. Document each
    setting's spelling, default, unit, accepted range, zero/omission meaning,
    mutability, precedence, consuming component, backpressure and observable
    error/fallback behavior. This truth must land in Slice 90, not be deferred
    to Slice 140. Slice 114 later audits that documented behavior across every
    engine component; it does not defer or reopen Slice 90's functional runtime
    contract.
  - **Batch-fallback deadlock action:** before executor changes, add a bounded
    RED test for `embed_projection_batch` calling `per_job()` from its
    returned-error/timeout path while `embed_serialize` is held
    (`fn embed_projection_batch` in
    `src/rust/crates/fathomdb-engine/src/projection_worker.rs`). Resolve every
    route so the guard or new
    executor permit is dropped before per-job fallback; require a mutant that
    restores the under-guard fallback to fail. Do not let the executor rewrite
    erase the investigation without proving the replacement lock order.
    Test the breaker-open fast-failure route separately; it is not the same
    reacquisition witness. Bound the mutant independently of Engine Drop.
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

Consumed from Slice 85 after its boundaries are settled:

- the non-root semantic owners of the reader carriers, `TelemetrySink`,
  `EvidenceCapture`, and `begin_attributed_reader_tx`, with private fields and
  rooted contracts preserved (or an item-specific reviewed root exception);
- the characterized narrow filter and graph-handler errors, with their public
  mappings and refusal precedence preserved; Slice 90 consumes these owners
  without merging them back into search or moving them for facade convenience;
- the facade/handler separation that removes all four Slice 80 cycles; and
- the normal-lint dependency-direction gate: complete source inventory,
  bounded governed read/search/graph policy, exact graph under its enforced
  source grammar, item-level root executable nodes, reviewed contract and
  out-of-scope classifications, and the narrow three-cycle boundary allowlist.
  None of the four Slice 80 cycles is eligible for retention or exception.

Slice 90 must consume those boundaries, not redesign them while extracting the
runtime facade. When Slice 90 adds or relocates a runtime owner, it updates the
classification and ownership policy only for the modules and root executable
items it touches, then reviews the resulting governed graph. It does not turn
the Slice 85 gate into whole-crate import normalization, absorb unrelated
cycles, or auto-expand an allowlist. Broad import/navigation cleanup remains
Slice 140 work. The following runtime-owned work remains in Slice 90:

- **Reader-loop runtime arms.** The inline WAL and diagnostic match arms
  (`HoldWalSnapshot*`, `LookasideStatus`, `CacheStatus`,
  `SecureDeleteStatus`, and `Wal*Inventory`) currently live with
  `reader_worker_loop`; they remain reader-owned connection operations in
  `reader_pool`. Slice 90 closes their ownership review and tests the typed
  capabilities, worker-zero routing, exact connection lifecycle, and pause
  ordering. Engine-facing WAL orchestration moves to `wal_runtime`.
- **Open-path helpers.** `configure_reader_lookaside`,
  `apply_perf_experiment_reader_pragmas`, and `Engine::usable_dense_runtime`
  move to `connection_runtime` and `embedding` respectively.
- **Index projectors.** `index_projector` owns `project_canonical_node_row`,
  `project_canonical_edge_row`, `IndexTargetSet`,
  `index_targets_for_row_kind`,
  `reproject_search_index_after_tokenizer_upgrade`,
  `search_index_tokenizer_reproject_complete`, `CanonicalNodeRow`,
  `canonical_node_rows`, and `row_kind_from_column`.
- **Search-owned runtime fields.** The four search-owned
  `ProjectionRuntimeShared` fields remain physically on that shared runtime
  allocation, with each exact field and reason recorded in the design.
  Slice 90 verifies defaults, atomic ordering, lifetime, and existing search
  test seams; it does not duplicate the values or move them solely for naming.
- **Test-gate carry-overs (Slice 80 post-hoc test review).**
  `slice60_fix1_wire`'s negative scan names its files explicitly; any new
  `graph_expand/*.rs` file must be added to it. Slice 85 supplies the scripted
  dependency-direction check; Slice 90 keeps it green.
  A green non-Linux build of the moved `graph_expand/execution.rs` arm is
  required by Slice 90 closeout, alongside the configuration/feature matrix in
  the dedicated design. Slice 150 still owns final-candidate qualification.

### Slice 100 — PyO3 binding decomposition

**PLANNED; uncommissioned.** Execute the prospective
[`Slice 100 design`](0.8.27/features/slice-100/design.md) after Slice 90's
configuration handoff is complete. Decompose native errors, FFI execution,
carriers and semantic operations while retaining one `_fathomdb` initializer
and one native Engine identity. Exact source/registration inventories,
GIL/lifetime/error tests, immutable-plus-reviewed-delta surface comparisons,
fresh installed-wheel evidence and independent review/verification are exit
requirements. Python SDK decomposition remains Slice 130. Zero open Slice
100 obligations is a prerequisite for Slice 110.

### Slice 110 — napi-rs binding decomposition

**PLANNED; uncommissioned.** Execute the prospective
[`Slice 110 design`](0.8.27/features/slice-110/design.md) after Slice 100 closes.
Use shared vocabulary only where responsibilities match; preserve NAPI's
language-specific conversion, registration and async boundaries. The design
requires exact native/runtime/declaration inventories, resolved executor and
subscriber contracts, async/lifetime/FFI proof, and an installed thin-main plus
platform-binary package pair. Production declaration generation and runtime
exports are separate oracles. Zero open Slice 110 obligations unblocks Slice
120. No Slice 111 is allocated: bounded sub-batches provide sufficient review
and verification boundaries within 110.

### Slice 114 — engine configuration, constants, and documentation audit

**PLANNED; uncommissioned.** After the native binding decompositions, perform a
component-by-component audit of Rust engine configuration, setting consumers,
constants, and variables. Identify unused constants and variables; classify
each as retained with a documented reason, removed, or assigned to a separately
reviewed follow-up. Do not use the audit as authority for opportunistic behavior
changes.

For every user- or operator-relevant setting, record its owner, spelling,
default, unit, effective precedence, mutability, consumer, and observable
failure or fallback behavior. Where a bounded value is applicable, propose a
default and accepted range with evidence and state explicitly when no safe range
can yet be justified. Reconcile those records with Slice 90's runtime contract,
Rust/Python/TypeScript interfaces, and public configuration guidance. Any
recommended setting change requires its own reviewed behavior and performance
evidence before adoption.

### Slice 115 — engine performance data collection and lightweight profiling

**PLANNED; uncommissioned.** On the refactored Rust engine, collect repeatable
performance data and lightweight profiling evidence for representative
open/close, write/ingest, projection/embed, search, graph, evidence, and
erasure paths. Record the exact candidate, hardware and software environment,
workload shape, warm-up, repetitions, collected metrics, profiler method, and
known measurement limits.

This slice characterizes the release candidate; it does not tune by anecdote or
silently alter feature/function. It must preserve raw or reproducible receipts
and identify any regression or profile hotspot that needs a separately reviewed
remediation. Its data and methods become the input to Slice 135's full
0.8.26 comparison.

### Slice 120 — TypeScript SDK decomposition

Consumes the complete, reviewed native substrate and installed-package
receipts from Slice 110. No native conversion, callback/executor, error,
registration, declaration-generation or packaging obligation is deferred here.

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

### Slice 135 — 0.8.26 performance preservation and improvement qualification

**PLANNED; uncommissioned.** Compare the completed 0.8.27 candidate against
0.8.26 using a fixed, representative workload and the Slice 115 collection
method. Verify that performance matches or improves on 0.8.26 without reducing
any supported feature or function. Hold workload, dataset, feature set,
configuration, hardware, software environment, warm-up, and repetition policy
constant unless the comparison explicitly documents and justifies a difference.

Pre-register the metrics, tolerances, and decision rule before the comparison.
Publish reproducible receipts and an explicit feature/function parity inventory.
Any regression, inconclusive result, or attempted feature trade-off blocks
closeout until a separately reviewed resolution preserves the release contract.

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

Carried from Slice 80 (`features/slice-80/design.md`, post-hoc design review
2026-09-27). These test seams moved out of `lib.rs`, or already lived in
`graph_expand.rs` and moved with its split. Slice 140's test-seam inventory
and gating ruling must cover them at their new owners:

- **`reader_pool.rs`:** the pool methods `wal_connection_inventory_for_test`
  and `wal_native_state_inventory_for_test` (`pub(crate)`).
- **`search.rs`:** the `test-hooks` search witnesses
  `append_json_witness_for_test`, `record_fts_route_for_test`,
  `slice71_search_statement_trace`,
  `record_slice71_profile_statement_for_test`, and
  `record_fts_query_plan_for_test`.
- **`filter.rs`:** the `Filter` methods `to_search_filter_for_test` and
  `lower_for_read_list_for_test`.
- **`graph_api.rs`:** all graph `Engine` seams, including the root
  `explain_graph_neighbors_for_test` seam and
  `measure_graph_expand_for_test`,
  `seed_graph_expand_dependency_closure_for_test`,
  `seed_graph_expand_nonterminal_dependency_closure_for_test`,
  `seed_graph_expand_erasure_for_test`,
  `seed_graph_expand_projection_state_for_test`,
  `graph_expand_current_rss_samples_for_test`,
  `graph_expand_isolated_process_rss_samples_for_test`,
  `graph_expand_with_rendezvous_for_test`,
  `graph_expand_with_projection_state_for_test`,
  `graph_expand_with_statement_count_for_test`,
  `graph_expand_with_projection_generation_for_test`, and
  `explain_graph_expand_for_test`.
- **`graph_expand/execution.rs`:** only its private non-`Engine` helpers
  `measure_isolated_process_rss_arm_for_test` and
  `seed_graph_expand_rss_fixture_for_test`, plus the `*ForTest` carriers
  `GraphExpandRetentionCountersForTest`, `GraphExpandRendezvousForTest`,
  `GraphExpandMeasurementForTest`, `GraphExpandCurrentRssSampleForTest`,
  `GraphExpandIsolatedProcessRssSampleForTest`,
  `GraphExpandProjectionStateForTest`,
  `GraphExpandProjectionGenerationForTest`, and
  `GraphExpandReaderControlsForTest`.
- **`graph_expand/types.rs`:**
  `graph_expansion_degradation_codes_for_test`.
- **Root wrappers:** the root `pub fn *_for_test` wrappers still in
  `lib.rs` now call these moved `pub(crate)` items. They include
  `vector_phase1_sql_for_test`, `slice35_ranked_eligibility_sql_for_test`,
  and `take_slice71_search_statement_trace_for_test`.

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

As a Slice 150-owned gate on the exact final candidate, run strict security through
`dev/release/ac-037-live-netns-hitl-runbook.md`, capture the required live
AC-037 pass/catch/summary lines plus grant and revert evidence, and bind that
receipt to the candidate SHA. The historical `66e27983` run is not substitute
evidence for this qualification gate.

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

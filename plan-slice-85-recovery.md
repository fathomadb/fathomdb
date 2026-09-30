---
title: Slice 85 recovery plan
status: IN_PROGRESS
target_release: 0.8.27
date: 2026-09-29
branch: slice-85-fix
starting_sha: df1ffd000e1ff734e6b7068597388ac7d56e2b5c
execution_status: in progress
reviewed_on: 2026-09-29
---

# Slice 85 recovery plan

## Objective and authorization

Preserve Slice 85's engine ownership changes, narrow errors, and removal of
the four inherited dependency cycles. Reduce the developer gate to a bounded
syntactic architecture check that is inexpensive to understand and maintain.
After removal and cleanup, finish the bounded engine review and qualification
work described in Phase B. Admit implementation work only where the reviewed
benefit is at least 4/5: protected correctness, an unsafe-lifetime regression
oracle, or reliable acceptance evidence. Cosmetic preferences and speculative
language coverage do not meet that threshold.

The owner authorized implementation of this plan on 2026-09-29. Work starts
from the committed plan at `c64fad7b6`; the removal and remaining-engine phases
are authorized. Landing, release rebinding, pushes, tags and publication remain
outside this instruction. The final receipt records completed work and limits.

## Starting point

- Worktree: `/home/coreyt/projects/fathomdb-worktrees/s85-fix`.
- Branch/head: `slice-85-fix` at the `starting_sha` above; clean when assessed.
- Review branch base: `b4e90afe7`, the original Slice 85 closeout binding.
- Pre-implementation baseline: `8b2a9edaf`.
- Release-state candidate: `7a2f9bf90783f545603516502bac0016d4b93a14`;
  the recovery branch is not bound as a landed candidate.
- Current gate: 6,548 Rust source lines; policy: 4,691 lines.
- Policy contains 4,083 frozen edges, 233 receiver exceptions, 42 frozen
  module-cycle/SCC entries, and 82 frozen inherent declarations.
- The last recorded mutation receipt describes 240 assertions and about
  9.2 minutes. It predates the stopped commits and is not a HEAD measurement.

Recover forward from the current head. Do not reset to the review base or a
cycle checkpoint: that loses useful corrections and retains substantial
inventory machinery. Use historical commits only to recover specific code
or understand a retained fix.

## Work to preserve

- Carrier ownership, typed reader-request factories, facade moves, and the
  `structural_state`, `wal_attribution`, and `reader_transaction` owners.
- Handler-owned narrow errors and unchanged public error mappings and refusal
  precedence; no handler dependency on `reader_pool`.
- Graph error-impl colocation, private retention counters and their accessor,
  explicit semantic-owner imports, and removal of unused root filter exports.
- Real filter error-route tests and the corrected feature-gated source scrape
  that follows moved seams into `graph_api.rs`.
- Macro-body extraction, capitalized paths, ordinary type/struct/pattern
  references, source-derived Engine field/method ownership, and diagnostics.
- Distinct executable root-item identities and transitive helper reachability
  through reported/admitted modules; do not collapse these into a root node.
- Descendant-aware forbidden dependencies, forbidden cycles, and the required
  forbid floor, including the new search/reader-pool cycle prohibition.
- Wrapper build/execute directory alignment and manifest/lockfile freshness.
- Cheap fail-closed checks for unknown relevant cfg predicates, relevant
  unparsed macros, source-splicing `include!`, `extern crate`, and `#[path]`.
- Existing public contracts, schema, SQL, ordering, transaction lifetime,
  attribution, teardown order, feature gates, and test identities.

The accepted runtime-topology ADR predates Slice 85 implementation and belongs
to Slice 90. Preserve it. Preserve the older 0.8.25 Slice 85 manifest checker
and tests; their similar names do not place them in this recovery scope.

## Removal scope

| Removal | Replacement or retained responsibility |
| --- | --- |
| 4,083 frozen `edge` records and their comparison/regeneration machinery | Extract graphs for boundary verdicts; retain source locations, not an exact census of ordinary edges. |
| 233 `external-receiver`/`typed-receiver` entries and general receiver inference | Recognize Engine receivers and basic aliases; use explicit owner/import/type dependencies for other boundary checks. |
| 42 `module-cycle`/`module-scc` records and freeze checks | Retain prohibited-cycle detection; broad SCC reporting may remain only if inexpensive and useful. |
| 82 frozen `inherent` records and exact declaration equality checks | Retain source-derived method ownership and specific carrier/construction assertions. |
| General Rust namespace resolver and advanced laundering semantics | One compact resolver for the engine-used grammar; reject unsupported owner-bearing indirection. |
| Consumer profiles, ML feature cross-products, 512-slot capacity machinery, and configuration-expression minimization | Small reviewed configuration space tied to actual conditional boundary edges, validated against the manifest. |
| Mutation variants and 36 compiling-sibling duplicates | One representative per independent mechanism; replace a weak original rather than retaining duplicate siblings. |
| Production mutation qualification in the fast tier | Explicit gate-change/closeout qualification; retain the normal-lint production check and cheap fixtures. |
| Repeated cycle bookkeeping and accumulated plan clarification paragraphs | One current contract and one recovery receipt, with compact historical evidence. |
| Unused `pub(crate)` exposure of `graph_expand::traversal` | Private module with its existing named re-exports. |

The four policy inventories account for 4,440 rows, about 95% of the current
policy. This is a measured deletion opportunity, not a promised final code
size or runtime. Measure the implemented result once at closeout.

## Review findings and blast radius

This review inspected the implementation and its callers; it did not execute
the proposed replacement gate. The eight findings below are addressed in the
execution steps. Code feasibility and coverage remain future qualification
obligations, not verified results of this plan review.

| ID | Finding | Plan remedy |
| --- | --- | --- |
| F1 | Receiver policy supplies actual method targets; deleting its entries before replacing inference breaks graph construction or makes the intermediate state falsely incomplete. | Retire receiver behavior, entries, and dependent tests in one coherent checkpoint; independently remove only passive freezes first. |
| F2 | Data that looks inventory-only also validates the named type admission; configuration expressions also appear in retained diagnostics. | Preserve source-derived admission validation and configuration masks; replace diagnostic formatting before deleting minimization. |
| F3 | Import/type edges do not replace executable receiver edges. Reduced grammar and cfg coverage have no precise no-silent-omission rule. | Specify both graph roles, receiver limits, supported lexical imports, pre-filter cfg scanning, and zero-configuration rejection independently of the freeze. |
| F4 | Normal lint runs only the production checker. Removing the existing fast registration loses unit, wrapper, colocation, and private-counter coverage; moving it to heavy still runs it under default all. | Keep an explicitly named cheap fast suite; move only production qualification outside fast/heavy/all. |
| F5 | Retained cycle fixtures append retired inventory directives and assert their report rows. The existing compiled helper does not compile, and a test profile does not activate cfg(test). | Adapt fixture setup and architectural diagnostics; compile each retained representative in its actual feature/test configuration. |
| F6 | Slice 90 consumes the gate's stronger extraction contract and the master plan still describes its old allowlist. | Reconcile live Slice 85/90 design consumers while preserving touched-owner, admission, source-scraper, and forbidden-cycle obligations. |
| F7 | The verification step does not explicitly require AC27-85F's official hidden capture and candidate-bound native receipt. | Name immutable public comparison, hidden structural/test inventories and release probe, fresh native receipt, exact SHA, and reviewed deltas. |
| F8 | Completion requires landing/rebinding, but execution excludes merging. | End implementation with a verified candidate awaiting authorized landing; make release rebinding a separate conditional handoff. |

Affected maintained surfaces are bounded as follows:

| Surface | Required treatment during later execution |
| --- | --- |
| Gate lib/main/config, policy, and wrapper | Reduced grammar and graphs; preserve parser/dependency/license/lock/cache contract and source locations. |
| Slice 85 master section and execution plan | Rewrite C/D/E and contradictory frozen-policy/receiver/profile prose together; preserve A/B/F/G. |
| Slice 90 master-plan consumption section and design AC27-90H | Consume the reduced grammar and retained transitive checks; do not preserve a false exact-inventory or three-cycle-allowlist claim. |
| Gate tests and script runner | Preserve cheap tests in fast/default-all and existing CI; qualification becomes explicit only. |
| Fixture helpers and report consumers | Remove retired directive setup and serialized inventory expectations while retaining architectural witnesses and benign controls. |
| Engine | Phase A has two identified privacy/import cleanups. Phase B permits only the specified high-value reviews and demonstrated targeted fixes; preserve public mappings, APIs, and existing test oracles. |
| Surface/native evidence | Qualify the actual resulting candidate, including expected internal visibility/test-inventory deltas. |
| Status, historical receipts, release state, and generated views | Separate current candidate evidence from history and defer release rebinding until authorized landing. |

No new runtime/binding behavior, root workspace member, dependency, general
resolver, CI job, or whole-crate import normalization is planned. If wiring
changes introduce another lint command, its copied harness in
`scripts/tests/test_actionlint_go_install_version.sh` must be updated; the
current harness stubs only the production boundary checker.

## Phase A: removal and cleanup

### 1. Establish the reduced contract

- Recheck branch, head, worktree cleanliness, release state, and memory before
  editing. Reconcile any changes since the starting SHA; do not overwrite them.
- Execute with one writer per checkout. Any delegated writer must use its own
  worktree; independent read-only review may share this checkout.
- Rewrite the Slice 85 section of `dev/plans/plan-0.8.27.md`, especially
  AC27-85C/D/E, and reconcile its execution plan with the reduced contract.
  Keep AC27-85A/B/F/G's engine and evidence obligations.
- Reconcile the same master plan's Slice 90 consumption section and
  `dev/plans/0.8.27/features/slice-90/design.md`, including AC27-90H and its
  preceding extraction paragraph. Preserve touched-owner classification,
  transitive checks, the named type admission, source-scraper oracles, and no
  forbidden cycles or automatic exception growth. Check Slice 140's consumers
  without changing its scope or allocating recovery work to it. Historical
  review findings stay historical, with one supersession pointer if needed.
- Replace the exact edge and inherent-declaration freezes with explicit
  architectural prohibitions and source-derived ownership checks.
- Specify the supported grammar: direct qualified paths, relative anchors,
  ordinary grouped imports, simple aliases, declared engine-used re-exports,
  existing simple lexical block imports, and root-helper forms needed by
  current production code. Classify existing forms before removing resolution;
  do not mistake a newly unsupported existing form for hypothetical laundering.
- Retain conservative treatment of existing outside globs. Reject new
  unsupported indirection where it can influence the boundary graph. Do not
  normalize the whole crate or revive a sequence of laundering-form patches.
- State the receiver limit honestly: this gate does not prove arbitrary
  inferred method-call dependencies. Retain Engine-specific recognition and
  the explicit dependencies needed for the actual architecture checks.
- Define two retained graph roles explicitly: the governed-module graph uses
  ordinary import/type dependencies to enforce direction and type-only cycles;
  the item graph follows explicit calls/callable references and recognized
  Engine field/method routes through distinct root/helper items. An import does
  not become a call to every item in its module. Preserve actual type edges and
  the narrow composition admission; do not claim equivalent arbitrary receiver
  return-path coverage. Keep existing production boundary facts verified.
- Inventory cfg predicates that affect boundary edges or their helper paths.
  Preserve test-hooks, tc5-benchmark, operator, test/non-test, Linux/non-Linux,
  and debug/release distinctions. Remove automatic ML/consumer profiles;
  irrelevant features must not multiply equivalent graphs. If another actual
  predicate affects the boundary, explicitly cover it or fail closed rather
  than silently omit it. Do not build a general configuration solver.
- Scan declarations, parent/inline modules, statements and expressions for cfg
  predicates before filtering. Write a compact supported-predicate and concrete
  configuration table, including feature closure from the manifest and relevant
  helper paths; no unmodeled relevant feature is silently assumed false. Reject
  unknown relevant predicates and relevant declarations/edges active in none of
  the reviewed configurations, independently of `frozen_scope` or edge records.
  Retain per-edge masks and source-derived method ownership under cfg. Use simple
  configuration labels in diagnostics instead of canonical minimized expressions.

### 2. Select and pin essential coverage

- Retain existing tests for the protected engine behavior. Do not weaken them.
- Define a compact gate suite, targeting roughly 30-45 production cases;
  the number is a budget, not a substitute for mechanism coverage.
- Cover every required forbid direction, the four former cycles and
  search/reader-pool, descendant forbids, Engine fields/methods and aliases,
  callable references, macro bodies, capitalized paths, root-helper chains,
  paths leaving and returning through reported modules, cfg distinctions,
  missing/stale owner inputs, unsupported syntax, and wrapper freshness.
- Include benign controls so conservative extraction cannot pass merely by
  rejecting every source edit. Keep genuine external same-name calls legal.
- Keep cheap guards for graph error-impl colocation, private retention-counter
  fields, and the prohibition on direct `ReaderRequest` variant construction
  outside the pool. Removing the 82-method census must not remove constructor,
  private-field, raw-sender, or carrier-owner protections. Confirm which are
  enforced by privacy, the gate, and focused tests; do not invent type inference
  to replace the census.
- Prefer compiling representatives for architectural reverse-edge fixtures.
  Retain parser-only cases for deliberately rejected syntax and label them as
  such. A helper named `compiled` is not compilation evidence.
- For any new behavior, write and stage or commit a failing test before its
  implementation. Retire tests only when their specified machinery is retired;
  never rewrite a retained assertion to make a failure disappear.
- Establish the fast/qualification split before retiring semantics, or update
  affected fixtures in the same retirement batch. Steps 3-5 describe dependency
  order, not permission to leave registered tests asserting removed directives
  between verified checkpoints. Keep retained cycle witnesses active throughout.

### 3. Delete passive freezes without losing shared graph data

- In `dev/tools/module-boundary-policy.txt`, remove `edge`, receiver,
  `module-cycle`, `module-scc`, and `inherent` rows as their consumers retire.
  Remove passive edge/module freezes first. Defer receiver rows and inference
  consumers to the coherent checkpoint in step 4.
- In `dev/tools/module-boundary-gate/src/main.rs`, remove their policy parsing,
  exact comparisons, stale-entry tracking, report serialization, and inventory
  regeneration support. Preserve graphs used by actual forbidden-edge/cycle
  verdicts, owner maps, admissions, and location diagnostics.
- Remove configuration-expression formatting that exists only for frozen
  records after retained diagnostics use the replacement labels. Keep masks
  needed by graph evaluation; delete matching retired tests and commentary.
- Keep source-derived inherent ownership used by any retained check until that
  check has a replacement; deleting the policy census does not delete discovery.
- Preserve a small set of actual admission-relevant type edges, with source,
  source item, destination, destination item, and type kind. The current
  `validate_cycle_policy` consumes `merged_edge_kinds` to reject stale
  `admit-type`; keep that validation independent of the removed frozen records.
  Preserve its exact payload identities and classification checks. An admitted
  error payload must not authorize executable paths through its module.
- Do not merely disable comparisons while leaving unused extraction caches,
  policy structures, or regeneration helpers behind. Determine retained
  consumers before removing each shared collection.

### 4. Reduce extraction and resolution

- In the gate's `lib.rs`, `main.rs`, and `config.rs`, replace general receiver
  inference, namespace forks, recursive alias/projection chasing, serde
  string-path semantics, and consumer-profile enumeration with the bounded
  contract from step 1.
- Replace receiver graph construction and retire its policy parsing, entries,
  exception comparisons, and dependent fixtures in the same reviewable batch.
  Run the selected checks after the complete migration; do not commit or report
  a verified intermediate state with one side of that migration removed.
- Keep a single coherent owner-resolution path for supported constructs;
  remove the old parallel resolution implementations as well as the new
  general resolver. Preserve lexical correctness within supported forms.
- Keep macro-body extraction and cheap rejection guards. Unsupported macro
  expansion or source indirection must not turn into an invisible edge.
- Keep root functions separate and traverse explicit helper references to a
  fixed point with a visited set. Removing the inventory must not remove
  transitive cycle detection.
- Compare old and reduced gate behavior on the unchanged engine and selected
  common configurations. Inspect differences in the governed graph and named
  root-helper witnesses; explain lost inferred-only coverage under the revised
  contract. Do not regenerate a golden edge census or require equality with
  false-positive edges from the old resolver. Current sources and all selected
  controls must pass before removing the old implementation.
- Check the gate crate directly for compilation, formatting, lint, and unit
  tests: it is outside the root workspace and workspace checks alone miss it.
- Fix only lint debt remaining in retained code. Do not finish the cancelled
  resolver refactor or polish machinery scheduled for deletion.

### 5. Trim and retier the tests

- Reduce `scripts/tests/test_module_boundary_gate.sh` to the selected cases.
  Delete advanced serde/qualified-self, root-glob/module-alias, block-binding,
  receiver-classification, profile, and frozen-inventory permutations whose
  semantics were removed. Retain compact rejection tests for unsupported forms.
- Retained cycle bodies must no longer append `edge`, `module-cycle`, or
  `module-scc` directives or regenerate inventories. Use only surviving
  classifications/admissions/cycle directives, and assert the architectural
  diagnostic with named endpoints/configuration. Remove exact inventory-report
  row expectations; preserve cycle bodies and successful admission controls.
- Replace useful noncompiling originals with compiling representatives; delete
  the duplicate `compiled-*` block rather than preserving both sets.
- Keep compact wrapper tests for missing/fresh/stale binaries, modified source,
  redirected `CARGO_TARGET_DIR`, and manifest/lockfile edits without relinking.
- Keep `scripts/tests/test_module_boundary_gate.sh` as the cheap suite: gate
  unit tests, wrapper regressions, cheap engine assertions, and small benign
  controls. Keep its existing `test-module-boundary-gate` fast registration.
  Normal lint continues to run the production checker only. Update the binding
  C/D/E text to state this actual division instead of claiming lint runs fixtures.
- Put production mutants in the proposed
  `scripts/tests/test_module_boundary_gate_qualification.sh`, invoked explicitly
  when the gate/policy/supported grammar changes and at exact-candidate closeout.
  Do not register it in fast, heavy, or all; the local default is all, so a move
  to heavy would not remove the routine burden. No new CI job is needed for the
  fast suite. Check the existing fast CI path, suite inventory, and runner tests
  in `scripts/tests/test_agent_test_tiers.sh` and
  `scripts/tests/test_agent_test_collect_all.sh` after splitting.
- Before trimming, list retained architectural fixture IDs, feature sets, lib
  or test target, and expected diagnostic. Qualification must compile each
  representative in that applicable configuration, then run its gate check.
  `cargo check --profile test --lib` does not set cfg(test); use `cargo check
  --tests` or the appropriate `cargo test --no-run` target for test-only code.
  Respect target `required-features`; do not label a cfg-disabled fixture compiled.
  Parser-only rejection fixtures remain separate. Engine-mutant compilation
  stays out of lint and the cheap suite; gate unit tests may build their small
  standalone crate. Copy needed workspace inputs into a disposable fixture;
  do not compile mutants by patching the production checkout.
- Verify fixture restoration on success and failure. Fixtures must operate on
  copies and leave production source and policy unchanged.

### 6. Complete the two small engine cleanups

- Make `traversal` private in `src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs`;
  preserve its named re-exports and verify all callers compile.
- Move the parent-scoped test-only `encode_graph_evidence_request` import into
  `graph_evidence_request_tests`. Preserve both assertions and test identities.
- Keep `execution` and `SCHEMA_VERSION` access needed by `graph_api`; do not
  commission another encapsulation redesign. Counter fields are already
  private and need no additional rollback.
- Leave tiny gate-motivated UFCS rewrites alone unless removing them is a
  direct simplification. They do not justify another cleanup project.

## Phase B: finish remaining Slice 85 work

Start this phase after the reduced gate and cleanup pass their selected checks.
Review the surviving engine against its actual design obligations; do not
restart eight cycles of general adversarial review. The ratings below express
expected benefit for this recovery, not historical effort or unconditional
permission to rewrite working code. A review may conclude that no fix is needed.

### Reviewed disposition of the supplied list

| Supplied concern | Value and disposition after cleanup |
| --- | --- |
| Ownership model and Slice 90 compatibility | **5/5: B1.** Review actual owners, teardown obligations, and named future consumers; repair only a demonstrated contradiction. |
| Narrow-error granularity | **4/5: B1.** Trace producers, conversions, and precedence. The read-owned `EngineError` payload is not inherently overbroad. |
| 144-byte benchmark request versus 128-byte bound | **4/5: B3.** Resolve the existing internal size-budget failure with a measured, minimal fix; do not simply relax the assertion. |
| Graph visibility and 49 WAL fields | **4/5: B1.** Account for real callers. Moved WAL `pub(super)` fields retain the former private-root reach; their spelling alone is not a widening. |
| Runtime-topology ADR | **4/5: B1 compatibility review.** Read accepted obligations against code and Slice 90 design. Future executors, deadlines, and configuration remain Slice 90 implementation work. |
| Allegedly absent WAL finish-order and worker-zero tests | **5/5: B2/B5 qualification; premise corrected.** Existing tests exercise both. Retain and run them; do not duplicate them. |
| Drop order | **4/5: B2.** Existing close/drop tests do not directly witness profile-context lifetime relative to managed connections. Add one deterministic witness covering both paths. |
| Brittle `slice60_fix1_wire` source scrape | **2/5: preserve the repaired test.** Replacing it is not required recovery work; no equivalent compile/runtime oracle has yet been established. |
| Allegedly unreachable `InvalidFilter` arms | **2/5: no deletion campaign.** B1 checks exact producers and defensive mappings; lack of facade coverage is not proof an arm is invalid. |
| Exact filter-error text | **4/5: preservation review in B1.** Keep refactor parity checks. **2/5:** creating a permanent punctuation-level public contract. |
| Features omitted from routine agent-test | **5/5: B5 exact-candidate non-ML qualification.** **4/5:** focused existing ML/GPU runtime checks for actually touched paths, subject to a capable executor. |
| Global reader-search hook flake | **4/5: B4 bounded reliability fix.** Scope the affected hook and make the existing rendezvous cancellation-safe; no general hook framework. |
| Readability of moved files | **4/5 as part of B1:** inspect owner clarity and unsafe invariants. **2/5:** a separate style sweep or six-file import relocation. |
| Gate-driven owner paths, removed root exports, and UFCS | **2/5: leave them.** Explicit owner paths support the reduced gate. The full resolver is being removed, so its alias support is not a reason to reverse these changes. |
| Counter accessor and colocated error impls | **4/5 preservation check:** already fixed; include in final review. No new redesign. |
| Bottom-of-file imports | **2/5: leave them**, except the one test-only import already specified in Phase A. Escalate only a concrete cfg/name-resolution defect. |
| Eleven interrupted FIX-1 commits | **4/5: review surviving changes once at closeout.** Their changed files are gate source, policy, and the mutation script, not engine source. Do not review removed resolver machinery or finish the cancelled refactor. |

Historical FIX-2 receipts already record substantial test-hooks execution,
including 1,170 passes across 177 binaries. They correct the claim that those
features were never tested; they do not qualify the final recovery candidate.
The ledger's seq-102 records historical parallel-test contention, not proof
that a particular present failure was caused by Slice 85. The concrete hook
ownership and unbounded-wait defects below are independent source evidence.

### B1. One code-grounded design and boundary review

- Read `dev/adr/ADR-0.8.27-engine-owned-runtime-topology.md` against the actual
  engine and `dev/plans/0.8.27/features/slice-90/design.md`. Separate accepted
  synchronous substrates from future executor obligations. Inspect concrete
  `wal_runtime` and `embed_dispatch` consumers without implementing Slice 90.
- Trace `reader_pool` protocol/factories, `read_api` and `graph_api` dispatch,
  `reader_transaction`, `structural_state`, and `wal_attribution`. Verify handler
  independence from the pool and one-way `search_api` to `graph_expand` use.
  Review the moved constructors and unsafe teardown invariants for clarity;
  do not repeat a byte-for-byte faithfulness audit of all moves.
- For `SnapshotFilterError`, `SearchExpandHandlerError`, and `PageReaderError`,
  identify each actual producer and public conversion, including validation
  precedence. Retain necessary defensive mappings. Remove an arm or variant
  only with a complete caller/producer proof and an unchanged public oracle;
  do not introduce production calls solely to cover a defensive arm.
- Preserve `slice85_filter_error_routes.rs` and the `filter.rs` unit test as
  checks that the ownership refactor retains existing typed outcomes and
  payloads. Interfaces specify typed invalid-filter behavior; the exact tested
  sentence has not been established as a permanent message ABI. Do not weaken
  those assertions or invent a new wording contract to justify them.
- Check the broadened graph items against actual consumers, grouped by owner
  rather than a frozen item census. Preserve required `graph_api` access to
  execution/schema/query/validation helpers and counters. The WAL fields' new
  `pub(super)` reach is equivalent to their old private-root reach; narrow only
  demonstrated unnecessary exposure. The private traversal change is in Phase A.
- Exit with a short disposition in the single final receipt: sound as written,
  a concrete targeted defect and its test, or a specific Slice 90 prerequisite.
  Any public-surface change requires its accompanying ADR/interface update.
  Cosmetic readability requests below 4/5 do not become implementation tasks.

### B2. Prove the remaining teardown lifetime invariant

- Preserve the existing reader close/drop and WAL tests. In particular,
  `reader_workers_exit_on_drop_without_explicit_close` checks a pre-drop worker
  count; it does not directly prove post-drop worker completion or profile-box
  lifetime. Do not claim it covers those stronger invariants.
- Add at most one narrowly scoped test fixture with explicit-close and implicit
  drop cases. Its observer must establish that callbacks are uninstalled and
  managed connections/workers can no longer use their profile contexts before
  those contexts are destroyed. Cover contexts whose lifetime the unsafe
  callback contract actually requires, including non-reader managed connections.
- Use private/test-gated lifecycle observations and real engine connections.
  Avoid dereferencing a freed pointer, a source-field-order regex, sleeps as an
  oracle, or a new shutdown framework. Preserve the production field/drop order.
- Stage the new test first and show a safe controlled early-context-release or
  incomplete-shutdown mutation makes it fail deterministically. Use bounded
  waits and cleanup on failure/unwind. Then make a minimal production fix only
  if the genuine invariant is violated. If a safe meaningful witness cannot
  be built within this scope, record that specific gap as unverified; do not
  substitute a non-failing cosmetic test or claim complete coverage.

### B3. Resolve the benchmark request-envelope failure

- Reproduce `reader_request_envelope_stays_bounded_as_search_capabilities_grow`
  in both default and `tc5-benchmark` configurations. Ledger seq-263 documents
  the pre-existing 144-byte failure against 128; this is not caused by recovery.
- Treat 128 as the existing internal per-query envelope budget, not a public
  wire/ABI limit or a proven performance threshold. `VectorStageRequest` is
  currently inline in the enum and grows every request slot when enabled.
- Use minimal indirection for that capability payload if it satisfies the
  measured budget. Preserve typed pool construction, vector-stage behavior,
  and the existing size assertion; run the existing vector-stage tests too.
  Do not create a general allocator/benchmark study or move the carrier owner.
- If the existing budget cannot reasonably be retained, record the concrete
  tradeoff for a separate decision instead of raising the bound to get green.
  Closeout must disclose an unresolved failure; it cannot count as acceptance.

### B4. Isolate the affected reader-search test hook

- Limit this change to `reader_search_hook` in `test_hooks.rs`, its invocation
  in the reader search path, and its consumers, especially
  `undeclared_after_concurrent_drop_is_typed_invalidfilter_not_storage_race`
  in `slice15e_prekn_filterable.rs`. The current process-global armed hook can
  be consumed by another engine; the test's waits and cleanup are not bounded
  on all exit paths.
- First stage a regression showing another engine's search cannot consume the
  intended pause. Scope ownership to the intended engine/request, and use a
  bounded rendezvous with release/disarm on timeout, failure, and unwind.
  Check all consumers of the affected seam before changing its test-only API;
  document an intentional exposed test-hook signature delta where required.
- Preserve the existing concurrent projection-drop and typed-refusal oracle.
  Verify it alongside the unrelated engine's search. Serial execution remains
  the canonical qualification discipline; a passing retry is not the fix.
  Do not disable the test, soften its assertion, migrate unrelated hooks, or
  build another general framework.

### B5. Qualify the final engine, using existing coverage first

- Run exact-candidate checks after both phases. Reuse these library tests:
  `wal_attribution_reader_handoff_is_idle_before_materialized_reply`,
  `wal_attribution_owned_reader_typed_refusal_then_post_release_sampler_is_recorded`,
  `wal_attribution_post_commit_acknowledges_and_records_raw_checkpoint_diagnostic`,
  and the existing engine-scoped handoff/snapshot/completion pause controls.
  They cover finish-before-reply, worker-zero pinning/release, acknowledgements,
  and hook ownership; do not add duplicate tests for these claims.
- Include `slice80_reader_transaction_release`, reader-pool close/drop and
  snapshot-progress tests, `slice85_filter_error_routes`, `slice60_fix1_wire`,
  and `slice60_graph_expand` with `test-hooks` activated. Include B2/B4 tests
  and the envelope test in both B3 configurations. Verify each selected test
  actually runs, respecting required features and platform gates.
- Run relevant existing operator, migration, and benchmark targets serially
  according to touched code. Do not substitute historical receipts, routine
  default tests, or a compile-only matrix for execution of these feature paths.
- Identify actual moved ML/GPU facade routes and run their existing focused
  runtime checks on a capable executor. Record unavailable assets/devices as
  explicit unverified evidence and handoff requirements; do not claim runtime
  coverage from compilation or start a new ML feature cross-product.
- Review retained hunks of the eleven interrupted commits once as part of the
  final reduced candidate review: path guards, supported lexical bindings,
  forbid floor, Engine recognition, and necessary deduplication. Machinery
  removed in Phase A requires no separate acceptance review or resurrection.

## Integrated qualification and handoff

### 7. Verify and record the exact reduced candidate

- After meaningful implementation edits, run the repository's required
  `scripts/agent-verify.sh` in lint/typecheck/test order. Preserve diagnostics;
  distinguish environmental blockers and pre-existing failures from passes.
- Run direct standalone gate checks and the trimmed mutation qualification.
  Validate its compiling representatives using their applicable configurations.
- Run focused reader-pool, graph traversal/expansion, frozen-read, filter/error
  routes, request-envelope, and affected source-scraper tests, including their
  required test-hooks and tc5-benchmark features, as specified in B5. Record
  the B3 result explicitly; an unresolved benchmark envelope failure is never
  a pass. Consolidate overlapping checks rather than repeating whole suites.
- Require AC27-85F's official public comparison against its immutable accepted
  baseline, official hidden structural/test inventories and release probe against
  the accepted hidden baseline, and a candidate-bound native receipt with focused
  FFI verification. Record exact candidate SHA, feature sets, artifact hashes,
  and results. Review intended internal privacy/test-inventory differences
  explicitly; no unintended public, structural, or test-oracle changes pass.
  Reserve Slice 140's intentional seam-gating successor baseline for that slice.
- Native builds must follow repository worktree rules; do not install Python
  native modules from this worktree or reuse the old native receipt after its
  engine source fingerprint changes. Record hardware/disk/tooling blockers as
  unverified evidence, not passes. Do not repeat the entire release performance
  or GPU-runtime program merely for developer-tool edits without a concrete
  affected behavior; retain the named acceptance evidence and focused checks.
- Record one warm-gate and mutation-runtime comparison, final code/policy
  sizes, and the supported grammar. Target approximately one minute for warm
  mutation qualification; if missed, explain the remaining essential cost
  rather than starting another optimization/review loop.
- Obtain an independent read-only review against the reduced contract. Fix
  reproducible defects in protected behavior or supported grammar. Record
  requests to support new Rust forms as outside scope; do not auto-expand it.
- Replace current status/acceptance prose with one accurate candidate receipt.
  Retain compact historical RED/GREEN evidence and prior candidate receipts;
  stop refreshing their counts or claiming they verify a later head.
- Preserve append-only ledgers; any necessary entry uses the repository tools.
  End execution with a clean, committed, reviewed exact candidate and receipt
  awaiting authorized landing. Do not change release-state binding, mark Slice
  85 recovery settled for Slice 90, or rewrite historical evidence as current.

### 8. Conditional landing handoff

- Landing is outside execution authorization. Supply the reviewed candidate
  SHA, receipt, intended release branch, and any unavailable evidence in the
  handoff; request no pushes, merging, tags, or publication through this plan.
- After a separate landing instruction, verify the resulting tree on
  `release/0.8.27` matches the reviewed tree. If landing changes relevant code,
  qualify the changed result rather than reusing the receipt automatically.
- Then bind resulting candidate/closeout SHAs and applicable receipts through
  release-state JSON and regenerate its declared views. Preserve provenance
  between reviewed and landed SHAs; do not hand-edit generated blocks.

## Completion criteria and stop rules

- All ten removal categories are completed and reflected in AC27-85C/D/E.
- The six retired policy directives (`edge`, `external-receiver`,
  `typed-receiver`, `module-cycle`, `module-scc`, `inherent`) and their machinery
  are gone; the required forbid floor and ownership assertions remain.
- No general Rust resolver, arbitrary receiver inference, consumer-profile
  cross-product, or duplicate syntactic/compiling mutation suite remains.
- Actual engine boundaries, supported explicit root-helper paths, cfg distinctions,
  benign controls, and cheap unsupported-syntax guards remain non-vacuous.
- Production qualification is absent from the fast tier. Exact-candidate
  checks, evidence limitations, and maintenance/runtime savings are recorded.
- The engine deliverable and useful corrections remain intact. Phase A's two
  cleanups and Phase B's bounded review, request-budget disposition, teardown
  witness, affected-hook isolation, and feature qualification are recorded.
  Every new fix addresses a demonstrated issue rated at least 4/5; unresolved
  failures or unavailable evidence remain explicit and cannot be called passes.
- Cheap coverage remains in fast/default-all and current fast CI. Qualification
  compiles the retained architectural fixtures under active configurations and
  fails for architectural reasons, not removed policy syntax or stale report rows.
- Live Slice 85/90 design promises agree with the reduced contract. Admission
  validity, relevant zero-configuration rejection, and protected construction
  checks survive independently of the inventories.
- Execution ends with one clean committed candidate and exact-SHA receipt
  awaiting landing. Release authority remains unchanged until the conditional
  landing handoff is separately authorized and completed.

If a retained invariant fails, fix the implementation. If an unsupported
language form appears in actual boundary code, choose the smallest explicit
source spelling or a bounded contract revision; stop before adding a general
resolver or inference subsystem. Apply the repository's two-attempt retry
rule. Do not chase hypothetical variants absent from the engine, regenerate
golden oracles, widen engine fields for tests, or start Slice 90/runtime work.
Pushes, merging, tags, and publication are outside this plan's execution scope.

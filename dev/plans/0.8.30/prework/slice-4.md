---
title: 0.8.30 prework Slice 4 architecture and source alignment
status: COMPLETE
slice: 4
target_release: 0.8.30
date: 2026-11-18
---

# Slice 4 — Architecture and source alignment

## Inputs and boundary

Read the reviewed [master](../plan.md), [historical findings](../testing-robustness-findings.md),
[Slice 0 inventory](slice-0.md), Slice 2 dispositions and Slice 3 local drafts.
For the dependency seam, checked the actual workspace/source-replacement pins
and [fork allocation](../fork-maintenance-plan.md). The completed
[Slice1 sweep](slice-1.md) was reconciled after this initial source read: proposed
Markdown/TLS-root/Tokio fixes belong to10, Node26LTS selection requires explicit
SDK/loader/platform qualification, and major native/fork candidates retain60
ownership. None establishes a need for a new storage/runtime architecture.
Selected API or feature changes must still be reflected in the owning design
and affected proof before implementation. Evidence here is a representative source and
test inspection at `18427b7a8`. No architectural or runtime qualification ran.

Use the active [architecture v2](../../../design/fathomdb-data-plane-architecture-v2.md),
[ADR index](../../../adr/ADR-0.6.0-decision-index.md) and maintained
[Rust](../../../interfaces/rust.md), [SDK](../../../interfaces/rust-sdk.md)
and [MCP](../../../interfaces/mcp.md) boundaries. Read the accepted
[visibility](../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md),
[record identity](../../../adr/ADR-0.8.29-retrieval-record-identity.md) and
[SDK observation](../../../adr/ADR-0.8.29-sdk-qualification-observation.md)
decisions. Different owners already exist for durable state, snapshot-local
search work, response shaping and installed delivery.

## Representative source matches and changes

| Area | What the source actually does | Proposed change or retention |
| --- | --- | --- |
| Storage, write lanes and visibility | `commit_projection_outcomes` takes the shared commit gate. The main writer owns canonical mutation and reader work uses pooled transactions. `projection_batches_coalesce_visibility_invalidation` and the owner startup/isolation cases are in the engine's own test module. [Projection commit](../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs), [reader pool](../../../../src/rust/crates/fathomdb-engine/src/reader_pool.rs), [engine in-crate tests](../../../../src/rust/crates/fathomdb-engine/src/lib.rs). | Architecture and source align at the checked boundary. Retain format 37, the shared commit ordering and separate connection owners. For 30, place any projection-state observation beside the actual read transaction. No new write lane, persisted stamp or visibility cache. |
| Rank, hydration and identity | `RecordKey` is canonical node/cursor, edge/cursor or ownerless FTS rowid. `fuse_records` merges by that key and preserves first rank. Search uses a winner-hydration route when its conditions hold and otherwise uses reference. Existing lightweight controls compare actual complete outputs and exercise source identities and malformed losing rows. [Fusion](../../../../src/rust/crates/fathomdb-engine/src/fusion.rs), [search](../../../../src/rust/crates/fathomdb-engine/src/search.rs), [thin controls](../../../../src/rust/crates/fathomdb-engine/tests/slice50_lightweight_controls.rs). | Distinct equal-body records are supported today. Keep identity and choose 30's smallest snapshot/candidate observation with an independent rank oracle. The S50 archive had reference/reference at the differing operation. Owning an optimized route supplies no cause. Amend historical “body dedup” wording when the adjacent contract changes. |
| Coverage and typed open | `projection_generation.rs` commits the capture transaction before `publish_stored_head_coverage`. Search looks up scored extent using `&tx` and `hit.key()` for an actual vector participant. In-crate tests arrange a failed commit, an external mutation and a pinned-reader race. `open.rs` uses the existing format/profile validation ordering. [Generation](../../../../src/rust/crates/fathomdb-engine/src/projection_generation.rs), [coverage tests](../../../../src/rust/crates/fathomdb-engine/src/unit_tests/stored_coverage_tests.rs), [open](../../../../src/rust/crates/fathomdb-engine/src/open.rs). | Preserve those mechanisms. Slice 30 should install an independent expected-count property over NULL and zero and choose a malformed-native composition only if the existing precedence cases cannot distinguish it. No new cache/snapshot or admission rewrite. |
| MCP response ownership | `fold_response_refs` folds only kept page hits into unique/ambiguous per-response cursor state. `feedback` checks reference membership, store and final epoch before recording a selected unique cursor. A mixed named-ref refusal asserts zero telemetry labels in the integration suite. The existing standalone transport is stdio. [Session](../../../../src/rust/crates/fathomdb-mcp/src/session.rs), [tools](../../../../src/rust/crates/fathomdb-mcp/src/tools.rs), [feedback suite](../../../../src/rust/crates/fathomdb-mcp/tests/feedback.rs). | Keep atomic response-scoped refusal. Slice 30 reviews more precise labels as a contract choice. Selecting a new label needs design for expiry, paging, both renderers, budget/fetch distinction and atomic tests. No new storage or engine ID. |
| Runtime and model ownership | `Engine::close` stops admission, waits for maintenance, stops projection and reader owners, releases the connection and lock, then joins provider drain and drops the provider outside the close guard. In-crate cases distinguish live reader/primary-connection ownership and incomplete close. The Python/Node CLS cells hold static borrows and the engine reranker singleton returns a static model reference. [Close](../../../../src/rust/crates/fathomdb-engine/src/runtime_lifecycle.rs), [close tests](../../../../src/rust/crates/fathomdb-engine/src/unit_tests/slice90_concurrent_close_tests.rs), [Python CLS](../../../../src/rust/crates/fathomdb-py/src/embedding.rs), [Node CLS](../../../../src/rust/crates/fathomdb-napi/src/embedding.rs), [reranker](../../../../src/rust/crates/fathomdb-engine/src/rerank.rs). | Keep existing close ordering. If 9 selects B30-01, 50 designs explicit model ownership that permits dropping only after concurrent calls finish and the exact 60 safe-drop prerequisite is satisfied. Error parity and repeated release/reload need proof. No automatic release or reset is implied. |
| Delivery control plane | The current ordinary `promote-npm-latest` references `NPM_TOKEN`; the scoped recovery uses pinned OIDC npm and an immediate readback. The Rust registry SDK smoke opens and closes without an independent persisted read. The tag bootstrap whitelist names .28/.29 instead of .30. [Normal release](../../../../.github/workflows/release.yml), [OIDC recovery](../../../../.github/workflows/promote-npm-release.yml), [SDK smoke](../../../../scripts/release/smoke/smoke-crates-sdk.sh), [tag route](../../../../scripts/release/check-tag-cuda-route.sh). | These are concrete differences from Slice 3's .30 delivery proposal. Slice 20 owns shared pinned promotion, bounded visibility, authorized route correction and discriminator tests for missing persistence/false success. Slice 70 joins exact candidate proof. Slice 80 proves fresh public consumption when authorized. |
| Pins and installed platform scope | Workspace pins identify qualified Git revisions. The same-commit HTTPS replacement config is tracked and the staged Tegra fix carries it into the source copy. This does not establish current downstream fork resolution or selected binary target execution. [Cargo](../../../../Cargo.toml), [replacement config](../../../../.cargo/config.toml), [fork plan](../features/slice-60/plan.md). | Keep exact pins through investigation. Slice 60 records consumed pin and feature graph plus real artifact evidence for any change. Slice 70 matches actual native inputs and verifies the applicable .29 Tegra baseline through prework 6/7. No new precision, toolkit support or ARM GPU inference. |

## Inspector branch fit

- `git diff 42629c2a1...7d3637657` shows 137 changed files, including workspace
  membership/lock, engineering lifecycle inventory, docs and MkDocs navigation
  and the new `src/inspector` tree. No path under `src/rust/crates` differs in that
  branch-side diff. Absence of a direct engine edit does not establish a clean
  merge or a feature-compatible lock.
- Its own workspace member is unpublished v0.1.0 with a local Engine dependency,
  default-empty features and an explicit fixture-control feature. It has a separate
  local browser transport, private process ownership and its own disclosure rules.
  The database keeps its in-process/local-subprocess Engine/SDK security boundary.
  Inspector's standalone transport must have its own accepted review if integrated.
- The Inspector Slice 20 status is ACTIVE and records partial foundation,
  development GUI, startup and resource results. The branch tip adds packaging
  probe behavior after that status. Final review/Terra, installed qualification
  and the remaining Inspector feature ladder cannot be inferred from the Git tip.
- Recommend Slice 9 preserve separate work by default. If selected for .30,
  assign an explicit integration step and owner. Review workspace membership,
  Cargo/SQLite feature and lock closure, source/admission/transport boundaries,
  public navigation and browser/platform/runtime gates. Do not replace either
  release's selected quality work or attach the branch automatically.

## Proposed disposition for Slice 9

- Include factual architecture status and ownership clarification in approved
  Slice 10 documentation only. Keep accepted behavior and historical paths.
- Use the existing seams for the selected 20/30/40/50/60 cases and revise the
  individual designs where a proven source gap requires a change. Existing
  passive SDK diagnostics remain closed and opt-in under their ADR. For 40,
  reuse qualified observation seams where they fit and keep performance timing
  independent of debug tracing. A trace opportunity alone is not a bottleneck.
- Close R30-TEST by proving the selected oracles through owners 20/30/50 and
  accounting for them in 70. Read in-crate tests before asserting a gap.
  Keep the no-refactor disposition unless a bounded case falsifies a contract.
- Treat separately reported successful .29 Tegra runtime/public proof and stale
  .29 pending state as a documentation/state reconciliation for its owner.
  Leave the live state and public release status untouched in .30 prework.

Source review and proposals complete. No implementation, test, state edit,
branch integration, browsing session or deployment occurred.

---
title: FathomDB 0.8.27 Slice 132 — dedicated Rust SDK parity
status: PLANNED
target_release: 0.8.27
planning_baseline: 316ac4769
---

# Slice 132 — dedicated Rust SDK parity

Slice 132 follows Slice 130 and precedes Slice 135. It adds `fathomdb-sdk`
(`fathomdb_sdk`), a Rust application SDK whose public surface reproduces the
Python and TypeScript SDK surface that Slices 120 and 130 left in place. The
[design](design.md) owns the boundary, signatures, and error model.

## Reconciliation of the draft

The draft (`68cd874c4`, rebased to `d2800a158` on `316ac4769`) was reviewed
against the current tree, the Slice 120/130 closeouts, and source inventories
of all three SDKs.

| Change or finding since the draft | Decision |
| --- | --- |
| `release/0.8.27` gained `316ac4769` (Slice 90 evidence and Slice 150 size-review documentation only). | No effect on Slice 132. |
| Slices 120 and 130 closed as behavior-preserving moves; the public Python and TypeScript surfaces are unchanged. Both SDKs open through `open_with_choice_and_config` with `EmbedderChoice::{Default, None}` and return an `Engine` plus `open_report`. | The Rust SDK reproduces that open shape. Core `Engine::open` and `OpenedEngine` are not exposed. |
| The draft required a three-language installed-artifact fixture harness, signed type/error/config inventories, and a cross-crate disposition audit before any code changed. | **Adjusted — overbuilt.** The governed operation map gains a `rust` column. The SDK's types and errors are fixed in the design and tested in Rust against the documented Python/TypeScript values. Rust behavior tests use a real database. No new installed-artifact harness is added. |
| The draft would add `embed_batch_cls` to the signed allowlist. That allowlist is pinned by HITL signature (`scripts/governed-surface-pin.json`). | **Rejected.** Changing it needs a HITL re-signature for no SDK gain. `embed_batch_cls` stays a documented, tested root operation outside the signed map, which is how both existing oracles already treat it. |
| The draft left the fate of the existing `fathomdb` crate as OPEN HITL. | **Resolved minimally.** `fathomdb` stays unchanged as the lower-level engine facade (BIND-RUST allowlist, operator seam). Its README points application users to `fathomdb-sdk`. Deprecating or retiring it would be a later, separate decision. The unruled `slice-132-external-provider-disposition` decision (provider injection and plugin crates) stays open, does not block this slice, and is carried forward at close. |
| The draft classified custom provider injection, operator/recovery, raw SQL, and test hooks as out of scope. | **Approved.** None are reachable from `fathomdb-sdk`. |
| The draft's Rust-only overloads (`search_*_with_limit`, `*_view`, `search_filtered`, `search_reranked`, `search_explained`, `bm25f_search`, `write_node_importance`, `node_importance`) were left uncertain. | **Excluded.** Python and TypeScript express these as options on one call, and the SDK does the same. |
| The inventory found TypeScript-vs-Python differences: `search_frozen` `pool_n` defaults to `0` in Python but to `rerank_depth` in TypeScript; `drain` takes seconds in Python but milliseconds in TypeScript; `graph.search_expand` takes keyword arguments in Python but a `SearchFilter` in TypeScript; Python's `DependencyTraceError` is missing from `errors.__all__`. | The SDK follows TypeScript and non-frozen `search` (`pool_n` defaults to `rerank_depth`), takes milliseconds, and takes a `SearchFilter`. The two Python defects go to the todo ledger, along with Python `rerank`'s missing non-finite-`alpha` check found in design review. Changing Python behavior is outside this slice. |
| The SDK crate is new and publishable. | Wire it into the version axes, the publishable-crate lists, the license check, the test matrix, the hidden-surface probe, and a `release.yml` publish step that mirrors `fathomdb`. Publication itself remains separately authorized. |

## Needs, requirements, and acceptance

`dev/acceptance.md` stays locked. These IDs are release-local.

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-132A | One reviewed contract covers three SDKs. | AC27-132A: `ADR-0.8.27-rust-sdk-parity.md` is accepted and indexed (row 60); it amends BIND-RUST without changing the `fathomdb` allowlist. `dev/interfaces/rust-sdk.md` gives the per-operation Rust/Python/TypeScript mapping, the translation rules, and the exclusions. `rust.md`, `python.md`, `typescript.md`, and `docs/positions/sdk-parity.md` cross-reference it. |
| R27-132B | An independent Rust SDK exposes only the governed surface. | AC27-132B: a consumer test imports `fathomdb_sdk::{Engine, EngineConfig, OpenOptions, Error, ErrorKind, read, graph, admin, rerank, embed_batch_cls}` and the shared DTO names without depending on `fathomdb_engine`. `compile_fail` doctests prove that the core engine, `open_with_choice`, `EmbedderChoice`, `read_get`-style core methods, operator verbs, and `execute_for_test` are unreachable through `fathomdb_sdk`. |
| R27-132C | Operation membership matches exactly. | AC27-132C: `governed-operation-parity.json` maps all 44 live operations for `rust`, and `check-sdk-surface-parity.py` includes `rust`. The observed Rust surface equals the live set, 44/44. The checker's mutation test rejects a missing, an extra, and a renamed Rust member. The shared non-command members, `open_report`, and `embed_batch_cls` exist, each with a test. |
| R27-132D | Behavior matches the documented SDK contract. | AC27-132D: real-database Rust tests cover: open/report/config; idempotent `close`; `Closing` after close; write → `read` namespace; `search` defaults (`limit` 10, `alpha` 0.3, `pool_n` = `rerank_depth`) and limit-range refusal; text-only and projected search; frozen search; evidence; graph `neighbors`/`expand`/`search_expand`; `admin.configure` and `configure_runtime`; lifecycle/erasure; `rerank` identity path, validation, and ordering; `embed_batch_cls` empty input and feature refusal; `ErrorKind` set equal to the Python/TypeScript leaf set; and every engine, open, and runtime-configuration error mapping. |
| R27-132E | The crate ships through the existing release machinery. | AC27-132E: workspace, version-axis, publishable-crate, license, test-matrix, and `release.yml` lists include `fathomdb-sdk`, and their self-tests pass. Slice 150 adds the rustdoc hidden-surface probe row. The crate README, `docs/` reference page, and `CHANGELOG.md` describe it. |
| R27-132F | The candidate is reviewed and verified. | AC27-132F: the design review and the code review (Opus, high) have no open P0/P1 finding. An independent Sonnet verification confirms AC A–E on the candidate SHA. Strict `./scripts/agent-verify.sh` passes on the candidate, or each environment-only failure is reproduced on the base and recorded. |

## Execution

1. **Design and contract.** Rewrite [design.md](design.md). Independent design
   review by an Opus high subagent; record the result in `design-review.md`
   and resolve its findings. Done: APPROVE-WITH-FIXES, two P1s and ten P2s
   folded into the design. Write the ADR, `dev/interfaces/rust-sdk.md`,
   and the cross-references.
2. **RED.** Scaffold `fathomdb-sdk` with the manifest and an empty `lib.rs`,
   then add the human-authored tests: consumer surface, `compile_fail`
   absence doctests, behavior, NUL-guard, and error-kind tests. Add the `rust` column and the checker change,
   with the checker's mutation tests. Commit them while they fail to compile or
   fail the parity check. Tests stay read-only from then on.
3. **GREEN.** Implement `lib.rs` exports, `Engine`, `read`, `graph`, `admin`,
   `rerank`, `embed_batch_cls`, and `Error`/`ErrorKind` mapping as thin
   adapters over core semantics. Run the crate tests and the parity checker
   after each concern.
4. **REFACTOR and release wiring.** Remove duplication; add the release-list,
   docs, and README updates; run the scoped checks named in the design.
5. **Review and verify.** Run the code review (Opus high subagent) on the
   diff and fix its findings with focused re-runs. Run strict
   `./scripts/agent-verify.sh` once on the candidate. Run the independent
   Sonnet verification.
6. **Close.** Write `status.md`; update `release-state-0.8.27.json` (Slice 132
   complete, next 135) and regenerate the views. Append the Python defects to
   the todo ledger with `ledgerwrite`. Merge into `release/0.8.27`, verify
   from Git, then remove the slice worktree and branch.

## Boundaries

- No raw-SQL, operator/recovery, custom-provider, or test-hook route in the
  SDK. No compatibility shim.
- No change to the Python or TypeScript public surface, the `fathomdb` crate
  surface, the signed allowlist or its pin, or the engine's semantics. An
  engine defect found while adapting is fixed in the engine with a test.
- No tag, publish, registry write, or Pages deployment.

---
title: FathomDB 0.8.27 Slice 132 — dedicated Rust SDK parity status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
source_sha: 2996417f0
---

# Slice 132 Rust SDK parity status

Slice 132 adds `fathomdb-sdk`, a Rust application SDK with the Python and
TypeScript SDK surface. The source candidate is `2996417f0`, made in a
temporary worktree from `316ac4769`. The [plan](plan.md) reconciled and
trimmed the draft. The [design](design.md) passed three reviews, all recorded
in [design-review.md](design-review.md):

- an independent Opus high design review;
- a one-shot Fable 5.1 review, at the user's request;
- an Opus high code review.

Each returned APPROVE-WITH-FIXES, and every finding was resolved or recorded.

## Result and acceptance

| Acceptance | Evidence |
| --- | --- |
| AC27-132A | `ADR-0.8.27-rust-sdk-parity.md` is accepted and indexed as row 60. It amends BIND-RUST without superseding it. `dev/interfaces/rust-sdk.md` holds the per-operation mapping, the translation rules, a type-mapping table, and the exclusions. `rust.md`, `python.md`, `typescript.md`, `dev/design/bindings.md`, the interfaces README, and `docs/positions/sdk-parity.md` refer to it. |
| AC27-132B | `tests/surface.rs` imports only `fathomdb_sdk` and pins every operation's signature with a function-pointer type. Seven `compile_fail` doctests, each pinned to `E0432` or `E0599` and paired with a compiling control, prove that these do not resolve: `EmbedderChoice`, `OpenedEngine`, `open_with_choice`, `read_get`, `check_integrity`, `execute_for_test`, and `search_with_limit`. No `operator`, test-hook, or benchmark feature is forwarded. |
| AC27-132C | The operation map has `rust` endpoints, and `check-sdk-surface-parity.py --binding rust --rust-crate src/rust/crates/fathomdb-sdk` reports `rust 44/44`. The checker test (28 cases) rejects missing, extra, renamed, async, and other-file members. It also refuses unapproved trait impls, public or tuple `Engine` fields, crate/`self`/absolute/glob/function/type-alias re-exports of the core, and the core engine in public signatures or impl targets. The signed allowlist and its pin are unchanged. The Python parity oracle passes against the new map and checker. |
| AC27-132D | 25 behavior tests run against real temporary databases, plus a separate binary for the runtime configuration. They cover: open, report, and config; invalid config; refusal of the default embedder without its feature; idempotent `close` and `Closing` afterwards; subscriber replacement; the `read` namespace; search defaults and limit/`alpha` refusals; the unified filter; text-only search; declared projected-text search; frozen search and expansion; evidence search and resolve; graph `neighbors`, `expand`, and `search_expand` (limit first, attributes refused); `admin.configure` and `configure_runtime`; lifecycle and erasure; the rerank identity path and its validation; `embed_batch_cls` refusal without the feature; the NUL guard (including predicates and cursors) with `source_id` preserved; and the mapping of every non-operator core error. The surface test pins the 42-name `ErrorKind` set and its hierarchy. The suite also passes with the engine's `operator` feature unified on. |
| AC27-132E | The crate is in the workspace, in the Axis-W lists of `set-version.sh` and its tests, in `PUBLISHABLE_CRATES`, among the dependent crates of the publish helper and its test, in a `release.yml` T6 publish/wait step (`actionlint` passes), in the regenerated test matrix, and in the license check (`OK (MIT)`). The crate README, `docs/reference/rust-sdk.md` (mkdocs nav), the install and reference pages, `CHANGELOG.md`, the root README and AGENTS member counts, and `dev/design/release.md` describe it. |
| AC27-132F | An independent Sonnet verification confirmed A, B, C, and E at `ea4054d46` and found D partial. Two of its gaps, evidence and positive projected-text coverage, were closed at `2996417f0`. The other two are recorded under Limits. The gate results are below. |

## Verification and limits

- **Strict gate.** Strict `./scripts/agent-verify.sh` ran at `36869bc14`. Lint,
  typecheck, AC-036/AC-037 security, and every Rust suite passed. Of 184
  suites, 179 passed, 2 were skipped, and 3 failed:
  - `test-public-doc-truth`: its fixture rewrote the literal README word
    "Ten" to fake a false member count. The fixture is now count-agnostic and
    passes.
  - `test-python` and `test-python-native-receipt`: both need a checkout-owned
    native test-hooks receipt. This worktree's `.venv` is a symlink to the
    shared environment and was not written to, by instruction. Python source
    is unchanged in this slice. The Python parity oracle, run against this
    branch's checker and map, passes.
- **Later commits.** The commits after `36869bc14` change only the SDK crate,
  the checker and its tests, the publish-helper test, and documentation.
  Their focused checks passed on `2996417f0`:
  - `cargo test -p fathomdb-sdk`, with and without the operator feature;
  - SDK clippy at `-D warnings`, with and without the model features;
  - the checker suite;
  - the publish-helper and public-doc-truth tests;
  - `agent-lint`.
- **No full-gate claim.** No single uninterrupted full-workspace green run is
  claimed.
- **Coverage limits.** These are not exercised:
  - rerank reordering, which needs the `default-reranker` feature and model
    weights;
  - the `alpha` and `pool_n` defaults, beyond the default-struct assertions;
  - CLS embedding with the `default-embedder` feature.
- **Order differences.** Two validation orders differ from Python and are
  documented: `search_with_evidence` checks `limit` before the core's schema
  check, and the page functions check strings before the core's page checks.
- **Carried forward:**
  - todo-ledger seq 271: three Python parity defects;
  - seq 272: `erase_source` cannot take a `source_id` that contains NUL;
  - seq 273: the first crates.io publish of `fathomdb-sdk` needs a one-time
    HITL token bootstrap before the `v0.8.27` tag, because trusted publishing
    cannot create a crate;
  - the unruled `slice-132-external-provider-disposition` decision. The
    `fathomdb` crate is unchanged, and its README points application users to
    the SDK.
- **Deferred.** Slice 150 owns the rustdoc hidden-surface probe row for the
  SDK. Publication remains unauthorized.

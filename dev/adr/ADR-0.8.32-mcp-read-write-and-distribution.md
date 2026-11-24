---
title: ADR-0.8.32 — Production read+write fathomdb-mcp, startup conflict matrix, write safety, and 3-channel distribution
date: 2026-11-24
target_release: 0.8.32
desc: Graduate fathomdb-mcp to a supported read+write MCP server with allow_writes=false default, per-store opt-in + bound caller ScopePolicy, fail-fast SC-01..SC-10 startup conflict matrix, AC32-MCP-SAFETY-01..09 write-safety bar, atomic PutLatestState + EP-07 + EP-01 SDK seams (Pin C), and 3-channel distribution (5-target GH Releases, lockstep PyPI fathomdb-mcp wheels, and crates.io publish=true).
blast_radius: src/rust/crates/fathomdb-mcp/**; src/rust/crates/fathomdb-engine/**; src/rust/crates/fathomdb-sdk/**; src/rust/crates/fathomdb-schema/**; src/python/**; src/python-mcp/**; src/ts/**; dev/interfaces/mcp.md; .github/workflows/release.yml; .github/workflows/promote-npm-release.yml; scripts/**
status: accepted
---

# ADR-0.8.32: Production read+write `fathomdb-mcp`, startup conflict matrix, write safety, and 3-channel distribution

## Context

Prior to `0.8.32`, `fathomdb-mcp` shipped only as an un-published evaluation adapter (`publish = false` in `src/rust/crates/fathomdb-mcp/Cargo.toml`) hard-locked to read-only, unscoped execution (`write: false`, `scope: null`, `max_open_stores = 1`), carrying evaluation-only `[[example]]` targets (`locomo_load`, `locomo_eval`) and missing crate-root `LICENSE` and `README.md` artifacts.

Three structural gaps blocked graduating `fathomdb-mcp` into a production read+write agent interface:

1. **Write-capability and startup safety:** Enabling writes unconditionally or failing lazily when a misconfigured store receives its first tool call creates silent data-corruption and cross-tenant write hazards. Moreover, `fathomdb-mcp --check-config` previously exited before `Adapter::new` validation, and `bridge::call` (`src/rust/crates/fathomdb-mcp/src/bridge.rs`) plus `registry::shutdown` (`src/rust/crates/fathomdb-mcp/src/registry.rs`) could leave blocking engine closures running past client cancellation or shutdown bounds.
2. **Engine/SDK seam under `MR-070.2`:** `fathomdb-mcp` obeys `MR-070.2` (`fathomdb-mcp/tests/manifest.rs`), linking only `fathomdb-sdk` in runtime code with zero direct `fathomdb-engine` or `rusqlite` dependency. However, `ActuationOperationV1` (`src/rust/crates/fathomdb-engine/src/actuation.rs`) had no `latest_state` (`operational_state`) checkpoint variant, `_fathomdb_actuation_receipts` (`src/rust/crates/fathomdb-schema/src/lib.rs`) had no caller `scope` binding, `OpenOptions` (`src/rust/crates/fathomdb-sdk/src/options.rs`) lacked `create_if_missing: false` (`EP-07`, leaving a check-then-open race in `registry.rs`), and `fathomdb-sdk` lacked an `O(1)` reader-pool visibility-state snapshot (`EP-01`).
3. **Distribution channels and feature consistency:** Users installing via GitHub Releases, PyPI, or `crates.io` need clear, consistent defaults (`default-embedder` in prebuilt binaries/wheels vs. minimal default features on `crates.io`) while preserving `AC-041` (empty `[project.scripts]` on the base `fathomdb` Python package).

## Decision (Owner Rulings 1 & 3, HITL Gate 1 Decisions 4 & 5, 2026-11-24)

### 1. Default Read-Only Posture and Per-Store Opt-In Write Gating

- **Global and per-store default (`allow_writes: false`):** `fathomdb-mcp` opens and serves stores in read-only mode by default. When `allow_writes` is `false` for a store (or across the entire configuration):
  - Write and checkpoint tools are omitted from `tools/list` for read-only profiles (preserving the read-only `7`-tool surface when no store enables writes).
  - Direct invocation of any write/checkpoint tool against a read-only store fails deterministically with `writes_disabled` and zero database mutation.
- **Per-store write opt-in requires bound caller `scope`:** Enabling writes on a store requires **both** explicit per-store `allow_writes: true` **and** a non-empty bound caller `scope` policy (`ScopePolicy`) on that store entry (or matching host `EngineConfig` scope policy when embedded via `Adapter::register_engine`).
- **Auto-discovered `roots` are strictly read-only:** Directory-discovered stores under `roots` must never inherit `allow_writes: true`.
- **Replace evaluation keys cleanly:** Rename the evaluation-era `"write"` / `"scope": null` configuration fields to `"allow_writes"` and `"scope"` (plus optional per-store write-constraint metadata such as `allowed_kinds`), removing read-only evaluation scaffolding from `dev/interfaces/mcp.md`, `config.rs`, `edge.rs`, `error.rs`, and `lib.rs` (ADR-0.6.0-no-shims-policy).

### 2. Fail-Fast Runtime Startup Conflict Matrix (`SC-01`–`SC-10`)

Every startup rule below MUST be validated identically across `fathomdb-mcp --config <path.json> --check-config` (exit `2`, empty `stdout`, structured error JSON on `stderr`, zero engine opens/locks), `Config::load` / `Config::validate`, `Adapter::new`, and `Adapter::register_engine` **before** opening any database, creating a `.lock` file, modifying disk state, or advertising `tools/list`:

| Case | Conflicting Startup Inputs | Fail-Fast Requirement |
| --- | --- | --- |
| `SC-01` | Store `allow_writes: true` with `scope: null`, omitted, or empty `""` | Reject: mutating store requires an explicit non-empty bound caller `scope` policy |
| `SC-02` | Store `allow_writes: true` paired with a read-only `scope` grant | Reject: store write enablement conflicts with non-writing caller grant |
| `SC-03` | Store `allow_writes: false` paired with write-only/write-grant keys or `create: true` | Reject: incoherent read-only store configuration |
| `SC-04` | Legacy evaluation key `"write"` combined with `"allow_writes"` (or `"write": true`) | Reject: legacy/ambiguous key conflict naming the exact offending keys |
| `SC-05` | `roots` directory auto-discovery paired with write enablement | Reject: `roots` entries must remain strictly read-only |
| `SC-06` | Two store names resolving via canonical path or symlink to the same database file with incompatible `allow_writes`, `scope`, or `eager` settings | Reject at startup: prevents cross-scope policy bypass and self-lock deadlock |
| `SC-07` | Multiple `eager: true` stores when `max_open_stores == 1` | Reject: single-open-store invariant violation |
| `SC-08` | Contradictory or malformed `scope` policy grammar (empty/duplicate patterns, wildcard `"*"` on writes, contradicting allow/deny clauses, NUL or control characters) | Reject fail-closed in the `ScopePolicy` parser |
| `SC-09` | Non-addressable `default_store`, `search_mode: "hybrid"` with `use_default_embedder: false`, or `run_limit_ms < call_deadline_ms` | Reject: cross-field invariants preserved |
| `SC-10` | Embedded `Adapter::register_engine` conflicts with configured `stores[name]` or host `EngineConfig` caller `ScopePolicy` | Reject before serving MCP tools, leaving the hosted engine untouched |

### 3. Comprehensive Write-Safety Contract (`AC32-MCP-SAFETY-01`–`09`)

All read+write MCP behavior is gated by `AC32-MCP-SAFETY-01` through `AC32-MCP-SAFETY-09` plus exact SDK-vs-MCP frozen-evidence equivalence (`mcp_frozen_evidence.rs`):

- **`AC32-MCP-SAFETY-01` (Read-only default & per-store write serialization):** `allow_writes: false` omits write tools from `tools/list` and refuses direct write calls with zero mutation; concurrent MCP write requests targeting a write-enabled store serialize deterministically without SQLite busy-lock races.
- **`AC32-MCP-SAFETY-02` (Idempotent `(store, scope, operation_id)` replay & conflict refusal):** Every mutating MCP request requires a caller `operation_id`. Replaying the same `(store, scope, operation_id)` with identical canonical payload across the same session, idle-close/reopen, or adapter restart returns the persisted receipt idempotently without re-applying mutations. Reusing `operation_id` with a different payload or from a different scope fails atomically with a typed conflict refusal and zero mutation.
- **`AC32-MCP-SAFETY-03` (CAS `expected_write_boundary` preconditions):** Concurrent writers racing with the same `expected_write_boundary` yield exactly one committed winner; losing writers fail atomically with `stale_write_boundary`, leave database state untouched, and succeed on retry against the new boundary.
- **`AC32-MCP-SAFETY-04` (Two-caller `scope` isolation & anti-spoof):** Separate caller scopes over overlapping data cannot read, checkpoint, traverse, freeze, resolve evidence, or advance continuations across scopes. Cross-scope references and cursors return nondisclosing `not_found`, and caller payload fields cannot override opener-bound `scope`.
- **`AC32-MCP-SAFETY-05` (Strict-only MCP writes & atomic rollback):** MCP writes enforce strict endpoint integrity (`superseded_at IS NULL AND state = 'active'` within the bound scope), typed property bounds, and batch-size caps. Any invalid item in a batch rolls back the entire transaction atomically, and bulk/non-strict flags are always refused.
- **`AC32-MCP-SAFETY-06` (Blocking cancellation & shutdown drain fence):** Client cancellation (`notifications/cancelled`), `run_limit_ms` timeouts, idle drains, and `Adapter::shutdown()` enforce bounded drain/fencing around blocking SDK closures in `bridge.rs` and `registry.rs` so cancelled/timed-out writes cannot outlive shutdown, wedge admission permits, or race reopen (`AC32-LEAK-01`).
- **`AC32-MCP-SAFETY-07` (`SIGKILL` crash/reopen durability):** Killing the MCP process mid-workload leaves the database passing `PRAGMA integrity_check == ok`, releases stale locks cleanly on reopen, and preserves committed `operation_id` receipts for safe replay.
- **`AC32-MCP-SAFETY-08` (Adversarial secret-scrubbed errors):** Errors across all tool classes and `stderr` diagnostics use adapter templates only and never disclose record bodies, secret tokens, raw SQL text, or out-of-scope record existence.
- **`AC32-MCP-SAFETY-09` (Forbidden operator escape hatches):** Recovery and destructive/bulk maintenance commands (`recover`, `restore`, `repair`, `fix`, `rebuild`, `purge`, `erase_source`, `migrate`, `safe_export`, bulk non-strict writes, raw SQL) are never exposed in `tools/list` and are always refused.
- **SDK-vs-MCP frozen-evidence equivalence:** Scoped frozen evidence flows (`freeze_context`, `search_evidence`, `resolve_evidence`) run through `fathomdb-mcp` must match `fathomdb-sdk` identically on exact body bytes, `artifact_revision_id`, caller-scope filtering, and typed drift, erasure, and truncation outcomes.

### 4. Atomic Governed Write / `latest_state` Checkpoint Engine Seam & Gate 1 Pin C

To satisfy `AC32-MCP-SAFETY-02`–`05` without violating `MR-070.2` or introducing multi-call race windows:

1. **Atomic `PutLatestState` inside `ActuationBatchV1`:** Extend `ActuationOperationV1` with `PutLatestState` (updating `operational_state` / `operational_mutations` under registered collection schema validation inside the same `ActuationBatchV1` transaction that validates `expected_write_boundary` and records `_fathomdb_actuation_receipts`). Bind `_fathomdb_actuation_receipts` rows to caller `scope` so receipt lookup and replay never leak or collide across scopes.
2. **Promote `EP-07` (`OpenOptions.create_if_missing: bool`):** Add `create_if_missing: bool` (default `true` for SDK source compatibility) to `OpenOptions`. With `create_if_missing: false`, `fathomdb-engine` verifies file existence and non-zero size inside `admit_current_database` *after* acquiring the sidecar `.lock`, returns typed `DatabaseNotFound` (cleaning up a newly created `.lock` on refusal), and closes the `path.try_exists()` TOCTOU window in `registry.rs`. `fathomdb-mcp` passes `create_if_missing: entry.create` and maps `DatabaseNotFound` to `unavailable/missing_file`.
3. **Promote `EP-01` (`read::visibility_state`):** Add `fathomdb_sdk::read::visibility_state(engine) -> Result<VisibilityStateV1>` (plus Python `fathomdb.read.visibility_state` and TypeScript `read.visibilityState`), executed on the reader pool inside one deferred read transaction without acquiring the writer lock (`database_id`, `generation`, `nonce`, `canonical_write_boundary`, `projection_cursor`), enabling live MCP envelope `state`, drift labels, and `canonical_write_boundary` hints.
4. **Deferred EPs & Pin C governance:** Defer `EP-05` (`read::kinds`), `EP-10`'s `origin_class`, and `EP-16` post-`0.8.32`. Pre-authorize the targeted `scripts/governed-surface-pin.json` update (Gate 1 Decision 4 Pin C) while holding `core` (`5`) and `recovery_denylist` (`5`) strictly invariant.

### 5. Three-Channel Distribution & Crate Hygiene (Owner Ruling 3 & Gate 1 Decision 5)

`fathomdb-mcp` ships across three synchronized release channels (with an npm wrapper package explicitly deferred out of `0.8.32`):

1. **GitHub Releases (5 native targets):** Build standalone stdio binary archives (`fathomdb-mcp-<version>-<target>.{tar.gz,zip}` + `.sha256`) for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc` with **`--features default-embedder`** (CPU built-in embedder enabled so `search_mode: "hybrid"` works out of the box), reusing the `manylinux_2_28` glibc-checked Linux binaries in both `.github/workflows/release.yml` and `.github/workflows/promote-npm-release.yml`.
2. **Lockstep PyPI Wheel (`src/python-mcp/`):** Publish `fathomdb-mcp==<version>` using `maturin>=1.9,<2`, `bindings = "bin"`, `targets = [{ name = "fathomdb-mcp", kind = "bin" }]`, PEP-639 `license = "MIT"`, `license-files = ["LICENSE"]`, byte-identical `LICENSE`, and `README.md`, built with **`--features default-embedder`** (`manylinux: "2_28"` on Linux) across all 5 targets, validated by `scripts/check_python_wheel_contents.py --kind mcp` (forbidding `_fathomdb*.so/.pyd`). Expose optional `mcp = ["fathomdb-mcp==<version>"]` on `src/python/pyproject.toml` while keeping base `fathomdb`'s `[project.scripts]` strictly empty (`AC-041`).
3. **`crates.io` (`fathomdb-mcp`):** Flip `publish = true` in `src/rust/crates/fathomdb-mcp/Cargo.toml`, add byte-identical `LICENSE` and `README.md`, **keep `default = []`** in `Cargo.toml` for thin library/source builds (documenting `--features default-embedder` in `README.md`), retain `rmcp = "=3.5.1"` confinement in `edge.rs` / `main.rs`, wire `publish-rust-t7-mcp` after `publish-rust-t6-facade` (`fathomdb-sdk`), and **detach the evaluation `[[example]]` targets `locomo_load` (`examples/locomo_load.rs`) and `locomo_eval` (`examples/locomo_eval/main.rs`)** from the published `fathomdb-mcp` crate manifest while **retaining `slice30_m_helper`** (`required-features = ["test-seams"]`, active in `scripts/test-slice30-m-*.sh`).

## Consequences

- Default MCP deployments stay strictly read-only (`allow_writes: false`) while opt-in write deployments require explicit scope binding, pass fail-fast startup conflict validation, and run against atomic idempotent/CAS SDK primitives.
- `scripts/set-version.sh`, `scripts/check-license-consistency.sh`, `scripts/verify-release-gates.sh`, `scripts/release/cargo-publish-if-new.sh`, `scripts/release/wait-for-registry-version.py`, `scripts/generate-public-docs.py`, release workflows, contract tests, and post-publish consumer smokes (`smoke-crates-mcp.sh`, clean-venv PyPI `fathomdb[mcp]`, and 5-target GH Release stdio smokes) must update in lockstep in Slice 40, with PyPI Pending Publisher and crates.io bootstrap configured ahead of tag time.

## Citations

- Owner Rulings 1 & 3 (2026-11-24, `dev/plans/0.8.32/prework/slice-0.md`)
- HITL Gate 1 Decisions 4 (Pin C) & 5 (`dev/plans/0.8.32/prework/slice-9-triage.md`)
- `dev/design/mcp-adapter/engine-proposals/EP-01-store-state-read.md` and `EP-07-open-without-create.md`
- `ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md` (`AC-041` lockstep binary wheel precedent)

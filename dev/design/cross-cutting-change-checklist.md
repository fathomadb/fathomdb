---
status: ACTIVE
---

# Cross-Cutting Change Checklist

Operational checklist for changes that span multiple crates, language bindings,
conformance manifests, or repository gates. Pair this with the subsystem owner
map in [`README.md`](README.md) and the governing decisions in
[`../adr/ADR-0.6.0-decision-index.md`](../adr/ADR-0.6.0-decision-index.md).

## 1. Schema and table additions (`TABLE_MANIFEST` & `ExportClass`)

Whenever adding, renaming, or retiring a SQLite table, virtual table, trigger,
or schema version:

1. Update the schema DDL, `SCHEMA_VERSION` contract, and single-source
   `TABLE_MANIFEST` (`36` tables: `31` regular + `5` virtual; `20`
   visibility-trigger tables / `60` triggers) in
   `src/rust/crates/fathomdb-schema/src/lib.rs` (additive-only when staying on
   the same schema version; never write `INSERT...SELECT` data migrations in
   managed-vec releases — enforced by `scripts/agent-lint-migrations.sh`).
2. Keep `ExportClass` (`src/rust/crates/fathomdb-engine/src/export_manifest.rs`,
   which inherits the table inventory directly from `fathomdb_schema::TABLE_MANIFEST`)
   and internal `fathomdb.logical-export/v1`
   (`src/rust/crates/fathomdb-engine/src/logical_export.rs`, strictly gated
   behind `#[cfg(any(test, feature = "test-hooks", feature = "operator"))]`)
   synchronized so erasure completeness, logical export/restore
   (`src/rust/crates/fathomdb-engine/tests/slice50_logical_export_and_rebuild.rs`),
   and `sqlite_master` audit tests
   (`src/rust/crates/fathomdb-engine/tests/slice38_erasure_and_audit.rs`) cover
   every object with zero uncategorized tables.
3. Keep Schema 39 / `ADR #103` visibility and admission touchpoints
   synchronized across crates:
   - `_fathomdb_scope_visibility_watermarks` (`WITHOUT ROWID`) and
     `_fathomdb_read_visibility_state.{global_struct_gen, global_struct_nonce}`;
   - the 20-table / 60-trigger classification (`8` scope-bearing, `5`
     row-owned, `7` global metadata);
   - `VisibilityCoalescingGuard` / `VisibilityDelta`
     (`canonical_batch_has_no_custom_triggers` vs.
     `projection_batch_has_no_custom_triggers`);
   - `ReadSurface` and `validate_on_open` in
     `src/rust/crates/fathomdb-engine/src/frozen_read.rs`;
   - `projection_generation.rs::bootstrap_is_fresh`;
   - `src/inspector/src/admission.rs`.
4. Update `dev/interfaces/wire.md` and `docs/compatibility/index.md` if the
   format boundary or migration note changes.
5. Keep `scripts/check-public-doc-truth.py` `RELEASE_SCHEMAS` untouched until
   the target release locks its `SCHEMA_VERSION` and publication truth flips.

## 2. Governed SDK operation parity and surface pin

Whenever adding, renaming, or changing a governed SDK operation, option, or
error type across Rust, Python, and TypeScript:

1. Update all three binding implementations in lockstep (`fathomdb-sdk`,
   `fathomdb-py` + `src/python/fathomdb/`, `fathomdb-napi` + `src/ts/src/`),
   including `EngineConfig` knobs (such as `scope_policy`/`scopePolicy` and
   `sync_mode`/`syncMode`) and `fathomdb-mcp` store config parity
   (`SC-03`/`SC-05`/`SC-06`/`SC-10`).
2. Update `src/conformance/governed-operation-parity.json` and
   `src/conformance/governed-surface-allowlist.json`.
3. Only update `scripts/governed-surface-pin.json` after explicit HITL
   pre-authorization for the exact surface delta (`58` live governed ops / `97`
   allowlist members, `runtime_controls = 2`, `49` shared error classes = `1`
   base + `48` leaf; `core = 5` and `recovery_denylist = 5` stay invariant).
4. Update `dev/interfaces/{rust,rust-sdk,python,typescript}.md` and
   `docs/reference/{rust-sdk,rust-api,python-api,typescript-api,errors}.md`.
5. Run `python3 scripts/check-sdk-surface-parity.py` and
   `bash scripts/check-governed-surface-pin.sh`.

## 3. `fathomdb-mcp` configuration, tools, and packaging

Whenever modifying MCP configuration, startup validation, tool contracts, or
distribution posture:

1. Update `src/rust/crates/fathomdb-mcp/src/config.rs` and the runtime tool
   handlers with fail-fast startup conflict validation before opening engines
   or serving `tools/list`.
2. Keep `dev/interfaces/mcp.md` synchronized with every config field, refusal
   code, and tool schema, and ensure no recovery-denylist or bulk escape hatches
   are exposed.
3. Keep `src/rust/crates/fathomdb-mcp/Cargo.toml`, its `LICENSE` copy (required
   by `scripts/check-license-consistency.sh` since `publish = true`), and
   `AGENTS.md` / public docs aligned with actual publish posture.

## 4. Module-boundary splits and test relocations

Whenever splitting a module inside `fathomdb-engine` (or relocating `*_tests.rs`
files) or adding/removing cross-module edges:

1. Update module declarations in `src/rust/crates/fathomdb-engine/src/lib.rs`.
2. Register every moved, added, or split module and allowed edge in
   `dev/tools/module-boundary-policy.txt` (and check
   `dev/tools/module-boundary-gate/src/config.rs` if gate structural rules
   change).
3. Run `bash scripts/check-module-boundaries.sh` and
   `bash scripts/tests/test_module_boundary_gate.sh`.

## 5. Release smoke and published-surface verification

Whenever changing public `fathomdb-sdk` APIs, package versions, platform
support matrices, or native artifact glibc floors:

1. Keep `scripts/release/smoke/registry-sdk.rs` compiling against the current
   `fathomdb-sdk` surface and verify with
   `bash scripts/check-release-smoke-sdk-compile.sh`.
2. Keep `docs/compatibility/index.md` `Measured glibc floor (<family>): X.Y`
   lines synchronized with `scripts/release/glibc-floor-contract.sh` and verify
   with `bash scripts/check-glibc-floor-doc-truth.sh`.
3. Verify `python3 scripts/check-public-doc-truth.py` and
   `bash scripts/tests/test_public_doc_truth.sh`.

## 6. Generated public docs and design-doc lifecycle

Whenever changing workspace manifests, governed parity files, typed developer
scripts, `AGENTS.md`, or `dev/design/**/*.md`:

1. Regenerate tracked generated pages with a Python `>= 3.11` interpreter:
   `python3 scripts/generate-public-docs.py --write`, then confirm
   `python3 scripts/generate-public-docs.py --check`.
2. When adding, moving, or reclassifying any file under `dev/design/`, update
   both `dev/design/document-lifecycle.json` and
   `dev/design/current-owner-authority.json` in lockstep, use governed
   uppercase YAML frontmatter `status: ACTIVE` for new maintained docs, and
   verify `python3 scripts/check-design-lifecycle.py`,
   `bash scripts/lint-design-status.sh`, and
   `python3 scripts/check-architecture-authority.py`.
3. When editing `AGENTS.md`, keep it `<= 300` lines and verify every cited path
   resolves via `python3 scripts/check-agents-md-paths.py --root .`.

## 7. Standalone `fuzz/` workspace and durability harnesses (`ADR #101`)

Whenever adding or updating fuzz targets or block-level durability harnesses:

1. Keep all `libfuzzer-sys` and `arbitrary` dependencies strictly inside the
   standalone `fuzz/` workspace (`exclude = ["fuzz"]` in root `Cargo.toml`,
   `publish = false`, `tier = "dev-tooling"` in `scripts/sbom-survey/tiers.toml`),
   leaving root `rust-toolchain.toml` unchanged at `1.95.0` and shipping crate
   dependency graphs free of fuzz crates
   (`ac32_fuzz_01_shipping_crates_have_zero_fuzz_dependencies`).
2. Commit every confirmed fuzz defect or edge regression as a deterministic
   stable `#[test]` (`slice50_fuzz_regressions.rs`) runnable under stable
   `cargo test` with zero fuzz linkage.
3. Keep QEMU/KVM `dm-flakey` workloads (`fathomdb-dmflakey-harness`) confined to
   the `eval-only` `fathomdb-tc5-benchmark` crate and keep VM images/results
   staged outside the git checkout (`scripts/run-dmflakey-campaign.sh`, zero
   host `sudo`).

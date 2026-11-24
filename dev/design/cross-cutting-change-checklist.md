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

1. Update the schema DDL and `SCHEMA_VERSION` contract in
   `src/rust/crates/fathomdb-schema/src/lib.rs` (additive-only when staying on
   the same schema version; never write `INSERT...SELECT` data migrations in
   managed-vec releases — enforced by `scripts/agent-lint-migrations.sh`).
2. Classify every table in `ExportClass` / `TABLE_MANIFEST`
   (`src/rust/crates/fathomdb-engine/src/export_manifest.rs`) so erasure
   completeness, logical export, and `sqlite_master` audit tests
   (`src/rust/crates/fathomdb-engine/tests/slice38_erasure_and_audit.rs`) cover
   the new object with zero uncategorized tables.
3. Update `dev/interfaces/wire.md` and `docs/compatibility/index.md` if the
   format boundary or migration note changes.
4. Keep `scripts/check-public-doc-truth.py` `RELEASE_SCHEMAS` untouched until
   the target release locks its `SCHEMA_VERSION` and publication truth flips.

## 2. Governed SDK operation parity and surface pin

Whenever adding, renaming, or changing a governed SDK operation, option, or
error type across Rust, Python, and TypeScript:

1. Update all three binding implementations in lockstep (`fathomdb-sdk`,
   `fathomdb-py` + `src/python/fathomdb/`, `fathomdb-napi` + `src/ts/src/`).
2. Update `src/conformance/governed-operation-parity.json` and
   `src/conformance/governed-surface-allowlist.json`.
3. Only update `scripts/governed-surface-pin.json` after explicit HITL
   pre-authorization for the exact surface delta; `core` (5) and
   `recovery_denylist` (5) stay invariant.
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
   by `scripts/check-license-consistency.sh` once `publish = true`), and
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

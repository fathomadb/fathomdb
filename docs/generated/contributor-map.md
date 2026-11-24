# Contributor source map

Generated from tracked source; do not edit this page by hand.
Regenerate with `python3 scripts/generate-public-docs.py --write`.

Sources: [Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/Cargo.toml).

This index lists workspace owners and typed developer entry points.
For TDD, worktree isolation, verification, and release-state guidance,
see [Contributing](../contributing/index.md). Package versions are omitted.

## Rust workspace

| Package | Manifest |
| --- | --- |
| `fathomdb-inspector` | [src/inspector/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/inspector/Cargo.toml) |
| `fathomdb` | [src/rust/crates/fathomdb/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb/Cargo.toml) |
| `fathomdb-cli` | [src/rust/crates/fathomdb-cli/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-cli/Cargo.toml) |
| `fathomdb-embedder` | [src/rust/crates/fathomdb-embedder/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-embedder/Cargo.toml) |
| `fathomdb-embedder-api` | [src/rust/crates/fathomdb-embedder-api/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-embedder-api/Cargo.toml) |
| `fathomdb-engine` | [src/rust/crates/fathomdb-engine/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-engine/Cargo.toml) |
| `fathomdb-mcp` | [src/rust/crates/fathomdb-mcp/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-mcp/Cargo.toml) |
| `fathomdb-napi` | [src/rust/crates/fathomdb-napi/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-napi/Cargo.toml) |
| `fathomdb-py` | [src/rust/crates/fathomdb-py/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-py/Cargo.toml) |
| `fathomdb-query` | [src/rust/crates/fathomdb-query/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-query/Cargo.toml) |
| `fathomdb-schema` | [src/rust/crates/fathomdb-schema/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-schema/Cargo.toml) |
| `fathomdb-sdk` | [src/rust/crates/fathomdb-sdk/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-sdk/Cargo.toml) |
| `fathomdb-tc5-benchmark` | [src/rust/crates/fathomdb-tc5-benchmark/Cargo.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/rust/crates/fathomdb-tc5-benchmark/Cargo.toml) |

## Python and TypeScript packages

| Language | Package | Manifest |
| --- | --- | --- |
| Python | `fathomdb` | [src/python/pyproject.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/python/pyproject.toml) |
| Python | `fathomdb-cli` | [src/python-cli/pyproject.toml](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/python-cli/pyproject.toml) |
| TypeScript | `fathomdb` | [src/ts/package.json](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/src/ts/package.json) |

## Typed developer entry points

| Verb | Script |
| --- | --- |
| `build` | [scripts/agent-build.sh](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/agent-build.sh) |
| `lint` | [scripts/agent-lint.sh](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/agent-lint.sh) |
| `typecheck` | [scripts/agent-typecheck.sh](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/agent-typecheck.sh) |
| `test` | [scripts/agent-test.sh](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/agent-test.sh) |
| `verify` | [scripts/agent-verify.sh](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/scripts/agent-verify.sh) |

<!-- source-sha256: scripts/agent-build.sh bae6ec55fe6cd875141e56df1c4386a3ef6e99caf2ede309642cc30294eb1f94 -->
<!-- source-sha256: scripts/agent-lint.sh ac7786c51221a1226569172edf02752416d29c46d4249257004013c1b2e621b0 -->
<!-- source-sha256: scripts/agent-typecheck.sh f7011356df0fa8060754d6910d56e729145f05e6f6b5cc5abf402d16b7ac1b3b -->
<!-- source-sha256: scripts/agent-test.sh d5e6a9ff19814a7c64757e4994fdd0c6c7c0dd91fa9c51ab83ac81f14e5d6396 -->
<!-- source-sha256: scripts/agent-verify.sh f0f656c4ec0616441d1ec8551a24a7ad14a6e0c630390b3d1755a30441dca5f9 -->

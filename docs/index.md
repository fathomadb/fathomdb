# FathomDB Public Docs

FathomDB is a local-first retrieval and graph-oriented data system
designed for application and agent workloads. This site is the public
documentation source for users, operators, and SDK consumers.

> **Documentation target: 0.8.32 (pre-publish, pre-1.0 beta); newest published release: 0.8.31.**
> The surface may change between micro releases. `0.8.32` documents Schema
> **39**, opt-in `ScopePolicy` caller isolation with a single-scope /
> single-agent default, and offline `38 -> 39` / chained `37 -> 39` migration.
> Version **0.8.31** is the newest published release on crates.io, PyPI
> (`fathomdb` and `fathomdb-cli`), npm, and GitHub Releases (with FathomDB
> Inspector `0.3.0`), plus its matching Tegra CUDA wheel and Node addon on
> first-party GitHub Pages. See [Install](install/python.md) and the
> [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/CHANGELOG.md).

Native Python and Node artifacts cover Linux x86_64/glibc, Linux
AArch64/glibc, macOS x64/arm64, and Windows x64. See the
[compatibility matrix](compatibility/index.md) for CPU/CUDA and Jetson routes.

## Start here

- [Getting Started](getting-started/index.md) — pick your SDK + land
  on the [quickstart](getting-started/quickstart.md).
- [Install — Python](install/python.md) /
  [TypeScript](install/typescript.md) /
  [Rust](install/rust.md).
- [Quickstart](getting-started/quickstart.md) — install, open, write,
  search, counters, close, exit.

## Reference

- [Reference — overview](reference/index.md)
- [Rust SDK](reference/rust-sdk.md) — Rust application SDK
- [Rust API](reference/rust-api.md) — lower-level facade
- [Python API](reference/python-api.md)
- [TypeScript API](reference/typescript-api.md)
- [CLI](reference/cli.md)
- [MCP Server](reference/mcp.md) — stdio server config, startup guards, and tool surface
- [SDK Operation Map](generated/sdk-operation-map.md) — cross-language governed operation index
- [Errors](reference/errors.md)
- [Config](reference/config.md)

## Background

- [Concepts](concepts/index.md) — engine lifecycle, five-verb surface,
  canonical rows + projections, embedder model, recovery surface.
- [Compatibility](compatibility/index.md) — supported platforms,
  toolchains, two-axis versioning, performance posture.
- [Positions](positions/index.md) — consumer-relevant technical
  positions (SDK parity, recovery surface, tokenizer policy,
  embedder identity).
- [0.8.31 release notes](release-notes/0.8.31.md) (and earlier
  [0.8.30 release notes](release-notes/0.8.30.md)).
- [Release changelog](https://github.com/fathomadb/fathomdb/blob/release/0.8.31/CHANGELOG.md) — 0.8.31 changes.
  Earlier detailed changes are recorded in the repo
  [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md).

## Guides and operations

- [Guides](guides/index.md) — hybrid search + filtering, structured
  search hits, retrieve-by-id, [frozen evidence](guides/frozen-evidence.md),
  graph expansion, and the [MCP server guide](guides/mcp-server.md).
- [Operations](operations/index.md) — including
  [erasure](operations/erasure.md): what `erase_source` / `purge`
  guarantee and what they do not; and
  [worktree and branch consolidation](operations/worktree-consolidation.md),
  the manifest-gated local cleanup workflow.

## Inspector

[FathomDB Inspector](inspector/index.md) is a local browser GUI for canonical
records, FTS/BM25, vectors, and graphs. See its [roadmap](inspector/ROADMAP.md)
for scope and future candidates.

# FathomDB Public Docs

FathomDB is a local-first retrieval and graph-oriented data system
designed for application and agent workloads. This site is the public
documentation source for users, operators, and SDK consumers.

> **Release 0.8.27, pre-1.0 beta.** The surface may change between micro
> releases. The release provides Python, TypeScript, and Rust application SDKs,
> plus a separate operator CLI.
> See [Install](install/python.md) and the
> [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/CHANGELOG.md).

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
- [0.8.27 release notes](release-notes/0.8.27.md).
- [Release changelog](https://github.com/fathomadb/fathomdb/blob/release/0.8.27/CHANGELOG.md) — 0.8.27 changes.
  [0.8.26 release notes](release-notes/0.8.26.md) document the preceding release.
  Earlier detailed changes are recorded in the repo
  [CHANGELOG](https://github.com/fathomadb/fathomdb/blob/main/CHANGELOG.md).

## Guides and operations

- [Guides](guides/index.md) — hybrid search + filtering, structured
  search hits, retrieve-by-id.
- [Operations](operations/index.md) — including
  [erasure](operations/erasure.md): what `erase_source` / `purge`
  guarantee and what they do not; and
  [worktree and branch consolidation](operations/worktree-consolidation.md),
  the manifest-gated local cleanup workflow.

# FathomDB

FathomDB is a local-first retrieval and graph-oriented data system for
application and agent workloads. It embeds SQLite (FTS5 + `sqlite-vec`) and
ships one engine behind three SDKs — Python, TypeScript and Rust — plus an
operator CLI.

- Hybrid retrieval: a vector branch and an FTS5 branch fused by Reciprocal Rank
  Fusion, with an optional cross-encoder rerank and an optional graph-BFS third
  arm. On a CUDA-capable artifact, BGE embedding and cross-encoder reranking
  can independently select `FATHOMDB_EMBED_DEVICE` and
  `FATHOMDB_RERANK_DEVICE` (`auto`, `cpu`, or `cuda:N`), including the same GPU.
  Retrieval, FTS, fusion, graph, and SQLite vector stages remain CPU-only.
- Governed, allowlisted command surface with a single typed error taxonomy
  shared 1:1 across the bindings.
- Bitemporal-ish record model: transaction-time supersession, a world-time
  validity window on nodes, and edge `t_valid` / `t_invalid`.
- Lifecycle and erasure verbs in the SDK — `transition`, `purge`,
  `erase_source` — so a consumer with no CLI on `PATH` can still discharge a
  deletion obligation.
- Versioned source provenance and immutable source-to-derived dependencies,
  with atomic caller-decided actuation and dependency-aware closure.
- Authenticated frozen reads, stable pagination, source evidence, dependency
  tracing, and constrained graph expansion across the Python and TypeScript
  SDKs and Rust facade.
- Optional in-process default embedder (`bge-small-en-v1.5`, pure Rust).

**Status: 0.8.26, pre-1.0 beta.** The surface may change between micro
releases. **v0.8.26 is published** to crates.io, PyPI, and npm; native Python
and npm artifacts cover Linux x86_64/glibc, Linux AArch64/glibc, macOS x64,
macOS arm64, and Windows x64. The main npm package is on both the `latest` and
`next` dist-tags.
Licensed **MIT** (see `LICENSE`).

Platform support for the published 0.8.26 artifacts. "CUDA-capable" artifacts
contain both CPU and CUDA paths and fall back to CPU under the default `auto`
policy when no GPU is usable; "CPU-only" artifacts never use a GPU.

| Channel | Artifact | Platforms | GPU |
| --- | --- | --- | --- |
| PyPI | `fathomdb` wheel | Linux x86_64/glibc | CUDA-capable (embedding and reranking) |
| PyPI | `fathomdb` wheel | Linux AArch64/glibc, macOS x64/arm64, Windows x64 | CPU-only |
| Tegra index | `fathomdb==0.8.26+tegra` wheel | Classic Jetson Orin (JetPack 6, CUDA 12.6) | CUDA-capable (embedding); exact 0.8.26 only, not on PyPI |
| npm | `fathomdb` + `fathomdb-linux-x64-gnu` | Linux x86_64/glibc | CUDA-capable (embedding and reranking) |
| npm | `fathomdb` + `fathomdb-linux-arm64-gnu` | Linux AArch64/glibc, including Jetson | CPU-only |
| npm | `fathomdb` + `fathomdb-darwin-x64`, `fathomdb-darwin-arm64`, `fathomdb-native-win32-x64-msvc` | macOS x64/arm64, Windows x64 | CPU-only |
| crates.io | `fathomdb` library, `fathomdb-cli` | Built from source | CPU by default; CUDA via the opt-in `embed-cuda` / `rerank-cuda` features |

The Linux AArch64 npm package is CPU-only, so GPU acceleration under Node.js on
a Jetson currently requires building the CUDA addon from source on the device
(the `build:native:cuda` npm script targets Linux x86_64 only). Planned
0.8.27 Slice 117 (`dev/plans/0.8.27/features/slice-117/plan.md`) will update
this statement when it ships a Jetson CUDA Node addon. No prebuilt CLI binary
is published; install it with `cargo install`. Details:
`docs/compatibility/index.md`, `docs/install/` and `docs/embedder.md`.

Public documentation: `docs/` (built with `mkdocs build --strict`).
Changes since 0.8.9: `CHANGELOG.md`.

Repository layout:

- `docs/` contains public MkDocs source and client-facing technical positions.
- `dev/` contains internal engineering material: requirements, architecture,
  ADRs, subsystem design, interface contracts, and planning notes.
- `src/` contains implementation roots and unit-test-adjacent code.
- `test/` contains cross-language, smoke, fixture, and performance assets that
  are not package-local unit tests.

Implementation roots:

- Eleven Rust workspace members live under `src/rust/crates/`
- Python package root lives under `src/python/`
- TypeScript package root lives under `src/ts/`

Start here:

- Agent instructions: `AGENTS.md` — the canonical operating manual for AI
  coding agents; read it first if you are one
- Public docs: `docs/index.md`
- Internal docs index: `dev/README.md` — and from there the release
  schedule-of-record and the live board under `dev/plans/`
- Workspace checks: `scripts/check.sh`

Common commands:

```bash
cargo check --workspace
pip install -e src/python/
cd src/ts && npm install
mkdocs build --strict
```

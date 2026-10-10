# FathomDB

FathomDB is an embedded retrieval and graph data system for applications and
agents. It combines SQLite, FTS5, and `sqlite-vec` in your process: open a
local database file, write records with provenance, and retrieve them through
the Python, TypeScript, or Rust application SDK.

## Release 0.8.27

FathomDB is pre-1.0 beta; public contracts can change between micro releases.
0.8.27 provides three application SDKs and classic Jetson Orin CUDA delivery
for Python and Node.js. See the [release notes](docs/release-notes/0.8.27.md)
and [changelog](CHANGELOG.md).

## What you can build

- **Retrieval:** full-text and vector search fused with Reciprocal Rank Fusion,
  optional cross-encoder reranking, filters, and graph-assisted retrieval.
- **Governed records:** stable identities, lifecycle transitions, validity
  windows, and source-based erasure across records and their derived indexes.
- **Provenance and dependencies:** versioned sources, immutable dependency
  edges, bounded dependency tracing, and atomic caller-decided actuation.
- **Consistent reads:** authenticated frozen contexts, stable pagination,
  source evidence resolution, and constrained graph expansion.
- **Derived projections:** declarative text, vector, filter, and ranking
  configuration with readiness and runtime diagnostics.
- **Operations:** a separate CLI for integrity inspection and explicit recovery.

The three application SDKs share governed operations and option defaults with
language-specific naming, calling conventions, and error handling. Rust uses
one `Error` type with `ErrorKind` categories; Python and TypeScript expose
error classes. See [SDK parity](docs/positions/sdk-parity.md).

## Choose an SDK

| Language | Package and installation | Import | Documentation |
| --- | --- | --- | --- |
| Python | `python -m pip install fathomdb==0.8.27` | `from fathomdb import Engine, read, graph, admin` | [Install](docs/install/python.md) · [API](docs/reference/python-api.md) |
| TypeScript / Node.js | `npm install --save-exact fathomdb@0.8.27` | `import { Engine, read, graph, admin } from "fathomdb"` | [Install](docs/install/typescript.md) · [API](docs/reference/typescript-api.md) |
| Rust application SDK | `cargo add fathomdb-sdk@=0.8.27` | `use fathomdb_sdk::{Engine, read, graph, admin};` | [Install](docs/install/rust.md#install-the-application-sdk) · [API](docs/reference/rust-sdk.md) |

The [`fathomdb` Rust crate](docs/reference/rust-api.md) is the
lower-level engine facade. The application SDK is `fathomdb-sdk`.

## Quickstart: write and retrieve a note

Install the Python SDK, save this as `quickstart.py`, and run
`python quickstart.py`. It uses a fresh temporary database and full-text
retrieval, so no model download is needed.

```python
from pathlib import Path
from tempfile import TemporaryDirectory
from fathomdb import Engine

with TemporaryDirectory() as directory:
    engine = Engine.open(str(Path(directory) / "notes.sqlite"))
    try:
        engine.write([{
            "kind": "note",
            "body": "The quick brown fox crosses the river.",
            "source_id": "demo-source",
            "logical_id": "note:1",
        }])
        for hit in engine.search_text_only("brown fox").results:
            print(hit.id, hit.body)
    finally:
        engine.close()
```

For hybrid retrieval, enable the pinned default embedder and configure vector
projections for your data. The [quickstart](docs/getting-started/quickstart.md)
and [embedder guide](docs/embedder.md) explain the options, model cache,
CPU/GPU policy, and typed failures.

## Platforms and operators

Published Python and Node artifacts cover Linux x86_64/glibc, Linux
AArch64/glibc, macOS x64/arm64, and Windows x64. Linux x86_64 artifacts contain
CPU and CUDA paths for embedding and reranking; generic Linux AArch64, macOS,
and Windows artifacts are CPU-only. The default `auto` policy uses CPU when
no usable GPU is available. Retrieval, fusion, graph traversal, and SQLite
vector stages run on CPU. See the [compatibility matrix](docs/compatibility/index.md)
for exact versions. Classic Jetson Orin uses the separate Python `+tegra`
wheel and matched-version Node Pages tarball; generic ARM64 remains CPU-only.
See [Python installation](docs/install/python.md#jetson--tegra-cuda) and
[Node installation](docs/install/typescript.md#classic-jetson-orin-with-cuda).

The operator CLI is installed from source; no prebuilt CLI binary is published:

```bash
cargo install fathomdb-cli --version '=0.8.27' --locked
```

Use the [CLI reference](docs/reference/cli.md) and
[integrity inspection guide](docs/operations/integrity-inspection.md) before
operating on a store. Application SDKs do not expose recovery or raw SQL.

## Develop and contribute

Use the [SDK operation map](docs/generated/sdk-operation-map.md) to find
matching governed names across the three languages.

- [Contributor guide](docs/contributing/index.md) and generated
  [source map](docs/generated/contributor-map.md): workflow and current owners.
- [Public documentation](docs/index.md): installation, concepts, references,
  and operations; built with `mkdocs build --strict`.
- [Engineering index](dev/README.md): requirements, architecture, accepted
  ADRs, interface contracts, and release plans.
- [Agent/contributor instructions](AGENTS.md): setup, TDD, isolated worktrees,
  and required verification. Find the live release with
  `python3 scripts/release-current.py`.
- Implementation: [Rust workspace](src/rust/crates/), [Python SDK](src/python/),
  [TypeScript SDK](src/ts/). [Cross-language fixtures](test/) complement
  package-local tests.

After setup, use `./scripts/agent-verify.sh` for the developer gate;
`./scripts/check.sh` adds the broader checks and documentation build.

Licensed [MIT](LICENSE).

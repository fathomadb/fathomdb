# FathomDB

FathomDB is an embedded retrieval and graph data system for applications and
agents. It combines SQLite, FTS5, and `sqlite-vec` in your process: open a
local database file, write records with provenance, and retrieve them through
the Python, TypeScript, or Rust application SDK.

## Release 0.8.32

FathomDB is pre-1.0 beta; public contracts can change between micro releases.
**0.8.32** is published on crates.io (including `fathomdb-mcp`), PyPI
(`fathomdb`, `fathomdb-cli`, and `fathomdb-mcp`), npm, and GitHub Releases
(store format **39**, opt-in `ScopePolicy` caller isolation with a
single-scope / single-agent default, offline `38 -> 39` / chained `37 -> 39`
migration, `fathomdb-mcp` binaries, and FathomDB Inspector **0.3.0**), with its
matching Tegra CUDA wheel and Node addon available through first-party GitHub
Pages. See the [0.8.32 release notes](docs/release-notes/0.8.32.md) and the
[changelog](CHANGELOG.md).

## What you can build

See [FathomDB Use Cases](docs/use-cases.md) for practical engineering, workflow,
business, knowledge management and personal agent examples.

- **Retrieval:** full-text and vector search fused with Reciprocal Rank Fusion,
  optional cross-encoder reranking, filters, and graph-assisted retrieval.
- **Governed records:** stable identities, lifecycle transitions, validity
  windows, and source-based erasure across records and their derived indexes.
- **Provenance and dependencies:** versioned sources, immutable dependency
  edges, bounded dependency tracing, and atomic caller-decided actuation.
- **Consistent reads:** authenticated frozen contexts with Schema 39 partitioned
  additive stability (`ADR #103`), stable pagination, source evidence
  resolution, and constrained graph expansion.
- **Derived projections:** declarative text, vector, filter, and ranking
  configuration with readiness and runtime diagnostics.
- **Bulk loading:** strict endpoint validation remains the
  default on `write` and `actuate`; opt-in `write_with_options` /
  `writeWithOptions` (`bulk_non_strict`) supports out-of-order ingestion with
  truthful `dangling_edge_endpoints` counts and read-only integrity verification.
- **Scope isolation (Schema 39):** By default a FathomDB
  database is single-scope / single-agent (`scope = ""`,
  `ScopePolicy.unscoped`); multi-agent or shared-store deployments can opt into
  engine-enforced `ScopePolicy` read and write boundaries.
- **MCP server:** production stdio [`fathomdb-mcp`](docs/guides/mcp-server.md)
  server (`allow_writes: false` default, opt-in per-store writes with bound
  scope, startup conflict guards, and bounded read/evidence/graph/write tools).
- **Operations & durability:** a separate CLI (`doctor`, `recover`, and offline
  `migrate` for `38 -> 39` and chained `37 -> 38 -> 39` upgrades) for integrity
  inspection, backup/restore/rebuild, schema upgrade, and explicit recovery,
  paired with declared `SyncMode` (`"normal"` default / `"full"` opt-in) WAL
  durability across writer and projection connections.

The three application SDKs share governed operations and option defaults with
language-specific naming, calling conventions, and error handling. Rust uses
one `Error` type with `ErrorKind` categories; Python and TypeScript expose
error classes. See [SDK parity](docs/positions/sdk-parity.md).

## Choose an SDK

| Language | Package and installation | Import | Documentation |
| --- | --- | --- | --- |
| Python | `python -m pip install fathomdb==0.8.32` | `from fathomdb import Engine, read, graph, admin` | [Install](docs/install/python.md) · [API](docs/reference/python-api.md) |
| TypeScript / Node.js | `npm install --save-exact fathomdb@0.8.32` | `import { Engine, read, graph, admin } from "fathomdb"` | [Install](docs/install/typescript.md) · [API](docs/reference/typescript-api.md) |
| Rust application SDK | `cargo add fathomdb-sdk@=0.8.32` | `use fathomdb_sdk::{Engine, read, graph, admin};` | [Install](docs/install/rust.md#install-the-application-sdk) · [API](docs/reference/rust-sdk.md) |

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

Python and Node native artifacts cover Linux x86_64/glibc, Linux
AArch64/glibc, macOS x64/arm64, and Windows x64. Linux x86_64 artifacts contain
CPU and CUDA paths for embedding and reranking; generic Linux AArch64, macOS,
and Windows artifacts are CPU-only. The default `auto` policy uses CPU when
no usable GPU is available. Retrieval, fusion, graph traversal, and SQLite
vector stages run on CPU. See the [compatibility matrix](docs/compatibility/index.md)
for exact versions, ABI floors, and the Node 25/26 contract. Classic Jetson Orin
uses a separate `0.8.32+tegra` Python wheel and matching 0.8.32 Node addon through
first-party GitHub Pages, with exact URLs and digests in the
[retained manifest](scripts/release/tegra-pages-retained.json). Keep the Node
main/addon versions matched; omit the addon to use the generic ARM64 CPU
package. Earlier verified Tegra artifacts remain available.
CUDA 13 artifact delivery is deferred beyond 0.8.32 (`0.8.32` retains the CUDA 12.6 + classic Jetson Orin stack).
See [Python installation](docs/install/python.md#jetson-tegra-cuda) and
[Node installation](docs/install/typescript.md#classic-jetson-orin-with-cuda).

On CUDA-capable artifacts, `FATHOMDB_EMBED_DEVICE` and
`FATHOMDB_RERANK_DEVICE` independently accept `auto`, `cpu`, or `cuda:N`
(for example, `cuda:0`). The [embedder guide](docs/embedder.md) describes
device selection, shared-GPU limits, and
[`release_models()` / `releaseModels()`](docs/embedder.md#release-module-models).
Explicit release drops cached standalone CLS and cross-encoder models while
engine-owned Mean providers retain their own close lifecycle.

Install the operator CLI from crates.io or via the lockstep `fathomdb-cli` PyPI
binary wheel (`pip install 'fathomdb[cli]==0.8.32'`, keeping the base `fathomdb`
SDK wheel free of console scripts):

```bash
cargo install fathomdb-cli --version '=0.8.32' --locked
```

Install the `fathomdb-mcp` stdio server via `pip install 'fathomdb[mcp]==0.8.32'`,
`cargo install fathomdb-mcp --version '=0.8.32' --locked`, or the 5-target
GitHub Releases binaries.

Use the [CLI reference](docs/reference/cli.md) and
[integrity inspection guide](docs/operations/integrity-inspection.md) before
operating on a store (including offline `fathomdb migrate --backup-dir <dir> <db_path>`
upgrades to Schema 39). Application SDKs do not expose recovery,
migration, or raw SQL.

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
- Implementation: 13 Rust workspace members ([core crates](src/rust/crates/) and
  [inspector](src/inspector/)), [Python SDK](src/python/),
  [TypeScript SDK](src/ts/). [Cross-language fixtures](test/) complement
  package-local tests.

Each checkout owns its Python environment and npm installs; use
`bash scripts/setup-python-owned.sh --provision` for a noneditable native Python
build and pinned developer tools. Use `./scripts/agent-verify.sh` for source,
test, or executable-script changes; Markdown-only work uses the scoped checks
in the contributor guide. `./scripts/check.sh` adds the broader checks and
documentation build.

Licensed [MIT](LICENSE).

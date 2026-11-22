---
title: FathomDB 0.8.33 release scope — public open-source readiness and documentation excellence
status: PROPOSED
target_release: 0.8.33
date: 2026-11-21
---

# FathomDB 0.8.33 release scope

## Theme

Get the repository, contributor documentation, auto-documentation, engine
internals documentation, SDK/MCP/CLI/Inspector references, and user-facing
guides and recipes into shape as a **clear, articulate, concise public
open-source database candidate** — documenting the complete stabilized
`0.8.31` + `0.8.32` production surface.

## In scope

### 1. Repository and `dev/` contributor hygiene

- Remove stray working files committed at repo root (`0.8.23-release-todo.md`,
  `CORPUS-01-supersession-questions.txt`, `plan-slice-85-recovery.md`,
  `plan-slice-85-review.md`) and audit top-level files for public cleanliness.
- Streamline `dev/` (currently over 11,000 files, ~9,900 under `dev/plans/`):
  - Archive or prune historical per-release slice runs, intermediate scratch
    packets, and superseded draft notes out of the active contributor tree
    (preserving git tags/history and load-bearing ADRs, interfaces, and active
    plans).
  - Fix `dev/hygeine-roadmap.md` filename typo and staleness; refresh
    `dev/ROADMAP.md` and `dev/README.md` into a crisp contributor map.
- Add standard public open-source repository files: `CONTRIBUTING.md`,
  `SECURITY.md`, `CODE_OF_CONDUCT.md`, GitHub issue/PR templates, and a sharp,
  compelling root `README.md`.

### 2. Working, valuable auto-documentation pipeline

- Upgrade `scripts/generate-public-docs.py` beyond the two bare name-only
  tables (`docs/generated/sdk-operation-map.md`,
  `docs/generated/contributor-map.md`):
  - Generate rich, source-derived API references: `rustdoc` (`fathomdb-sdk` and
    `fathomdb`), Python API docs from docstrings/type stubs, TypeScript API
    docs from `.d.ts` (`typedoc`), CLI `--help` reference, and **MCP
    tool/schema reference** directly from `fathomdb-mcp` JSON schemas.
  - Enforce CI freshness checks so generated docs cannot drift from code.
- Upgrade `mkdocs.yml` from the plain default `mkdocs` theme to a polished
  public documentation theme with full search, clean navigation, and automated
  Mermaid/SVG diagram rendering (reusing the `dev/tools/mermaid` sanitizer and
  rendering checks qualified in `0.8.30`).

### 3. Engine internals documentation ("how it works")

- Author clear, illustrated public architecture docs under `docs/architecture/`
  (replacing raw GitHub links into `dev/design/`):
  - On-disk SQLite, WAL, and sidecar lock model.
  - Canonical nodes, edges, inline outside-`body` typed properties, and
    declared typed B-tree/`vec0` indexes (`0.8.31`).
  - Immutable revisions, versioned provenance, and dependency closure.
  - Derived projection pipeline (FTS5, property FTS, `sqlite-vec`, synchronous
    vs. asynchronous readiness generations).
  - Hybrid RRF fusion, cross-encoder reranking, fused-hit graph seeding, and
    bitemporal property-graph traversal.
  - Strict vs. bulk opt-in write integrity, caller scope isolation, erasure
    lifecycle, and crash/power-loss recovery (`0.8.32`).

### 4. Excellent SDK, MCP, CLI, and Inspector documentation

- **SDKs (Python, TypeScript, Rust)**: complete task-oriented guides + accurate
  API references + package-local agent-consumable documentation (carrying
  forward the documentation half of
  [03-fathomdb-agent-storage-proposal.md](../../notes/agent-storage-sdk/03-fathomdb-agent-storage-proposal.md)).
  Explicitly document `ingest_with_extractor` as legacy/non-provenanced and
  guide governed agents to `Engine.actuate` + `WriteProvenanceV1`,
  and verify full coverage of `fathomdb-cli` PyPI installation, PyPI CUDA/CE
  wheel support, `use_graph_arm` seeding modes, and mixed-`source_id` batch
  atomicity.
- **MCP (`docs/mcp/`)**: first-class public documentation covering setup for
  Claude Desktop, Cursor, Hermes, OpenClaw, and generic MCP hosts; configuration schema;
  all read and write/checkpoint tools (`0.8.32`) and error codes; caller
  scoping/security; token budgets; frozen-evidence workflow; and
  troubleshooting.
- **Inspector (`docs/inspector/`)**: integrate Inspector's user and operator
  documentation into `mkdocs.yml` covering the `0.8.31`/`0.8.32` property,
  history, and scope views.

### 5. User-facing value proposition, visual illustrations, and tested recipes

- Clearly articulate **why FathomDB** (benefits and honest tradeoffs vs. plain
  SQLite+FTS5, vector-only stores, and split graph+vector stacks).
- Add **visual diagrams** across `docs/` (currently zero diagrams): system
  architecture, write→projection→read lifecycle, frozen-evidence verification
  loop, property-graph + bitemporal supersession diagram, and SDK/MCP agent
  topology.
- Ship **CI-executed, runnable use-case recipes** (`docs/recipes/`):
  1. Coding agent searchable engineering memory.
  2. Computer agent runbooks and workflow checkpoints (SDK + read/write MCP).
  3. Business process / design-controls evidence assistant (exact citations and
     host-level abstention).
  4. Knowledge management (property graph + historical path replay).
  5. Personal agent local memory (Hermes / OpenClaw / generic local-agent
     pattern, including caller scope, erasure, and backup/restore).

## Out of scope

- Engine schema changes or new database features (frozen on `0.8.31`/`0.8.32`
  schema).
- Full general multi-version `fathomdb migrate` and personal-agent reference
  connectors (allocated to [0.8.34](../0.8.34/scope.md)).
- Full publishable latency and competitive benchmark campaigns (allocated to
  [0.8.35](../0.8.35/scope.md), though `0.8.33` sets up the clean `docs/`
  structure ready to receive `docs/benchmarks/`).

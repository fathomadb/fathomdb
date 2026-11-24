---
title: FathomDB 0.8.33 release plan
status: PROPOSED
target_release: 0.8.33
date: 2026-11-21
---

# FathomDB 0.8.33 release plan

> **Status: PROPOSED — awaiting completion of `0.8.31`–`0.8.32` and owner
> go-ahead.**
> Execution requires an approved `release-state-0.8.33.json`, live board
> `dev/plans/runs/STATUS-0.8.33.md`, and separate publication authority.

## Goal

Transform FathomDB into a **clear, articulate, well-illustrated public
open-source database repository** with clean contributor documentation under
`dev/`, a working CI-enforced auto-documentation pipeline, thorough engine
internals docs, excellent SDK/MCP/CLI/Inspector docs covering the complete
`0.8.31`–`0.8.32` feature set, visual architecture diagrams, and CI-tested
end-to-end use-case recipes.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  `0.8.33` is a documentation, auto-doc, recipe, and repo-hygiene release; it
  adds no engine-runtime changes. Quality discipline applies by executing every
  published recipe and code snippet against real `0.8.32`/`0.8.33` databases in
  CI (no untested doc snippets or mocked databases), enforcing strict `mkdocs`
  builds, link integrity (`lychee`), Markdown lint, and auto-doc freshness.
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  No tripwire touched; no retrieval constants or storage layout change in
  `0.8.33`.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R33-01 | Clean OSS repo root, streamlined `dev/`, and permanent anti-bloat guard | Zero stray working files at repo root; `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and issue/PR templates exist; `dev/` pruned to `< 400` files and `< 10 MB` with `≤ 25` live `dev/design/` subsystem owner docs; `scripts/check-dev-hygiene.sh` is wired into `agent-lint.sh`; cold-start agent smoke test enumerates all touchpoints for a schema + SDK addition from `AGENTS.md` in `≤ 4` file reads; `./scripts/agent-lint-md.sh` passes. |
| R33-02 | Capability-scoped `dev/interfaces/` and CI-enforced auto-documentation pipeline | Append-only `dev/interfaces/*.md` logs are refactored into capability-scoped current-state contracts (`≤ 35 KB` per file) + thin language deltas; `scripts/generate-public-docs.py` (and companion rustdoc/Python/TS/CLI/MCP extractors) generates complete, accurate API and schema docs from source; CI fails if committed generated docs drift; `mkdocs build --strict` passes with zero warnings. |
| R33-03 | Engine internals documentation ("how it works") | `docs/architecture/` covers storage/WAL/locks, canonical inline properties + declared typed indexes, revisions/provenance/closure, projection pipeline/readiness, RRF + rerank + fused-hit seeding, bitemporal traversal, strict vs. bulk writes, scoping, erasure, and crash/power-loss recovery without relying on internal `dev/design/` links. |
| R33-04 | Complete SDK, MCP, CLI, and Inspector public docs | Python, TypeScript, and Rust SDK docs achieve parity with runnable examples and package-local agent docs (including `ingest_with_extractor` legacy posture vs. `actuate`, `fathomdb-cli` PyPI installation, CUDA/CE wheel matrix, `use_graph_arm` seeding, and batch write atomicity); `docs/mcp/` documents installation, config, all read + write/checkpoint tools, scoping, budgets, and frozen evidence; `docs/inspector/` is integrated into `mkdocs.yml`. |
| R33-05 | Visual illustrations across core concepts and flows | Rendered, sanitized Mermaid/SVG diagrams exist in `docs/` for architecture, write→visibility-trigger→projection→frozen-read→erasure coupling, frozen-evidence resolution, property-graph + bitemporal supersession, and SDK/MCP agent topology; browser/sanitizer check passes. |
| R33-06 | CI-tested runnable use-case recipes | Five end-to-end recipes (coding agent, computer agent, compliance/design controls, knowledge management, personal agent) execute cleanly against real databases in CI and pass all assertions. |

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Inventory, `dev/` (`plans`/`design`/`interfaces`) pruning & refactor manifest, and docs IA design | All | `0.8.32` published | Produces exact keep/archive/delete manifest for `dev/plans/` and `dev/design/` (approved by HITL before destructive cleanup), capability-scoped `dev/interfaces/contracts/` split plan, quantitative `check-dev-hygiene.sh` budgets, and target `mkdocs.yml` IA. |
| 10 | Repo root hygiene, OSS community files, `dev/` declutter, and `check-dev-hygiene.sh` | R33-01 | 0 + HITL approval on prune manifest | Removes root clutter, prunes historical `dev/plans/` and dated slice designs from `dev/design/`, wires `scripts/check-dev-hygiene.sh` into `agent-lint.sh`, refreshes `dev/README.md` / `dev/ROADMAP.md` / `dev/design/README.md` / `dev/design/cross-cutting-change-checklist.md` / `README.md`, and runs the cold-start agent orientation smoke test. |
| 20 | Auto-documentation pipeline, `dev/interfaces/` capability refactor, and upgraded MkDocs + diagram toolchain | R33-02, R33-05 | 10 | Upgrades `scripts/generate-public-docs.py`, rustdoc/Python/TS/CLI/MCP generation, refactors `dev/interfaces/*.md` into capability contracts + thin language deltas, adds CI drift checks, theme, and Mermaid/SVG build. |
| 30 | Engine internals docs and visual architecture/coupling illustrations | R33-03, R33-05 | 20 | Authors `docs/architecture/` and core technical diagrams (including the write→trigger→projection→frozen-read→erasure coupling map). |
| 40 | SDK, MCP, CLI, and Inspector documentation excellence | R33-04 | 20 | Can run parallel to 30; updates SDK refs, package-local agent docs, `docs/mcp/`, and `docs/inspector/`. |
| 50 | User-facing value prop, use cases, and CI-tested runnable recipes | R33-05, R33-06 | 30, 40 | Builds `docs/recipes/` backed by executable real-DB test scripts covering all five agent use cases. |
| 60 | Editorial review, strict doc/link/recipe gate, and publication | All | 50 | Runs full `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, `mkdocs build --strict`, `lychee`, recipe test suite, and publishes updated GitHub Pages + `0.8.33` release. |

## Open HITL decisions required before Slice 0 starts

1. **`dev/` pruning policy**: Confirm moving historical pre-`0.8.30` slice runs
   and scratch packets out of `main` (relying on git tags for archaeology) vs.
   retaining a compressed `dev/archive/` subtree.
2. **MkDocs theme choice**: Confirm adopting Material for MkDocs (or another
   specified open-source theme) and hosting `rustdoc` / API bundles alongside
   the MkDocs site on GitHub Pages.
3. **Versioned release vs. docs-only publish**: Confirm whether `0.8.33` cuts a
   full synchronized package bump (carrying package-local agent docs,
   docstrings, and the deferred Rust `1.95.0 → 1.100.0` toolchain/MSRV upgrade
   into crates/PyPI/npm) or splits the toolchain bump from docs-site work.
4. **Single-agent default posture relaxation (owner `2026-11-24` note):**
   Revisit `0.8.32`'s single-scope / one-agent-per-database default user-doc
   guidance (`ADR-0.8.32-caller-scope-isolation.md`) and decide how broadly
   `0.8.33` docs and recipes should relax it for multi-agent shared-store
   deployments.

---
title: 0.8.32 Prework Slice 6 — Stale Documentation Forensics
status: COMPLETE
target_release: 0.8.32
slice: 6
date: 2026-11-24
---

# 0.8.32 Prework Slice 6 — Stale Documentation Forensics

Strictly read-only audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`) across all three documentation audiences:

1. **End-user docs** (`README.md`, `docs/`, `mkdocs.yml`, `scripts/check-public-doc-truth.py`, `docs/inspector/`, `docs/install/`, `docs/guides/`, `docs/compatibility/index.md`, `docs/positions/`),
2. **Developer / consumer docs** (`dev/interfaces/*.md`, `docs/reference/*.md`, `docs/generated/`, package `README.md` files, and `fathomdb-mcp` crate docs/examples), and
3. **Contributor / agent docs** (`AGENTS.md`, `docs/contributing/index.md`, `dev/adr/ADR-0.6.0-decision-index.md`, `dev/design/`, `dev/DOC-INDEX.md`, `dev/ROADMAP.md`, `ROADMAP.md`, `dev/README.md`, `dev/hygeine-roadmap.md`, and parity/boundary gate diagnostics).

All findings below are verified against tracked source, `scripts/check-public-doc-truth.py`, `scripts/check-agents-md-paths.py`, `dev/plans/0.8.32/{plan.md,scope.md,prework/slice-0.md}`, and `dev/plans/0.8.33/scope.md`.

---

## Executive Summary

- **Residual `0.8.30` version-truth drift after `0.8.31` close:** Although `scripts/check-public-doc-truth.py` exits `0` (`documentation target 0.8.31; newest published receipt 0.8.31`), it only checks that `0.8.31` appears *somewhere* in each `PUBLIC_DOCS` file. Across **13+ active public pages and 8 package READMEs**, prose, install commands, compatibility footnotes, and `blob/release/0.8.30/` links still present **`0.8.30`** as the newest published release (`pip install fathomdb==0.8.30`, `npm install --save-exact fathomdb@0.8.30`, `cargo install fathomdb-cli --version '=0.8.30'`, `fathomdb==0.8.30+tegra`). Separately, `docs/compatibility/index.md:116` still conflates Python (`manylinux_2_28`, glibc `2.28`) with N-API CUDA (`glibc 2.39`), and `docs/install/{python,typescript}.md` lack the per-target CUDA + cross-encoder wheel matrix.
- **`fathomdb-mcp` lockstep documentation updates required in `0.8.32`:** `Cargo.toml`, `lib.rs`, `config.rs`, `tests/readonly.rs`, `dev/interfaces/mcp.md`, `docs/use-cases.md`, `AGENTS.md`, and `dev/agent-memory/release-and-packaging-traps.md` currently declare `fathomdb-mcp` read-only, unscoped, evaluation-only, and unpublished (`publish = false`). Once `0.8.32` lands production read+write `fathomdb-mcp` (default `allow_writes: false`, per-store opt-in + bound caller scope policy, startup conflict check, crates.io + PyPI + GitHub Releases packaging), all of these must update in lockstep.
- **`scope` and write-mode invariants moving in `0.8.32`:** `dev/interfaces/wire.md`, `dev/interfaces/mcp.md`, `fathomdb-mcp/src/config.rs`, and `docs/inspector/*` describe schema-38 `scope` as inert/reserved/empty; SDK reference docs (`python-api.md`, `typescript-api.md`, `rust-sdk.md`, `rust-api.md`, `errors.md`) and `dev/interfaces/*` state `engine.write` always rejects dangling endpoints (`dangling_edge_endpoints == 0` on every commit); and `docs/operations/index.md` still cites power-loss proof and fuzzing as `0.8.32` backlog.
- **R32-07 contributor orientation & gate-diagnostic gaps confirmed:** `dev/design/cross-cutting-change-checklist.md` does not exist; `dev/design/README.md` is a lifecycle-policy note rather than a 1-page live subsystem owner map (27 `maintained` files in `document-lifecycle.json`); `dev/adr/ADR-0.6.0-decision-index.md` mixes active and superseded decisions chronologically (plus duplicate row `#66`, unnumbered `#67`, and `status: in-progress`); parity/boundary gates omit actionable file pointers (`dev/tools/module-boundary-policy.txt`, `src/conformance/governed-operation-parity.json`); and `dev/plans/0.8.32/scope.md:128` mis-cites `dev/tools/module-boundary-gate/src/config.rs` instead of `dev/tools/module-boundary-policy.txt`.
- **Consumer-project boundary:** Public `README.md`, `docs/`, `dev/interfaces/`, and `dev/agent-memory/` are completely free of external consumer specifics. However, **2 maintained `dev/design/` files** (`fathomdb-data-plane-architecture-v2.md`, `nested-source-projections.md`) still retain external consumer-project framing, alongside historical `dev/adr/`, `dev/DOC-INDEX.md`, and test-fixture references, plus `"agent:hermes"` in `src/python/fathomdb/engine.py:574`.
- **Deferred to `0.8.33`:** Root stray-file cleanup, `dev/` pruning (>10,300 files / ~377 MB), `dev/hygeine-roadmap.md` rename, OSS community files, full `scripts/generate-public-docs.py` auto-doc redesign, `mkdocs.yml` theme + diagrams, `docs/architecture/` internals rewrite, `docs/mcp/` host-setup guides, legacy `ingest_with_extractor` framing, and runnable `docs/recipes/`.

---

## Findings by Audience

### Audience 1 — End-User Docs (`README.md`, `docs/`, `mkdocs.yml`, `scripts/check-public-doc-truth.py`, `docs/inspector/`, `docs/install/`, `docs/guides/`, `docs/compatibility/`, `docs/positions/`)

#### 1.1 Residual `0.8.30` / pre-`0.8.31`-close drift in public pages

`scripts/check-public-doc-truth.py` passes (`exit 0`) because `0.8.31` appears somewhere in every `PUBLIC_DOCS` file, leaving stale `0.8.30` statements inside those same files:

- `docs/index.md:50`: still lists `[0.8.30 release notes](release-notes/0.8.30.md)` as the secondary "earlier" parenthetical beside `0.8.31`, while `mkdocs.yml:49-53` lists `0.8.31`–`0.8.28`.
- `docs/getting-started/quickstart.md:5`: `"Python and TypeScript examples use the 0.8.30 SDKs."`
- `docs/use-cases.md:7`: `"These examples use the published 0.8.30 SDKs and, where identified, the repository MCP adapter."`
- `docs/concepts/index.md:3,18`: `"Mental model for the 0.8.31 release target (newest published release: 0.8.30)"` and `"(published 0.8.30 uses schema 37)"`.
- `docs/compatibility/index.md:26,116,127,157,194`:
  - Line 116: `| Linux | x86_64-unknown-linux-gnu | **yes** (CUDA host glibc 2.39) |` still conflates Python `manylinux_2_28` (glibc `2.28`) with N-API CUDA (`glibc 2.39`) and omits CE reranker coverage (keep `Measured glibc floor (<family>): X.Y` lines at 40, 48, 54 intact for `scripts/check-glibc-floor-doc-truth.sh`).
  - Line 127: `"excluded from the 0.8.30 release qualification scope."`
  - Line 157: `"0.8.30 follows two-axis versioning (W = 0.8.30, E = 0.8.0):"`
  - Line 194: links `[0.8.30 release status](https://github.com/fathomadb/fathomdb/blob/release/0.8.30/dev/plans/runs/STATUS-0.8.30.md)`.
- `docs/embedder.md:60,344,544,551`: line 60 has `python -m pip install fathomdb==0.8.30` (contradicting line 9 `Release 0.8.31 is published`); lines 344, 544, 551 reference `0.8.30` qualification / `blob/release/0.8.30/...` (keep historical Orin TC5 measurement explicitly labeled as historical `0.8.30` baseline).
- `docs/install/python.md` & `docs/install/typescript.md`: still omit explicit per-target Embedder / Cross-Encoder (`rerank-cuda`) / CUDA / glibc support matrices distinguishing Linux `x86_64` from CPU-only generic targets.
- `docs/operations/index.md:3,35,39`: line 3 has `"(newest published release: 0.8.30)"`; lines 35–40 cite power-loss proof and fuzzing as `0.8.32` backlog (update after Slice 50/durability campaign lands).
- `docs/operations/integrity-inspection.md:10-11`: `"For published 0.8.30, use cargo install fathomdb-cli --version '=0.8.30' --locked."`
- `docs/operations/erasure.md:183,223`: `"(format-37 in published 0.8.30)"` and `"(published 0.8.30 uses format 37)"`.
- `docs/positions/sdk-parity.md:4,19`: `"share the high-level application operation surface in 0.8.30."` and `"In 0.8.30 this includes the three typed CUDA allocator and context errors;"`.

#### 1.2 `scripts/check-public-doc-truth.py` readiness for `0.8.32`

- `scripts/check-public-doc-truth.py:61`: `RELEASE_SCHEMAS` must add `"0.8.32": 38` when `release-state-0.8.32.json` activates (`SCHEMA_VERSION` stays `38`).
- `scripts/check-public-doc-truth.py:285-290`: enforces only that `version` appears *anywhere* outside URLs/comments, allowing stale `"newest published release: 0.8.30"` strings to pass. Adding a negative check for stale `"newest published release: <older>"` phrases prevents repeat drift.
- `scripts/check-public-doc-truth.py:307`: hardcodes `SCHEMA_VERSION` to `**{schema}**` and `"no public automatic migration"` in `docs/compatibility/index.md` — keep intact for `0.8.32`.

#### 1.3 `fathomdb-mcp` claims in public end-user docs

- `docs/use-cases.md:29-30,40-41,103`: states `"The repository MCP adapter remains an evaluation build available from source."` and `"the adapter exposes no checkpoint-write or actuation tools."`
- `README.md`, `docs/index.md`, `docs/getting-started/index.md`: do not yet mention `fathomdb-mcp` as an installable surface (GitHub Releases binaries, `crates.io` `fathomdb-mcp`, or PyPI `fathomdb-mcp`). Detailed `docs/mcp/` host guides stay in `0.8.33`, while overview/install pointers update in `0.8.32`.

#### 1.4 Inspector docs (`docs/inspector/*`) under `0.8.32` caller scope

All active Inspector docs explicitly state `scope` is read-only/empty in `0.8.31` and defer multi-tenant `scope` filtering to `0.8.32` (`docs/inspector/{index,concepts/index,guides/index,reference/index,operations/index,contributing/index,ROADMAP}.md`). Historical `docs/inspector/releases/0.{2.0,2.1,3.0}.md` and `docs/release-notes/0.8.{28..31}.md` must remain untouched.

---

### Audience 2 — Developer & Consumer Docs (`dev/interfaces/*.md`, `docs/reference/*.md`, `docs/generated/`, package READMEs, `fathomdb-mcp`)

#### 2.1 `docs/reference/*.md` residual `0.8.30` drift & `0.8.32` contract shifts

- `docs/reference/index.md:3,13`: `"newest published release: 0.8.30"` and `"Rust SDK ... (0.8.30)"`.
- `docs/reference/rust-sdk.md:3-4`, `docs/reference/rust-api.md:3,12`, `docs/reference/cli.md:3,109-112,123,180-182`, `docs/reference/config.md:29`: residual `0.8.30` install commands, schema-37 notes, and `fathomdb==0.8.30+tegra`.
- **Strict-only write claims requiring opt-in bulk/non-strict updates:** `docs/reference/{python-api.md:158-170,721,727-733; typescript-api.md:165-177,661,668-674; rust-sdk.md:151-158; rust-api.md:101-108; errors.md:64}` state `dangling_edge_endpoints` is unconditionally `0` on every commit.
- *(Verified error-class counts: `46` concrete subclasses across Python, TypeScript, and `docs/reference/errors.md` match `0.8.31`; recount if `0.8.32` adds scope/bulk error types.)*

#### 2.2 Package READMEs (`src/**/README.md`) still pinned to `0.8.30`

All 8 published-package READMEs still carry `0.8.30` install commands, banners, and `blob/release/0.8.30/` links:

- `src/ts/README.md` (lines 12, 18, 26, 29, 30, 37, 100, 125, 126)
- `src/python/README.md` (lines 12–13, 19, 24, 98, 113, 119, 120)
- `src/rust/crates/fathomdb/README.md` (lines 8, 21)
- `src/rust/crates/fathomdb-sdk/README.md` (lines 18–19, 25, 31, 69, 70, 106)
- `src/rust/crates/fathomdb-cli/README.md` (lines 20, 24–25)
- `src/rust/crates/fathomdb-engine/README.md` (lines 35–37)
- `src/rust/crates/fathomdb-schema/README.md` (lines 13–15, 40–41)
- `src/rust/crates/fathomdb-embedder/README.md` (lines 56, 66)

#### 2.3 `dev/interfaces/*.md` & `docs/generated/*`

- `dev/interfaces/{cli,python,rust,rust-sdk,typescript,wire}.md`: headers declare `target_release: 0.8.31` (`2026-11-23`).
- `dev/interfaces/wire.md:73-75`: states Schema 38 adds `inert scope columns` — update when `scope` activates.
- `dev/interfaces/{python,typescript,rust,rust-sdk}.md`: state strict-only endpoint integrity (`dangling_edge_endpoints == 0` on every commit) — update for opt-in bulk mode.
- `docs/generated/{contributor-map,sdk-operation-map}.md` and `scripts/generate-public-docs.py:18`: `SOURCE_URL` points to `blob/release/0.8.31/` (bump to `0.8.32` and regenerate via `--write` before release).

#### 2.4 `fathomdb-mcp` crate docs, config, tests & examples

- **Missing crate `README.md`:** `src/rust/crates/fathomdb-mcp/` has no `README.md`; publishing to crates.io and PyPI in `0.8.32` requires one.
- `Cargo.toml:3,9-10`, `src/lib.rs:3`, `src/config.rs:185,236-247`, `tests/readonly.rs`, and `dev/interfaces/mcp.md` (`lines 4-7, 12-20, 61-62, 456-458, 513-520`) all assert evaluation-only, read-only, unscoped, `publish = false` posture and must update in lockstep with `allow_writes: false` default, per-store opt-in + bound caller scope policy, startup conflict rejection, and governed write/checkpoint tools.

---

### Audience 3 — Contributor & Agent Docs (`AGENTS.md`, `docs/contributing/`, `dev/adr/`, `dev/design/`, `dev/DOC-INDEX.md`, `dev/ROADMAP.md`, gate diagnostics)

#### 3.1 `AGENTS.md` and `dev/agent-memory/`

- `AGENTS.md` passes `scripts/check-agents-md-paths.py` (`exit 0`, 186 lines <= 300).
- `AGENTS.md:31`: describes `fathomdb-mcp` as `[MCP server, publish = false]` — update when `Cargo.toml` flips `publish = true`.
- `AGENTS.md` §§ 1, 9: does not yet link `dev/design/cross-cutting-change-checklist.md` or `dev/design/README.md` (`R32-07`).
- `dev/agent-memory/release-and-packaging-traps.md:10`: lists `PyPI fathomdb and fathomdb-cli` — add `fathomdb-mcp` when MCP packaging lands.

#### 3.2 `R32-07` orientation artifacts & gate diagnostic gaps

1. **Missing `dev/design/cross-cutting-change-checklist.md`** (does not exist yet).
2. **`dev/design/README.md` lacks a live subsystem owner map** (currently 46 lines of lifecycle policy rather than a 1-page map of the 27 `maintained` owners).
3. **`dev/adr/ADR-0.6.0-decision-index.md` needs Active/Governing vs. Superseded/Historical split:** frontmatter has `target_release: 0.6.0`, `status: in-progress`, `blast_radius: TBD`; tables mix active and superseded decisions; duplicate row `#66` (lines 173, 195); missing `#67` table row (`seq-359` / `ADR-0.8.28-vector-compatibility-open-policy.md` sits as unformatted prose on lines 175–182); row `#65` has `none`.
4. **Parity and module-boundary gate diagnostics:**
   - `scripts/check-sdk-surface-parity.py:634-638`: prints raw sets without naming `src/conformance/governed-operation-parity.json`, `src/conformance/governed-surface-allowlist.json`, or `python3 scripts/generate-public-docs.py --write`.
   - `scripts/check-governed-surface-pin.sh:343-409`: names `FILE`/`PIN`; add pointer to `dev/design/cross-cutting-change-checklist.md`.
   - `dev/tools/module-boundary-gate/src/main.rs:38-42,197-201,467-525`: errors omit the policy file path `dev/tools/module-boundary-policy.txt`.
   - **Plan-doc correction:** `dev/plans/0.8.32/scope.md:128` mis-cites `dev/tools/module-boundary-gate/src/config.rs`; module classifications and edges live in **`dev/tools/module-boundary-policy.txt`**.

#### 3.3 Internal roadmap, sequencing & index currency

- `dev/ROADMAP.md:24-34,54-56,173-178` & `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md:28-30`: still list `0.8.30` as newest published and `0.8.31` as active.
- `dev/DOC-INDEX.md:70-71`: still highlights `0.8.30` near the top; no `0.8.32` section yet.
- `ROADMAP.md` (root): lines 8, 85, 119 reference `0.8.28` / `Slice 77` / `Slice 79`.
- `docs/contributing/index.md`: link `dev/design/cross-cutting-change-checklist.md` and `fathomdb-mcp` verification notes.

#### 3.4 External consumer-project specifics in tracked files

- **Clean surfaces:** `README.md`, `docs/`, `dev/interfaces/`, and `dev/agent-memory/` have **zero** external consumer-project references.
- **Maintained `dev/design/` leaks (must generalize):**
  - `dev/design/fathomdb-data-plane-architecture-v2.md:275,299,382,397`
  - `dev/design/nested-source-projections.md:7,21,28,45,62,87,121,145`
- **Shipped docstring note:** `src/python/fathomdb/engine.py:574` uses `"agent:hermes"` as a sample `label_source`; replace with product-generic `"agent:assistant"`.

---

## Explicitly Deferred to `0.8.33` (`dev/plans/0.8.33/scope.md`)

1. Repo & `dev/` declutter (`R33-01`: root stray files, pruning >10,300 historical files, `dev/hygeine-roadmap.md` rename, `check-dev-hygiene.sh`, OSS community files).
2. Auto-documentation redesign (`R33-02`: `rustdoc`/`typedoc`/MCP schema pipeline, `mkdocs.yml` theme + diagrams).
3. `docs/architecture/` internals rewrite (`R33-03`).
4. Full `docs/mcp/` host guides, legacy `ingest_with_extractor` framing, and runnable `docs/recipes/` (`R33-04`–`R33-06`).

---

## Numbered Proposals (`P6-01` – `P6-11`)

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation | Target Slice / Rationale |
| --- | --- | --- | --- | --- | --- | --- |
| `P6-01` | Create `dev/design/cross-cutting-change-checklist.md`, expand `dev/design/README.md` into a 1-page live subsystem owner map (27 maintained owners), split `dev/adr/ADR-0.6.0-decision-index.md` into Active/Governing vs. Superseded/Historical tables (fixing `#66`/`#67` numbering & `status`), and link checklist from `AGENTS.md` + `docs/contributing/index.md`. | H | L | M | include | **Slice 10 (`Prework Prep` / `R32-07`):** Explicitly scheduled before engine refactors and `scope` edits begin. |
| `P6-02` | Tighten diagnostic error messages in `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, and `dev/tools/module-boundary-gate/src/main.rs` to print exact config/registry paths (`dev/tools/module-boundary-policy.txt`, `src/conformance/governed-operation-parity.json`, `scripts/governed-surface-pin.json`); fix `dev/plans/0.8.32/scope.md:128` to cite `dev/tools/module-boundary-policy.txt`. | H | L | S | include | **Slice 10 (`Prework Prep` / `R32-07`):** Prevents wasted triage loops during hotspot splits and `scope` parity additions. |
| `P6-03` | Purge residual `0.8.30` `"newest published release"` / install commands / `blob/release/0.8.30/` links across `docs/` and all 8 `src/**/README.md` files; add explicit per-target CUDA + CE matrix to `docs/install/{python,typescript}.md` and split Python vs. N-API glibc cells in `docs/compatibility/index.md:116`. | H | L | M | include | **Slice 10 (`Prework Prep`) & Qualification slice (final `0.8.32` bump):** Fixes broken `0.8.30` copy-paste install commands immediately. |
| `P6-04` | Sync `dev/ROADMAP.md`, `dev/plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`, and `dev/DOC-INDEX.md` to `0.8.31` published / `0.8.32` active; add `"0.8.32": 38` to `scripts/check-public-doc-truth.py:61` alongside a stale-version negative check when `release-state-0.8.32.json` activates. | H | L | S | include | **Slice 10 (`Prework Prep`):** Keeps `release-current.py`, `DOC-INDEX.md`, and `check-public-doc-truth.py` aligned. |
| `P6-05` | Generalize external consumer-project references out of the 2 maintained `dev/design/` owner docs (`fathomdb-data-plane-architecture-v2.md`, `nested-source-projections.md`) and replace `"agent:hermes"` in `src/python/fathomdb/engine.py:574` docstring with `"agent:assistant"`. | H | L | S | include | **Slice 10 (`Prework Prep`):** Enforces `HANDOFF-07` product-generic boundary on active design owners and shipped Python docstrings. |
| `P6-06` | Update `dev/interfaces/wire.md` (`inert scope`), `dev/interfaces/{python,typescript,rust,rust-sdk}.md`, and `docs/reference/*` for activated Schema-38 caller `scope` enforcement (preserving zero-overhead single-scope `""` fast path and no breaking schema bump). | H | M | M | include | **Scope slice (`R32-01`):** Contract and reference docs must land in lockstep with engine + SDK `scope` activation. |
| `P6-07` | Update `dev/interfaces/{python,typescript,rust,rust-sdk}.md`, `docs/reference/{python-api,typescript-api,rust-sdk,rust-api,errors}.md`, and active `docs/inspector/*` pages (plus `docs/inspector/ROADMAP.md`) for opt-in bulk / non-strict write mode and Inspector's caller-scope UI view. | H | M | M | include | **Bulk/Inspector slice (`R32-03`, `R32-06`):** Keeps SDK write-integrity and Inspector docs truthful. |
| `P6-08` | Update `fathomdb-mcp` docs/metadata in lockstep for production read+write + scoped MCP: add `src/rust/crates/fathomdb-mcp/README.md`, update `Cargo.toml` (`publish = true`, description), `lib.rs`, `config.rs`, `tests/readonly.rs`, `dev/interfaces/mcp.md`, `docs/use-cases.md`, `AGENTS.md:31`, and `dev/agent-memory/release-and-packaging-traps.md`, plus `fathomdb-mcp` install/overview pointers in `README.md`, `docs/index.md`, and `docs/getting-started/index.md`. | H | M | M | include | **MCP slice (`R32-02`) & Qualification/Publish slices:** Keeps published `fathomdb-mcp` docs aligned with `allow_writes: false` default, startup validation, and write/checkpoint tools. |
| `P6-09` | Update `docs/operations/index.md:35-40`, `docs/compatibility/index.md`, and `dev/interfaces/cli.md` / `docs/reference/cli.md` with the declared `0.8.32` durability/fault model, backup/restore/rebuild proof (`logical-export/v1`), and coverage-guided fuzzing scope/limits. | H | L | S | include | **Durability slice (`R32-04`, `R32-05`):** Replaces `"0.8.32 backlog"` placeholder with verified durability contract. |
| `P6-10` | Final `0.8.32` release-truth sweep: bump `SOURCE_URL` in `scripts/generate-public-docs.py`, regenerate `docs/generated/*.md`, verify `check-public-doc-truth.py`, `check-glibc-floor-doc-truth.sh`, `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, and author `docs/release-notes/0.8.32.md` + `CHANGELOG.md`. | H | L | S | include | **Qualification & Publication slices:** Pre-tag documentation consistency gate. |
| `P6-11` | Defer repo/root declutter, `dev/` pruning (>10,300 files), `dev/hygeine-roadmap.md` rename, OSS community files, full `generate-public-docs.py` `rustdoc`/`typedoc`/MCP schema pipeline, `mkdocs.yml` theme/diagrams, `docs/architecture/`, `docs/mcp/` host guides, `ingest_with_extractor` framing, and `docs/recipes/`. | H | L | L | postpone | **`0.8.33` (`R33-01`–`R33-06`):** Explicitly out of `0.8.32` scope. |

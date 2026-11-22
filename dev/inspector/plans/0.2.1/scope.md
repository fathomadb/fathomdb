---
title: Inspector 0.2.1 adopted scope — FathomDB 0.8.30 support
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: ACTIVE
desc: Adopted compatibility scope for Inspector 0.2.1 on FathomDB 0.8.30 with native Ubuntu 24.04 delivery
---

# Inspector 0.2.1 scope (FathomDB 0.8.30 support)

## Objective and release boundary

Deliver **Inspector 0.2.1** as a bounded compatibility release embedding the
delivered **FathomDB 0.8.30** Engine (schema 37, Candle 0.13.0 / cudarc 0.21.0,
Tokio 1.54.1, `webpki-roots` 1.0.12, `rustls-webpki` 0.103.16) and qualified
for closed real databases created by both published **FathomDB 0.8.29** and
**FathomDB 0.8.30** without per-version compatibility shims.

This scope builds on:

- [Inspector 0.2.0 full assessment](../../reviews/assessment-0.2.0.md)
- [FathomDB 0.8.30 compatibility assessment](../../reviews/compatibility-0.8.30-assessment.md)
- [FathomDB quality testing roadmap](../../../notes/fathomdb-quality-testing-roadmap.md)
- [Inspector 0.2.0 adopted scope](../0.2.0/scope.md) and
  [release decisions](../0.2.0/release-decisions.md)

Execution is sequenced in four slices in the [0.2.1 release plan](plan.md):
**Slice 10** (0.8.30 base integration and workspace/lockfile hygiene),
**Slice 20** (0.8.30 retrieval, provenance, and cached-CPU qualification),
**Slice 30** (native Ubuntu 24.04 Docker-free tooling, 0.2.1 packaging, dual
published producer qualification, and local release), and
**Slice 40** (documentation reconciliation and release closure).

No Engine, SDK, MCP, schema, ranker, or storage-format changes are authorized.
Inspector 0.2.1 preserves all 0.2.0 functional, disclosure (D-01), confinement,
and resource-bound invariants, and preserves `dist/inspector/0.2.0/` untouched.

## Adopted owner decisions (2026-11-21)

### D1 · 0.8.30 integration base ref — Option A (`release/0.8.30`)

- **Description:** Choose which 0.8.30 Git ref `inspector/dev/0.2.1` branches
  from and records as `ENGINE` in `tools/local-release.py`. Both candidates have
  byte-identical `src/rust/`, `Cargo.toml`, `Cargo.lock`, and
  `.cargo/config.toml`; they differ only in 33 post-tag documentation,
  release-state, workflow, and `scripts/` files.
- **Options:**
  - **A — `release/0.8.30` (`72fe93634321616214352a7e07066ce0f93bcf4c`):**
    Post-publication release branch tip (identical Engine/Cargo tree plus
    finalized `scripts/agent-test.sh`, `scripts/check-public-doc-truth.py`,
    smoke scripts, and 0.8.30 publication docs).
  - **B — Peeled `v0.8.30` (`e3d31f5cdc0df3e32dab9bad151c5ab5c321222c`):**
    Immutable tag commit (identical Engine/Cargo tree, pre-publication scripts
    and docs).
- **Ruling:** **Option A (`release/0.8.30`, `72fe93634`)**, recording in
  release metadata that its Rust/Cargo tree is byte-identical to `v0.8.30`
  (`e3d31f5cd`).
- **Justification:** Preserves the exact delivered 0.8.30 Engine and lockfile
  while avoiding stale pre-publication `scripts/` and doc-truth gate failures
  during `./scripts/agent-verify.sh`.

### D2 · Local release and human-study disposition — Option A (carry forward `seq-1`)

- **Description:** Decide whether Inspector 0.2.0's owner ruling
  (`INS-020-LOCAL-RELEASE`, `seq-1`: local-only promotion to gitignored `dist/`,
  no registry publication, and no five-person UX study) carries forward to
  0.2.1.
- **Options:**
  - **A — Carry forward `seq-1` to 0.2.1:** Promote locally to
    `dist/inspector/0.2.1/` (preserving `dist/inspector/0.2.0/`), keep all
    technical, disclosure, browser, and installed qualifications mandatory, and
    waive the five-person UX study and public distribution channel.
  - **B — Require a five-person UX study and/or public distribution for 0.2.1:**
    Expand 0.2.1 beyond compatibility work to add participant sessions or
    public release/signing/CI infrastructure.
- **Ruling:** **Option A (carry forward `seq-1` to 0.2.1).**
- **Justification:** 0.2.1 is an Engine compatibility update with no UI redesign
  or new user workflows (all seven expert UX findings were already remediated
  and verified in 0.2.0 Slice 75). Re-opening a human study or public publisher
  would stall 0.8.30 compatibility on unrelated scope.

### D3 · Producer fixture authority — Option A (published 0.8.29 + 0.8.30)

- **Description:** Define which real closed producer databases (carrying `.lock`
  sentinels; no header-only version edits) are required to qualify 0.2.1 and
  what support claim they authorize.
- **Options:**
  - **A — Published 0.8.29 + published 0.8.30 registry-installed producers:**
    Create fresh fixtures from both published releases; authorizes an
    unqualified dual 0.8.29 + 0.8.30 support claim.
  - **B — Published 0.8.29 + 0.8.30-native Rust workspace producer:** Qualify
    0.8.29 from a published release and 0.8.30 from the integrated 0.8.30 Rust
    Engine; authorizes dual support with an explicit release-note caveat.
  - **C — 0.8.30-only producer fixture:** Qualify only 0.8.30 databases and
    narrow the 0.2.1 support claim to 0.8.30 only.
- **Ruling:** **Option A (published 0.8.29 + published 0.8.30 registry-installed producers).**
- **Justification:** 0.8.29 and 0.8.30 share schema 37, so one shim-free binary
  can serve both when backed by real closed databases from each published
  release, establishing an unqualified dual-version support claim.

### D4 · Build and runtime ABI floor — Option B (drop Ubuntu 22.04 / remove Docker)

- **Description:** Decide whether 0.2.1 keeps 0.2.0's Ubuntu 22.04 (`glibc 2.35`)
  Docker build and installed-test harness or drops Ubuntu 22.04 and Docker to
  build and qualify natively on Ubuntu 24.04 (`glibc 2.39`).
- **Options:**
  - **A — Keep Ubuntu 22.04 Docker (`glibc 2.35` floor):** Reuse
    `inspector70-jammy:desktop`, `local-ci.sh`, ELF `GLIBC <= 2.35`, and the
    network-none X11 container harness.
  - **B — Drop Ubuntu 22.04 and remove Docker (`glibc 2.39` floor):** Run
    `package.sh` and `local-ci.sh` natively on Ubuntu 24.04 (`GLIBC <= 2.39`),
    replace `openbox`/`xdotool` in `installed-harness.mjs` with `/proc` child
    discovery and CDP (`Browser.close` / `page.close()`) under host `Xvfb`,
    run `qualify-installed-network.sh` via parent-wrapped `strace -ff`, and
    retire `tools/ubuntu22.Dockerfile`.
- **Ruling:** **Option B (drop Ubuntu 22.04 support and remove Docker).**
- **Justification:** Inspector is a local web UI with zero Docker or X11
  runtime code; Docker existed only to compile for `glibc 2.35` from an Ubuntu
  24.04 host and supply `openbox`/`xdotool` for the test harness. Dropping
  Ubuntu 22.04 removes container infrastructure and offline container cargo
  friction while keeping full host `Xvfb` + CDP + `strace` qualification and
  Inspector's own Landlock/seccomp/`memfd` confinement intact.

## In scope

| Area | Bounded 0.2.1 deliverable | Owning slice |
| --- | --- | --- |
| Exact 0.8.30 integration | Dedicated `inspector/dev/0.2.1` branch/worktree off `release/0.8.30` (`72fe93634`), selective re-wiring of workspace/CI/docs files without reverting 0.8.30, unchanged `src/rust/` and `.cargo/config.toml`, `Cargo.lock` adding only `fathomdb-inspector`, bare and `cached-cpu` compilation without CUDA/reranker/`fixture-control` | Slice 10 |
| Retrieval ownership & copy | Reconcile `lexical.rs`, `app.js`, and `tests/integration/lexical.rs` with 0.8.29/0.8.30 record-keyed fusion; remove stale `"all_equal_body_contributors": "unknown"` body-dedup copy; qualify equal-body node/edge multiplicity, repeated IDs, retained revisions, A/B separation, `embedder_absent`/`embedder_timeout`, `Closing`, diverged-FTS refusal, and `OwnerlessNodeFts` | Slice 20 |
| Provenance & cached-CPU confinement | Qualify 0.8.30 `source_link_valid` whole-body and partial `utf8_bytes` derivations in both directions alongside raw recorded links, plus bad digest, invalid/mid-character UTF-8, and empty (`start == end`) negative boundaries; requalify deferred open `vector_spans::capture` + `retain_matching_open_seed` and Candle 0.13 / cudarc 0.21 / Tokio 1.54 `cached-cpu` under Landlock/seccomp/`memfd`/`deny_dense_writes` | Slice 20 |
| Docker-free Ubuntu 24.04 tooling & 0.2.1 local release | Drop Ubuntu 22.04/Docker/`openbox`/`xdotool` across `tools/` and `installed-harness.mjs` (native Ubuntu 24.04 `glibc 2.39`, host `Xvfb` + CDP close, parent `strace -ff`); bump binary/packaging/test identity to `0.2.1` and `ENGINE` to `72fe93634`; generate fresh closed published 0.8.29 and 0.8.30 fixtures; pass full `agent-verify.sh`, native `local-ci.sh`, and independent final-dist verification; promote atomically to `dist/inspector/0.2.1/` preserving `dist/inspector/0.2.0/` | Slice 30 |
| Documentation reconciliation & closure | Update all public `docs/inspector/` pages (including new `docs/inspector/releases/0.2.1.md` with final promoted SHA-256 digests and qualification measurements, preserving `0.2.0.md`), `src/inspector/README.md`, `src/inspector/tools/README.md`, and `dev/inspector/` current status/index/plan records; pass `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, and `local-release.py verify --source` | Slice 40 |

## Explicitly out of scope for 0.2.1

- Any change to FathomDB Engine, SDK, MCP, schema, ranker, or shared crate
  versions/pins.
- Exposing `StoredHeadCoverageV1` aggregate counters or `SnippetV1.scored_bytes`
  in Inspector UI/API.
- Adding SDK `release_models()` or any model-release UI control.
- Claiming the 0.8.30 hybrid benchmark speedup (blocked by `include_explanation = true`).
- GPU / CUDA / Metal or cross-encoder reranker execution.
- Multi-window vector embeddings (`RM-12`), nonempty WAL/journal or live-writer
  attachment (`RM-05`, `RM-06`), and backlog items `INSP-B05`–`INSP-B10`.
- Ubuntu 22.04 (`glibc 2.35`) compatibility, Docker build/test pipelines,
  public registry publication, signing, release-tag automation, or OS/arch
  expansion beyond Ubuntu 24.04 / `glibc 2.39` Linux x86_64 with Landlock
  ABI ≥ 3 and Chromium.
- Five-person UX study (waived per D2).

## Quality-testing roadmap disposition (`AGENTS.md` § 1)

Consulting [fathomdb-quality-testing-roadmap.md](../../../notes/fathomdb-quality-testing-roadmap.md):

- **Applied in 0.2.1:** Anomaly/fault boundary testing (diverged FTS,
  `OwnerlessNodeFts`, `Closing` during active query, invalid/empty provenance
  spans, preflight refusals, cache tamper/race), resource-leak and cleanup
  verification on the fresh extracted 0.2.1 binary (headed-browser close,
  nonblocking `flock` reacquisition, private state removal, parent `strace -ff`
  confinement trace), and boundary-value/property checks on equal-body
  multiplicity and UTF-8 byte spans.
- **Deferred:** Disabled-optimization differential testing (Engine owns
  `ranking_tests.rs`, and Inspector runs `include_explanation = true`),
  coverage-guided fuzzing, and 100% MC/DC targets.

## Release acceptance criteria

1. **Exact 0.8.30 Engine composition:** `build-description.json` and
   `candidate.json` identify Inspector `0.2.1`, schema `37`, Engine commit
   `72fe93634321616214352a7e07066ce0f93bcf4c` (byte-identical Rust/Cargo source
   to `v0.8.30` `e3d31f5cd`), `cached_default_cpu: true`, and
   `fixture_control: false`; zero `src/rust/` or 0.8.30 pin regressions.
2. **Faithful 0.8.29/0.8.30 semantics:** Record-keyed retrieval multiplicity,
   updated lexical response/UI copy, degradation codes, `Closing` propagation,
   FTS/ownerless handling, and whole/partial/empty provenance boundary tests
   pass on real databases.
3. **Dual published producer qualification:** Fresh closed real databases from
   published FathomDB 0.8.29 and published FathomDB 0.8.30 (with preserved
   originals and `.lock` sentinels) pass end-to-end workflows across all views,
   all 8 D-01 disclosure combinations, and A/B comparison.
4. **Native Ubuntu 24.04 confinement & lifecycle:** Fresh extracted Linux x86_64
   bundle passes ELF `GLIBC <= 2.39` with zero Docker/Ubuntu 22.04/`openbox`/
   `xdotool` dependencies, 0 Internet-family socket attempts, `O_RDONLY`-only
   pinned asset access, zero cache mutation, bounded cancellation, and clean
   CDP-closed headed-browser shutdown with lock release and descendant cleanup.
5. **Clean full verification & atomic local promotion:** Full
   `scripts/agent-verify.sh`, native `tools/local-ci.sh`, and independent
   final-dist verification pass on `dist/inspector/0.2.1/` while
   `dist/inspector/0.2.0/` remains intact.

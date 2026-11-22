---
title: Inspector 0.2.0 full product and technical assessment
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Source- and evidence-grounded assessment of locally released Inspector 0.2.0 and readiness for 0.2.1
---

# Inspector 0.2.0 full assessment

Inspector 0.2.0 is a **complete, locally released, technically qualified**
connected-exploration GUI for FathomDB schema 37 databases on its declared Linux
x86_64 pairing. Its core functional, safety, disclosure, confinement, UX
remediation, and local-packaging obligations are met. Its **primary technical
gap** is Engine currency: 0.2.0 compiles against pre-release `dev/0.8.29` commit
`6edc1155f039f87cd0a37660cb27ae4373bbce71` (nominally 0.8.28 workspace packages,
Candle 0.11 / cudarc 0.19.10), so it does not reflect published FathomDB 0.8.29
or 0.8.30 retrieval ownership, degradation, provenance, or dependency behavior.

## Release and artifact baseline

| Dimension | Verified 0.2.0 state |
| --- | --- |
| Release branch / closure commit | `inspector/dev/0.2.0` · Slice 90 release at `32593f524165eff613881a8e9b91da917665e503` |
| Source / packaging provenance SHA | `7449ee0108d503cbed3b6a33e2c841072d6de0ce` |
| Local distribution directory | Gitignored `dist/inspector/0.2.0/` (archive `fathomdb-inspector-0.2.0-linux-x86_64.tar.gz`, SHA-256 `0a731d3243b2c5b2e2611578ac3a9c06b4f12ba488af558c4a9530c341dc8e0f`; binary SHA-256 `4fce1f5b03bde56021721d48f990b1427e33a712c40231c9fb15266ee856d2d9`) |
| Embedded Engine & format | `6edc1155f039f87cd0a37660cb27ae4373bbce71` (pre-release `dev/0.8.29`, schema 37, `rusqlite` 0.41.1, `libsqlite3-sys` 0.39.1, `sqlite-vec` 0.1.9) |
| Qualified host pairing | Ubuntu 22.04 / glibc 2.35 Linux x86_64 (ELF max `GLIBC_2.34`), kernel `7.0.0-43-generic` (Landlock ABI 8; minimum required ABI 3), Chromium `156.0.8078.4` |
| Feature posture | Bare builds model-free; shipping artifact enables `--features cached-cpu` and excludes `fixture-control` (`cached_default_cpu: true`, `fixture_control: false`) |
| Distribution posture | Unsigned local tar/bundle only; no public registry publication, no upload tooling |

## Delivered scope and slice closure

All commissioned slices in the [0.2.0 ladder](../plans/0.2.0/README.md) are
closed:

| Slice | Scope delivered | Closure status and evidence |
| --- | --- | --- |
| 0–7, 9, 10 | Architecture, compatibility baseline, security/confinement design, scope ratification, prework (Slice 8 reserved) | Complete; [adopted scope](../plans/0.2.0/scope.md) ratified 2026-11-19 |
| 20 | Coordinated record workspace: catalog, paging, sampling, document/tree/chunk/attribute views, D-01 disclosure, narrow erasure-pending export | Complete (`95a59e5da` product source) |
| 30 | Native lexical inspection: indexed-text windows, Body FTS + highlights snippets, Named Property FTS, native BM25F, exact origin navigation | Complete |
| 40 | Stored single-head vectors, exact bounded L2 neighborhoods, opt-in sealed-cache CPU frozen hybrid search (`search_frozen` with explanations) | Complete (`S9-010` narrows to single head window) |
| 50 | Directed stored graph traversal, full edge content, recorded source/derived links, registered dependencies, retained revision history, frozen graph expansion | Complete (`ee073b74f` candidate) |
| 60 | Bounded two-observation A/B comparison (typed multiset union/intersection + separately verified exact-origin correspondence), end-to-end person investigation, keyboard Back | Complete (`41748e588` full-gate candidate) |
| 70 | Installed technical qualification on Ubuntu 22.04/glibc 2.35, F06 100k+ scale/cancellation/memory/close checks, real 30 s lexical deadline proof | Complete by owner disposition ([INS-020-LOCAL-RELEASE](../plans/0.2.0/release-decisions.md), `seq-1`, removed the unperformed 5-person UX study) |
| 75 | Expert UX evaluation remediation (all 7 P1/P2 findings: durable attachment state, comparison correspondence, layout containment, bounded titles, declared property choices, hub expansion context, compact evidence) | Complete (all 7 RED→GREEN; Sol-high & Terra verified) |
| 80 | Local CI (`tools/local-ci.sh`), offline Docker packaging (`tools/package.sh`), candidate sealing/verification/atomic promotion (`tools/local-release.py`), public & developer docs | Complete (`206/206` strict suites, `12` tooling, `10` JS units, `81` installed checks PASS) |
| 90 | Atomic promotion of qualified Slice 80 candidate to `dist/inspector/0.2.0/` and independent final-dist Terra verification | Complete (`2/2` final installed flows PASS; Alt+F4 close `194 ms` / `159 ms`, `0` network sockets, `3` `O_RDONLY` pinned asset opens) |

## Architecture and codebase health

Inspector's source under `src/inspector/` is compact, self-contained, and
dependency-light (`18` Rust modules, `5` static vanilla web assets with no
bundler/framework, `67` Rust/JS/Python test files, `18` tool scripts, `3`
examples):

- **Strict confinement and fail-closed safety:** The owned worker applies
  Landlock filesystem restriction (requiring kernel ABI ≥ 3), seccomp-BPF
  blocking `socket`/`socketpair`/`connect` on x86_64/aarch64, `memfd_create`
  sealed pinned model assets verified by SHA-256 from `build-description.json`,
  and a SQLite authorizer (`model.rs::deny_dense_writes`) preventing accidental
  mutation of dense/probe tables. Read connections enforce `query_only=ON`,
  30 s / 100k-tick progress-handler deadlines, bounded per-request queued-erasure
  cursor caches (`session.rs`), and 1 MiB response-size / 100k-row caps.
- **Truthful epistemic boundaries:** Lexical, stored-vector L2, serving hybrid,
  stored graph, recorded links, and frozen native evidence stay explicitly
  labeled with separate snapshot/read contexts. D-01 disclosure flags and
  physical deletion semantics hold uniformly across views and export.
- **Clean local release pipeline:** `tools/local-ci.sh`, `tools/package.sh`, and
  `tools/local-release.py` enforce clean-tree checks, offline glibc 2.35 builds,
  ELF symbol-version caps (`GLIBC <= 2.35`), inner/outer SHA-256 manifests, and
  atomic `renameat2` no-overwrite promotion.

## Technical gaps, risks, and stale assumptions in 0.2.0

A full source audit against delivered FathomDB 0.8.30 (`e3d31f5cd` / `72fe93634`)
identifies five concrete gaps in Inspector 0.2.0:

1. **Stale embedded Engine and dependency cohort (primary blocker for 0.8.30):**
   Inspector 0.2.0 embeds `6edc1155f` (pre-release `dev/0.8.29`, Candle 0.11.0,
   cudarc 0.19.10, Tokio 1.52.3, `webpki-roots` 1.0.7, `rustls-webpki` 0.103.13)
   rather than delivered `0.8.30` (Candle 0.13.0, cudarc 0.21.0, Tokio 1.54.1,
   `webpki-roots` 1.0.12, `rustls-webpki` 0.103.16, updated fork patches and
   `.cargo/config.toml` HTTPS mirrors).
2. **Body-deduplication contract baked into code, UI copy, and tests:**
   Published 0.8.29 replaced body-keyed fusion with record-keyed ownership
   (`FusionOwnership` / `RecordKey`). In 0.2.0:
   - `src/inspector/tests/integration/lexical.rs`
     (`native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results`)
     asserts `hits.len() == 2` and single-rank credit for equal-body
     `lexical-b`/`lexical-twin`, which fails immediately on 0.8.29/0.8.30.
   - `src/inspector/src/lexical.rs` (`Reader::lexical`) emits
     `"all_equal_body_contributors": "unknown"`, and `src/inspector/assets/app.js`
     renders `Equal-body contributors: unknown` on body-route lexical hits—stale
     copy once Engine stops merging equal-body records.
3. **Uncovered 0.8.29/0.8.30 Engine semantics:**
   - Partial `utf8_bytes` provenance derivations (0.8.30 `source_link_valid`
     whole-body digest + span validation, plus empty `start == end` refusal) are
     not exercised end-to-end by 0.2.0's tests.
   - New `DegradationReason::EmbedderAbsent` / `EmbedderTimeout`,
     `DispatchError::Closing -> EngineError::Closing`, stricter FTS-vs-canonical
     `body`/`kind` checks, and `OwnerlessNodeFts` rejection in
     `structural_lifecycle_state` / `evidence.rs` are not qualified in 0.2.0.
   - Deferred open-time `vector_spans::capture` + `retain_matching_open_seed`
     has not been exercised on real 0.8.29 and 0.8.30 producer fixtures.
4. **Workspace and offline-build drift relative to 0.8.30:**
   `inspector/dev/0.2.0` modifies 23 files outside `src/inspector/`,
   `dev/inspector/`, and `docs/inspector/`. Carrying Inspector onto 0.8.30
   requires selective re-wiring (especially `.github/workflows/ci.yml`,
   `scripts/test-feature-matrix.toml`, `scripts/tests/test_ci_proportional_routing.py`,
   `mkdocs.yml`, and doc indexes) while keeping
   `scripts/check-cudarc-fathomdb-lock-set.py` and
   `scripts/check-pinned-override-rot.py` green, preserving `.cargo/config.toml`,
   and pre-populating `$CARGO_HOME` for offline Docker builds.
5. **Hardcoded release identity and fixture currency:**
   Version `0.2.0` and Engine SHA `6edc1155f...` are hardcoded across
   `Cargo.toml`, `tools/package.sh`, `tools/local-ci.sh`, `tools/local-release.py`,
   `package/README.md`, browser/tool tests, and docs. The pre-generated
   `local-ci.sh` fixture corpus predates published 0.8.29/0.8.30 producers.

## Quality-testing roadmap alignment

Per `AGENTS.md` § 1, [fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)
guides release scope without imposing automatic gates:

| SQLite-adapted method | 0.2.0 posture | Applicability to 0.2.1 |
| --- | --- | --- |
| Anomaly / fault boundary testing | Covered preflight refusal (missing lock, WAL, corrupt header/schema/profile), cache swap/race, SQLite progress interruption, stale handle expiry, native request deadline | **Apply:** retain all existing boundary tests and add diverged-FTS, `OwnerlessNodeFts`, `Closing` during active query, and invalid/empty provenance-span cases |
| Resource-leak checks | Covered in Slice 70/90 (Alt+F4 close latency, owned descendant cleanup, private dir removal, nonblocking `flock` reacquisition, RSS/heap samples, `strace` 0-socket / `O_RDONLY` trace) | **Apply:** re-run on the fresh 0.2.1 Ubuntu 22.04 extracted binary with the Candle 0.13 / Tokio 1.54 stack |
| Boundary-value & property testing | Covered via `proptest` in lexical, vector, tree, graph, and all 8 D-01 disclosure combinations | **Apply:** add equal-body multiplicity, repeated public IDs, retained revisions, and partial/multibyte/empty provenance span boundaries |
| Disabled-optimization differential checks | Not applicable to Inspector's own frozen-hybrid route (`include_explanation = true` disables 0.8.30's winner-hydration fast path) | **Defer:** Engine's own `ranking_tests.rs` covers that route |
| Coverage-guided fuzzing / 100% MC/DC | Excluded in 0.2.0 | **Defer:** disproportionate to a bounded compatibility release |

## Disposition: what belongs in 0.2.1 vs. backlog

- **Address in Inspector 0.2.1:** Gaps 1–5 above (exact 0.8.30 Engine
  integration, stale equal-body code/UI/test updates, provenance and degradation
  qualification, offline build/lock-set compliance, real 0.8.29 + 0.8.30
  producer qualification, and `0.2.1` local release delivery). See
  [Inspector 0.2.1 scope](../plans/0.2.1/scope.md) and
  [release plan](../plans/0.2.1/plan.md).
- **Retain in backlog / roadmap (outside 0.2.1):** `StoredHeadCoverageV1`
  aggregate UI, `SnippetV1.scored_bytes`, SDK `release_models()`, GPU/CUDA or
  reranker execution, multi-window vectors (`RM-12`), nonempty WAL / live-writer
  attachment (`RM-05`/`RM-06`), public registry publication/signing, additional
  OS/arch platforms, five-person UX study, and `INSP-B05`–`B10`.
- **Delivery outcome:** Inspector `0.2.1` completed all four slices (Slices
  10–40) and promoted locally to `dist/inspector/0.2.1/`; see
  [0.2.1 delivery index](../plans/0.2.1/README.md) and
  [0.2.1 release notes](../../../docs/inspector/releases/0.2.1.md).

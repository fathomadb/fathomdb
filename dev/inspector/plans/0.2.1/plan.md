---
title: Inspector 0.2.1 release plan — FathomDB 0.8.30 support
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: ACTIVE
desc: Four-slice sequential delivery plan for Inspector 0.2.1 on FathomDB 0.8.30 and native Ubuntu 24.04
---

# Inspector 0.2.1 release plan

## Authority and target topology

This plan operationalizes the [adopted Inspector 0.2.1 scope](scope.md) and its
four owner rulings (**D1 = A**, **D2 = A**, **D3 = A**, **D4 = B**). The owner
commissioned Slice 10 on 2026-11-21 with a four-slice ladder (Slices 10, 20,
30, and 40).

- **Release branch & worktree:** `inspector/dev/0.2.1` at
  `/mnt/data/worktrees/inspector-0.2.1`, cut from
  `release/0.8.30` (`72fe93634321616214352a7e07066ce0f93bcf4c`, whose
  `src/rust/`, `Cargo.toml`, `Cargo.lock`, and `.cargo/config.toml` are
  byte-identical to peeled `v0.8.30` `e3d31f5cdc0df3e32dab9bad151c5ab5c321222c`).
- **Writer discipline:** One writer per checkout (`AGENTS.md` § 7). Slices 10,
  20, 30, and 40 execute sequentially in the dedicated `inspector/dev/0.2.1`
  worktree; never mutate the primary checkout (`/home/coreyt/projects/fathomdb`)
  or `dist/inspector/0.2.0/`.
- **Host & runtime floor:** Ubuntu 24.04 / `glibc 2.39` Linux x86_64, kernel
  Landlock ABI ≥ 3, Chromium, host `Xvfb` + CDP, and host `strace` (zero
  Docker, Ubuntu 22.04, `openbox`, or `xdotool` dependencies).
- **Invariants:** TDD mandatory (failing test first; never edit tests merely to
  match broken code; never bless pre-0.8.29 body-dedup outputs as new oracles);
  no mocking the database; no Engine/SDK/MCP/schema/ranker/pin changes; D-01
  disclosure and physical deletion unchanged; checkout-owned `.venv` for all
  Python tooling and full verification.

## Four-slice delivery ladder

| Slice | Name | Depends on | Primary outcome | Exit gate |
| --- | --- | --- | --- | --- |
| **Slice 10** | 0.8.30 base integration & workspace hygiene | Adopted scope (`D1–D4`) | Dedicated `inspector/dev/0.2.1` worktree on `release/0.8.30` (`72fe93634`), Inspector carried forward, selective workspace/CI/docs wiring, clean `Cargo.lock`, checkout `.venv` | Bare + `cached-cpu` compile; `check-cudarc-fathomdb-lock-set.py`, `check-pinned-override-rot.py`, `agent-lint-md.sh`, and `test_ci_proportional_routing.py` PASS; `src/rust/` and `.cargo/config.toml` untouched; pre-existing body-dedup test failure observed and handed off to Slice 20 |
| **Slice 20** | 0.8.30 retrieval, provenance & confinement qualification | Slice 10 | Record-keyed lexical/hybrid behavior, updated `lexical.rs` / `app.js` copy, degradations/`Closing`/FTS/ownerless tests, whole/partial/empty provenance tests, deferred open & Candle 0.13 `cached-cpu` confinement | All Inspector Rust unit/integration/proptest, JS unit, and Playwright browser suites PASS; zero Engine/pin changes; independent code review APPROVE |
| **Slice 30** | Native Ubuntu 24.04 tooling, 0.2.1 packaging, dual-producer qualification & local release | Slice 20 | Docker/Ubuntu 22.04/`openbox`/`xdotool` removed; binary/packaging/test identity bumped to `0.2.1`; fresh published `0.8.29` + `0.8.30` fixtures qualified; full gates run; atomic promotion to `dist/inspector/0.2.1/` | Full `agent-verify.sh`, native `local-ci.sh`, and independent final-dist verification (0 network sockets, `O_RDONLY` assets, ELF `GLIBC <= 2.39`, dual published producers) PASS; `dist/inspector/0.2.0/` preserved; independent release review APPROVE |
| **Slice 40** | Documentation reconciliation & release closure | Slice 30 | Public `docs/inspector/` (including `releases/0.2.1.md` with final artifact digests & qualification evidence), `src/inspector/{README,tools/README}.md`, and `dev/inspector/` status/index/plan records reconciled without changing code or artifacts | `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, and `local-release.py verify "$PWD/dist/inspector/0.2.1" --source` PASS (docs-only descendant contract preserved) |

---

## Slice 10 · 0.8.30 base integration and workspace hygiene

### Objective

Land Inspector cleanly onto `release/0.8.30` (`72fe93634`) in a new
`inspector/dev/0.2.1` worktree without breaking any `0.8.30` Engine, lock-set,
or CI invariants.

### Work items

1. **Create release worktree & branch:**
   - Run `scripts/preflight.sh` checks (clean base, disk space).
   - Cut `inspector/dev/0.2.1` from `release/0.8.30` (`72fe93634`) at
     `/mnt/data/worktrees/inspector-0.2.1`.
   - Record adopted decisions `D1–D4` in `dev/inspector/plans/0.2.1/decisions.jsonl`
     (extending the Inspector-local decision ledger pattern from `0.2.0`).
2. **Carry Inspector trees:**
   - Import `src/inspector/`, `dev/inspector/`, and `docs/inspector/` from the
     approved `inspector/dev/0.2.0` closure HEAD.
3. **Selectively re-wire non-Inspector workspace files:**
   - Re-apply only Inspector wiring to `release/0.8.30`:
     - `Cargo.toml` (add `"src/inspector"` to `[workspace].members`; preserve
       all `[patch.crates-io]` and `[workspace.dependencies]` verbatim).
     - `.github/workflows/ci.yml` (`inspector` path filter + `verify` trigger).
     - `scripts/test-feature-matrix.toml` (`fathomdb-inspector` `features = []`).
     - `scripts/tests/test_ci_proportional_routing.py` (Inspector path cases).
     - `mkdocs.yml`, `docs/index.md`, `docs/generated/contributor-map.md`.
     - `dev/DOC-INDEX.md`, `dev/README.md`, `dev/ROADMAP.md`,
       `dev/design/document-lifecycle.json`, and `dev/design/db-inspector/`
       redirects.
   - **Explicit trap guard:** Do **not** port `0.2.0`'s diffs to
     `dev/platform-capabilities.json`, `scripts/tests/test_public_doc_truth.sh`,
     `scripts/tests/test_jetson_tegra_cuda_evidence_ci_job.sh`,
     `scripts/tests/test_tegra_publication_operator_path.sh`, or
     `src/python/tests/test_slice85_maintenance.py`—verify whether `0.8.30`
     already covers them and only adjust if an Inspector doc/path gate actually
     fails.
4. **Update `Cargo.lock` with zero unintended drift:**
   - Resolve `Cargo.lock` strictly to add the `fathomdb-inspector` workspace
     package (`cargo check -p fathomdb-inspector --no-default-features` and
     `--features cached-cpu` without running `cargo update`).
   - Confirm `git diff release/0.8.30 -- src/rust/ .cargo/config.toml` is empty.
   - Confirm `scripts/check-cudarc-fathomdb-lock-set.py` and
     `scripts/check-pinned-override-rot.py` pass unchanged.
5. **Provision checkout-owned `.venv` and baseline build/test observation:**
   - Set up checkout-owned `.venv` (`scripts/bootstrap.sh` or
     `scripts/setup-python-owned.sh`, Python ≥ 3.12) and `node_modules` symlink
     for `markdownlint-cli2`.
   - Verify bare (`--no-default-features`) and `--features cached-cpu` builds
     compile cleanly (`cached_default_cpu: true`, `fixture_control: false`, no
     CUDA or reranker enabled).
   - Run `cargo test -p fathomdb-inspector` to observe and record the expected
     RED failure on `native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results`
     (body-dedup oracle change) without mutating product logic yet.

### Exit gate (Slice 10)

- `git diff release/0.8.30 -- src/rust/ .cargo/config.toml` is empty.
- Bare and `cached-cpu` Inspector builds pass `cargo check` / `cargo clippy`.
- `scripts/check-cudarc-fathomdb-lock-set.py`,
  `scripts/check-pinned-override-rot.py`,
  `scripts/tests/test_ci_proportional_routing.py`, `scripts/agent-lint-md.sh`,
  and `scripts/agent-lint-docs.sh` pass.
- Independent review confirms clean diff hygiene and no reverted `0.8.30`
  changes.

---

## Slice 20 · 0.8.30 retrieval, provenance, and confinement qualification

### Objective

Reconcile Inspector code, UI copy, and tests with published `0.8.29`/`0.8.30`
retrieval ownership, degradations, FTS consistency, provenance validation, and
the Candle 0.13 / cudarc 0.21 / Tokio 1.54 cached-CPU runtime.

### Work items

1. **Retrieval ownership & stale body-dedup copy (RED → GREEN):**
   - Review `fusion.rs`, `search.rs`, `search_types.rs`, `embedding.rs`,
     `snippet.rs`, and `evidence.rs` against `src/inspector/src/lexical.rs`,
     `src/inspector/src/vector.rs`, `src/inspector/assets/app.js`, and
     `src/inspector/assets/comparison.mjs`.
   - Update `src/inspector/src/lexical.rs` (`Reader::lexical`) and
     `src/inspector/assets/app.js` so body-route lexical hits no longer report
     `"all_equal_body_contributors": "unknown"` / `Equal-body contributors: unknown`
     (reflecting record-keyed fusion accurately while keeping BM25F's
     `"not_inferred"` truthful if retained).
   - Update `native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results`
     in `src/inspector/tests/integration/lexical.rs` to assert native
     record-keyed multiplicity (all 3 hits: `lexical-a`, `lexical-b`, and
     `lexical-twin`, with exact RRF ranks and origins).
   - Add focused RED→GREEN integration and browser/unit tests for:
     - Distinct equal-body canonical nodes, equal-body node/edge pairs,
       repeated public IDs, and retained revisions across lexical, frozen
       hybrid, and A/B comparison (confirming typed multiset membership and
       canonical correspondence remain separate).
2. **Degradations, `Closing`, FTS consistency, and `OwnerlessNodeFts` (RED → GREEN):**
   - Test `DegradationReason::EmbedderAbsent` (`"embedder_absent"`) and
     `DegradationReason::EmbedderTimeout` (`"embedder_timeout"`) passthrough
     in `vector.rs::serving` and `DispatchError::Closing` propagation during
     active-query cancellation/close.
   - Test stricter FTS-vs-canonical `body`/`kind` mismatch refusal
     (`EngineError::Storage` → Inspector `"storage"`).
   - Test `OwnerlessNodeFts` handling: `lexical.rs` withholds ownerless hits
     cleanly while `vector.rs::serving` (`include_explanation = true`) refuses
     truthfully (`"native_query_refused"`).
3. **Provenance semantics (RED → GREEN):**
   - Add real-DB integration tests exercising 0.8.30's
     `dependency_trace.rs::source_link_valid` via
     `structural_state.rs::structural_dependency_state` alongside Inspector's
     own `graph.rs::investigate` recorded links:
     - Valid `whole_body` and partial `utf8_bytes` excerpt derivations (with
       whole-source SHA-256 digest) in both directions (`recorded_source` /
       `recorded_derived`, `registered_dependency_to_source` /
       `registered_dependency_to_dependents`), proving native
       `StructuralDependencyStateV1::Registered`.
     - Negative boundaries: corrupted digest, out-of-bounds or mid-character
       UTF-8 byte offsets, and empty `utf8_bytes` ranges (`start == end`),
       proving raw recorded links remain visible and distinct while native
       structural dependency evidence stays `NotRegistered`.
4. **Admission and cached-CPU confinement requalification:**
   - Exercise Engine open with deferred `vector_spans::capture` +
     `retain_matching_open_seed`, invalid span/profile/layout refusal, and
     model-free fallback.
   - Run `cached-cpu` integration and confinement tests (Landlock ABI ≥ 3,
     seccomp network deny, `memfd` sealed pinned assets, SQLite
     `deny_dense_writes` authorizer, cache tamper/swap/race refusal) on Candle
     0.13.0 / cudarc 0.21.0 / Tokio 1.54.1 / `webpki-roots` 1.0.12.

### Exit gate (Slice 20)

- All Inspector Rust unit, integration, contract, worker, and proptest suites
  (bare and `cached-cpu`), JS unit tests (`comparison.mjs`, `ux.mjs`), and
  Playwright browser suites pass.
- `git diff release/0.8.30 -- src/rust/ .cargo/config.toml` remains empty.
- Independent adversarial code review APPROVE with no open material findings.

---

## Slice 30 · Native Ubuntu 24.04 tooling, 0.2.1 packaging, dual-producer qualification, and local release

### Objective

Remove Ubuntu 22.04 and Docker from Inspector's build/qualification pipeline,
update binary, packaging, and test identity to `0.2.1`, qualify against fresh
closed real databases from both published `0.8.29` and published `0.8.30`, pass
the full source and installed gates, and atomically promote
`dist/inspector/0.2.1/`.

### Work items

1. **Remove Docker, Ubuntu 22.04, `openbox`, and `xdotool` (D4 = B, RED → GREEN):**
   - Update `src/inspector/tools/local-ci.sh`: build natively on Ubuntu 24.04
     (`package.sh` + release examples directly on host; verify `glibc 2.39` and
     ELF `GLIBC <= 2.39`; remove `docker` prerequisite and container stage).
   - Update `src/inspector/tools/run-installed.sh`: run directly on host under
     `Xvfb` with clean temporary `HOME`, read-only permissions check on cache
     inputs, Landlock ABI ≥ 3 probe, and existing `installed-browser.sh` wrapper
     (no Docker, no `openbox`).
   - Update `src/inspector/tests/browser/installed-harness.mjs`: replace
     `xdotool` window-PID lookup with `/proc` child/cmdline discovery and
     replace `xdotool key alt+F4` with CDP window close (`Browser.close` /
     `page.close()`), preserving the exact 25 ms RSS/heap sampling, 30 s close
     deadline, owned descendant exit proof, nonblocking `flock` reacquisition,
     and `fdi-*` scratch removal assertions.
   - Update `src/inspector/tools/qualify-installed-network.sh`: wrap the
     production `fathomdb-inspector` launch with parent `strace -ff -e trace=network,%file`
     (avoiding Ubuntu's `ptrace_scope=1` sibling-attach restriction without
     `sudo` or Docker root) and retain the exact 0-Internet-socket and
     `O_RDONLY`-only pinned asset checks.
   - Retire `src/inspector/tools/ubuntu22.Dockerfile` and update
     `src/inspector/tests/tools/test_local_ci.py` to cover the native Ubuntu
     24.04 flow.
2. **Bump package/code/test release identity to `0.2.1`:**
   - Update version (`0.2.0` → `0.2.1`), archive/directory name
     (`fathomdb-inspector-0.2.1-linux-x86_64`), Engine commit
     (`72fe93634321616214352a7e07066ce0f93bcf4c`, noting byte-identical
     Rust/Cargo source to `v0.8.30` `e3d31f5cd`), and Ubuntu 24.04 /
     `glibc 2.39` support metadata across:
     - `src/inspector/Cargo.toml` (and matching `Cargo.lock` package version)
     - `src/inspector/tools/package.sh`
     - `src/inspector/tools/local-ci.sh`
     - `src/inspector/tools/local-release.py`
     - `src/inspector/package/README.md` (bundled inside the shipping tar)
     - `src/inspector/tests/browser/package.mjs`
     - `src/inspector/tests/tools/test_local_release.py`
     - `src/inspector/tests/tools/test_package.py`
     - `mkdocs.yml` (if nav adjustments are needed before Slice 40, keep code/package boundary clean)
3. **Create and preserve dual published producer fixtures (D3 = A):**
   - Generate fresh closed real databases (carrying `.lock` sentinels;
     originals preserved read-only; no header-only edits) from **published
     FathomDB 0.8.29** and **published FathomDB 0.8.30**, covering catalog,
     body/tree, attributes, FTS/BM25F/property, single-head vectors/L2,
     cached-CPU hybrid, graph/history/whole+partial provenance, A/B comparison,
     all 8 D-01 disclosure combinations, queued erasure, and physical absence.
   - Refresh the baseline fixtures consumed by `tools/local-ci.sh`.
4. **Full qualification gates & atomic local promotion (D2 = A):**
   - Run full `./scripts/agent-verify.sh` once on the clean final candidate
     (checkout-owned `.venv`; unconfined if AC-036 `strace` requires ptrace).
   - Run native `src/inspector/tools/local-ci.sh`, `local-release.py seal`, and
     `local-release.py verify --source`.
   - Promote atomically via `local-release.py promote` into gitignored
     `dist/inspector/0.2.1/` while verifying `dist/inspector/0.2.0/` is
     untouched and zero `dist/` files are tracked.
   - Run independent final-dist verification on a fresh extraction of
     `dist/inspector/0.2.1/fathomdb-inspector-0.2.1-linux-x86_64.tar.gz`
     against both published `0.8.29` and published `0.8.30` fixtures (including
     `strace -ff` 0-socket / `O_RDONLY` proof, ELF `GLIBC <= 2.39`, browser
     close timing, descendant/lock/scratch cleanup, and memory samples).

### Exit gate (Slice 30)

- Full `./scripts/agent-verify.sh` PASS (0 failures, 0 unapproved skips) on the
  final code SHA.
- Native `tools/local-ci.sh` PASS (0 Docker, 0 `openbox`/`xdotool`, ELF
  `GLIBC <= 2.39`).
- Independent final-dist verification PASS on fresh tar extraction against both
  published `0.8.29` and published `0.8.30` fixtures.
- `dist/inspector/0.2.1/` verified via `local-release.py verify --source`;
  `dist/inspector/0.2.0/` preserved byte-for-byte.
- Independent final code/release review APPROVE.

---

## Slice 40 · Documentation reconciliation and release closure

### Objective

Bring all user-facing, contributor, and internal engineering documentation into
exact alignment with the promoted Inspector `0.2.1` release—recording actual
final SHA-256 digests, qualification measurements, and the Ubuntu 24.04 /
Docker-free / dual `0.8.29`+`0.8.30` boundary—strictly as a docs-only
descendant that preserves `local-release.py verify --source`.

### Work items

1. **Public documentation (`docs/inspector/`):**
   - Author `docs/inspector/releases/0.2.1.md` recording the actual Slice 30
     source/packaging SHA, `dist/inspector/0.2.1/` archive and binary SHA-256
     digests, Engine `72fe93634` (`v0.8.30` `e3d31f5cd`) identity, Ubuntu 24.04
     / `glibc 2.39` pairing, dual published `0.8.29` + `0.8.30` qualification
     summary, and unchanged limits. Retain `docs/inspector/releases/0.2.0.md`
     unchanged as historical record.
   - Update `docs/inspector/index.md`, `docs/inspector/getting-started/index.md`,
     `docs/inspector/reference/index.md`, `docs/inspector/ROADMAP.md`,
     `docs/inspector/contributing/index.md`, and `docs/inspector/operations/index.md`
     to reflect `0.2.1`, Engine `0.8.30`, record-keyed retrieval semantics,
     native Ubuntu 24.04 installation, and `dist/inspector/0.2.1/`.
   - Ensure `mkdocs.yml` includes `docs/inspector/releases/0.2.1.md` (note:
     if `mkdocs.yml` must be touched outside the `verify --source` markdown
     allowlist, add the `0.2.1.md` nav entry cleanly during Slice 30 or verify
     `local-release.py` allowlist before sealing Slice 30!).
2. **Source & tooling READMEs:**
   - Update `src/inspector/README.md` and `src/inspector/tools/README.md` to
     document `0.2.1`, Engine `0.8.30`, native Ubuntu 24.04 `local-ci.sh` /
     `run-installed.sh` / `qualify-installed-network.sh`, and `dist/inspector/0.2.1/`.
3. **Internal engineering docs (`dev/inspector/`):**
   - Record final statuses in `dev/inspector/plans/0.2.1/slice-{10,20,30,40}/status.md`,
     update `dev/inspector/plans/0.2.1/README.md`, `dev/inspector/plans/0.2.1/plan.md`,
     `dev/inspector/README.md`, `dev/inspector/reviews/assessment-0.2.0.md`
     (link to `0.2.1` completion), and `dev/inspector/BACKLOG.md` / `ROADMAP.md`
     where stale `0.2.0`-only statements exist.
4. **Closure validation (scoped doc gates + provenance check):**
   - Run `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, and
     `mkdocs build --strict`.
   - Run `.venv/bin/python src/inspector/tools/local-release.py verify "$PWD/dist/inspector/0.2.1" --source`
     to confirm only permitted documentation-closure paths changed after the
     sealed Slice 30 candidate SHA.

### Exit gate (Slice 40)

- `./scripts/agent-lint-md.sh`, `./scripts/agent-lint-docs.sh`, and
  `mkdocs build --strict` PASS with zero warnings/errors.
- `local-release.py verify "$PWD/dist/inspector/0.2.1" --source` PASS from
  clean HEAD (proving zero post-seal code or artifact drift).
- `docs/inspector/releases/0.2.0.md` and `dist/inspector/0.2.0/` remain
  untouched; zero tracked `dist/` files.

---

## Acceptance criteria traceability

| ID | Criterion | Owning slice | Primary verification evidence |
| --- | --- | --- | --- |
| AC21-01 | `inspector/dev/0.2.1` rooted at `release/0.8.30` (`72fe93634`) with untouched `src/rust/` and `.cargo/config.toml` | Slice 10 | `git diff release/0.8.30 -- src/rust/ .cargo/config.toml` empty; `check-pinned-override-rot.py` PASS |
| AC21-02 | `Cargo.lock` adds only `fathomdb-inspector` with zero `cudarc-fathomdb` closure drift | Slice 10 | `scripts/check-cudarc-fathomdb-lock-set.py` PASS |
| AC21-03 | Bare (`--no-default-features`) and `--features cached-cpu` compile cleanly without CUDA, reranker, or `fixture-control` | Slice 10, 30 | `cargo build`/`clippy`, `build-description.json` assertions |
| AC21-04 | Equal-body records/pairs, repeated IDs, retained revisions, and updated lexical response/UI copy reflect record-keyed fusion | Slice 20 | Updated `tests/integration/lexical.rs`, new multi-record integration/browser tests, `comparison.mjs` unit tests |
| AC21-05 | `embedder_absent`, `embedder_timeout`, `Closing`, diverged-FTS refusal, and `OwnerlessNodeFts` behave truthfully | Slice 20 | Focused RED→GREEN Rust integration tests in `lexical.rs` and `vector.rs` |
| AC21-06 | Whole-body, partial `utf8_bytes`, bad digest, invalid/mid-UTF-8, and empty (`start == end`) provenance boundaries hold | Slice 20 | Focused RED→GREEN Rust integration tests in `graph.rs` / `worker.rs` |
| AC21-07 | Deferred open (`vector_spans::capture` + `retain_matching_open_seed`) and Candle 0.13 `cached-cpu` confinement pass | Slice 20, 30 | `model.rs` confinement child tests, admission tests, parent `strace -ff` 0-socket / `O_RDONLY` proof |
| AC21-08 | Ubuntu 22.04, Docker, `openbox`, and `xdotool` removed; native Ubuntu 24.04 (`glibc 2.39`) `Xvfb` + CDP pipeline passes | Slice 30 | Native `tools/local-ci.sh`, ELF `GLIBC <= 2.39`, `tests/tools/test_*.py`, `installed-harness.mjs` |
| AC21-09 | Fresh closed real databases from published `0.8.29` and published `0.8.30` pass all views, D-01 flags, and A/B comparison | Slice 30 | Independent installed qualification logs with preserved original fixture SHA-256s |
| AC21-10 | `dist/inspector/0.2.1/` atomically promoted and verified (`verify --source`) while `dist/inspector/0.2.0/` stays intact | Slice 30, 40 | `local-release.py verify --source`, outer/inner SHA-256 checks, `git check-ignore`, `0.2.0` checksum check |
| AC21-11 | Public `docs/inspector/`, `src/inspector/{README,tools/README}.md`, and `dev/inspector/` reconciled as docs-only closure | Slice 40 | `agent-lint-md.sh`, `agent-lint-docs.sh`, `mkdocs build --strict`, `local-release.py verify --source` |

## Key risks and mitigations

| Risk | Slice | Mitigation |
| --- | --- | --- |
| Accidental lockfile drift breaks `check-cudarc-fathomdb-lock-set.py` | Slice 10 | Never run `cargo update`; resolve `fathomdb-inspector` incrementally on top of `release/0.8.30`'s `Cargo.lock` and run the lock-set guard immediately. |
| Porting all 23 non-Inspector files from `0.2.0` reverts `0.8.30` scripts/docs | Slice 10 | Cherry-pick only the minimal Inspector wiring files (`Cargo.toml`, `ci.yml`, `test-feature-matrix.toml`, routing tests, docs/index). |
| Ubuntu host `yama.ptrace_scope=1` blocks `strace -p` sibling attach once Docker is removed | Slice 30 | Launch `fathomdb-inspector` directly under parent `strace -ff` in `qualify-installed-network.sh` rather than attaching to an already-running sibling PID. |
| CDP window close leaves Chromium helper processes or fails to trigger supervisor exit | Slice 30 | Verify supervisor exit code 0, nonblocking `flock` reacquisition, `/proc` descendant termination, and `fdi-*` scratch deletion within 30 s inside `installed-harness.mjs`. |
| Published `0.8.30` wheel/package unavailable offline when building fixtures | Slice 30 | Install published `0.8.29` and `0.8.30` producers in isolated temporary environments ahead of offline qualification runs, preserving original closed `.sqlite` + `.lock` files with recorded SHA-256 digests. |

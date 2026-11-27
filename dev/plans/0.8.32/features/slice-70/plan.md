---
title: Slice 70 Plan — Authorized 0.8.32 Multi-Registry, GitHub Release & Tegra CUDA Publication
status: DRAFT
target_release: 0.8.32
slice: 70
date: 2026-11-27
---

# Slice 70 Plan — Authorized `0.8.32` Publication (`PUBLICATION`)

- **Gate Posture:** **HELD** until explicit owner HITL publication authorization (Gate 2) is granted. Zero `git push`, zero `v0.8.32` tag creation, and zero registry or Pages uploads occur before authorization.
- **Branch / Worktree:** `dev/0.8.32` in `/mnt/data/worktrees/release-0.8.32` (integrating onto `main` + `refs/heads/release/0.8.32` upon authorization; never mutate `/home/coreyt/projects/fathomdb`).
- **Inherited Frozen Qualification Base:** Slice 60 ([../slice-60/status.md](../slice-60/status.md)) qualified frozen product SHA **`8c0021f051ba5a6a6d0432ac658b531deac9cd49`** across all local gates (`agent-verify.sh` `246/249`, `AGENT_LONG=1` / `--release` perf gates, paired `10k`/`50k`/`100k` + `Gate1-D10` latency report, RTX 3090 `test-feature-complete.sh` `227/227`, Inspector `local-ci.sh --full-workspace` `89/89` Playwright + `qualify-installed-network.sh`, 3-wheel clean-venv + stripped-`PATH` `migrate` / `R32-04` + `690/690` TS tests, and golden Schema 39 archive).
- **Zero-Redundancy Design:** Because Slice 70 performs only the mechanical Axis-W version bump (`0.8.31 -> 0.8.32`, zero product-code changes) and the post-publish documentation/truth flip, Slice 70 **inherits all Slice 60 qualification receipts verbatim** and does **NOT** re-run:
  - `./scripts/agent-verify.sh` or `AGENT_LONG=1` / `--release` perf gates,
  - paired `v0.8.31` vs. `0.8.32` latency benchmarks,
  - local RTX 3090 `test-feature-complete.sh`,
  - local Inspector `local-ci.sh --full-workspace` or `qualify-installed-network.sh`,
  - local wheel `migrate` / `R32-04` proofs, or
  - a separate remote `workflow_dispatch` `dry_run=true` pass of `release.yml` (reconciling [slice-60/plan.md](../slice-60/plan.md) § 1: replaced by fast local `scripts/release/local-dry-run.sh` + `verify-embedder-api-no-drift.sh` + `actionlint` before pushing `v0.8.32`, given `scripts/release/smoke/registry-sdk.rs` and all smoke scripts are already verified compatible with `0.8.32`, and `release.yml`'s `all-builds-passed` gate blocks all registry publication if any cross-OS build such as `build-python-mcp` fails).

---

## 1. Prerequisites & Owner-Confirmed Controls (Phase 70A Entry)

Before pushing `v0.8.32`, confirm all four external prerequisites:

1. **PyPI Pending Publisher (new package `fathomdb-mcp`):** Configured on PyPI for repository `fathomadb/fathomdb`, workflow `release.yml`, environment `pypi` (matching `fathomdb` and `fathomdb-cli`).
2. **crates.io `CARGO_REGISTRY_TOKEN` (`publish-new` scope):** Present as repository secret `CARGO_REGISTRY_TOKEN` for initial creation of `fathomdb-mcp` on crates.io (required by `.github/workflows/release.yml` `"Require first-publish MCP API token before any release tier"` for `RELEASE_TAG == 'v0.8.32'`; subsequent releases migrate to crates.io OIDC Trusted Publisher).
3. **Self-hosted GPU0 runner (`windchill3`):** Register temporary ephemeral org GPU0 runner (`windchill3-gpu0-0832`) on RTX 3090 GPU `0` only (never GPU `2` Quadro K620) in selected GPU group `3`, and verify `cuda-preflight.sh` readiness.
4. **Self-hosted Jetson Orin runner (`jetson-fathomdb` on `10.83.10.13`):** Start owned listener cleanly before dispatching `jetson-tegra-cuda-evidence.yml` (leaving any unrelated runners untouched).

---

## 2. Sequenced Execution Phases (Deduplicated & Parallelized)

### Phase 70A — Mechanical Version Bump & Fast Local Preflight (~5 min)

1. Fast-forward / merge `dev/0.8.32` into `main` (verifying `HEAD` has zero product-code diffs vs. `8c0021f05`).
2. Run mechanical Axis-W version bump (keeping Axis-E `fathomdb-embedder-api` at `0.8.0` and Inspector at `0.3.0`):
   - `bash scripts/set-version.sh --workspace 0.8.32`
   - `cargo update --workspace`
   - Update `CHANGELOG.md` heading to `## 0.8.32 - <YYYY-MM-DD>` (keeping publication status note truthful until live verification completes).
3. Run **fast pre-tag validators only** (zero duplicate heavy qualification):
   - `bash scripts/set-version.sh --check-files` (`Axis W = 0.8.32; Axis E = 0.8.0`)
   - `bash scripts/check-license-consistency.sh`
   - `bash scripts/release/local-dry-run.sh` (covers `verify-release-gates.sh`, `verify-embedder-api-no-drift.sh`, workspace build, leaf-crate package + dry-run publish)
   - `actionlint -config-file .github/actionlint.yaml .github/workflows/*.yml`
4. Commit single mechanical bump commit on `main`: `chore(release): bump to 0.8.32` (recording its 40-hex commit SHA as `RELEASE_SHA`).
5. Create release branch `release/0.8.32` strictly at `RELEASE_SHA` (`git branch release/0.8.32 "$RELEASE_SHA"`), guaranteeing `release/0.8.32` points to the `0.8.32` bump commit rather than `0.8.31`.

### Phase 70B — Pre-Tag Environment Policy, Parallel Tag Cut (`release.yml`) & Jetson Tegra Publication (`jetson-tegra-cuda-evidence.yml`)

Run pre-tag environment policy configuration first, then execute both independent workflows **concurrently** on the exact same immutable bump commit SHA (`RELEASE_SHA`):

1. Push `main` and `refs/heads/release/0.8.32` (`RELEASE_SHA`), and wait for hosted `ci.yml` on `main` to go green per `dev/design/release.md`.
2. **Pre-tag environment policy configuration (mandatory BEFORE pushing `v0.8.32` or dispatching `jetson-tegra-cuda-evidence.yml`):**
   - Run `bash scripts/release/configure-tag-cuda-route.sh --pre-tag v0.8.32` — adds tag policy `v0.8.32` to the `cuda-unmerged-preflight` environment (required by `scripts/hooks/pre-push` -> `check-tag-cuda-route.sh` and `release.yml`'s `build-cuda-linux-x64-gnu`) AND adds branch policy `release/0.8.32` to the `github-pages` environment (required by `jetson-tegra-cuda-evidence.yml`'s `deploy-tegra-pages`).
3. Create signed annotated tag `v0.8.32` at `RELEASE_SHA` (`git tag -a v0.8.32 -m "Release v0.8.32"`).
4. **Stream 1 (`release.yml`):**
   - Push `v0.8.32` with first-tag CUDA route bootstrap (`FATHOMDB_RELEASE_TAG_BOOTSTRAP=v0.8.32 git push origin v0.8.32`).
   - Immediately run post-tag `bash scripts/release/configure-tag-cuda-route.sh v0.8.32` (patching org runner group `3` with the newly created tag ref) and verify marker-free `bash scripts/release/check-tag-cuda-route.sh v0.8.32` **before** approving the required `cuda-unmerged-preflight` GPU environment deployment in GitHub Actions.
   - Monitor `release.yml` through: 5-target builds (`fathomdb`, `fathomdb-cli`, `fathomdb-mcp` with `--features default-embedder`, `fathomdb-inspector` `0.3.0`, N-API, CUDA `linux_x86_64`), tiered crates.io publish (T1–T7 + `fathomdb-mcp` thin default), PyPI OIDC publish (`fathomdb`, `fathomdb-cli`, `fathomdb-mcp`), npm publish (`next` -> `latest` promotion after all registry smokes), `co-tagging-assert`, all post-publish registry consumer smokes (including `crates-sdk`, `crates-cli`, `crates-mcp`, `pypi-wheel`, `pypi-mcp`, `npm-package`, and 5-target native smokes), and GitHub Release `v0.8.32` asset upload.
5. **Stream 2 (`jetson-tegra-cuda-evidence.yml`, launched immediately after Step 2 `--pre-tag` succeeds — in parallel with `release.yml`):**
   - Dispatch `jetson-tegra-cuda-evidence.yml` on `refs/heads/release/0.8.32` with `candidate_sha=<RELEASE_SHA>`, `candidate_version=0.8.32`, `publish_to_pages=true`.
   - Monitor all 7 jobs: candidate validation, Jetson build + CUDA witness, driverless ARM64 Node import, Tegra Pages build & deploy, public URL verification, and Jetson installed public wheel + Node tarball smokes.

### Phase 70C — Independent Live-Registry Consumer Smokes & Runner Teardown

1. Run local installed-from-registry verification against live public registries (verifying exact `0.8.32` artifacts — zero local wheel/path fallbacks):
   - `bash scripts/release/smoke/smoke-pypi-wheel.sh 0.8.32` (`fathomdb[cli]==0.8.32`)
   - `bash scripts/release/smoke/smoke-pypi-mcp.sh 0.8.32` (`fathomdb[mcp]==0.8.32` / `fathomdb-mcp==0.8.32`)
   - `bash scripts/release/smoke/smoke-crates-sdk.sh 0.8.32`
   - `bash scripts/release/smoke/smoke-crates-mcp.sh 0.8.32`
   - Verify `npm view fathomdb dist-tags` (`latest: "0.8.32"`, `next: "0.8.32"`), GitHub Release `v0.8.32` assets & checksums (`fathomdb`, `fathomdb-cli`, `fathomdb-mcp`, `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz`), and live Tegra Pages retention + new `0.8.32` artifacts via `bash scripts/release/verify-tegra-pages-live.sh --manifest scripts/release/tegra-pages-retained.json --added "$WHEEL_FILENAME=$WHEEL_SHA256" --added "$TARBALL_FILENAME=$TARBALL_SHA256"`.
2. **Runner cleanup (mandatory before closeout):**
   - Stop and deregister temporary GPU0 org runner (`windchill3-gpu0-0832`), confirm `orgs/fathomadb/actions/runners` `total_count: 0`, and delete its scratch directory.
   - Stop owned Jetson Orin listener `jetson-fathomdb` cleanly while preserving its registration/credentials and leaving unrelated runners untouched.

### Phase 70D — Single Post-Publication Truth Flip, 4-Tier Docs Finalization & Closeout

1. Delegate to a **single dedicated 4-tier documentation update subagent** to perform the atomic post-publication truth flip:
   - Add `"0.8.32": 39` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py`.
   - Advance `dev/platform-capabilities.json` to `0.8.32`.
   - Append the two verified `0.8.32` Tegra Pages artifacts (wheel + Node tarball, byte sizes + SHA-256) to `scripts/release/tegra-pages-retained.json`, preserving all prior retained entries.
   - Remove pre-publication qualification banners/stamps across `docs/release-notes/0.8.32.md`, `mkdocs.yml`, `CHANGELOG.md`, `README.md`, and any remaining `docs/` references; regenerate `docs/generated/*` if needed.
   - Update `dev/plans/release-state-0.8.32.json` matching `scripts/check-public-doc-truth.py` and `dev/plans/release-state-0.8.31.json`:
     - `"published"` JSON object: `{ "tag": "v0.8.32", "tag_object": "<40-hex>", "tag_commit": "<RELEASE_SHA>", "published_on": "<YYYY-MM-DD>", "npm_dist_tag": "latest", "release_run_id": <id>, "tegra_run_id": <id> }`
     - `"landed": [10, 20, 30, 40, 41, 50, 60, 70]`, `"remaining_ladder": []`, `"next_slice": null`
     - `"publish_gate": { "status": "PUBLISHED", "publication_authorized": true, "scope": "..." }`
     - `"release_readiness": { "status": "PUBLISHED_COMPLETE", "publication_authorized": true, "pages_publication_authorized": true, "public_release_notes": "https://fathomadb.github.io/fathomdb/release-notes/0.8.32/" }`
   - Update `dev/plans/runs/STATUS-0.8.32.md` and write `dev/plans/0.8.32/features/slice-70/status.md`.
2. Run **scoped doc/truth/state/MkDocs validators only** (per `AGENTS.md` § 3 — zero duplicate `agent-verify.sh` since zero product source changed):
   - `.venv/bin/python scripts/check-public-doc-truth.py`
   - `bash scripts/tests/test_public_doc_truth.sh`
   - `bash scripts/check-glibc-floor-doc-truth.sh`
   - `.venv/bin/python scripts/check-release-contract-truth.py`
   - `bash scripts/check-platform-capabilities.sh`
   - `bash scripts/release/verify-tegra-pages-live.sh --manifest scripts/release/tegra-pages-retained.json`
   - `bash scripts/check-release-state-views.sh --write && bash scripts/check-release-state-views.sh`
   - `bash scripts/agent-lint-docs.sh`
   - `bash scripts/agent-lint-md.sh`
   - `mkdocs build --strict -d /tmp/fathomdb-0832-final-mkdocs`
3. Commit and push post-publication truth flip on `main`, confirm public docs GitHub Pages deployment succeeds, and record Slice 70 `COMPLETE`.

---

## 3. Acceptance Criteria (`AC32-70-*`)

| AC ID | Falsifiable Verification Signal |
| --- | --- |
| `AC32-70-01` | Mechanical bump commit `chore(release): bump to 0.8.32` (`RELEASE_SHA`, with `release/0.8.32` created at `RELEASE_SHA`) carries zero product-code diffs vs. Slice 60 frozen SHA `8c0021f05`; `set-version.sh --check-files` (`Axis W = 0.8.32; Axis E = 0.8.0`), `check-license-consistency.sh`, `local-dry-run.sh`, and `actionlint` exit `0`. |
| `AC32-70-02` | Pre-tag `configure-tag-cuda-route.sh --pre-tag v0.8.32` runs before tag push and Tegra dispatch; signed annotated tag `v0.8.32` passes post-tag `configure-tag-cuda-route.sh v0.8.32` + marker-free `check-tag-cuda-route.sh v0.8.32`; `release.yml` completes all builds, crates.io T1–T7 + `fathomdb-mcp`, PyPI (`fathomdb`, `fathomdb-cli`, `fathomdb-mcp`), npm (`latest` = `0.8.32`), `co-tagging-assert`, all post-publish registry smokes, and GitHub Release `v0.8.32` (including Inspector `0.3.0`). |
| `AC32-70-03` | Concurrent `jetson-tegra-cuda-evidence.yml` (`release/0.8.32`, `publish_to_pages=true`) passes 7/7 jobs and serves verified `0.8.32` Tegra wheel + Node tarball on GitHub Pages with all prior retained artifacts intact. |
| `AC32-70-04` | Independent local live-registry smokes (`smoke-pypi-wheel.sh 0.8.32`, `smoke-pypi-mcp.sh 0.8.32`, `smoke-crates-sdk.sh 0.8.32`, `smoke-crates-mcp.sh 0.8.32`, `npm view`, and `verify-tegra-pages-live.sh --manifest scripts/release/tegra-pages-retained.json --added "$WHEEL_FILENAME=$WHEEL_SHA256" --added "$TARBALL_FILENAME=$TARBALL_SHA256"`) exit `0`; temporary GPU0 runner deregistered (`total_count: 0`) and `jetson-fathomdb` listener stopped cleanly. |
| `AC32-70-05` | Post-publication 4-tier docs & truth flip (`RELEASE_SCHEMAS["0.8.32"] = 39`, `dev/platform-capabilities.json`, `tegra-pages-retained.json`, full `"published"` JSON object + `"next_slice": null` + `"remaining_ladder": []` + `"publish_gate.status": "PUBLISHED"` + `"release_readiness"` with `pages_publication_authorized: true`, pre-publication stamps removed) passes all scoped doc/truth/state/live-manifest/MkDocs validators and deploys public docs cleanly. |

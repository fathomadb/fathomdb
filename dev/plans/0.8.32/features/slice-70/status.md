---
title: Slice 70 Status — Authorized 0.8.32 Publication
status: COMPLETE
target_release: 0.8.32
slice: 70
date: 2026-11-27
---

# Slice 70 Status — Authorized `0.8.32` Publication

Core multi-registry, GitHub Release (including 5-target `fathomdb-mcp` archives
and FathomDB Inspector `0.3.0`), and 4-tier public documentation truth-flip
delivery for FathomDB **`0.8.32`** are complete; the final Phase 70C Jetson
Tegra Pages refresh on `release/0.8.32` (`06abc24c6`) follows immediately in a
separate follow-up commit.

- **Signed annotated tag `v0.8.32`:** tag object
  `cb0710636b45f01ca79cb36edbff045db8c291c1`, peeled commit (`RELEASE_SHA`)
  `06abc24c6fdbaf046a5b75120ed2e1b677b1c844`.
- **Version axes:** Axis-W = `0.8.32`, Axis-E (`fathomdb-embedder-api`) =
  `0.8.0` (verified drift-free via `verify-embedder-api-no-drift.sh`),
  Inspector = `0.3.0`, `SCHEMA_VERSION` = `39`.
- **Pre-tag commits on top of Slice 60 base (`8c0021f05`):**
  - `46b3db441` — `ci(release): publish fathomdb-mcp via crates.io OIDC Trusted Publishing and approve Slice 70 plan`
  - `947787625` — `chore(release): bump to 0.8.32`
  - `15d19f573` — `fix(engine): capture monotonic visibility generation floor before WAL read tx and restore group-3 repos via PUT` (remediated the `read_visibility_generation` race surfaced before tagging)
  - `06abc24c6` — `ci(release): stage MCP release archives under workspace-relative path and normalize Windows backslashes in upload-artifact-retry` (remediated Windows MCP archive upload path handling before final tag cut)

## 1. Phase 70A & 70B — Core Multi-Registry Delivery & Finalization Recovery

- **Production `release.yml` run [`46080698565`](https://github.com/fathomadb/fathomdb/actions/runs/46080698565) (`52/53` jobs PASSED):**
  - Passed all **24** builds: `build-cuda-linux-x64-gnu` on temporary GPU0
    runner `windchill3-gpu0-0832`, `build-inspector` (`0.3.0`), all `5`
    `build-python-mcp`, all `5` `build-python-cli`, all `4` hosted
    `build-python`, all `4` hosted `build-napi`, and `build-rust`.
  - Published **9/9** crates.io crates via OIDC Trusted Publishing (tiers
    `T1–T7`, including `fathomdb-mcp@0.8.32`).
  - Published **15/15** PyPI wheels via OIDC (`fathomdb==0.8.32`,
    `fathomdb-cli==0.8.32`, and `fathomdb-mcp==0.8.32` across all 5 native
    targets).
  - Published **6/6** npm packages under dist-tag `next` (`5` platform
    packages + `fathomdb@0.8.32`).
  - Passed `co-tagging-assert`, `wait-for-package-registry-visibility`, and
    all **10/10** post-publish registry consumer smoke jobs.
  - Sole failure: `promote-npm-latest` failed with `E403`, which caused
    `github-release` to be skipped by gate design.
- **Recovery / finalization `promote-npm-release.yml` run [`46088912921`](https://github.com/fathomadb/fathomdb/actions/runs/46088912921) (`3/3` jobs PASSED):**
  - Passed `verify-production`, `promote-npm-latest`, and `github-release`.
  - Verified live npm `fathomdb` dist-tags: `{ latest: '0.8.32', next: '0.8.32' }`.
  - Verified live GitHub Release [`v0.8.32`](https://github.com/fathomadb/fathomdb/releases/tag/v0.8.32)
    with all **27** assets (`5` `fathomdb` wheels, `5` `fathomdb_cli` wheels,
    `5` `fathomdb_mcp` wheels, `5` `fathomdb-mcp` archives + `5` `.sha256`
    checksums, and `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` + `.sha256`).
- **GPU0 runner cleanup:**
  - Temporary GPU0 runner `windchill3-gpu0-0832` (`id: 177`) stopped,
    deregistered (`orgs/fathomadb/actions/runners` `total_count: 0`), and its
    scratch directory removed cleanly.

## 2. Phase 70C — Local Live-Registry Smokes & Final Jetson Orin Tegra Pages Refresh

- **Independent local live-registry consumer smokes (4/4 PASSED, zero local wheel/path fallbacks):**
  - `bash scripts/release/smoke/smoke-pypi-wheel.sh 0.8.32` → `EXIT 0` (`fathomdb[cli] 0.8.32` + CLI + Slice 55 native trace/explain/refusals + clean process exit).
  - `bash scripts/release/smoke/smoke-pypi-mcp.sh 0.8.32` → `EXIT 0` (`fathomdb[mcp] 0.8.32` installed from PyPI + stdio MCP smoke).
  - `bash scripts/release/smoke/smoke-crates-sdk.sh 0.8.32` → `EXIT 0` (`fathomdb-sdk 0.8.32` resolved from crates.io + independent write/reopen).
  - `bash scripts/release/smoke/smoke-crates-mcp.sh 0.8.32` → `EXIT 0` (`fathomdb-mcp 0.8.32` installed from crates.io + stdio MCP smoke).
- **Pre-fix `947787625` Tegra withdrawal:**
  - Pre-fix `947787625` Tegra artifacts were published by run
    [`46054193600`](https://github.com/fathomadb/fathomdb/actions/runs/46054193600)
    prior to `15d19f573` (`read_visibility_generation` race fix) and `06abc24c6`
    (Windows MCP upload fix).
  - `docs-pages.yml` run [`46090774665`](https://github.com/fathomadb/fathomdb/actions/runs/46090774665)
    (`4/4` jobs PASSED on `6854ed767`) published the live `0.8.32` docs site
    and explicitly withdrew the pre-fix `947787625` `0.8.32+tegra` files while
    preserving all 11 retained `0.8.26`–`0.8.31` entries.
  - Because GitHub Pages deduplicates `actions/deploy-pages` by
    `pages_build_version` (`GITHUB_SHA`), run
    [`46090992338`](https://github.com/fathomadb/fathomdb/actions/runs/46090992338)
    on `6854ed767` passed Jetson build, CUDA witness, driverless ARM64 import,
    and Pages artifact assembly (`b4f24ef1...` / `6f1f9194...`) but GitHub Pages
    reused the `46090774665` deployment for `6854ed767`. Advancing `main` and
    `release/0.8.32` via this status-only commit (zero product-code diffs vs.
    `06abc24c6`, outside `docs-pages.yml` push paths) supplies a distinct
    `pages_build_version` for the final 7/7 `jetson-tegra-cuda-evidence.yml`
    run, after which `tegra_run_id` and `scripts/release/tegra-pages-retained.json`
    will be finalized.

## 3. Phase 70D — Post-Publication 4-Tier Truth Flip & Public Docs

- Added `"0.8.32": 39` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py`
  and advanced `dev/platform-capabilities.json` to `0.8.32`.
- Removed pre-publication qualification banners/stamps across `README.md`,
  `CHANGELOG.md`, `mkdocs.yml`, `src/ts/README.md`, and `docs/**`, advancing
  all install, reference, operations, compatibility, and release-note pages to
  published `0.8.32`.
- Updated `dev/plans/release-state-0.8.32.json` (`published` receipt,
  `completion: { ref: "origin/release/0.8.32", main_integration: "COMPLETE" }`,
  `Slice70-Publish` moved to `decisions.ruled`, `landed: [10, 20, 30, 40, 41, 50, 60, 70]`,
  `remaining_ladder: []`, `next_slice: null`, `publish_gate.status: "PUBLISHED"`,
  `release_readiness.pages_publication_authorized: true`), closed
  `dev/plans/runs/STATUS-0.8.32.md` and `dev/plans/0.8.32/plan.md`, and
  regenerated release-state views.

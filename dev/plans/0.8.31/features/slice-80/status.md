---
title: Slice 80 Status — Authorized 0.8.31 Publication
status: COMPLETE
target_release: 0.8.31
slice: 80
date: 2026-11-23
---

# Slice 80 Status — Authorized `0.8.31` Publication

Registry, Tegra CUDA Pages, GitHub Release (including FathomDB Inspector `0.3.0`), and final public documentation delivery for FathomDB **`0.8.31`** are complete.

- **Annotated tag `v0.8.31`:** object `48a224e5ac691d5b88a7979b920e87352d31e87f`, peeled commit `8965af93d6f42d160474acb382596c5aab6b5c2d` (`chore(release): bump to 0.8.31`, zero product-code changes relative to frozen Slice 70 qualification SHA `f13244689`).
- **Version axes:** Axis-W = `0.8.31`, Axis-E (`fathomdb-embedder-api`) = `0.8.0` (verified drift-free via `verify-embedder-api-no-drift.sh`), Inspector = `0.3.0`, `SCHEMA_VERSION` = `38`.
- **Pre-tag checks:** `scripts/set-version.sh --check-files`, `scripts/check-license-consistency.sh` (MIT), `scripts/release/local-dry-run.sh` (all 5 steps GREEN), first-tag CUDA route bootstrap (`FATHOMDB_RELEASE_TAG_BOOTSTRAP=v0.8.31`), and marker-free `scripts/release/check-tag-cuda-route.sh v0.8.31` all passed before `cuda-unmerged-preflight` approval.

## 1. Core Multi-Registry Delivery & Finalization Recovery

- **Production release run [`45318429916`](https://github.com/fathomadb/fathomdb/actions/runs/45318429916):**
  - Passed **all** preflight, build, publish, visibility, and co-tagging jobs: `verify-release`, `verify-cuda-trusted-route`, `gitleaks-history-advisory`, `build-rust`, `build-inspector`, `build-cuda-linux-x64-gnu` (on temporary GPU0 runner `windchill3-gpu0-0831`, `cuda-preflight.sh --rerank-cuda`), all 5 `build-python-cli`, all 4 hosted `build-python`, all 4 hosted `build-napi`, `all-builds-passed`, `publish-rust-t1-embedder-api` through `publish-rust-t7-cli`, `publish-pypi` (`fathomdb==0.8.31` and new `fathomdb-cli==0.8.31` via OIDC), all 5 `publish-npm-platform-*` + `publish-npm`, `wait-for-package-registry-visibility`, and `co-tagging-assert`.
  - Passed **7 of 8** post-publish registry consumer smoke jobs: `crates-cli`, `pypi-wheel` (`fathomdb[cli]==0.8.31`), `npm-package`, `aarch64`, `darwin-x64`, `darwin-arm64`, and `win32-x64`.
  - Sole failure: `post-publish-smoke (crates-sdk)` failed compiling its smoke fixture `scripts/release/smoke/registry-sdk.rs` against published `fathomdb-sdk@0.8.31` (`E0063: missing field properties in initializer of PreparedWrite`). No published crate, wheel, or npm artifact was defective; `promote-npm-latest` and `github-release` were skipped by gate design.
- **Bounded finalization recovery (`64e21c3c5`, run [`45326999920`](https://github.com/fathomadb/fathomdb/actions/runs/45326999920)):**
  - Added `properties: None` to `scripts/release/smoke/registry-sdk.rs` and added bound `0.8.31` recovery mode (`--recover-0831-crates-sdk`) to `scripts/release/verify-public-smoke-recovery.py` and `.github/workflows/promote-npm-release.yml` (with TDD coverage in `scripts/tests/test_public_smoke_recovery.py`). No package rebuild, republish, or retag occurred.
  - Run `45326999920` passed `verify-production`, `recover-crates-sdk-smoke` (completing 8/8 registry consumer scopes), and `promote-npm-latest` via OIDC (`npm view fathomdb dist-tags` → `latest: "0.8.31"`, `next: "0.8.31"`).
  - Independent local live consumer proofs also passed: `scripts/release/smoke/smoke-crates-sdk.sh 0.8.31` against live crates.io and `scripts/release/smoke/smoke-pypi-wheel.sh 0.8.31` against live PyPI (`fathomdb[cli]==0.8.31`, stripped-`PATH` CLI + Slice 55 native smoke + clean process exit).
- **GitHub Release [`v0.8.31`](https://github.com/fathomadb/fathomdb/releases/tag/v0.8.31):**
  - Published (`isDraft: false`, `isPrerelease: false`) with all **12** exact production artifacts downloaded from run `45318429916`:
    - 5 `fathomdb-0.8.31-*.whl` SDK wheels (`linux_x86_64`, `linux_aarch64`, `macosx_10_12_x86_64`, `macosx_11_0_arm64`, `win_amd64`)
    - 5 `fathomdb_cli-0.8.31-*.whl` CLI binary wheels across the same 5 targets
    - `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` (`sha256: 17660b3b55f1a92f5d6057ebaaf8ee0fd12bbc2b0097880082d049cf41ac3edd`) and its `.sha256` checksum file.

## 2. Jetson Orin Tegra CUDA Delivery & Runner Cleanup

- **Tegra run [`45325322000`](https://github.com/fathomadb/fathomdb/actions/runs/45325322000)** on `release/0.8.31` (`8965af93d6f42d160474acb382596c5aab6b5c2d`, `publish_to_pages=true`): **7/7 jobs PASSED** (candidate validation, Jetson build + CUDA witness, driverless ARM64 Node import, Pages build & deploy, public URL verification, and Jetson installed public wheel + Node tarball smokes).
- **Verified public Tegra Pages artifacts (appended to `scripts/release/tegra-pages-retained.json`, preserving all 9 earlier entries):**
  - `fathomdb-0.8.31+tegra-cp310-abi3-linux_aarch64.whl` (`19,203,517` bytes, `sha256: 68317c85cfbe44033fb4c97d97ad2bb2db0581c254ea4bce7c749c4232c6be24`)
  - `fathomdb-linux-arm64-gnu-tegra-0.8.31.tgz` (`19,641,632` bytes, `sha256: 92383afdac15ffadef5400a0ba30845db3ed3ae5da5f13c9aef3b6a257fcf570`)
- **Runner cleanup:**
  - Temporary GPU0 org runner `windchill3-gpu0-0831` (`id=170`) stopped, deregistered (`orgs/fathomadb/actions/runners` `total_count: 0`), and `/tmp/fathomdb-gpu-release-0831` removed cleanly.
  - Owned Jetson Orin listener `jetson-fathomdb` (`id=22` on `10.83.10.13`) stopped cleanly while keeping its registration/credentials and the separate `memex` runner untouched.

## 3. Post-Publication Truth Flip (`P3-21`) & Public Docs Pages

- Added `"0.8.31": 38` to `RELEASE_SCHEMAS` in `scripts/check-public-doc-truth.py`, advanced `dev/platform-capabilities.json` to `0.8.31`, updated `scripts/release/tegra-pages-retained.json`, removed pre-publication banners across `README.md`, `CHANGELOG.md`, `mkdocs.yml`, and `docs/`, authorized `release_readiness.pages_publication_authorized: true` in `dev/plans/release-state-0.8.31.json`, and regenerated release-state views.

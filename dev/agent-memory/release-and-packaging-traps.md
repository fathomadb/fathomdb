# Release, Packaging, and CI Control-Plane Traps

Hard-won rules preventing partial publishes, broken smoke gates, and repo
corruption during FathomDB releases.

## Immutable tag & multi-registry publish hazards

- **Pushing any `v*` tag fires the real multi-registry publish**
  (`.github/workflows/release.yml` → crates.io `T1–T7`, PyPI `fathomdb` and
  `fathomdb-cli`, npm 5 platform packages + main `fathomdb`, GitHub Releases).
  Never push a `v*` tag without explicit HITL publication authorization.
- **Wait for cross-OS `ci.yml` on `main` before tagging.** Ensure macOS and
  Windows build/runtime checks on the release commit pass before cutting an
  immutable `vX.Y.Z` tag.
- **Check Trusted / Pending Publishers and new-package pre-flight before
  Slice 0 / pre-tag.** PyPI OIDC Trusted Publishing is per PyPI project name
  (Pending Publisher: `fathomadb/fathomdb`, workflow `release.yml`, environment
  `pypi`); crates.io OIDC Trusted Publishing cannot bootstrap a non-existent
  crate — reserve the name once with a `0.0.0-reserve` publish + `cargo yank`
  and bind OIDC to `release.yml` before tagging. Also verify any new cross-OS
  `upload-artifact-retry` glob stages under a workspace-relative forward-slash
  path (never raw `${{ runner.temp }}/...` mixed with Windows backslashes).
- **npm `promote-npm-latest` OIDC note (`0.8.32` follow-up):** In `v0.8.32`,
  `release.yml`'s `promote-npm-latest` failed with `E403` (`46080698565`),
  while `promote-npm-release.yml` (`46088912921`) promoted `fathomdb@0.8.32`
  to `latest` via OIDC cleanly. Audit `https://www.npmjs.com/package/fathomdb/access`
  in `0.8.33` Slice 0 so `release.yml` finishes 53/53 on the first attempt.
- **No "green CI = done."** After publication, directly verify live registry
  metadata (`crates.io`, `PyPI`, `npm` `dist-tags`), run clean-venv
  registry-installed consumer smokes (`smoke-pypi-wheel.sh`,
  `smoke-pypi-mcp.sh`, `smoke-crates-sdk.sh`, `smoke-crates-mcp.sh`), verify
  GitHub Release assets, and confirm HTTP `200` on live Docs Pages and all
  retained Tegra URLs.

## GitHub API & git control-plane hazards

- **Never hand-craft `PATCH orgs/fathomadb/actions/runner-groups/3`.** GitHub
  rejects `selected_repository_ids` in `PATCH` (HTTP 422) and wipes
  `selected_repositories` to `[]`, leaving `build-cuda-linux-x64-gnu` stuck in
  `queued`. Always use `scripts/release/configure-tag-cuda-route.sh <tag>`
  (which restores repositories via `PUT .../repositories`).
- **Never run `git fetch --depth=1` inside a full repo or worktree.** Doing so
  writes `.git/shallow` into the parent git directory and breaks ancestry checks
  such as `scripts/check-runtime-checkpoints.py`.
- **Gate Tegra `publish_to_pages=true` after `ci.yml` + `release.yml` builds
  settle, and never reuse a deployed SHA on GitHub Pages.** Run
  `jetson-tegra-cuda-evidence.yml` with `publish_to_pages=false` if starting
  early; only dispatch `publish_to_pages=true` on the final `RELEASE_SHA`, and
  verify `scripts/release/check-tegra-pages-route.sh <ver> fathomadb/fathomdb <sha>`
  passes so GitHub Pages does not deduplicate `actions/deploy-pages` by
  `pages_build_version` (`GITHUB_SHA`). Keep `main` and `release/<ver>` fixed
  while the Tegra workflow is in flight.
- **Guard `github-release` in `promote-npm-release.yml` with an explicit `if:`.**
  GitHub Actions skips downstream jobs without an explicit `if:` when any
  upstream sibling in the DAG was skipped.

## Python packaging & worktree environments

- **Never run `pip install -e` or `maturin develop` from a worktree**, and
  never share `.venv` across checkouts. Always provision a checkout-owned
  noneditable wheel via `bash scripts/setup-python-owned.sh --provision`.
- **Compile standalone release smoke fixtures before tagging.** Keep
  `scripts/release/smoke/registry-sdk.rs` compile-checked in
  `scripts/agent-typecheck.sh` so Rust SDK struct changes never break
  `post-publish-smoke (crates-sdk)` in production.

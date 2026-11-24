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
- **Check Trusted / Pending Publishers for any *new* package name before
  Slice 0 / pre-tag.** PyPI OIDC Trusted Publishing is per PyPI project name;
  crates.io OIDC also requires per-crate registration/bootstrap. Verify every
  new package (e.g., `fathomdb-cli` in `0.8.31`, `fathomdb-mcp` in `0.8.32`)
  before tagging so a partial publish cannot happen mid-run.
- **No "green CI = done."** After publication, directly verify live registry
  metadata (`crates.io`, `PyPI`, `npm` `dist-tags`), run clean-venv
  registry-installed consumer smokes (`smoke-pypi-wheel.sh`,
  `smoke-crates-sdk.sh`), verify GitHub Release assets, and confirm HTTP `200`
  on live Docs Pages and all retained Tegra URLs.

## GitHub API & git control-plane hazards

- **Never hand-craft `PATCH orgs/fathomadb/actions/runner-groups/3`.** Sending
  `visibility: "selected"` without `selected_repository_ids` silently wipes
  `selected_repositories` to `[]` and leaves `build-cuda-linux-x64-gnu` stuck
  in `queued`. Always use `scripts/release/configure-tag-cuda-route.sh <tag>`.
- **Never run `git fetch --depth=1` inside a full repo or worktree.** Doing so
  writes `.git/shallow` into the parent git directory and breaks ancestry checks
  such as `scripts/check-runtime-checkpoints.py`.
- **Keep `main` and `release/<ver>` fixed during Jetson Tegra dispatch.**
  `jetson-tegra-cuda-evidence.yml` checks source currency against `origin/main`;
  do not push post-publish closure commits to `main` until the Tegra workflow
  completes.
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

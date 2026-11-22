---
title: Slice 7 — CI/CD and release-friction forensics
status: COMPLETE
slice: 7
target_release: 0.8.31
date: 2026-11-22
---

# Slice 7 — CI/CD and release-friction forensics

Read-only inspection of `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
`.github/workflows/promote-npm-release.yml`, `scripts/agent-verify.sh`,
`src/inspector/tools/{local-ci.sh,run-installed.sh,qualify-installed-network.sh,installed-browser.sh,qualify-startup.mjs,local-release.py,package.sh}`,
`src/inspector/tests/{browser/installed-harness.mjs,browser/comparison.mjs,launch.rs,tools/test_local_ci.py}`,
`scripts/security/{check-no-listen.sh,check-netns-deny-egress.sh,check-netns-deny-egress-catch.sh,gitleaks-history.sh}`,
`scripts/release/{wait-for-registry-version.py,check-tag-cuda-route.sh,cuda-preflight.sh,smoke/smoke-pypi-wheel.{sh,ps1},smoke/smoke-npm-package.sh}`,
`scripts/{set-version.sh,verify-release-gates.sh,check-license-consistency.sh,check-release-contract-truth.py}`,
and `dev/plans/0.8.30/features/slice-80/status.md` in `release/0.8.31`.

*(Note: Slice 0 commits `db271323f` and `20269cee4` already closed the Ubuntu 24.04
`/usr/bin/python3.12` preference over `uv` Python 3.13 and `src/ts/node_modules`
preflight in `scripts/bootstrap.sh` / `test_dev_environment_tools.sh`; those are
not re-proposed below.)*

## Proposals

### 1. Scoped Inspector local CI (`local-ci.sh`)

- **P7-01 — Decouple `src/inspector/tools/local-ci.sh` from full-monorepo `agent-verify.sh` (~5 min scoped default, retaining `--full-workspace`)** *(Target: Slice 50, `R31-08`)*
  - **Observed gap:** `local-ci.sh` (`stage source bash scripts/agent-verify.sh`) unconditionally runs the full ~45-minute monorepo suite even when only `src/inspector/**` changed.
  - **Fix:** Default `local-ci.sh` to `--scoped` (rejecting combinations of `--scoped`, `--full-workspace`, and `--installed-only` with exit 2):
    - Run an Inspector-scoped source gate: `scripts/agent-lint.sh`, `scripts/agent-typecheck.sh` (or `cargo check --locked --offline -p fathomdb-inspector --all-targets` + `--no-default-features --features cached-cpu`), and `cargo test --locked --offline -p fathomdb-inspector` (both default features and `--no-default-features --features cached-cpu`), followed by existing `tooling` (`unittest`), `unit` (`node --test`), `native-build`, `seal`, `candidate`, `abi`, and `installed` stages.
    - Retain `--full-workspace` to invoke `bash scripts/agent-verify.sh`, and fail closed with an automatic escalation (or explicit refusal requiring `--full-workspace`) if `git diff --name-only` against `origin/main` (or `HEAD~1`) touches shared paths outside `src/inspector/**`, `dev/inspector/**`, and `docs/inspector/**` (`src/rust/`, `src/python/`, `src/ts/`, `Cargo.toml`, `Cargo.lock`, `scripts/`).
    - Update `src/inspector/tests/tools/test_local_ci.py` to cover both `--scoped` and `--full-workspace`, and update `src/inspector/tools/README.md` to record that `--scoped` is not a full-workspace `agent-verify.sh` claim (`AGENTS.md` §3).

### 2. Per-command timeouts on process-spawning, GUI, and `strace` harnesses

- **P7-02 — Bound Inspector GUI, `strace`, and process-spawning harnesses with explicit timeouts and non-blocking cleanup** *(Target: Slice 50, `R31-08`)*
  - **Observed gaps:**
    - `run-installed.sh` executes `node "$@"` with no `timeout` wrapper, and `cleanup()` calls `kill "$display_pid"` + `wait` on `Xvfb` without a deadline or `SIGKILL` fallback.
    - `qualify-installed-network.sh` runs `run-installed.sh` with no `timeout`; a traced child holding stdout/stderr FDs open after failure can block indefinitely.
    - `installed-harness.mjs` spawns `strace -ff ...` with undrained `stdout` pipe, runs `flock` and `--build-description` without `timeout:`, and awaits `browser.close()` / post-`SIGKILL` `exited` in `finally` without a deadline.
    - `qualify-startup.mjs`, `comparison.mjs`, `clipboard-diagnostic.mjs`, and `src/inspector/tests/launch.rs` lack bounded timeouts / `SIGKILL` escalation.
  - **Fix:** Wrap `run-installed.sh` and `qualify-installed-network.sh` in `timeout --kill-after=10s`, bound `Xvfb` teardown (`SIGTERM` -> 2s -> `SIGKILL`), drain/ignore unused `stdout` pipes, add explicit `timeout:` on every `execFileSync`/`spawnSync`, race `browser.close()` and post-`SIGKILL` exit against 5s timers with `/proc` descendant reaping, and bound `launch.rs` waits to 30s with `SIGKILL` escalation.

- **P7-03 — Add explicit `timeout` bounds to security `strace`/`unshare` gates** *(Target: Slice 40 or 50)*
  - Wrap `strace` / `unshare` calls in `scripts/security/check-no-listen.sh`, `check-netns-deny-egress.sh`, and `check-netns-deny-egress-catch.sh` in `timeout --kill-after=5s 60s` (`15s` for catch `/dev/tcp` probe) and emit explicit diagnostics on exit `124`.

### 3. Docker-free native Ubuntu 24.04 qualification (`D4 = B`)

- **P7-04 — Retain and regression-lock `D4 = B`, and update `local-release.py` for schema 38** *(Target: Slice 50, `R31-08`)*
  - Keep Docker-free native Ubuntu 24.04 qualification (`Xvfb -displayfd`, `/proc` PID discovery, CDP `Browser.close`, parent-wrapped `strace -ff`, ELF `GLIBC <= 2.39`), statically assert zero `docker`/`openbox`/`xdotool`/`sudo` calls in Inspector test scripts, and update `local-release.py` / `test_local_release.py` to `compiled_schema = 38` and the `0.8.31` Engine baseline.

### 4. `fathomdb-cli` PyPI binary wheels and Inspector release artifacts in `release.yml`

- **P7-05 — Wire lockstep `fathomdb-cli` PyPI binary wheels across build, version/license gates, visibility wait, and smoke** *(Target: Slice 40 / 60 / 70, `R31-05`)*
  - Add a `bindings = "bin"` Maturin manifest (`src/python-cli/pyproject.toml` pointing to `../rust/crates/fathomdb-cli/Cargo.toml` with `LICENSE` + `README.md`) and `cli = ["fathomdb-cli==<Axis-W>"]` extra in `src/python/pyproject.toml`.
  - Extend `scripts/set-version.sh`, `scripts/check-license-consistency.sh`, `wait-for-registry-version.py`, `smoke-pypi-wheel.{sh,ps1}`, `check-release-contract-truth.py`, `test_release_workflow_scope.sh`, and `test_linux_first_platform_scope.sh` in lockstep.

- **P7-06 — Wire Inspector release artifact build, `D4 = B` ABI/seal check, and GitHub Release upload into `release.yml`** *(Target: Slice 50 / 60 / 70, `R31-08`)*
  - Add a `build-inspector` job (`ubuntu-24.04`) running `package.sh`, `check-glibc-floor.sh`, `local-release.py seal`/`verify`, and `tests/browser/package.mjs`, uploading `inspector-dist-linux-x86_64` to `github-release`, and updating `test_release_workflow_scope.sh` and `check-release-contract-truth.py`.

### 5. Residual `0.8.30` Slice 80 release friction

- **P7-07 — Poll PyPI PEP 503/691 `/simple/<pkg>/` index in `wait-for-registry-version.py`** *(Target: Slice 40 / 60)*
  - Fixes root cause of `0.8.30` Slice 80 PyPI smoke failures: poll both `/pypi/<pkg>/<ver>/json` and `/simple/<pkg>/` (`Accept: application/vnd.pypi.simple.v1+json`, verifying all 5 target wheel filenames), plus bounded retry in `smoke-pypi-wheel.{sh,ps1}` for regional CDN skew.
- **P7-08 — Populate GitHub Release body automatically from `CHANGELOG.md`** *(Target: Slice 40 / 70)*
  - Extract `## [v]<version>` into `$RUNNER_TEMP/release-body.md` and pass `body_path:` to `softprops/action-gh-release`.
- **P7-09 — Propagate Bash 3 `EXIT`-trap completion sentinel and Windows Slice 55 smoke parity** *(Target: Slice 40 / 60)*
  - Add `smoke_complete=1` sentinel to `smoke-npm-package.sh`, `smoke-crates-cli.sh`, and `smoke-crates-sdk.sh`, and add `smoke_slice55_installed.py` parity to `smoke-pypi-wheel.ps1`.
- **P7-10 — Remove per-release hardcoded version gates and reconcile `gitleaks-history` advisory baseline** *(Target: Slice 40 / 60)*
  - Derive allowed tag in `check-tag-cuda-route.sh` from `Cargo.toml` `[workspace.package].version`, clean up stale `0.8.20`/`0.8.27`/`0.8.30` branches in release workflows, update `RELEASE_SCHEMAS` in `check-public-doc-truth.py` for `"0.8.31": 38`, and reconcile the 71 pre-`0.8.30` historical tuples in `gitleaks-history-manifest.json`.

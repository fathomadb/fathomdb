---
title: 0.8.32 Prework Slice 7 — CI/CD & Release-Friction Forensics
status: COMPLETE
target_release: 0.8.32
slice: 7
date: 2026-11-24
---

# 0.8.32 Prework Slice 7 — CI/CD & Release-Friction Forensics

## 1. Executive summary

Audit of `dev/0.8.32` (`38a1594bf`), main CI run `45414522686`, `0.8.31` release closure (`97580d39f`, `dev/plans/0.8.31/features/slice-80/status.md`), Owner Ruling 3 (`fathomdb-mcp` release distribution), and R32-07 diagnostic tightening targets identifies:

1. **Five concrete root causes behind the two failing CI jobs on `38a1594bf`** (`windows-wal-checkpoint-diagnosis` and 4 suites in `verify-fast`), all introduced by `97580d39f` (`0.8.31` post-publish doc truth flip), `1b07584e1` (`migrate_cli.rs` host-path assumption), and `38a1594bf` (wiring `migrate_cli` into Windows CI and `check-release-smoke-sdk-compile.sh` into `agent-typecheck.sh` without regenerating `docs/generated/contributor-map.md`).
2. **Full-stack `fathomdb-mcp` release pipeline wiring** required to satisfy Owner Ruling 3 (5-target GitHub Releases stdio binaries, crates.io `fathomdb-mcp`, and lockstep PyPI wheel `fathomdb-mcp` mirroring `src/python-cli/` and `build-python-cli`), including pre-tag registry publisher prerequisites and contract-test updates (`upload-artifact-retry` count, `promote-npm-release.yml` artifact parity, `set-version.sh`, `check-license-consistency.sh`, `wait-for-registry-version.py`).
3. **Specific R32-07 diagnostic tightening edits** across `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, and `dev/tools/module-boundary-gate/src/{main,config}.rs` so gate failures name exact owner files (`governed-operation-parity.json`, `governed-surface-allowlist.json`, `governed-surface-pin.json`, `module-boundary-policy.txt`, `config.rs`).

---

## 2. Forensics of CI run `45414522686` (`38a1594bf`)

### 2.1 `windows-wal-checkpoint-diagnosis` (`138517232891`) — all 5 `migrate_cli` tests failed

- **Observed failure:**
  Commit `38a1594bf` added `cargo test --locked -p fathomdb-cli --test migrate_cli -- --nocapture` to `.github/workflows/ci.yml` (`windows-wal-checkpoint-diagnosis`). All 5 tests panicked at `src\rust\crates\fathomdb-cli\tests\migrate_cli.rs:31:43` (`copy golden schema-37 fixture: Os { code: 3, kind: NotFound }`).
- **Root cause:**
  `src/rust/crates/fathomdb-cli/tests/migrate_cli.rs:13-14` hardcodes `"/mnt/data/worktrees/fathomdb-0.8.31-golden/golden-0.8.30-schema37.sqlite"`, which only exists on the local Linux workstation.
- **Latent sibling defect:**
  `src/rust/crates/fathomdb-engine/tests/slice50_migrate_37_to_38.rs:17-18` carries the identical hardcoded local path (masked only because `Cargo.toml:1027-1028` gates it behind `required-features = ["operator", "migration-test-hooks", "test-hooks"]`).
- **Remediation:**
  Track the immutable 634,880-byte Schema-37 golden fixture (`sha256: 59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548`) under `dev/fixtures/golden-0.8.30-schema37.sqlite` (outside `src/rust/crates/*` so published `.crate` tarballs and wheels do not bloat), and resolve it in both tests via `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../dev/fixtures/golden-0.8.30-schema37.sqlite")` (+ optional `FATHOMDB_GOLDEN_SCHEMA_37_PATH` override) while retaining the SHA-256 assertion.

### 2.2 `verify-fast` (`138517232905`) — 4 of 227 suites failed (`223 passed, 4 failed`)

All 4 failures were directly reproduced read-only on `/mnt/data/worktrees/release-0.8.32`:

| Suite | Script | Observed Error | Root Cause & Commit | Structural Fix |
| --- | --- | --- | --- | --- |
| `test-tegra-publication-operator-path` | `scripts/tests/test_tegra_publication_operator_path.sh` | `FAIL  runbook names the exact Tegra distribution` (exit 1) | Line 46 hardcodes `0.8.30+tegra`, whereas `97580d39f` updated `docs/operations/tegra-pages-publication.md:9` to `fathomdb==0.8.31+tegra`. *(Secondary catch: `docs/reference/cli.md:182` still cites `0.8.30+tegra`.)* | Derive expected newest `+tegra` version dynamically from `scripts/release/tegra-pages-retained.json` instead of hardcoding `0.8.30+tegra`; update `docs/reference/cli.md:182`. |
| `test-pages-site` | `scripts/tests/test_pages_site.sh` | `stale generated documentation: docs/generated/contributor-map.md; run python3 scripts/generate-public-docs.py --write` (exit 1) | `38a1594bf` modified `scripts/agent-typecheck.sh` without regenerating `docs/generated/contributor-map.md:50`. | Run `python3 scripts/generate-public-docs.py --write` **and** wire `python3 scripts/generate-public-docs.py --check` into `scripts/agent-lint.sh`. |
| `test-tegra-pages-retention` | `scripts/tests/test_tegra_pages_retention.sh` | `FAIL  site builder failed` (cascading 5 sub-failures, exit 1) | Identical root cause to `test-pages-site` (`build-pages-site.sh:119` runs `generate-public-docs.py --check`). | Same fix as `test-pages-site` above. |
| `test-agent-typecheck-pyright-version` | `scripts/tests/test_agent_typecheck_pyright_version.sh` | `FAIL typecheck-rust-release-smoke-sdk (exit=127)` (`check-release-smoke-sdk-compile.sh: No such file or directory`) | `38a1594bf` added `check-release-smoke-sdk-compile.sh` to `scripts/agent-typecheck.sh:38-39`; isolated `$FIX` fixture omits it and exits 127 before reaching Pyright guard. | Move Pyright version preflight before Rust `run_capped` checks in `scripts/agent-typecheck.sh` **and** copy/stub `check-release-smoke-sdk-compile.sh` in `$FIX`. |

---

## 3. Remaining `0.8.31` release friction & baseline observations

1. **Resolved by `38a1594bf`:** `check-release-smoke-sdk-compile.sh` in `agent-typecheck.sh`; `configure-tag-cuda-route.sh`; `promote-npm-release.yml:148` `github-release` `if:` guard.
2. **Still open entering `0.8.32`:**
   - **First-time registry onboarding for `fathomdb-mcp`:** PyPI (`404`) and crates.io (`publish = false`) have no existing project/crate. Crates.io OIDC Trusted Publishing cannot publish a brand-new crate without prior crate registration or `CARGO_REGISTRY_TOKEN` (mirroring `v0.8.27` `fathomdb-sdk` in `release.yml:112,1312`), and PyPI requires a Pending Publisher for `fathomdb-mcp` configured before tagging.
   - **Duplicate `github-release` artifact list in `promote-npm-release.yml`:** Any new `fathomdb-mcp` artifacts must be added to **both** `release.yml` and `promote-npm-release.yml`.
   - **Stale `0.8.30+tegra` doc reference:** `docs/reference/cli.md:182`.
   - **Hardcoded version-test gaps:** `scripts/tests/test_release_version_surfaces.sh:95-106` omits `fathomdb-mcp`; `scripts/check-public-doc-truth.py:61` stops `RELEASE_SCHEMAS` at `"0.8.31": 38`.

---

## 4. Exact wiring checklist for Owner Ruling 3 (`fathomdb-mcp` distribution)

### 4.1 Crate & Python package manifests

1. **`src/rust/crates/fathomdb-mcp/`:** Flip `publish = true`, update `description`, add `readme = "README.md"` + `keywords`, add `README.md` + byte-identical `LICENSE`. Freeze release feature set (`default = []` vs `default-embedder`) across crates.io, PyPI, and GitHub Release binaries.
2. **`src/python-mcp/`:** Mirror `src/python-cli/` (`pyproject.toml` with `maturin>=1.9,<2`, `bindings = "bin"`, `targets = [{ name = "fathomdb-mcp", kind = "bin" }]`, byte-identical `LICENSE`, `README.md`); decide whether `fathomdb[mcp]` extra mirrors `fathomdb[cli]`.

### 4.2 Version, license, docs, and registry-gate scripts

| File | Required Change |
| --- | --- |
| `scripts/set-version.sh` | Wire `src/python-mcp/pyproject.toml` (and optional `mcp` extra). |
| `scripts/verify-release-gates.sh` | Add `fathomdb-mcp` to `PUBLISHABLE_CRATES` and preflight first-publish token/publisher check. |
| `scripts/release/cargo-publish-if-new.sh` | Add `fathomdb-mcp` to `is_dependent_crate()` (depends on `fathomdb-sdk` at T6). |
| `scripts/check-license-consistency.sh` | Add `src/python-mcp` to `python_pkg_dirs`. |
| `scripts/check_python_wheel_contents.py` | Add `--kind mcp` requiring `.data/scripts/fathomdb-mcp(?:\.exe)?$` and forbidding `_fathomdb*.so/.pyd`. |
| `scripts/release/wait-for-registry-version.py` | Validate `src/python-mcp/pyproject.toml` and include `pypi/fathomdb-mcp`. |
| `scripts/generate-public-docs.py` | Add `src/python-mcp/pyproject.toml` to `contributor_index()`. |
| `scripts/release/verify-public-smoke-recovery.py` | Include `crates-mcp` / `pypi-mcp` checks. |

### 4.3 GitHub Actions workflows (`.github/workflows/release.yml` & `promote-npm-release.yml`)

1. **`build-python-mcp` job (5-target matrix matching `build-python-cli`):** `PyO3/maturin-action` (`manylinux: "2_28"` on Linux x64/arm64, `auto` on macOS/Windows), validate wheel via `check_python_wheel_contents.py --kind mcp`, run `check-glibc-floor.sh` on Linux binary, package that same `manylinux_2_28`-compliant binary into standalone GitHub Release archives (`fathomdb-mcp-<version>-<target>.{tar.gz,zip}` + `.sha256`), and upload `python-mcp-dist-*` + `mcp-bin-*` via `upload-artifact-retry`.
2. **`all-builds-passed` gate:** Require `build-python-mcp` (preserving `v0.8.20` recovery clause).
3. **`publish-rust-t7-mcp`:** Publish in Tier 7 after `publish-rust-t6-facade` (`fathomdb-sdk`), handling first-publish auth cleanly, gated into `wait-for-package-registry-visibility` and `co-tagging-assert`.
4. **`publish-pypi`:** Download `python-mcp-dist-*` alongside `python-dist-*` and `python-cli-dist-*` (PyPI Pending Publisher pre-configured).
5. **Post-publish registry smokes:** Add `scripts/release/smoke/smoke-crates-mcp.sh` (install + stdio `initialize`/`tools/list` round-trip) and PyPI `fathomdb-mcp` wheel + glibc-floor + stdio smoke across platforms.
6. **`github-release` in BOTH `release.yml` AND `promote-npm-release.yml`:** Download `python-mcp-dist-*` and `mcp-bin-*`.

### 4.4 Static release-contract tests to update in lockstep

- `scripts/check-release-contract-truth.py`
- `scripts/tests/test_release_workflow_scope.sh` *(note line 296 currently hardcodes `upload-artifact-retry` count `15`!)*
- `scripts/tests/test_release_contract_truth.sh`, `test_runtime_release_alignment.sh` *(line 262 exact smoke matrix string)*, `test_linux_first_platform_scope.sh`, `test_actionlint_fixture.sh`
- `scripts/tests/test_set_version.sh`, `test_verify_release_gates.sh`, `test_release_version_surfaces.sh`, `test_check_license_consistency.sh`, `test_python_wheel_contents.py`, `test_wait_for_registry_visibility.py`, `test_cargo_publish_if_new.sh`, `test_generated_public_docs.py`, `test_smoke_scripts.sh`

---

## 5. R32-07 diagnostic tightening targets

1. **`scripts/check-sdk-surface-parity.py`:** Name `src/conformance/governed-operation-parity.json`, `src/conformance/governed-surface-allowlist.json`, `scripts/governed-surface-pin.json`, `scripts/check-sdk-surface-parity.py` constants, and corresponding binding source paths on every failure.
2. **`scripts/check-governed-surface-pin.sh`:** Cross-reference `governed-operation-parity.json`, `check-sdk-surface-parity.py`, and HITL sign-off (`dev/steward/steward-ledger.jsonl`) in `fail(...)`/`die_env(...)`.
3. **`dev/tools/module-boundary-gate/src/{main,config}.rs`:** Append explicit pointers to `dev/tools/module-boundary-policy.txt`, `dev/tools/module-boundary-gate/src/config.rs`, and `src/rust/crates/fathomdb-engine/src/lib.rs` on every policy/classification/cfg failure.

---

## 6. Proposals (`P7-01` – `P7-10`)

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation | Target Slice / Rationale |
| --- | --- | --- | --- | --- | --- | --- |
| `P7-01` | Track `golden-0.8.30-schema37.sqlite` under `dev/fixtures/` and resolve via `CARGO_MANIFEST_DIR` (+ env override) in `fathomdb-cli/tests/migrate_cli.rs` and `fathomdb-engine/tests/slice50_migrate_37_to_38.rs` | H | L | S | include | **Slice 10 (`Prework Prep`)** — Fixes `windows-wal-checkpoint-diagnosis` failure on `38a1594bf` without bloating `.crate`/wheel archives. |
| `P7-02` | Derive expected latest `+tegra` version dynamically from `scripts/release/tegra-pages-retained.json` in `scripts/tests/test_tegra_publication_operator_path.sh` and update `docs/reference/cli.md:182` | H | L | S | include | **Slice 10 (`Prework Prep`)** — Fixes `test-tegra-publication-operator-path` failure on `38a1594bf` and prevents repeat post-publish breakage. |
| `P7-03` | Regenerate `docs/generated/contributor-map.md` (`generate-public-docs.py --write`) and wire `python3 scripts/generate-public-docs.py --check` into `scripts/agent-lint.sh` | H | L | S | include | **Slice 10 (`Prework Prep`)** — Fixes both `test-pages-site` and `test-tegra-pages-retention` failures and catches generated-doc drift in `lint`. |
| `P7-04` | Move Pyright version preflight before Rust `run_capped` checks in `scripts/agent-typecheck.sh` and copy/stub `check-release-smoke-sdk-compile.sh` in `scripts/tests/test_agent_typecheck_pyright_version.sh` | H | L | S | include | **Slice 10 (`Prework Prep`)** — Fixes `test-agent-typecheck-pyright-version` exit-127 failure on `38a1594bf`. |
| `P7-05` | Tighten error diagnostics in `scripts/check-sdk-surface-parity.py`, `scripts/check-governed-surface-pin.sh`, and `dev/tools/module-boundary-gate/src/{main,config}.rs` to name exact registry/policy/config files | H | L | S | include | **Slice 10 (`Prework Prep` / `R32-07`)** — Directly aids Slice 20 hotspot splits and `scope` parity changes. |
| `P7-06` | Wire `fathomdb-mcp` crates.io publishing (`publish = true`, `LICENSE`, `README.md`, `verify-release-gates.sh`, `cargo-publish-if-new.sh`, `publish-rust-t7-mcp`, `smoke-crates-mcp.sh`, and first-publish `CARGO_REGISTRY_TOKEN` / Trusted Publisher bootstrap) | H | M | M | include | **MCP slice (publisher setup in Slice 10)** — Implements crates.io leg of Owner Ruling 3. |
| `P7-07` | Create `src/python-mcp/` (`pyproject.toml`, `LICENSE`, `README.md`) and wire 5-target `build-python-mcp` (`manylinux_2_28` glibc-floor checked), standalone 5-target GitHub Release archives + `.sha256`, `publish-pypi`, `github-release` (in both `release.yml` and `promote-npm-release.yml`), `set-version.sh`, `check-license-consistency.sh`, `check_python_wheel_contents.py`, `wait-for-registry-version.py`, `generate-public-docs.py`, and PyPI Pending Publisher | H | M | L | include | **MCP slice (Pending Publisher in Slice 10)** — Implements PyPI wheel + GitHub Release 5-target stdio binary legs of Owner Ruling 3. |
| `P7-08` | Update static workflow/contract tests alongside `fathomdb-mcp` pipeline edits, replacing hardcoded `upload-artifact-retry` count (`15`) with named job assertions | H | M | M | include | **MCP slice** — Prevents `verify-fast` contract-test regressions. |
| `P7-09` | Freeze `fathomdb-mcp` release feature set (`default = []` vs `default-embedder`) in Slice 10 ADR before MCP packaging | H | L | S | include | **Slice 10 (`Prework Prep` / ADR)** — Aligns `cargo install`, `pip install`, and GitHub Release binary capabilities and size. |
| `P7-10` | Add `fathomdb-mcp` + `src/python-mcp/pyproject.toml` assertions to `test_release_version_surfaces.sh` and `test_verify_release_gates.sh`, and record `"0.8.32": 38` in `check-public-doc-truth.py` | H | L | S | include | **MCP slice & Qualification/Publish slices** — Closes remaining release-surface test drift. |

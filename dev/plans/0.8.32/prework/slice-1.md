---
title: 0.8.32 Prework Slice 1 — Library and security sweep
status: COMPLETE
target_release: 0.8.32
slice: 1
date: 2026-11-24
---

# 0.8.32 Prework Slice 1 — Library and security sweep

**Scope & subject:** read-only manifest, lockfile, fork-pin, packaging-readiness, fuzzing-isolation, and live GitHub Dependabot alert audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bff7c11b12468581da70eef55e6884373`). No files were modified, and no packages were installed.

**Executive summary:**

- **Active shipping manifests carry `0` of the `150` open GitHub Dependabot alerts.** Root `Cargo.lock` (455 records across 13 workspace members, including `rustix 1.1.6`, `zip 9.8.1`, `rustls 0.23.46`, `rustls-webpki 0.103.16`, `webpki-roots 1.0.12`, and `tokio 1.54.1`), `src/ts/package-lock.json` (zero runtime dependencies), `src/python/pyproject.toml`, `src/python/uv.lock`, and `src/python-cli/pyproject.toml` all have **zero** open alerts.
- **All `150` open alerts come from non-shipping surfaces:** `1` low-severity dev-only npm alert in root `package-lock.json` (`katex 0.16.47`), `43` alerts in the offline CPython 3.12 ONNX exporter (`dev/tools/onnx/export-requirements.txt`), `20` orphan alerts against a **deleted pre-0.6.0 file** (`python/uv.lock`, removed in `39ee27126`), `5` alerts in a historical 0.8.29 experiment lockfile (`dev/experiments/0.8.29-slice60-projected-metadata/driver/Cargo.lock`), and `81` alerts across 9 immutable 0.8.27 benchmark-result lockfiles (`dev/plans/0.8.27/features/slice-135/results/*/Cargo.lock`).
- **7 of 10 remote `origin/dependabot/*` branches** are now satisfied or superseded on `HEAD` (including `multi-def6ea3883`, which landed in `0.8.31`) and can be pruned cleanly.
- **`fathomdb-mcp` publication readiness** requires concrete manifest, license, wheel-packaging, release-script, and publisher-setup work before tag time (`LICENSE`, `README.md`, `publish = true`, `examples/` & self-`dev-dependency` package audit, `src/python-mcp/`, `set-version.sh`, `check-license-consistency.sh`, `cargo-publish-if-new.sh`, `wait-for-registry-version.py`, `release.yml`, and PyPI/crates.io publisher registration).
- **Coverage-guided fuzzing** must isolate `cargo-fuzz`/`libfuzzer-sys`/`arbitrary` inside a standalone `fuzz/` workspace excluded from root `Cargo.toml` so shipping crate dependency graphs incur zero bloat.

---

## 1. Triage of all 150 open Dependabot alerts

Source: live GitHub Dependabot API snapshot (`dependabot-summary.md`, 2026-11-24) reconciled against tracked files in `dev/0.8.32` @ `38a1594bf` and `origin/main`.

### 1.1 Summary by manifest, surface class, and severity

| Manifest path | Ecosystem | Exists in `HEAD`? | Surface class | Crit | High | Med | Low | Total | Packages affected & disposition |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| `Cargo.lock` (root workspace) | `rust` | Yes | **Active shipping** | 0 | 0 | 0 | 0 | **0** | Clean (`zip 9.8.1`, `rustls 0.23.46`, `rustix 1.1.6`, `tokio 1.54.1`, `webpki-roots 1.0.12`). |
| `src/ts/package-lock.json` | `npm` | Yes | **Active shipping** | 0 | 0 | 0 | 0 | **0** | Clean (zero runtime deps; 4 dev packages). |
| `src/python/uv.lock` | `pip` | Yes | **Active shipping** *(stale dev lock)* | 0 | 0 | 0 | 0 | **0** | Clean (20 packages, no ML runtime deps; frozen at `fathomdb 0.8.20`). |
| `package-lock.json` (root) | `npm` | Yes | Active dev tooling (`private: true`, never shipped) | 0 | 0 | 0 | 1 | **1** | `katex 0.16.47` (`#77`, `GHSA-238p-pmpm-9mq7 / CVE-2026-103923`, patched `0.18.2`). Pulled transitively via `markdownlint-cli2 0.24.2` → `markdownlint 0.42.2` → `micromark-extension-math 3.1.0` (`katex: ^0.16.0`). **Keep `overrides` empty**; do not add a governed npm override for a non-shipping low-severity alert (`P1-03`, `P1-12`). |
| `dev/tools/onnx/export-requirements.txt` | `pip` | Yes | Offline dev/export tool (never shipped) | 1 | 9 | 24 | 9 | **43** | `torch==2.4.1+cpu` (1C/1H/3M/5L = 10), `transformers==4.44.2` (6H/11M/1L = 18), `filelock==3.32.3` (1H/9M/2L = 12), `setuptools==78.1.0` (1H/1M/1L = 3). `certifi==2026.10.18` already patched in 0.8.30 Slice 10. **Hold** until coordinated CPython 3.12 exporter refresh reproducing model SHA-256 `c92689ec…` (`P1-09`). |
| `python/uv.lock` | `pip` | **NO** *(deleted in `39ee27126`)* | **Stale / orphan deleted path** (pre-0.6.0) | 3 | 4 | 9 | 4 | **20** | `anyio` (1C/1M), `certifi` (1C), `sentence-transformers` (1C), `filelock` (1H/6M/2L), `fsspec` (1H), `transformers` (2H), `idna` (1M), `setuptools` (1M/1L), `torch` (1L). File does not exist in `HEAD` or `origin/main` (archived pre-0.6.0; `/python` explicitly excluded in `.github/dependabot.yml`). **Dismiss all 20 as `not_used` / deleted path** (`P1-01`). |
| `dev/experiments/0.8.29-slice60-projected-metadata/driver/Cargo.lock` | `rust` | Yes | Historical 0.8.29 experiment (`eval-only`, never shipped) | 1 | 3 | 1 | 0 | **5** | `zip 7.2.0` (`#204`–`#208`, patched `9.0.0`/`9.0.5`; `rustls` already `0.23.46`). Never built by CI or shipped (`tiers.toml` `eval-only`). **Dismiss as `not_used` / historical experiment**; defer `dev/` cleanup to `0.8.33` (`P1-01`, `P1-11`). |
| 9× `dev/plans/0.8.27/features/slice-135/results/*/Cargo.lock` | `rust` | Yes | Immutable 0.8.27 provenance snapshots (`historical-evidence`, never shipped) | 9 | 27 | 27 | 18 | **81** | 9 identical alert sets (9 per file): `zip 7.2.0` (1C/3H/1M = 5) + `rustls 0.23.40` (2M/2L = 4). Excluded as `historical-evidence` in `scripts/sbom-survey/tiers.toml`; provenance guarded by `scripts/tests/test_slice135_cargo_receipt_snapshots.py`. **Do not edit immutable provenance**; dismiss all 81 as `not_used` / historical evidence (`P1-01`, `P1-11`). |
| **Total** | **3 ecosystems** (`rust`: 86, `pip`: 63, `npm`: 1) | — | **0 shipping / 150 non-shipping** | **14** | **43** | **61** | **32** | **150** | **106 dismissible now (stale/historical) · 43 deferred exporter · 1 tracked dev-only low** |

### 1.2 Exact enumeration of the 9 historical `0.8.27` `slice-135` result lockfiles (81 alerts)

Each file below locks `zip 7.2.0` (5 alerts: `GHSA-m665-v7pm-7grj` crit, `GHSA-q7v9-x93c-xqv3` high, `GHSA-hjqx-j4rw-g7xv` high, `GHSA-9hj8-73fh-9963` high, `GHSA-8hxf-qr8p-62x4` med) and `rustls 0.23.40` (4 alerts: `GHSA-2mjx-qc3c-rqvc` med, `GHSA-c8m5-c952-6cm4` med, `GHSA-w44r-93j7-qv28` low, `GHSA-h2v8-394m-p7vv` low):

1. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/baseline/Cargo.lock` (`#36, #96, #105, #114, #129–#133`)
2. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e01-e12-feasibility/candidate/Cargo.lock` (`#37, #95, #104, #113, #134–#138`)
3. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/baseline-build/Cargo.lock` (`#38, #97, #106, #115, #139–#143`)
4. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/shared/candidate-build/Cargo.lock` (`#39, #98, #107, #116, #144–#148`)
5. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-capability/Cargo.lock` (`#40, #100, #109, #118, #149–#153`)
6. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-current-refresh/Cargo.lock` (`#41, #102, #111, #120, #154–#158`)
7. `dev/plans/0.8.27/features/slice-135/results/2026-10-07-rust-sdk-s02-feasibility/Cargo.lock` (`#42, #103, #112, #121, #159–#163`)
8. `dev/plans/0.8.27/features/slice-135/results/2026-10-08-crash-permission-224e/Cargo.lock` (`#43, #99, #108, #117, #164–#168`)
9. `dev/plans/0.8.27/features/slice-135/results/2026-10-08-rust-sdk-capability-224e/Cargo.lock` (`#44, #101, #110, #119, #169–#173`)

### 1.3 Remote `origin/dependabot/*` branches (10 branches)

`.github/dependabot.yml` retains `open-pull-requests-limit: 0` across all 5 tracked directories (`/src/python`, `/`, `/src/ts`, root npm `/`, `github-actions /`).

| Remote branch | Status vs `HEAD` (`38a1594bf`) | Disposition |
| --- | --- | --- |
| `origin/dependabot/npm_and_yarn/multi-def6ea3883` | **Already satisfied** (`package.json` / `package-lock.json` have `markdownlint-cli2 0.24.2`, `re2js 2.9.1`, `js-yaml 5.5.0`) | **Prune (`P1-02`)** |
| `origin/dependabot/cargo/rustls-0.23.46` | **Already satisfied** (`Cargo.lock` has `rustls 0.23.46`) | **Prune (`P1-02`)** |
| `origin/dependabot/npm_and_yarn/dev/tools/mermaid/dompurify-3.4.17` | **Already superseded** (`dev/tools/mermaid/package-lock.json` has `3.4.18`) | **Prune (`P1-02`)** |
| `origin/dependabot/npm_and_yarn/dev/tools/mermaid/multi-010aad1c49` | **Already superseded** (`zod 4.6.7`, `puppeteer 25.18.1` landed) | **Prune (`P1-02`)** |
| `origin/dependabot/npm_and_yarn/multi-68e90142e5` | **Already satisfied** (`markdownlint-cli2 0.24.2`, `markdown-it 15.0.3`) | **Prune (`P1-02`)** |
| `origin/dependabot/pip/dev/tools/onnx/certifi-2026.10.18` | **Already satisfied** (`export-requirements.txt` has `certifi==2026.10.18`) | **Prune (`P1-02`)** |
| `origin/dependabot/pip/dev/tools/onnx/setuptools-83.0.0` | **Superseded** by `setuptools-84.1.0` | **Prune (`P1-02`)** |
| `origin/dependabot/pip/dev/tools/onnx/filelock-4.2.1` | Held (`export-requirements.txt` `filelock==3.32.3`; `ReadWriteLock` unused) | **Postpone (`P1-09`)** |
| `origin/dependabot/pip/dev/tools/onnx/setuptools-84.1.0` | Held (`export-requirements.txt` `setuptools==78.1.0`; Windows-only advisory on Linux exporter) | **Postpone (`P1-09`)** |
| `origin/dependabot/pip/dev/tools/onnx/transformers-5.10.1` | Held (`export-requirements.txt` `transformers==4.44.2`; major bump incompatible with `torch==2.4.1+cpu` lock) | **Postpone (`P1-09`)** |

---

## 2. Audit of every pinned version and fork source

### 2.1 Rust workspace (`Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`, `scripts/pinned-override-rot.json`)

Workspace footprint: **13 members** (12 core crates under `src/rust/crates/` + `src/inspector`), **35 unique external direct crates**, **455 `Cargo.lock` records**, Rust MSRV `1.95` (`rust-toolchain.toml` `1.95.0`).

| Crate / pin | Declared → locked / source | Exact files | Why pinned / current posture |
| --- | --- | --- | --- |
| `cudarc-fathomdb` | `=0.21.0` → `0.21.0` (`git+file:///home/coreyt/projects/fathomdb-forks/cudarc?rev=3509cd6b…`, replaced via `.cargo/config.toml` with `https://github.com/coreyt/cudarc.git` @ `3509cd6bd83b44204be1880e833474ab0f75e060`) | `Cargo.toml`, `.cargo/config.toml`, `src/rust/crates/fathomdb-embedder/Cargo.toml`, `experiments/sdk_drivers/rust/Cargo.toml`, `scripts/pinned-override-rot.json`, `scripts/cudarc-fathomdb-lock-set.json` | 0.8.30 Slice 60 qualified cohort (`seq-413`): preserves Tegra private CUDA memory pool (`ADR-0.8.28-tegra-private-cuda-pool.md`), monotonic typed CUDA loss, and single-`cudarc` invariant (`cudarc`, `ug`, `ug-cuda` verified **ABSENT** from `Cargo.lock`). Governed by `scripts/check-pinned-override-rot.py`. Retiring `[patch.crates-io]` deferred post-0.8.32 (`P1-07`, `P1-11`). |
| `candle-{core,nn,transformers,kernels}-fathomdb` | `=0.13.0` → `0.13.0` (`file:///…/candle-fathomdb` → HTTPS `https://github.com/coreyt/candle-fathomdb.git` @ `952edb7dcd1e29563acedd485a3c1ce8ef07a85c`) | `Cargo.toml`, `.cargo/config.toml`, `src/rust/crates/fathomdb-embedder/Cargo.toml`, `experiments/sdk_drivers/rust/Cargo.toml`, `scripts/pinned-override-rot.json` | 0.8.30 Slice 60 qualified cohort (`seq-413`) rebased onto upstream Candle `0.11.1`: owns static CUDART driverless linkage (`ADR-0.8.25-driverless-cuda-runtime-linkage.md`), `cudaforge 0.1.6` header invalidation, and conservative PTX default. Pulls patched `zip 9.8.1`. CUDA 13 is out of `0.8.32` scope (`P1-07`, `P1-11`). |
| `fathomdb-embedder-api` | `version = "0.8.0"` (Axis E) vs `[workspace.package] version = "0.8.31"` (Axis W) | `Cargo.toml`, `src/rust/crates/fathomdb-embedder-api/Cargo.toml` | Independent Axis E semver-stable trait surface per `dev/design/release.md` § Version axes (`P1-07`). |
| `rmcp` / `schemars` | `rmcp = "=3.5.1"` (`server`, `macros`, `transport-io`; dev `client`, `transport-async-rw`) → `3.5.1` (+ `rmcp-macros 3.5.1`); `schemars = "1"` → `1.2.3` | `src/rust/crates/fathomdb-mcp/Cargo.toml`, `src/rust/crates/fathomdb-mcp/tests/manifest.rs`, `Cargo.lock` | Exact pin enforced by `tests/manifest.rs::rmcp_is_pinned_exactly`; confines MCP protocol edge strictly to `src/edge.rs` and `src/main.rs` and preserves Draft 2020-12 output-schema derivation (`MR-071`). Pulls transitive `chrono 0.4.45` (`RUSTSEC-2026-0543` unmaintained — monitor only). Keep `=3.5.1` unless a concrete rmcp bug blocks read+write tools (`P1-04`, `P1-07`). |
| `sqlite-vec` | `=0.1.9` → `0.1.9` | `src/rust/crates/fathomdb-engine/Cargo.toml`, `src/rust/crates/fathomdb-schema/Cargo.toml` (dev), `src/inspector/Cargo.toml`, `Cargo.lock` | `ADR-0.6.0-sqlite-vec-acceptance.md`: exact pin protects `vec0` virtual-table ABI, reserved schema-38 filter slot, shadow-table layout, and zero-copy BLOB contract (`P1-07`). |
| `rusqlite` / `libsqlite3-sys` | `0.41` (`bundled`, `fallible_uint`) → `0.41.1` / `libsqlite3-sys 0.39.1` (SQLite `3.54.1`) | `src/rust/crates/{fathomdb-engine,fathomdb-schema,fathomdb-cli,fathomdb-mcp,fathomdb-sdk}/Cargo.toml`, `src/inspector/Cargo.toml`, `Cargo.lock` | `ADR-0.8.28-sqlite-per-connection-page-cache.md`: omits `SQLITE_ENABLE_MEMORY_MANAGEMENT` and bundles SQLite `3.54.1` with upstream Windows local-verbatim-path WAL fix (`P1-07`). |
| `pyo3` | `0.29` (`abi3-py310`; `pyo3/extension-module` opt-in via `src/python/pyproject.toml`) → `0.29.0` | `src/rust/crates/fathomdb-py/Cargo.toml`, `src/python/pyproject.toml`, `Cargo.lock` | Preserves Python `>=3.10` `abi3` wheels while keeping `cargo test --workspace` linkable on macOS/Windows (`P1-07`). |
| `napi` / `napi-derive` / `napi-build` | `2` (`napi8`, `async`, `tokio_rt`, `serde-json`) → `2.16.17` / `2.16.13` / `2.3.2` (`napi-sys 2.4.0`) | `src/rust/crates/fathomdb-napi/Cargo.toml`, `src/ts/package.json`, `Cargo.lock` | Coupled to `@napi-rs/cli ^2.18.4` and unscoped multi-platform package loader (`src/ts/src/platform.ts`) (`P1-07`). |
| `ort` / `ort-sys` | `=2.0.0-rc.10` (`load-dynamic`, optional `onnx-embedder`) → `2.0.0-rc.10` | `src/rust/crates/fathomdb-embedder/Cargo.toml`, `Cargo.lock` | `ADR-0.8.16-onnx-embedder-backend.md`: opt-in CPU evaluation backend (`rc.12` had `ort`/`ort-sys` VitisAI mismatch) (`P1-07`). |
| `dunce` | `=1.0.5` (`cfg(windows)`) → `1.0.5` | `src/rust/crates/fathomdb-engine/Cargo.toml`, `Cargo.lock` | Exact Windows local-verbatim-path simplification pin for SQLite WAL path handling (`P1-07`). |
| `httpmock` | `=0.8.3` (dev-only) → `0.8.3` | `src/rust/crates/fathomdb-embedder/Cargo.toml`, `Cargo.lock` | Removes `async-std` advisories without shifting product pins (`P1-07`). |
| `tokio`, `ureq`, `rustls`, `webpki-roots`, `rustix`, `zip` | `tokio 1.54.1`, `ureq 2.12.1`, `rustls 0.23.46`, `rustls-webpki 0.103.16`, `webpki-roots 0.26.11` → `1.0.12`, `ring 0.17.14`, `rustix 1.1.6`, `zip 9.8.1` | `src/rust/crates/{fathomdb-embedder,fathomdb-mcp,fathomdb-napi}/Cargo.toml`, `Cargo.lock` | All previously tracked Rust advisories closed; thin default builds carry zero TLS/network edges (`P1-07`). |
| Duplicate versions watch | `sha2` `0.10.9` (`fathomdb-mcp`, `cudaforge`) + `0.11.0`; `tokenizers` `0.20.4` + `0.23.2`; `thiserror` `1.0.69` + `2.0.18`; `getrandom` `0.2.17`/`0.3.4`/`0.4.2` | `Cargo.lock`, `src/rust/crates/{fathomdb-mcp,fathomdb-embedder}/Cargo.toml` | Aligning `fathomdb-mcp` `sha2` `0.10` → `0.11` is trivial in the MCP slice (`P1-04`). `tokenizers` convergence requires BGE/TinyBERT parity proof — defer (`P1-11`). |

### 2.2 Python manifests and tool pins (`src/python/`, `src/python-cli/`, `dev/tools/onnx/`, `tools/docs/`, `scripts/sbom-survey/`)

| Pin / manifest | Declared value | Exact files | Why pinned / current posture |
| --- | --- | --- | --- |
| `maturin` | `>=1.9,<2` (build backend & `dev`/`test` extras) | `src/python/pyproject.toml`, `src/python-cli/pyproject.toml` | Measured floor for PEP-639 `license-files = ["LICENSE"]` list-of-globs. Reuse in `src/python-mcp/pyproject.toml` (`P1-05`, `P1-08`). |
| `ruff` | `ruff==0.15.17` | `src/python/pyproject.toml`, `scripts/setup-python-owned.sh`, `scripts/tests/test_dev_environment_tools.sh` | Clean-clone lint reproducibility pin. Retain (`P1-08`). |
| `pyright` | `pyright==1.1.410` | `src/python/pyproject.toml`, `scripts/agent-typecheck.sh`, `scripts/tests/test_agent_typecheck_pyright_version.sh` | Clean-clone typecheck pin. **CI triage:** `verify-fast` failure of `test-agent-typecheck-pyright-version` is NOT a `pyright` mismatch — `agent-typecheck.sh:38-39` invokes `scripts/check-release-smoke-sdk-compile.sh`, which the test fixture omitted to copy. Retain `pyright==1.1.410` and fix fixture in Slice 10 (`P1-08`). |
| `tomli` | `tomli>=2.0; python_version < '3.11'` (`dev` extra) | `src/python/pyproject.toml` | Fallback for Ubuntu 22.04 / Jetson L4T Python 3.10 outside ≥3.11 `.venv` (`P1-08`). |
| `src/python/pyproject.toml` runtime deps | `dependencies = []`, `requires-python = ">=3.10"`, optional `cli = ["fathomdb-cli==0.8.31"]` | `src/python/pyproject.toml`, `scripts/set-version.sh` | Zero runtime dependencies on core wheel; evaluate parallel `mcp = ["fathomdb-mcp==0.8.32"]` extra (`P1-05`). |
| `src/python/uv.lock` | 20 packages (`fathomdb 0.8.20`) | `src/python/uv.lock` | Stale (11 releases behind), zero alerts; ignored by `setup-python-owned.sh --provision`. Defer reconciliation/retirement to `0.8.33` (`P1-11`). |
| `dev/tools/onnx/export-requirements.txt` | 28 SHA-256 hash-pinned CPython 3.12 x86_64-linux wheels | `dev/tools/onnx/export-requirements.txt` | Reproduces exact ONNX model digest `c92689ec…`. Never shipped. Defer refresh (`P1-09`). |

### 2.3 Root npm, TypeScript SDK, Mermaid renderer, and Node toolchain pins

| Pin / manifest | Declared → locked | Exact files | Why pinned / current posture |
| --- | --- | --- | --- |
| Root `package.json` (`private: true`) | `markdownlint-cli2: ^0.24.2` → `0.24.2`; `overrides: []` | `package.json`, `package-lock.json`, `scripts/pinned-override-rot.json` | Dev-only markdown lint tooling; `overrides` intentionally empty (`check-pinned-override-rot.py` fails closed on live npm overrides). Retain (`P1-03`, `P1-12`). |
| `src/ts/package.json` | Zero runtime deps; `@napi-rs/cli ^2.18.4`, `@types/node ^26.1.0`, `typescript ^6.0.3` | `src/ts/package.json`, `src/ts/package-lock.json` | Thin package; zero advisories (`P1-07`). |
| Node / npm workflow alignment | `.nvmrc` & `ci.yml`: Node `26.13.1` LTS; release/Tegra workflows: EOL Node `25.9.0` + Tegra `npm 11.12.1` | `.nvmrc`, `.github/workflows/*.yml`, `scripts/check-cuda-release-contract.py` | Explicit contract tests assert release/Tegra pins; postpone coordinated upgrade out of `0.8.32` (`P1-10`). |
| `dev/tools/mermaid/package.json` | `@mermaid-js/mermaid-cli: "12.2.0"`, `puppeteer: "25.18.1"` | `dev/tools/mermaid/package.{json,lock}` | Clean (`P1-08`). |

---

## 3. Packaging and dependency readiness for `0.8.32`

### 3.1 Newly published `fathomdb-mcp` (`crates.io` + lockstep PyPI wheel + 5-target GitHub Releases)

1. **`src/rust/crates/fathomdb-mcp/` crate-root artifacts & manifest:**
   - Add byte-identical `LICENSE` and `README.md`, flip `publish = true`, update description/keywords (omit `categories` per `seq-198`), run `cargo package --locked --list -p fathomdb-mcp` and `cargo publish --dry-run` (auditing `examples/`, `proptest-regressions/`, and self `dev-dependency`), retain `rmcp = "=3.5.1"`, optionally align `sha2` `0.10` → `0.11`.
   - Confirm shipped binary/wheel feature set (`--no-default-features` vs `--features default-embedder`, and whether `default-reranker` forwarding should be added).
2. **Lockstep PyPI binary wheel (`src/python-mcp/`):**
   - Mirror `src/python-cli/` (`maturin>=1.9,<2`, `bindings = "bin"`, `targets = [{ name = "fathomdb-mcp", kind = "bin" }]`, PEP-639 `license = "MIT"`, `license-files = ["LICENSE"]`, byte-identical `LICENSE`, `README.md`); optionally expose `mcp = ["fathomdb-mcp==0.8.32"]` in `src/python/pyproject.toml`.
3. **Release scripts, gates, workflows, and external publishers:**
   - Wire `scripts/set-version.sh`, `scripts/check-license-consistency.sh`, `scripts/check_python_wheel_contents.py --kind mcp`, `scripts/verify-release-gates.sh`, `scripts/release/cargo-publish-if-new.sh`, `scripts/release/wait-for-registry-version.py`, `scripts/sbom-survey/tiers.toml` (also adding `src/python-cli/`), `.github/workflows/release.yml` (5-target GitHub Release archives + `build-python-mcp` + terminal-tier `cargo publish` + PyPI OIDC + post-publish consumer smokes).
   - Pre-configure PyPI Pending Publisher (`fathomdb-mcp`) and crates.io bootstrap / Trusted Publisher during Slice 10/MCP slice (never at tag time).

### 3.2 Coverage-guided fuzzing dependency isolation (`cargo-fuzz` / `libfuzzer-sys` / `arbitrary`)

- Create standalone `fuzz/Cargo.toml` (`[workspace]`, `publish = false`) and add `exclude = ["fuzz"]` to root `Cargo.toml`.
- Keep `rust-toolchain.toml` at `1.95.0`; invoke fuzzing via `cargo +nightly-2026-08-01 fuzz ...` after installing `cargo install --locked cargo-fuzz`.
- Keep minimized regression reproducers as ordinary stable-Rust `#[test]` cases with zero `libfuzzer-sys`/`arbitrary` linkage, and assert zero fuzz deps in shipping crates via `cargo tree --locked`.
- Classify `fuzz/` in `scripts/sbom-survey/tiers.toml`.

---

## 4. Proposal register

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation (include / postpone / reject) | Target Slice / Rationale |
| --- | --- | :---: | :---: | :---: | :---: | --- |
| **P1-01** | Dismiss the `106` non-shipping, non-actionable GitHub Dependabot alerts via GitHub Security API/UI (`20` orphan alerts on deleted pre-0.6.0 `python/uv.lock`, `81` on 9× immutable `0.8.27` `slice-135/results/*/Cargo.lock`, and `5` on historical `0.8.29-slice60-projected-metadata/driver/Cargo.lock`) as `not_used` / historical provenance without modifying audited snapshots. | H | L | S | **include** | **Slice 10** — clears `70.7%` of alert noise immediately while keeping `test_slice135_cargo_receipt_snapshots.py` and immutable 0.8.27 provenance intact. |
| **P1-02** | Prune the `7` already-satisfied or superseded `origin/dependabot/*` remote branches (`multi-def6ea3883`, `rustls-0.23.46`, `dompurify-3.4.17`, `multi-010aad1c49`, `multi-68e90142e5`, `certifi-2026.10.18`, `setuptools-83.0.0`). | H | L | S | **include** | **Slice 10** — leaves only the 3 intentionally held ONNX exporter branches visible on `origin`. |
| **P1-03** | Keep root `package.json` `overrides` empty and track `katex 0.16.47` (alert `#77`, low, dev-only `GHSA-238p-pmpm-9mq7`) as accepted dev-tooling risk until upstream `micromark-extension-math` widens past `^0.16.0`. | H | L | S | **include** | **Slice 10** — preserves `scripts/check-pinned-override-rot.py` zero-override posture and `node_modules/js-yaml` / AST-guard invariants. |
| **P1-04** | Make `fathomdb-mcp` crates.io-ready: add byte-identical `LICENSE` and `README.md`, flip `publish = true`, update description/keywords, audit `examples/` & self-`dev-dependency` via `cargo package --list` and `--dry-run`, retain `rmcp = "=3.5.1"` confinement (`tests/manifest.rs`), optionally align `sha2 = "0.10"` → `"0.11"`, and wire `cargo-publish-if-new.sh`, `verify-release-gates.sh`, `wait-for-registry-version.py`, and `.github/workflows/release.yml` (terminal tier after `fathomdb-sdk`). | H | M | M | **include** | **MCP slice (`R32-02`)** — directly required by owner `HANDOFF-06` Option 1 ruling (`crates.io` publication). |
| **P1-05** | Create lockstep `src/python-mcp/` maturin binary wheel package (mirroring `src/python-cli/`), wire `set-version.sh`, `check-license-consistency.sh`, `check_python_wheel_contents.py --kind mcp`, `scripts/sbom-survey/tiers.toml` (also adding `src/python-cli/`), 5-target GitHub Releases stdio binaries, PyPI `fathomdb-mcp` OIDC job + installed consumer smokes, and pre-configure PyPI Pending Publisher & crates.io Trusted Publisher ahead of tag time. | H | M | M | **include** | **Slice 10 (publisher setup) / MCP slice (packaging & workflows)** — directly required by `R32-02` and `HANDOFF-06`; avoids tag-time publisher blockers. |
| **P1-06** | Isolate fuzzing dependencies (`cargo-fuzz`, `libfuzzer-sys`, `arbitrary`) inside a standalone `fuzz/` workspace (`[workspace]` in `fuzz/Cargo.toml`, `exclude = ["fuzz"]` in root `Cargo.toml`, `tiers.toml` classification), invoke with installed `nightly-2026-08-01` while keeping `rust-toolchain.toml` at `1.95.0`, and gate shipping crates with a zero-fuzz-dep `cargo tree --locked` check. | H | L | S | **include** | **Durability slice (`R32-05`)** — satisfies `plan.md` zero-shipping-bloat footprint posture. |
| **P1-07** | Retain all qualified Rust runtime, binding, and fork pins unchanged in `0.8.32`: `cudarc-fathomdb =0.21.0`, `candle-{core,nn,transformers,kernels}-fathomdb =0.13.0`, `fathomdb-embedder-api 0.8.0`, `rusqlite 0.41.1` / `libsqlite3-sys 0.39.1` (SQLite `3.54.1`), `sqlite-vec =0.1.9`, `pyo3 0.29.0`, `napi 2.16.17` / `@napi-rs/cli 2.18.4`, `rmcp =3.5.1`, `ort =2.0.0-rc.10`, `dunce =1.0.5`, `httpmock =0.8.3`, `tokio 1.54.1`, `rustls 0.23.46`, `rustix 1.1.6`, `zip 9.8.1`. | H | L | S | **include** | **All slices** — zero open advisories on shipping crates; preserves Schema-38 `vec0` ABI, per-connection page cache, and Slice 60 CUDA qualification. |
| **P1-08** | Retain reviewed developer quality-gate pins (`ruff==0.15.17`, `pyright==1.1.410`, `maturin>=1.9,<2`, `mkdocs==1.6.1`, `markdownlint-cli2 ^0.24.2`, Mermaid `12.2.0`/`puppeteer 25.18.1`, `.github/dependabot.yml` routine-PR pause) and fix the `test_agent_typecheck_pyright_version.sh` CI fixture omission (`check-release-smoke-sdk-compile.sh` copy) without changing the `pyright` pin. | H | L | S | **include** | **Slice 10** — restores green `verify-fast` CI while keeping deterministic clean-clone tool pins. |
| **P1-09** | Refresh `dev/tools/onnx/export-requirements.txt` (`43` alerts across `torch==2.4.1+cpu`, `transformers==4.44.2`, `filelock==3.32.3`, `setuptools==78.1.0`) with full CPython 3.12 hash-locked wheel re-export verifying ONNX model SHA-256 `c92689ec…`. | M | M | M | **postpone** | **Post-`0.8.32` (embedder/exporter refresh)** — offline non-shipping tool; major `torch`/`transformers` jumps risk model-digest drift. |
| **P1-10** | Retire EOL Node `25.9.0` and Tegra `npm 11.12.1` from release/Tegra workflows and reconcile `packageManager: npm@11.20.0` vs npm `12.x`. | M | M | M | **postpone** | **`0.8.33` / dedicated release-tooling slice** — requires coordinated 5-target + Jetson Orin + npm OIDC qualification. |
| **P1-11** | Retire root `[patch.crates-io]` fork patches, reconcile/retire stale `src/python/uv.lock` (`fathomdb 0.8.20`), deduplicate `tokenizers 0.20.4`/`0.23.2` and `thiserror 1.0.69`/`2.0.18` (monitoring `chrono 0.4.45`), and declutter historical `dev/` lockfiles. | M | M | M | **postpone** | **`0.8.33` (`dev/` declutter & hygiene)** — explicitly out of `0.8.32` scope. |
| **P1-12** | Inject an npm `overrides` pin forcing `katex@0.18.2` into root `package.json`, or add `libfuzzer-sys`/`arbitrary` to root workspace dependencies. | H | H | S | **reject** | **N/A (policy guard)** — npm override trips `check-pinned-override-rot.py` R2 governance; root fuzzing deps violate zero-shipping-bloat invariant. |

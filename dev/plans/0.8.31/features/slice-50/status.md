---
title: Slice 50 Status — SDK Ergonomics, OpenReport Capability Booleans, 37-to-38 Migrate Root, and CLI PyPI Wheel
status: CLOSED
target_release: 0.8.31
slice: 50
date: 2026-11-23
---

# Slice 50 Status — `0.8.31` SDK, CLI & Upgrade (`SDK-CLI-UPGRADE`)

- **Branch / Worktree:** `release/0.8.31` in `/mnt/data/worktrees/release-0.8.31`
- **Plan:** [`dev/plans/0.8.31/features/slice-50/plan.md`](./plan.md)
- **Commits:** `09dbec3bd` (Layer 1 — Rust core), `1b07584e1` (Layer 2 — `fathomdb migrate` CLI, `src/python-cli/` wheel, `build-python-cli` workflow & release script lockstep), `9780e9181` (Layer 3 — Python/TS bindings, governed surface `56/93` parity, docs & module-boundary hygiene), `fbb6591fc` (`governed-surface-pin.json` commit SHA pin), `28f5def55` (`FIX-1` — Py/PyO3 lone-surrogate AC-068b parity, Step-7 error propagation, `cli.md` JSON schema, and response validator tightening)
- **Verdict:** **CLOSED / LANDED** — all 3 implementation layers + `FIX-1` verified green across Rust, Python, TypeScript, CLI wheel smoke, release workflow contracts, and Markdown/doc gates; independent adversarial code review (`59395abd`) returned **`APPROVE`** at HEAD `28f5def55`.

## 1. Delivered Scope

1. **Layer 1 (`09dbec3bd`) — Rust Core (`fathomdb-embedder`, `fathomdb-engine`, `fathomdb`, `fathomdb-sdk`):**
   - `fathomdb-embedder::reranker_locally_ready()` inspecting `module_models::RERANKER` + local SHA-256 weight cache with zero network I/O, zero disk writes, and zero CUDA initialization.
   - `OpenReport.{reranker_compiled, cuda_compiled, rerank_available}` preserving fail-closed forced-CUDA open (`ADR-0.8.23` `#43`).
   - `resolve_evidence_many` (`EVIDENCE_RESOLVE_MANY_MAX = 50`, `EvidenceResolveManyRequestV1`, `EvidenceResolveManyItemV1::{Resolved, Refused}` per accepted ADR `#76`, deduplicating inside one authenticated snapshot with pre- and post-hook `validate_snapshot`, preserving duplicate arity/order and per-slot `/evidenceRefs/{i}` nondisclosure).
   - Opt-in `SearchResult.hints: Option<Vec<Hint>>` (`search`, `search_frozen`, `EvidenceSearchRequestV1::with_hints`), zero allocation when `hints == false`, preserving `size_of::<ReaderRequest>() <= 128`.
   - `operator::migrate_37_to_38` 7-step fail-closed upgrade (exclusive `.lock`, strict Schema-37 admission, 4 unfinished-erasure gates, fail-closed `dangling_edge_endpoints > 0` admitting `superseded_edge_endpoints > 0`, WAL checkpoint truncate + SHA-256-verified backup, single `BEGIN IMMEDIATE` `execute_batch(STEP_38_SQL)` + active-only `cpv` backfill + atomic Schema-38 `declaration_sha256` generation rehash + `PRAGMA user_version = 38`, post-upgrade full integrity + Schema-38 admission), re-exported behind `#[cfg(feature = "operator")]` with `compile_fail` absence proof in `fathomdb`.
2. **Layer 2 (`1b07584e1`) — `fathomdb-cli`, `src/python-cli/` Wheel, `release.yml`, & Release Scripts:**
   - Added `Command::Migrate(MigrateArgs)` alongside `Recover` and `Doctor` (all 17 `DoctorCommand` variants preserved), exit codes `0`/`65`/`70`/`71`, and real copied golden Schema-37 DB upgrade + `doctor check-integrity --full` (`exit 0`) integration tests.
   - Added `src/python-cli/` (`maturin>=1.9,<2`, `version = "0.8.30"` live Axis W, `bindings = "bin"`, `targets = [{ name = "fathomdb", kind = "bin" }]`, PEP-639 `license = "MIT"`, byte-identical `LICENSE`) and `cli = ["fathomdb-cli==0.8.30"]` extra on `src/python/pyproject.toml` with zero `[project.scripts]` (`AC-041`), verified in an isolated no-`cargo` venv smoke test.
   - Added 5-target `build-python-cli` job in `.github/workflows/release.yml` (`actionlint` clean), wired into `all-builds-passed.{needs,if}`, `publish-pypi`, and `github-release`, and updated `set-version.sh`, `check-license-consistency.sh`, `wait-for-registry-version.py`, `smoke-pypi-wheel.{sh,ps1}`, `check-release-contract-truth.py`, `check_python_wheel_contents.py`, and all 11 `scripts/tests/*` suites.
3. **Layer 3 (`9780e9181`, `fbb6591fc`) & `FIX-1` (`28f5def55`) — Python/TS Bindings, Governed Surface `56/93`, Docs & Hygiene:**
   - Wired Python (`fathomdb-py` + `src/python/`, EARP `CATALOG`, `python_surface_additions.json`, `test_slice130_package_boundary.py`, `test_slice50_sdk_ergonomics.py`) and TypeScript (`fathomdb-napi` + `src/ts/`, `slice50-sdk-ergonomics.test.ts`), including lone-surrogate `WriteValidationError` parity (`AC-068b`) and strict per-slot response validation.
   - Updated `governed-operation-parity.json`, `governed-surface-allowlist.json`, `scripts/governed-surface-pin.json` (pinned at `9780e9181`, `56` live ops / `93` allowlist / `5` core / `5` recovery_denylist / `2` runtime_controls; `test_check_governed_surface_pin.sh` 98/98 passed), and regenerated `docs/generated/sdk-operation-map.md`.
   - Updated targeted public & interface docs (`README.md`, `src/ts/README.md`, `dev/interfaces/*`, `docs/*`) and resolved Slice 20–40 module-boundary policy drift (`scripts/check-module-boundaries.sh` clean).

## 2. Verification Summary

- **Rust:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (0 warnings), `cargo clippy -p fathomdb-engine -p fathomdb -p fathomdb-sdk --features default-reranker --all-targets -- -D warnings` (0 warnings), `scripts/check-module-boundaries.sh` (PASS), `slice50_migrate_37_to_38` (8/8 incl. golden Schema-37 & rollback), `migrate_cli` (5/5), `slice50_resolve_evidence_many` (5/5), `slice50_search_hints` (2/2), `slice50_open_report_capabilities` (2/2 under `default-reranker`), `slice71_rerank_policy` (4/4), `slice71_open_report` (1/1), `fathomdb-cli` full suite, `fathomdb-sdk` + `fathomdb` (incl. `compile_fail` doc-test).
- **Python:** Owned wheel rebuilt via `scripts/setup-python-owned.sh --build`; `ruff` clean, `pyright` (0/0), full `pytest` (**1,800 passed**, 44 skipped, 0 failed) + `test_slice50_sdk_ergonomics.py` (4/4 incl. FIX-1 lone-surrogate tests).
- **TypeScript:** `npm run build`, `npm run typecheck`, full `npm test` (**662/662 passed**, 0 failed).
- **CLI Wheel, Release & Docs:** Real `maturin build` + `check_python_wheel_contents.py --kind cli` + `check-license-consistency.sh --only python --with-wheel` + isolated no-`cargo` venv smoke test + `actionlint` + `check-sdk-surface-parity.py` + `check-governed-surface-pin.sh` (98/98) + all 11 release script suites + `agent-lint-md.sh` + `agent-lint-docs.sh` + `check-public-doc-truth.py` + `check-design-refs.sh` — **ALL PASS**; `dev/acceptance.md` untouched; zero external consumer-project leaks.
- **Adversarial Code Review:** Independent `slice-reviewer` (`59395abd`) returned **`APPROVE`** at HEAD `28f5def55` (0 open P0/P1/P2/P3 findings).

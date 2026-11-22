---
title: Inspector 0.2.1 Slice 10 status
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: COMPLETE
desc: Base integration of Inspector onto FathomDB 0.8.30 with lockfile, CI, docs, and build hygiene verified
---

# Slice 10 status

**COMPLETE.** Dedicated release branch `inspector/dev/0.2.1` and worktree
`/mnt/data/worktrees/inspector-0.2.1` are cut from `release/0.8.30`
(`72fe93634321616214352a7e07066ce0f93bcf4c`, whose `src/rust/`, `Cargo.toml`,
`Cargo.lock`, and `.cargo/config.toml` are byte-identical to peeled `v0.8.30`
`e3d31f5cdc0df3e32dab9bad151c5ab5c321222c`). Inspector trees (`src/inspector/`,
`dev/inspector/`, `docs/inspector/`, and `dev/design/db-inspector/` redirects)
were carried cleanly from `inspector/dev/0.2.0` (`71900f605`), and required
workspace/CI/docs wiring was selectively re-applied without reverting any
0.8.30 changes.

## Integration hygiene and boundaries

- `git diff release/0.8.30 -- src/rust/ .cargo/config.toml`: **empty** (zero
  Engine, SDK, MCP, schema, embedder, or Cargo source-config modifications).
- `Cargo.lock`: **+17 / -0 lines** — strictly adds the `fathomdb-inspector`
  workspace package entry with zero drift in any existing crate resolution.
- Selective non-Inspector wiring updated: `Cargo.toml`, `.github/workflows/ci.yml`,
  `scripts/test-feature-matrix.toml`, `scripts/tests/test_ci_proportional_routing.py`,
  `scripts/tests/test_generated_public_docs.py`, `mkdocs.yml`, `docs/index.md`,
  `docs/generated/contributor-map.md` (regenerated via
  `scripts/generate-public-docs.py --write` targeting `release/0.8.30`),
  `dev/DOC-INDEX.md`, `dev/README.md`, `dev/ROADMAP.md`, and
  `dev/design/document-lifecycle.json`.
- Checkout-owned environments provisioned: `node_modules` installed offline
  via `npm ci --offline` (non-symlink, passes `setup-python-owned.sh --check-ownership`)
  and checkout-owned `.venv` provisioned via `scripts/setup-python-owned.sh --tools-only`
  (Python 3.13, `ruff 0.15.17`, `pyright 1.1.410`, `pytest 9.2.0`,
  `hypothesis 6.170.17`, `maturin 1.16.0`).

## Verification results

| Check | Result | Notes |
| --- | --- | --- |
| `scripts/check-cudarc-fathomdb-lock-set.py --root . --record scripts/cudarc-fathomdb-lock-set.json` | PASS | 22-package Linux-normal closure matches `66f7fd5675af` |
| `scripts/check-pinned-override-rot.py --root .` | PASS | Governed npm overrides, K9 kernels, and exact fork sources verified |
| `scripts/tests/test_ci_proportional_routing.py` | PASS | All Inspector and 0.8.30 routing scenarios pass (including `-lite`) |
| `scripts/tests/test_generated_public_docs.py` | PASS | 8/8 tests pass |
| `scripts/tests/test_public_doc_truth.sh` | PASS | All public-doc-truth checks pass |
| `cargo check` / `cargo clippy --all-targets -- -D warnings` (bare) | PASS | Zero errors/warnings; `cached_default_cpu: false`, `fixture_control: false`, schema `37` |
| `cargo check` / `cargo clippy --all-targets -- -D warnings` (`--features cached-cpu`) | PASS | Zero errors/warnings against Candle `0.13.0`, `webpki-roots 1.0.12`, `rustls-webpki 103.16`; `cached_default_cpu: true`, `fixture_control: false`, schema `37` |
| `cargo test -p fathomdb-inspector --no-default-features` (bin + `cli` + `launch` + `worker`) | Observed baseline (`74` passed, `1` expected RED) | `cli` 1/1, `launch` 2/2, `worker` 11/11, bin 61/62 PASS (including Landlock/seccomp confinement); sole failure is `worker::lexical_tests::native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results` asserting pre-0.8.29 body-dedup (`left: 3, right: 2`), handed off directly to Slice 20 |
| `scripts/agent-lint-md.sh` & `scripts/agent-lint-docs.sh` | PASS | Run after staging tracked files (including `dev/design/db-inspector/` redirects) |

## Handoff to Slice 20

Slice 20 starts directly on clean `inspector/dev/0.2.1` HEAD in
`/mnt/data/worktrees/inspector-0.2.1` with the observed RED baseline in
`src/inspector/tests/integration/lexical.rs` and proceeds to reconcile
record-keyed retrieval ownership, stale equal-body copy (`lexical.rs` /
`app.js`), degradations/`Closing`/FTS/ownerless tests, 0.8.30 provenance
semantics, and Candle 0.13 `cached-cpu` confinement.

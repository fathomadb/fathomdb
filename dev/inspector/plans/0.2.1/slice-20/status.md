---
title: Inspector 0.2.1 Slice 20 status
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
status: COMPLETE
desc: 0.8.30 retrieval ownership, stale copy updates, degradations/FTS/provenance/Candle 0.13 tests, and browser/confinement verification
---

# Slice 20 status

**COMPLETE.** Reconciled Inspector production metadata, UI copy, test suites,
and `cached-cpu` fixture tooling with delivered FathomDB `0.8.30` record-keyed
retrieval, degradations, `Closing` propagation, FTS divergence refusal,
partial UTF-8 provenance semantics, deferred vector-span open validation, and
Candle `0.13.0` CPU serving/confinement.

## Changes landed

- **Production code (2 files, minimal):**
  - `src/inspector/src/lexical.rs` (`Reader::lexical`): updated
    `origin.all_equal_body_contributors` from `"unknown"` to
    `"record_keyed_not_merged"` for body and property routes while preserving
    `"not_inferred"` on the BM25F route.
  - `src/inspector/assets/app.js` (`renderLexical`): updated hit-origin label
    from `Equal-body contributors: ...` to `Record identity: ...`.
- **Fixture & matrix hygiene (2 files):**
  - `src/inspector/tools/vector-fixture.py`: added Linux x86_64
    `SYS_memfd_create` (`319`) and `MFD_ALLOW_SEALING` / `F_ADD_SEALS` /
    `F_SEAL_*` fallback when running under standalone Python 3.13 builds
    (e.g., `uv` python-build-standalone compiled against an older glibc
    sysroot) while keeping `os.memfd_create` primary and preserving seal +
    SHA-256 pin verification.
  - `scripts/test-feature-matrix.toml`: added `[[extra]]` entry for
    `fathomdb-inspector` with `["cached-cpu"]` matching the new feature-gated
    Candle `0.13` test.
- **New & updated tests (5 files):**
  - `src/inspector/tests/integration/lexical.rs`: updated
    `native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results`
    from the Slice 10 RED baseline to expect all 3 record-keyed hits
    (`lexical-b`, `lexical-twin`, `lexical-a`) and `"record_keyed_not_merged"`;
    added
    `record_keyed_fusion_preserves_equal_body_node_edge_pairs_repeated_public_ids_and_retained_revisions`
    (5 distinct equal-body records) and
    `diverged_fts_body_or_kind_refuses_and_ownerless_node_fts_is_withheld_truthfully`
    (ownerless node FTS withheld truthfully; diverged node/edge `body` and
    node/edge `kind` all refuse with `Err("storage")`).
  - `src/inspector/tests/integration/vector.rs`: added
    `serving_reports_embedder_absent_degradation_rejects_ownerless_fts_and_propagates_closing`
    (`embedder_absent` degradation passthrough, ownerless FTS refusal on
    `serving`, `EngineError::Closing` on `search_frozen`, and exact
    `Err("storage")` on post-close `Reader::serving`) and feature-gated
    `cached_cpu_candle013_serving_preserves_equal_body_records_reports_timeout_and_validates_open_spans`
    (real Candle `0.13` CPU dense hits across equal-body nodes and edge,
    model-free fallback, real 1 ms `embedder_timeout` degradation routed
    through `Reader::serving`, and corrupted `_fathomdb_vector_spans` startup
    refusal with `Phase::Startup` / `"startup_failed_effects_possible"`).
  - `src/inspector/tests/integration/graph.rs`: added
    `provenance_partial_utf8_spans_and_negative_boundaries_distinguish_raw_links_from_native_eligibility`
    covering whole-body + partial `utf8_bytes` spans (whole-source SHA-256),
    bidirectional `recorded_source` / `recorded_derived` and
    `registered_dependency_to_source` / `registered_dependency_to_dependents`
    metadata, empty-span `NotRegistered`, and bad-digest / mid-UTF-8 refusal
    on `graph_serving` + `NotRegistered` on `serving` while keeping raw links
    visible in `investigate`.
  - `src/inspector/tests/unit/comparison.mjs`: added record-keyed equal-body
    node/edge and repeated-logical-ID membership/correspondence test.
  - `src/inspector/tests/browser/lexical.mjs`: asserted
    `Record identity: record_keyed_not_merged.` and absence of
    `Equal-body contributors: unknown`.

## Boundaries & review

- `git diff release/0.8.30 -- src/rust/ .cargo/config.toml`: **empty** (zero
  Engine, SDK, MCP, schema, embedder, or Cargo source-config edits).
- **Adversarial code review:** first pass returned `REQUEST_CHANGES` (1 P1 +
  5 P2 findings covering `Reader::serving` timeout routing, `XDG_CACHE_HOME`
  resolution, stronger `Closing`/startup/edge oracles, FTS `kind` divergence
  tests, reverse `registered_dependency_to_dependents` assertions, and
  `test-feature-matrix.toml`); all 6 findings plus `vector-fixture.py` were
  addressed and re-reviewed to **APPROVE**.

## Verification results

| Check | Result | Notes |
| --- | --- | --- |
| `cargo clippy --offline --locked -p fathomdb-inspector --all-targets --no-default-features -- -D warnings` | PASS | Zero warnings |
| `cargo clippy --offline --locked -p fathomdb-inspector --all-targets --no-default-features --features cached-cpu -- -D warnings` | PASS | Zero warnings against Candle `0.13.0` |
| `cargo fmt --all -- --check` & `ruff check` / `ruff format --check` | PASS | Clean formatting |
| `cargo test --offline --locked -p fathomdb-inspector --no-default-features` | PASS | 80 passed (66 bin + 1 cli + 2 launch + 11 worker), 0 failed |
| `cargo test --offline --locked -p fathomdb-inspector --no-default-features --features cached-cpu` | PASS | 81 passed (67 bin + 1 cli + 2 launch + 11 worker), 0 failed |
| Ignored `cached-cpu` worker confinement tests (`tests/worker.rs`) | PASS | All 5 real parent tests passed (fail-closed assets, symlink/relative admission, dense query without writes, cache-alias export exclusion, confined retained export) |
| Ignored `cached-cpu` vector numeric & disclosure tests (`worker::vector_tests`) | PASS | 2/2 passed (independent `f64` oracle & all disclosure combinations) |
| `node --test src/inspector/tests/unit/comparison.mjs src/inspector/tests/unit/ux.mjs` | PASS | 11/11 passed |
| Playwright browser suites (`lexical`, `comparison`, `graph`, `vector`, `workspace`, `ux`) | PASS | 52/52 passed against `cached-cpu` build |
| `python -m unittest discover -s src/inspector/tests/tools -p 'test_*.py'` | PASS | 12/12 passed |
| `scripts/check-test-target-coverage.py` & `scripts/tests/test_test_targets.py` | PASS | Matrix `[[extra]]` verified |
| `scripts/agent-lint-md.sh` & `scripts/agent-lint-docs.sh` | PASS | Markdown & docs gates green |

## Handoff to Slice 30

Slice 30 starts on clean `inspector/dev/0.2.1` HEAD in
`/mnt/data/worktrees/inspector-0.2.1` to remove Docker/Ubuntu 22.04 and
`openbox`/`xdotool`, migrate packaging/qualification natively to Ubuntu 24.04
(`glibc 2.39`), bump Inspector release identity to `0.2.1`, generate fresh
dual published FathomDB `0.8.29` and `0.8.30` fixtures, run full
`scripts/agent-verify.sh` + native `local-ci.sh`, and atomically promote the
verified candidate to `dist/inspector/0.2.1/`.

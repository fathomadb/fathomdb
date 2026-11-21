---
title: Inspector compatibility assessment for FathomDB 0.8.30
date: 2026-11-21
status: proposed
desc: Source-grounded upgrade scope from the released Inspector composition to Engine 0.8.30
---

# Inspector compatibility with FathomDB 0.8.30

A bounded Engine integration and qualification effort should suffice. No schema
migration, new storage adapter, UI redesign, or additional Engine feature is
identified, though targeted updates are required to remove stale
body-deduplication assertions and UI copy. The existing binary is not qualified
for 0.8.30: admitting the same file format would not establish faithful 0.8.30
retrieval or provenance behavior.

## Compared sources and evidence

| Subject | Exact source |
| --- | --- |
| Released Inspector 0.2.0 | `32593f524165eff613881a8e9b91da917665e503` (packaging source `7449ee0108d503cbed3b6a33e2c841072d6de0ce`) |
| Its compiled Engine, selected from development 0.8.29 | `6edc1155f039f87cd0a37660cb27ae4373bbce71`; packages nominally 0.8.28 |
| Published FathomDB 0.8.29 | Peeled `v0.8.29`: `b64d32f80bea2853bf4939b6334e32501174ffb1` |
| FathomDB 0.8.30 release source | Peeled `v0.8.30`: `e3d31f5cdc0df3e32dab9bad151c5ab5c321222c` |
| Subsequent 0.8.30 publication documentation | `release/0.8.30` at `72fe93634321616214352a7e07066ce0f93bcf4c` |

The last two refs have no differences under `src/rust/`, `Cargo.toml`,
`Cargo.lock`, or `.cargo/config.toml`; their 33-file delta covers post-tag
publication docs, release state, workflows, and `scripts/` (including
`scripts/agent-test.sh` and `scripts/check-public-doc-truth.py`). Selecting one
explicit integration base before execution is required so `local-release.py`'s
`engine_commit` and `scripts/agent-verify.sh` stay aligned; `release/0.8.30`
(`72fe93634`) is recommended because it retains identical Rust/Cargo sources
with current verification scripts and publication metadata. The older branch
named `dev/0.8.30` (`7d3637657`) is not the delivered 0.8.30 source. Publication
is recorded in the later release branch; this assessment did not independently
query registries.

Git object comparisons establish that `fathomdb-schema/src`,
`fathomdb-query/src`, and `fathomdb-embedder/src/loader.rs` are byte-identical
across the compiled baseline and both release tags. Schema remains 37, and
`rust-version` remains 1.95. Both releases retain `rusqlite` 0.41.1,
`libsqlite3-sys` 0.39.1, and `sqlite-vec` 0.1.9. The model asset pins, cache
recipe, and BGE identity remain unchanged. Inspector-used public opening,
search, frozen-read, and graph signatures show no required API rewrite
(`SnippetV1` and `DegradationReason` are `#[non_exhaustive]`).

This is a source review, including independent read-only review passes. No
0.8.30 build, new producer fixture, or runtime compatibility test was run.
Existing 0.2.0 evidence remains evidence for its original composition only.

## Required work

| Work | Why and affected area | Completion evidence |
| --- | --- | --- |
| Select and integrate exact 0.8.30 source | Inspector embeds Engine through path dependencies; it does not connect to a separately upgraded Engine service. Re-base `src/inspector/`, `dev/inspector/`, and `docs/inspector/` onto the chosen 0.8.30 base, selectively re-wiring the 23 non-Inspector workspace/CI/docs files (e.g., `Cargo.toml`, `.github/workflows/ci.yml`, `scripts/test-feature-matrix.toml`, `mkdocs.yml`, doc index/lifecycle, CI routing tests) without reverting 0.8.30 changes. Preserve `.cargo/config.toml` HTTPS fork mirrors and `Cargo.lock` pins so `scripts/check-cudarc-fathomdb-lock-set.py` and `scripts/check-pinned-override-rot.py` stay green. Pre-populate `$CARGO_HOME` with the 0.8.30 git checkouts and crate cohort before offline Docker builds (`local-ci.sh` mounts `/cargo:ro` with `--network none --locked --offline`). | Model-free and explicit `cached-cpu` Inspector builds compile with the updated lockfile (only adding `fathomdb-inspector`); production excludes `fixture-control`. Offline Ubuntu 22.04 Docker packaging succeeds with `.cargo/config.toml` intact. No unrelated Engine changes or pin reversions. |
| Qualify retrieval ownership, degradations, and explanations | Published 0.8.29 replaced body-keyed fusion with typed record/version ownership (`FusionOwnership` / `RecordKey`), added `DegradationReason::EmbedderAbsent` and `EmbedderTimeout`, changed `DispatchError::Closing` to return `EngineError::Closing` instead of silent text fallback, tightened FTS-to-canonical `body`/`kind` consistency in `filter.rs` / `search.rs`, and rejected `OwnerlessNodeFts` in `structural_lifecycle_state` / `evidence.rs`. Review `fusion.rs`, `search.rs`, `search_types.rs`, `embedding.rs`, `snippet.rs`, and `evidence.rs` against Inspector `lexical.rs`, `vector.rs`, `assets/app.js`, and `assets/comparison.mjs`. Update the existing body-dedup test (`native_body_and_property_queries_keep_score_families_real_snippets_and_exact_results` in `tests/integration/lexical.rs`, which asserts 2 hits for equal-body `lexical-b`/`lexical-twin`) and remove or update the stale `"all_equal_body_contributors": "unknown"` response field in `lexical.rs` and rendered copy in `assets/app.js`. | Distinct equal-body records, equal-body node/edge pairs, repeated public IDs, and retained revisions preserve native multiplicity, ranks, representative evidence, and exact origin choices. Lexical body-route responses and UI copy no longer misreport body-dedup contributors. `embedder_absent`/`embedder_timeout` degradations, `Closing` propagation, diverged-FTS refusal, and ownerless-FTS handling behave truthfully. A/B membership and canonical correspondence remain separate. |
| Qualify provenance semantics | 0.8.30 `dependency_trace.rs::source_link_valid` validates a partial `utf8_bytes` locator separately while authenticating the whole canonical source revision, and continues to require `start < end` (refusing empty ranges even though write/registration can store them). The old compiled Engine can reject a valid excerpt relationship. Native structural dependency evidence calls this validation through `structural_state.rs::structural_dependency_state` and `dependency_trace.rs::registered_dependency_for_cursor`; Inspector shows that evidence alongside its own recorded links. | Real whole-source and partial-span derivations retain both recorded-link directions and correct native structural dependency evidence. Bad digests, invalid or mid-character UTF-8 boundaries, and empty (`start == end`) ranges do not become registered-state proof. Inspector does not directly call the dependency-trace API; no new trace integration is required. Raw links remain distinct from native eligibility. |
| Requalify admission and cached CPU | Relative to `6edc1155f`, published 0.8.29 keeps the same open-time `vector_spans` SQL row-validation predicate inside `vector_spans::capture`, wraps span capture plus `StoredHeadCoverageCounts` accumulation and `frozen_read::validate_on_open` into one deferred transaction, and validates the retained open seed (`retain_matching_open_seed`) against `_fathomdb_read_visibility_state` and the default profile. (Stricter per-hit `vector_spans::scored_extent` validation applies only to snippet-enabled vector hits.) The model stack advances from Candle 0.11 / cudarc 0.19.10 to Candle 0.13.0 / cudarc 0.21.0, with updated fork pins/configuration, Tokio 1.54.1, `webpki-roots` 1.0.12, and `rustls-webpki` 0.103.16. | Fresh 0.8.29 and 0.8.30 producer fixtures admit through the real owned Engine. Actual cache-only CPU semantic/hybrid retrieval passes existing equivalence, confinement (Landlock, seccomp, `memfd` seals, SQLite dense-write authorizer), and readiness rules with sealed assets, zero network/cache mutation, bounded cancellation, and clean close. Invalid spans, profiles, or layouts refuse safely. |
| Update release identity and deliver a new candidate | Release and Engine identity are hardcoded across `src/inspector/Cargo.toml`, `src/inspector/tools/package.sh`, `src/inspector/tools/local-ci.sh`, `src/inspector/tools/local-release.py`, `src/inspector/package/README.md`, `src/inspector/tests/browser/package.mjs`, `src/inspector/tests/tools/test_local_release.py`, `src/inspector/tests/tools/test_package.py`, `src/inspector/README.md`, `src/inspector/tools/README.md`, `dev/inspector/`, and `docs/inspector/`. Existing 0.2.0 release artifacts must retain their original meaning. | Bump Inspector to 0.2.1, update Engine source identity, artifact names, matching tooling/package/browser tests, and current compatibility and release docs. Refresh pre-generated `local-ci.sh` fixtures with real 0.8.29/0.8.30 producers. Build and extract a fresh Linux x86_64 package; qualify declared glibc/kernel/browser bounds and installed workflows. Preserve `dist/inspector/0.2.0/` untouched. |

The dependency transition is larger than the published 0.8.29-to-0.8.30 diff:
current Inspector still embeds the earlier 0.8.28-era cohort. Do not copy the
old Inspector lockfile or workspace patches over 0.8.30. Keep bare builds
model-free and `cached-cpu` opt-in; do not accidentally enable CUDA or
reranking.

## Qualification scope

Use closed real databases created by each published producer version, carrying
the lock sentinel and preserving the originals. A header-only version edit is
not a positive compatibility fixture. A 0.8.30-native Rust producer is also
acceptable when explicitly identified; it is not registry-install evidence.

- Exercise catalog, body/tree, attributes, FTS/BM25F/property search, stored
  vectors/L2, actual CPU hybrid, graph/history/provenance, and A/B comparison.
- Cover missing/partial projections, current/history/inactive records, all eight
  D-01 flag combinations, legitimate queued erasure where it exists, and actual
  physical absence. Preserve deletion behavior and fresh disclosure checks.
- Check frozen native evidence versus later stored reads, exact large IDs,
  stale handles, limits/partiality, native-call cancellation, lock release, and
  final window/process closure on the selected new composition.
- Reuse passing fixtures and human-intent oracles; update tests that asserted
  pre-0.8.29 body deduplication (`tests/integration/lexical.rs`) and add focused
  RED/GREEN cases only for uncovered behavior or necessary code changes. Do not
  bless old body-deduplication outputs as the new oracle.
- Run affected checks while integrating, then the required full source gate and
  fresh extracted-package checks once on the final candidate. Note that the
  existing owner ruling (`INS-020-LOCAL-RELEASE`, `seq-1`) removing the
  five-person study and scoping local-only promotion is worded for Inspector
  0.2.0; record explicit owner carry-forward for 0.2.1 before cutting the
  release.

A single 0.8.30-backed Inspector implementation can be qualified for both
producer versions because their persisted source format is identical. That is a
proposed support claim contingent on both positive fixture sets, not a reason to
add per-version conditionals or compatibility shims.

## Optional or outside this compatibility effort

- Exposing Engine's aggregate stored-head coverage (`StoredHeadCoverageV1`) is
  optional feature work; Inspector already shows recorded/unknown per-row spans.
  The aggregate API arrived in published 0.8.29, not 0.8.30. Keep storage and
  serving readiness distinct.
- Surfacing `SnippetV1.scored_bytes` (new in 0.8.29) is optional feature work.
  Inspector's lexical route uses text-only search (where `scored_bytes` is
  always `None`), and its frozen hybrid route (`vector.rs::serving`) calls
  `search_frozen` without requesting snippets.
- SDK `release_models()` is unnecessary here. The accepted
  `ADR-0.8.30-explicit-module-model-release.md` excludes Engine-owned Mean
  providers. Inspector calls Engine directly, requests rerank depth 0, and
  closes its owned Engine/worker. No SDK dependency or release button is needed.
- Do not promise the 0.8.30 hybrid benchmark speedup: its optimized route
  requires explanations disabled (`!explain` guard in `search.rs`); Inspector's
  frozen hybrid call requests explanations (`include_explanation = true`).
  Measure the actual Inspector route if a latency claim is desired.
- Multi-window embedding, GPU execution, registry publication, additional
  platforms, and a plugin/descriptor framework are separate scope. This release
  adds neither a new persisted data structure requiring a view nor an
  Engine-owned inspection descriptor contract.

Recommended initial execution was one compatibility slice; the owner
subsequently adopted a four-slice delivery sequence (Slices 10, 20, 30, and 40)
and ruled to drop Ubuntu 22.04 / Docker in favor of native Ubuntu 24.04
delivery—see [Inspector 0.2.1 scope](../plans/0.2.1/scope.md) and
[release plan](../plans/0.2.1/plan.md). Implementation risk is moderate in
semantic/model integration and workspace re-wiring, and low in stored layout/UI
work. Build/tooling adaptation and installed dual-producer qualification are
likely the largest effort; no credible elapsed-time estimate follows from this
static review. No port or new release was performed by this assessment.

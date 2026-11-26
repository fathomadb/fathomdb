---
title: FathomDB Decision Index
date: 2026-11-24
target_release: cross-release
desc: Active and historical index for accepted, superseded, and deferred ADRs
status: ACTIVE
---

# Decision index

Use this index to locate governing ADRs and their superseded or historical
predecessors. Accepted ADRs and maintained [interfaces](../interfaces) define
current semantics. Run `python3 scripts/release-current.py` to find the active
release plan and board.

Numbering preserves historical IDs `#1`–`#76` (with `#67` formatted for
`ADR-0.8.28-vector-compatibility-open-policy.md`, the duplicate `#66` for
`ADR-0.8.29-agent-hints-sidecar.md` reassigned to `#77`, previously
unindexed ADRs assigned `#78`–`#98`, and `0.8.32` ADRs assigned `#99`–`#102`).

## Active / Governing ADRs

| # | Release | Category | Governing decision | Status / notes | ADR file |
| --- | --- | --- | --- | --- | --- |
| 1 | 0.6.0 | architecture | Async surface for engine API (sync engine + TS binding + Invariants A–D) | accepted; runtime mechanisms partially superseded by #56 | [ADR-0.6.0-async-surface.md](ADR-0.6.0-async-surface.md) |
| 2 | 0.6.0 | design | Default-embedder architecture (candle + tokenizers + sqlite-vec; mean-pool + L2-normalize; zerocopy BLOB) | implemented (0.7.1) | [ADR-0.6.0-default-embedder.md](ADR-0.6.0-default-embedder.md) |
| 3 | 0.6.0 | architecture | `sqlite-vec` accept-no-fallback policy | accepted | [ADR-0.6.0-sqlite-vec-acceptance.md](ADR-0.6.0-sqlite-vec-acceptance.md) |
| 4 | 0.6.0 | interface | Operator config = JSON-only (TOML dropped) | accepted | [ADR-0.6.0-operator-config-json-only.md](ADR-0.6.0-operator-config-json-only.md) |
| 5 | 0.6.0 | architecture | Typed write boundary; no raw SQL from clients | accepted | [ADR-0.6.0-typed-write-boundary.md](ADR-0.6.0-typed-write-boundary.md) |
| 6 | 0.6.0 | architecture | Operational store lives in same SQLite file (no dual-store) | accepted | [ADR-0.6.0-op-store-same-file.md](ADR-0.6.0-op-store-same-file.md) |
| 7 | 0.6.0 | design | Embedder protocol contract (sync, unit-norm, no-reentrancy, per-call timeout) | accepted; dispatch/deadline partially superseded by #56; amended by #62 | [ADR-0.6.0-embedder-protocol.md](ADR-0.6.0-embedder-protocol.md) |
| 8 | 0.6.0 | architecture | Vector BLOB on-disk invariants (LE f32, alignment, byte length, BLOB affinity) | accepted | [ADR-0.6.0-zerocopy-blob.md](ADR-0.6.0-zerocopy-blob.md) |
| 9 | 0.6.0 | interface | No pre-1.0 compatibility shims or multi-release deprecation cycles | accepted | [ADR-0.6.0-no-shims-policy.md](ADR-0.6.0-no-shims-policy.md) |
| 10 | 0.6.0 | architecture | Single-writer-thread engine; MVCC out of scope | accepted; writer mechanism partially superseded by #56 | [ADR-0.6.0-single-writer-thread.md](ADR-0.6.0-single-writer-thread.md) |
| 11 | 0.6.0 | design | Vector identity belongs to the embedder; vector configs never carry identity strings | accepted | [ADR-0.6.0-vector-identity-embedder-owned.md](ADR-0.6.0-vector-identity-embedder-owned.md) |
| 7b | 0.6.0 | acceptance | Single-process durability target (fsync policy, recovery time) | accepted (HITL 2026-04-27); amended by #101 | [ADR-0.6.0-durability-fsync-policy.md](ADR-0.6.0-durability-fsync-policy.md) |
| 8b | 0.6.0 | acceptance | Projection freshness SLI numerical target | accepted (HITL 2026-04-27) | [ADR-0.6.0-projection-freshness-sli.md](ADR-0.6.0-projection-freshness-sli.md) |
| 9b | 0.6.0 | acceptance | Retrieval p50/p99 latency gates | accepted (HITL 2026-04-27) | [ADR-0.6.0-retrieval-latency-gates.md](ADR-0.6.0-retrieval-latency-gates.md) |
| 10b | 0.6.0 | acceptance | Tier-1 CI platforms list | accepted (HITL 2026-04-27) | [ADR-0.6.0-tier1-ci-platforms.md](ADR-0.6.0-tier1-ci-platforms.md) |
| 11b | 0.6.0 | architecture | Crate topology (`fathomdb-engine` boundary) | accepted (HITL 2026-04-27) | [ADR-0.6.0-crate-topology.md](ADR-0.6.0-crate-topology.md) |
| 13 | 0.6.0 | architecture | Vector index location (`vec0` in same SQLite file) | accepted (HITL 2026-04-27) | [ADR-0.6.0-vector-index-location.md](ADR-0.6.0-vector-index-location.md) |
| 14 | 0.6.0 | architecture | Scheduler shape — tokio runtime + per-job tasks | accepted; runtime topology partially superseded by #56 | [ADR-0.6.0-scheduler-shape.md](ADR-0.6.0-scheduler-shape.md) |
| 16 | 0.6.0 | design | Projection model — push (eager) | accepted; admission/rediscovery partially superseded by #56 | [ADR-0.6.0-projection-model.md](ADR-0.6.0-projection-model.md) |
| 17 | 0.6.0 | design | Retrieval pipeline shape — fixed stages | accepted (HITL 2026-04-27) | [ADR-0.6.0-retrieval-pipeline-shape.md](ADR-0.6.0-retrieval-pipeline-shape.md) |
| 18 | 0.6.0 | design | Error taxonomy — per-module + top-level wrap | accepted (HITL 2026-04-27) | [ADR-0.6.0-error-taxonomy.md](ADR-0.6.0-error-taxonomy.md) |
| 19 | 0.6.0 | design | `PreparedWrite` shape | accepted (HITL 2026-04-27) | [ADR-0.6.0-prepared-write-shape.md](ADR-0.6.0-prepared-write-shape.md) |
| 20 | 0.6.0 | interface | Python API shape | accepted (HITL 2026-04-27) | [ADR-0.6.0-python-api-shape.md](ADR-0.6.0-python-api-shape.md) |
| 21 | 0.6.0 | interface | TypeScript API — idiomatic + 1:1 type names | accepted (HITL 2026-04-27) | [ADR-0.6.0-typescript-api-shape.md](ADR-0.6.0-typescript-api-shape.md) |
| 22 | 0.6.0 | interface | CLI scope — admin + recovery + read-only query | accepted; partially superseded by #55 | [ADR-0.6.0-cli-scope.md](ADR-0.6.0-cli-scope.md) |
| 23 | 0.6.0 | interface | Deprecation policy for 0.5.x names | accepted (HITL 2026-04-27) | [ADR-0.6.0-deprecation-policy-0-5-names.md](ADR-0.6.0-deprecation-policy-0-5-names.md) |
| 24 | 0.6.0 | acceptance | Write-throughput SLI | accepted (HITL 2026-04-27) | [ADR-0.6.0-write-throughput-sli.md](ADR-0.6.0-write-throughput-sli.md) |
| 25 | 0.6.0 | design | JSON Schema validation policy | accepted (HITL 2026-04-27) | [ADR-0.6.0-json-schema-policy.md](ADR-0.6.0-json-schema-policy.md) |
| 27 | 0.6.0 | acceptance | Recovery rank-correlation threshold (`AC-027d`) | accepted (HITL 2026-04-27) | [ADR-0.6.0-recovery-rank-correlation.md](ADR-0.6.0-recovery-rank-correlation.md) |
| 28 | 0.6.0 | design | Provenance retention shape (`AC-033`) | accepted (HITL 2026-04-27) | [ADR-0.6.0-provenance-retention.md](ADR-0.6.0-provenance-retention.md) |
| 29 | 0.6.0 | architecture | Corruption-on-open behavior | accepted (HITL 2026-04-29) | [ADR-0.6.0-corruption-open-behavior.md](ADR-0.6.0-corruption-open-behavior.md) |
| 31 | 0.6.0 | architecture | Database lock mechanism — reader-pool revision (sidecar `flock`, normal WAL `-shm`) | accepted (HITL 2026-05-02); supersedes #30 | [ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md](ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md) |
| 78 | 0.7.0 | acceptance | Revised text-query latency gates (`AC-012`/`AC-013`/`AC-019`/`AC-020` tiered budgets) | locked (HITL 2026-06-01); supersedes #26 | [ADR-0.7.0-text-query-latency-gates-revised.md](ADR-0.7.0-text-query-latency-gates-revised.md) |
| 79 | 0.7.0 | architecture | Vector binary quantization (Pack 1 + Pack 2, ANN fidelity floor `0.90`) | locked (HITL-ratified; 0.7.2 / 0.8.0 Slice 40) | [ADR-0.7.0-vector-binary-quant.md](ADR-0.7.0-vector-binary-quant.md) |
| 32 | 0.7.1 | design | Opt-in default-embedder weight fetch on first use, sha256-pinned, with `OpenReport` visibility | accepted (HITL 2026-05-28) | [ADR-0.7.1-default-embedder-weight-fetch.md](ADR-0.7.1-default-embedder-weight-fetch.md) |
| 34 | 0.8.0 | architecture | Canonical-identity substrate (`logical_id` + `superseded_at` on `canonical_nodes` and `canonical_edges`, partial-unique active indexes) | accepted (HITL 2026-06-03) | [ADR-0.8.0-canonical-identity-substrate.md](ADR-0.8.0-canonical-identity-substrate.md) |
| 35 | 0.8.0 | acceptance | Governed SDK surface superseding `AC-057a` five-verb cap while preserving parity, recovery denylist, and typed boundary | accepted (HITL 2026-06-03) | [ADR-0.8.0-supersede-five-verb-surface-cap.md](ADR-0.8.0-supersede-five-verb-surface-cap.md) |
| 80 | 0.8.0 | interface | Filter grammar (`Predicate` enum, parameterized compilation, no raw SQL DSL) | accepted (HITL 2026-06-06) | [ADR-0.8.0-filter-grammar.md](ADR-0.8.0-filter-grammar.md) |
| 81 | 0.8.0 | architecture | Graph model and edge addressing — one neutral substrate for corpus and memory | accepted (Slice 32) | [ADR-0.8.0-graph-model-and-edge-addressing.md](ADR-0.8.0-graph-model-and-edge-addressing.md) |
| 38 | 0.8.1 | acceptance | IR measure and evaluation design (candidate-recall CDF and end-to-end parity eval) | accepted (HITL 2026-06-13) | [ADR-0.8.1-ir-measure-eval-design.md](ADR-0.8.1-ir-measure-eval-design.md) |
| 39 | 0.8.1 | architecture | Graph substrate G11 migration (edge `body`/`t_valid`/`t_invalid`/`confidence`/`extractor_model_id` and edge projectability) | accepted (HITL 2026-06-13) | [ADR-0.8.1-graph-substrate-g11-migration.md](ADR-0.8.1-graph-substrate-g11-migration.md) |
| 82 | 0.8.6 | design | Generalized typed-task provider protocol (`OPP-8`) | accepted (HITL 2026-06-26); supersedes #37 | [ADR-0.8.6-generalized-provider-protocol.md](ADR-0.8.6-generalized-provider-protocol.md) |
| 83 | 0.8.6 | architecture | Governed-verb coupling hygiene (`OPP-5`) | accepted (HITL 2026-06-26) | [ADR-0.8.6-governed-verb-coupling-hygiene.md](ADR-0.8.6-governed-verb-coupling-hygiene.md) |
| 84 | 0.8.14 | architecture | Kind-tagged coexisting-index substrate migration (`EXP-S` + `F5` co-land) | accepted (HITL 2026-07-03) | [ADR-0.8.14-exp-s-kind-tagged-coexisting-index-substrate.md](ADR-0.8.14-exp-s-kind-tagged-coexisting-index-substrate.md) |
| 85 | 0.8.16 | design | `F9` importance/confidence ranking (off-by-default signal) | accepted (HITL 2026-07-08) | [ADR-0.8.16-f9-importance-confidence-ranking.md](ADR-0.8.16-f9-importance-confidence-ranking.md) |
| 86 | 0.8.18 | release | Full multi-registry publish pipeline (`#11-full`) | accepted (HITL 2026-07-09) | [ADR-0.8.18-full-publish-pipeline.md](ADR-0.8.18-full-publish-pipeline.md) |
| 87 | 0.8.18 | design | Vector-equivalence self-check (open-path baseline, binary-flip `0` and L2 `1e-5` floors, `TC-68` cache) | accepted (HITL 2026-07-09); refusal clauses partially superseded by #67 | [ADR-0.8.18-vector-equivalence-self-check.md](ADR-0.8.18-vector-equivalence-self-check.md) |
| 40 | 0.8.20 | release | Linux AArch64 native artifacts (`manylinux` Python wheel and unscoped npm package) | accepted (HITL 2026-08-02) | [ADR-0.8.20-linux-aarch64-native-artifacts.md](ADR-0.8.20-linux-aarch64-native-artifacts.md) |
| 88 | 0.8.20 | release | Unscoped npm native platform packages (`fathomdb-<triple>`) | accepted (2026-08-02); Windows x64 superseded by #42 | [ADR-0.8.20-unscoped-npm-platform-packages.md](ADR-0.8.20-unscoped-npm-platform-packages.md) |
| 41 | 0.8.21 | interface | Nested-source projections (declared literal member paths, engine-owned scalar EAV/property-FTS derivation, portable attribute filtering) | accepted (HITL 2026-08-04; `seq-242`) | [ADR-0.8.21-nested-source-projections.md](ADR-0.8.21-nested-source-projections.md) |
| 42 | 0.8.22 | release | Windows x64 MSVC native npm package named `fathomdb-native-win32-x64-msvc`; non-Windows retain `fathomdb-<triple>` | accepted (HITL 2026-08-09); partially supersedes #88 | [ADR-0.8.22-windows-native-npm-package.md](ADR-0.8.22-windows-native-npm-package.md) |
| 43 | 0.8.23 | interface | Dual-runtime CPU/GPU device policy (`auto`/`cpu`/`cuda:N`, `doctor gpu`, no silent fallback on forced CUDA) | accepted (`seq:250`, `seq:252`, 2026-10-10) | [ADR-0.8.23-dual-runtime-device-policy.md](ADR-0.8.23-dual-runtime-device-policy.md) |
| 44 | 0.8.25 | interface | Bounded, model-free, idempotent atomic actuation method (`actuate`) preserving `PreparedWrite`, typed boundary, and recovery denylist | accepted (0.8.25 Slice 25) | [ADR-0.8.25-bounded-atomic-actuation.md](ADR-0.8.25-bounded-atomic-actuation.md) |
| 45 | 0.8.25 | architecture | Direct single-source dependency lifecycle closure and erasure completion | accepted; retry scheduling partially superseded by #68 | [ADR-0.8.25-dependency-lifecycle-closure.md](ADR-0.8.25-dependency-lifecycle-closure.md) |
| 46 | 0.8.25 | interface | Pre-stage ranked-search eligibility filtering and stateless reproduce-or-fail frozen read context | accepted (0.8.25 Slice 35); amended on Schema `39+` by #103 | [ADR-0.8.25-eligibility-and-frozen-reads.md](ADR-0.8.25-eligibility-and-frozen-reads.md) |
| 47 | 0.8.25 | interface | Frozen keyset pagination for canonical logical nodes and governed reads for registered latest-state collections | accepted (0.8.25 Slice 45); amended on Schema `39+` by #103 | [ADR-0.8.25-frozen-pagination-and-operational-state.md](ADR-0.8.25-frozen-pagination-and-operational-state.md) |
| 48 | 0.8.25 | interface | Stateless, content-free compact source evidence references and eligibility-bound exact resolver | accepted (0.8.25 Slice 50); amended by #103 | [ADR-0.8.25-compact-source-evidence.md](ADR-0.8.25-compact-source-evidence.md) |
| 49 | 0.8.25 | interface | Eligibility-bound one-hop dependency trace, bounded operator integrity, and additive explained-search correlation | accepted (0.8.25 Slice 55) | [ADR-0.8.25-governed-dependency-trace-and-integrity.md](ADR-0.8.25-governed-dependency-trace-and-integrity.md) |
| 50 | 0.8.25 | architecture | Application-owned SQLite startup runtime configuration (performance default vs. diagnostics) | accepted (`seq-276`) | [ADR-0.8.25-sqlite-runtime-configuration.md](ADR-0.8.25-sqlite-runtime-configuration.md) |
| 51 | 0.8.25 | acceptance | Absolute sequential/concurrent read-performance successor retiring `AC-020` relative ratio | accepted (`seq-277`); supersedes #92 | [ADR-0.8.25-absolute-read-performance-successor.md](ADR-0.8.25-absolute-read-performance-successor.md) |
| 52 | 0.8.25 | architecture | Driverless CUDA runtime linkage for Linux `x86_64` artifacts (CPU mode runs without CUDA runtime or NVIDIA driver) | accepted (HITL 2026-08-31) | [ADR-0.8.25-driverless-cuda-runtime-linkage.md](ADR-0.8.25-driverless-cuda-runtime-linkage.md) |
| 53 | 0.8.26 | interface | Breaking V1 contract in place with fresh-database boundary; no parallel V1/V2 API pairs pre-1.0 | accepted (`seq-283`) | [ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md](ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md) |
| 54 | 0.8.26 | interface | Opt-in positional graph evidence sidecar and frozen-only opaque point resolver | accepted (`seq-290`) | [ADR-0.8.26-exact-graph-artifact-evidence.md](ADR-0.8.26-exact-graph-artifact-evidence.md) |
| 55 | 0.8.26 | interface | Preserve `recover` for CLI data-loss-authorized recovery; grandfather atomic `doctor recompute-mean` derived-vector maintenance | accepted (owner 2026-09-17) | [ADR-0.8.26-cli-derived-maintenance-boundary.md](ADR-0.8.26-cli-derived-maintenance-boundary.md) |
| 56 | 0.8.27 | architecture | Engine-owned projection orchestration and embed dispatch with universal deadlines, typed config, and two-phase shutdown | accepted (`seq-295`) | [ADR-0.8.27-engine-owned-runtime-topology.md](ADR-0.8.27-engine-owned-runtime-topology.md) |
| 57 | 0.8.27 | architecture | Default embed-dispatch worker count (`5`) under `D27` comparison and unchanged performance gates | accepted (`seq-299`, 2026-10-02) | [ADR-0.8.27-embed-dispatch-default-capacity.md](ADR-0.8.27-embed-dispatch-default-capacity.md) |
| 58 | 0.8.27 | interface | Bounded, nonblocking Python engine diagnostics logger adapter | accepted (owner 2026-10-03) | [ADR-0.8.27-python-subscriber-delivery.md](ADR-0.8.27-python-subscriber-delivery.md) |
| 89 | 0.8.27 | interface | Bounded TypeScript engine diagnostics subscriber delivery | accepted (2026-10-04) | [ADR-0.8.27-typescript-subscriber-delivery.md](ADR-0.8.27-typescript-subscriber-delivery.md) |
| 59 | 0.8.27 | interface | Offline physical erasure completion through one `recover` connection and `doctor` diagnosis | accepted (owner 2026-10-03); retry scheduling partially superseded by #68 | [ADR-0.8.27-offline-physical-erasure-completion.md](ADR-0.8.27-offline-physical-erasure-completion.md) |
| 60 | 0.8.27 | interface | Dedicated `fathomdb-sdk` Rust crate carrying governed Python/TypeScript parity | accepted (owner 2026-10-06); passive diagnostic addendum: [ADR-0.8.29-sdk-qualification-observation.md](ADR-0.8.29-sdk-qualification-observation.md) | [ADR-0.8.27-rust-sdk-parity.md](ADR-0.8.27-rust-sdk-parity.md) |
| 61 | 0.8.27 | release | Dual-runtime CUDA Node addon for Jetson Orin distributed as exact-version tarball via Tegra Pages route | accepted (owner 2026-10-09) | [ADR-0.8.27-jetson-tegra-node-addon-distribution.md](ADR-0.8.27-jetson-tegra-node-addon-distribution.md) |
| 62 | 0.8.28 | architecture | Jetson AGX Orin private capped CUDA memory pool (`tegra-pool`, release threshold 0, three typed `EmbedderError` variants) | accepted (owner 2026-10-08; amended Slice 31) | [ADR-0.8.28-tegra-private-cuda-pool.md](ADR-0.8.28-tegra-private-cuda-pool.md) |
| 63 | 0.8.28 | release | CUDA version support policy (CUDA 12.x/13.x on Turing+, 12.6 shipping floor, per-release usage review) | accepted (owner 2026-10-11; amended Slice 31) | [ADR-0.8.28-cuda-version-support-policy.md](ADR-0.8.28-cuda-version-support-policy.md) |
| 64 | 0.8.28 | architecture | Bundled SQLite per-connection page cache (drop `SQLITE_ENABLE_MEMORY_MANAGEMENT`, no global page-cache mutex) | accepted (owner 2026-10-16; Slice 42 complete) | [ADR-0.8.28-sqlite-per-connection-page-cache.md](ADR-0.8.28-sqlite-per-connection-page-cache.md) |
| 65 | 0.8.28 | interface | Opt-in role-tagged round body stored-bytes format (`U+001E` + role code + text, `\n`-joined) | accepted (owner 2026-10-16) | None; see `dev/plans/0.8.28/features/slice-41/design.md` and `dev/interfaces/python.md` |
| 66 | 0.8.28 | architecture | Store format 35 (64-row `vec0` chunks, cursor-keyed body FTS, per-vector UTF-8 head coverage, embedder-api Axis E `0.8.0`) | accepted (2026-11-01) | [ADR-0.8.28-store-format-35.md](ADR-0.8.28-store-format-35.md) |
| 67 | 0.8.28 | architecture | Narrow vector compatibility admission and deliberate access (built-in-only production admission pre-1.x, typed failed open on refusal, minimum baseline failure atomicity) | accepted (`seq-359`, `seq-357`); partially supersedes #87 | [ADR-0.8.28-vector-compatibility-open-policy.md](ADR-0.8.28-vector-compatibility-open-policy.md) |
| 68 | 0.8.28 | architecture | Scoped unrelated writes and autonomous resumable physical cleanup | accepted (Slice 83); explicit scheduling superseded by #69 | [ADR-0.8.28-resumable-physical-cleanup.md](ADR-0.8.28-resumable-physical-cleanup.md) |
| 69 | 0.8.28 | interface | Persisted opt-in explicit erasure maintenance (queued intent, exclusive maintenance windows, durable incomplete fences, format 36) | accepted (owner 2026-11-04) | [ADR-0.8.28-explicit-erasure-maintenance.md](ADR-0.8.28-explicit-erasure-maintenance.md) |
| 70 | 0.8.28 | architecture | Transaction-owned visibility coalescing and format-37 marker | accepted (Slice 95); extended by #103 | [ADR-0.8.28-transactional-visibility-coalescing.md](ADR-0.8.28-transactional-visibility-coalescing.md) |
| 71 | 0.8.28 | interface | Method-selected v2 attributable GPU allocation proof (`64 MiB` floor, live private-pool `USED` on integrated devices) | accepted (Slice 95) | [ADR-0.8.28-attributable-cuda-allocation-witness.md](ADR-0.8.28-attributable-cuda-allocation-witness.md) |
| 72 | 0.8.29 | architecture | Retrieval record identity fusion across arms and atomic refusal of ambiguous named feedback refs | accepted (`seq-374`/`seq-375`; delivered 0.8.30) | [ADR-0.8.29-retrieval-record-identity.md](ADR-0.8.29-retrieval-record-identity.md) |
| 77 | 0.8.29 | interface | FathomDB-authored data-free hints sidecar of suggested next calls (engine subset + MCP adapter wire type) | accepted (owner 2026-10-18) | [ADR-0.8.29-agent-hints-sidecar.md](ADR-0.8.29-agent-hints-sidecar.md) |
| 90 | 0.8.29 | interface | Narrow passive diagnostic addendum for `fathomdb-sdk` qualification observation | accepted (amends #60) | [ADR-0.8.29-sdk-qualification-observation.md](ADR-0.8.29-sdk-qualification-observation.md) |
| 91 | 0.8.30 | interface | Explicit module model release (`B30-01`) | accepted (0.8.30 Slice 50) | [ADR-0.8.30-explicit-module-model-release.md](ADR-0.8.30-explicit-module-model-release.md) |
| 73 | 0.8.31 | architecture | Schema 38 inline canonical JSON `properties` on nodes and edges + declared typed B-tree/`vec0` indexes, batch commit timestamps, reserved `scope` column, opt-in vector-committable kinds, and `ExportClass` erasure audit | accepted (owner 2026-11-21; amended 2026-11-22) | [ADR-0.8.31-property-graph-inline-and-declared-indexes.md](ADR-0.8.31-property-graph-inline-and-declared-indexes.md) |
| 74 | 0.8.31 | interface | Strict-only write endpoint integrity (`superseded_at IS NULL AND state = 'active'`) on all 0.8.31 write paths with atomic rollback | accepted (owner 2026-11-21); extended by #100 | [ADR-0.8.31-strict-endpoint-integrity.md](ADR-0.8.31-strict-endpoint-integrity.md) |
| 75 | 0.8.31 | architecture | Bitemporal/revision-aware `as_of` historical graph traversal, closed typed edge/target predicates, opt-in `graph_seed` fused-hit seeding + seed/depth provenance | accepted (owner 2026-11-21; amended 2026-11-22) | [ADR-0.8.31-historical-graph-and-fused-seeding.md](ADR-0.8.31-historical-graph-and-fused-seeding.md) |
| 76 | 0.8.31 | release | Lockstep `fathomdb-cli` PyPI binary wheels via Maturin `bindings = "bin"` and verified `fathomdb migrate` (`37 -> 38`) helper | accepted (owner 2026-11-21; amended 2026-11-22) | [ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md](ADR-0.8.31-cli-pypi-wheel-and-37-to-38-upgrade.md) |
| 99 | 0.8.32 | interface | Production read+write `fathomdb-mcp` (`allow_writes: false` default, per-store opt-in + bound `scope`), `SC-01`–`SC-10` startup conflicts, `AC32-MCP-SAFETY-01`–`09`, atomic `PutLatestState` + `EP-07` + `EP-01` (Pin C), and 3-channel distribution (`--features default-embedder` GH/PyPI, `default = []` crates.io, `AC-041` empty base scripts) | accepted (Owner Rulings 1 & 3; Gate 1 Decisions 4 & 5, 2026-11-24) | [ADR-0.8.32-mcp-read-write-and-distribution.md](ADR-0.8.32-mcp-read-write-and-distribution.md) |
| 100 | 0.8.32 | interface | Opt-in bulk/non-strict write mode (`WriteOptions` / `write_with_options`, `BulkNonStrict`) keeping default `Engine::write` and `Engine::actuate` strict, truthful `WriteReceipt.dangling_edge_endpoints`, and `fathomdb doctor` dangling + cross-scope edge verification | accepted (Gate 1 Decision 4 Pin B, 2026-11-24); extends #74 | [ADR-0.8.32-opt-in-bulk-write-mode.md](ADR-0.8.32-opt-in-bulk-write-mode.md) |
| 101 | 0.8.32 | acceptance | Declared WAL durability (`SyncMode::Normal` checkpoint-bounded prefix-recovery default + opt-in `SyncMode::Full`), QEMU/KVM `dm-flakey` power-loss oracle (`AC32-DMFLAKEY-01`–`04`), and isolated `fuzz/` workspace (`AC32-FUZZ-01`–`03`) | accepted (Gate 1 Decision 2 Option 1, 2026-11-24); amends #7b | [ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md](ADR-0.8.32-durability-sync-mode-and-power-loss-oracle.md) |
| 102 | 0.8.32 | architecture | Engine-enforced caller `scope` isolation (`ScopePolicy` anti-spoof, Pin A), Schema 39 composite `(scope, ...)` layout (`S39-COMPOSITE` + `fathomdb migrate 38 -> 39`), single-scope/single-agent default posture, mandatory 8-col `vec0`, and single-source `TABLE_MANIFEST` | accepted (Gate 1 Decision 1 Option 1, 2026-11-24); extended by #103 | [ADR-0.8.32-caller-scope-isolation.md](ADR-0.8.32-caller-scope-isolation.md) |
| 103 | 0.8.32 | architecture | Partitioned Schema 39 frozen-read additive stability (`_fathomdb_scope_visibility_watermarks`, `ReadSurface`-aware `validate_snapshot`, 20-table trigger classification, generalized coalescing, `fathomdb-mcp` `MR-070.2` drift probing) and artifact-scoped `SearchFilter.property_predicates` on evidence `source_filter` (`#269` Option A) | accepted (Gate 1 Decision 11, 2026-11-26); amends #46, #47, #48; extends #70, #102 | [ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md](ADR-0.8.32-frozen-read-additive-stability-and-evidence-eligibility.md) |

## Proposed / Pending Ratification ADRs

None currently pending ratification.

## Superseded, Historical, and Deferred ADRs

Preserved for decision traceability. Do not treat these as active requirements
without checking their named successor.

| # | Release | Category | Decision summary | Disposition / successor | ADR file |
| --- | --- | --- | --- | --- | --- |
| 12 | 0.6.0 | architecture | Single-writer thread vs. MVCC | resolved-by-deferral -> #10 ([ADR-0.6.0-single-writer-thread.md](ADR-0.6.0-single-writer-thread.md)) | [ADR-0.6.0-single-writer-thread.md](ADR-0.6.0-single-writer-thread.md) |
| 15 | 0.6.0 | architecture | Wire format for subprocess bridge | resolved-by-deferral (revisit post-0.8.0) | [ADR-0.6.0-subprocess-bridge-deferral.md](ADR-0.6.0-subprocess-bridge-deferral.md) |
| 26 | 0.6.0 | acceptance | Initial text-query latency gates (`FU-REQ-010`) | superseded by #78 ([ADR-0.7.0-text-query-latency-gates-revised.md](ADR-0.7.0-text-query-latency-gates-revised.md)) | [ADR-0.6.0-text-query-latency-gates.md](ADR-0.6.0-text-query-latency-gates.md) |
| 30 | 0.6.0 | architecture | Hybrid sidecar `flock` + `PRAGMA locking_mode=EXCLUSIVE` | superseded (2026-05-02) by #31 ([ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md](ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md)) | [ADR-0.6.0-database-lock-mechanism.md](ADR-0.6.0-database-lock-mechanism.md) |
| 92 | 0.7.0 | architecture | `AC-020` architectural lever (global page-cache premise) | superseded by #51 ([ADR-0.8.25-absolute-read-performance-successor.md](ADR-0.8.25-absolute-read-performance-successor.md)); page-cache premise corrected by #64 | [ADR-0.7.0-ac020-architectural-lever.md](ADR-0.7.0-ac020-architectural-lever.md) |
| 33 | 0.8.0 | design | Agent-memory retrieval and identity planning input (`G9`/`G10`/`G11` scoping) | historical planning input (HITL 2026-06-02); realized via #34 and #80 | [ADR-0.8.0-agent-memory-retrieval-and-identity.md](ADR-0.8.0-agent-memory-retrieval-and-identity.md) |
| 36 | 0.8.0 | design | Embedder identity-change workflow | draft (deferred) | [ADR-0.8.0-embedder-identity-change-workflow.md](ADR-0.8.0-embedder-identity-change-workflow.md) |
| 93 | 0.8.0 | design | Graph-traversal scope (`F1`/`G5`/`G6` 0.8.1 roadmap direction) | historical roadmap direction (HITL 2026-06-06); superseded by #39 and #75 | [ADR-0.8.0-graph-traversal-scope.md](ADR-0.8.0-graph-traversal-scope.md) |
| 37 | 0.8.1 | design | BYO-LLM extraction provider protocol (`fathomdb.extract.v1` stdio NDJSON) | superseded / generalized by #82 ([ADR-0.8.6-generalized-provider-protocol.md](ADR-0.8.6-generalized-provider-protocol.md)) | [ADR-0.8.1-byo-llm-extraction-protocol.md](ADR-0.8.1-byo-llm-extraction-protocol.md) |
| 94 | 0.8.1 | design | Deferred `F5`: fielded FTS / BM25F column-weighted scoring | deferred (0.8.1); advanced by #84 ([ADR-0.8.14-exp-s-kind-tagged-coexisting-index-substrate.md](ADR-0.8.14-exp-s-kind-tagged-coexisting-index-substrate.md)) | [ADR-0.8.1-deferred-f5-fielded-fts-bm25f.md](ADR-0.8.1-deferred-f5-fielded-fts-bm25f.md) |
| 95 | 0.8.1 | design | Deferred `F9`: confidence-vs-importance weighting | deferred (0.8.1); opened by #85 ([ADR-0.8.16-f9-importance-confidence-ranking.md](ADR-0.8.16-f9-importance-confidence-ranking.md)) | [ADR-0.8.1-deferred-f9-confidence-importance.md](ADR-0.8.1-deferred-f9-confidence-importance.md) |
| 96 | 0.8.11 | design | Filter-grammar unification (`G4` + `G10` unified `Filter` contract) | proposed (0.8.11 Slice 0 draft; historical) | [ADR-0.8.11-filter-grammar-unification.md](ADR-0.8.11-filter-grammar-unification.md) |
| 97 | 0.8.12 | design | Consolidation / recency provider (`OPP-2`) | proposed (0.8.12 Slice 0 draft; historical) | [ADR-0.8.12-consolidation-recency-provider.md](ADR-0.8.12-consolidation-recency-provider.md) |
| 98 | 0.8.16 | design | Cross-vendor ONNX embedder backend (`OrtBgeEmbedder`) | historical; runtime policy superseded by #43 and pre-1.x production admission closed by #67 | [ADR-0.8.16-onnx-embedder-backend.md](ADR-0.8.16-onnx-embedder-backend.md) |

## Categories

`acceptance` | `architecture` | `design` | `interface` | `release`

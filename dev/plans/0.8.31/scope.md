---
title: FathomDB 0.8.31 release scope — property graph, production readiness and Inspector alignment
status: PROPOSED
target_release: 0.8.31
date: 2026-11-21
---

# FathomDB 0.8.31 release scope

## Theme

Move FathomDB from a prototype retrieval store to a **production-grade embedded
property graph and evidence database** (typed properties on **both nodes and
edges**, without adding a graph query language), close the critical
production-readiness gaps identified in external review, preserve all erasure,
metadata, and latency invariants, and bring **Inspector** into the main
repository and release cycle.

## In scope

### 1. First-class property graph (nodes and edges; no GQL)

- **Typed properties on nodes and edges (`SCHEMA_VERSION` bump `37 -> 38+`)**:
  - Store arbitrary key-value typed properties (string, integer, float,
    boolean) **outside `canonical_nodes.body` and `canonical_edges.body`** so
    `body` remains clean plain text for FTS5 and 512-wordpiece embedding
    without envelope JSON keys, UUIDs, or hashes polluting retrieval.
  - Make typed node properties **filterable in `SearchFilter` / `Filter`** as
    well as `graph.expand`, and readable on `SearchHit` and resolved evidence,
    with full SDK parity across Rust (`fathomdb-sdk`), Python, and TypeScript.
    *(This eliminates the need for Memex ask R2 / projected-text indexing on
    the hybrid route.)*
  - Keep property updates strictly **immutable/revisioned** (new `write_cursor`,
    new `_fathomdb_artifact_revisions` entry, superseding the prior version) —
    never in-place mutation.
- **App-declared vector-committable kinds (Memex R1)**:
  - Lift the hardcoded 7-kind dense node allowlist (`email`, `article`, `paper`,
    `meeting`, `note`, `todo`, `doc`) so applications (such as Memex `mms1:*`,
    Hermes, OpenClaw, and domain apps) can declare their own vector-indexed
    node kinds without overloading `note`.
- **Typed edge/target predicates, opt-in fused-hit graph seeding, and seed provenance**:
  - Extend `GraphExpandRequestV1` to accept closed, validated typed predicates
    on **edges and target nodes** (e.g., `test_method = "bench"`,
    `confidence > 0.9`) reusing/extending `FilterTerm`/`Predicate` safely (no
    raw SQL or open query DSL).
  - Return edge properties on `GraphOriginV1` / `GraphTargetV1` and prove that
    ineligible edges cannot enter a traversal path or displace eligible results.
  - Add an **opt-in fused-hit seeding mode** (`graph_seed="fused_top_k" |
    "graph_fts" | "both"`, `graph_seed_k`) and **per-hit seed provenance + hop
    depth** in explanation and evidence (Memex Graph Asks 2 & 3), allowing
    dense/reranked hits and body-less property-graph edges to drive graph
    expansion while keeping default `use_graph_arm=False` zero-cost.
  - Add edge-kind/edge-property filtering on `use_graph_arm` and fix the
    mixed-edge `SEED_FTS_N` slot caveat so edges touching ineligible nodes
    cannot displace eligible seed slots (Memex Graph Ask 5).

### 2. Historical graph reconstruction (External Reviewer P2 — largest functional gap)

- Support **bitemporal / revision-aware graph traversal** (`as_of` / historical
  edge view mode) instead of unconditionally skipping superseded edges in
  `graph_expand/execution.rs`.
- Reconstruct the exact graph path before and after an edge is superseded,
  including when validity time (`t_valid` / `t_invalid`) and revision/supersession
  time differ, while remaining strictly subordinate to physical erasure.

### 3. Strict write endpoint integrity (External Reviewer P2)

- Provide an opt-in (request- or store-level) **strict referential integrity
  mode** on `engine.write` (matching derived-edge `actuate`) so any edge
  referencing a missing or inactive endpoint causes atomic refusal with zero
  partial state after reopen, while retaining explicit loose/bulk import when
  requested.

### 4. Production agent interface, SDK ergonomics, and access boundaries (External Reviewer P1 + Memex 0.6.2 asks)

- **Engine-enforced caller scope / isolation**:
  - Define and enforce a first-class caller/namespace scope boundary across
    `search` (including `use_graph_arm`), `fetch`, `expand`, frozen read
    contexts, continuations, and governed writes (Memex Graph Ask 6).
  - Prove with overlapping multi-caller fixtures that restricted records remain
    inaccessible across every route, including stale references/cursors after
    grant revocation, and that unauthorized/stale writes leave state unchanged
    while lost responses retry idempotently.
- **SDK evidence, hints, capability, and op-store parity (Memex Asks 1, 2, 4, R12)**:
  - Add bounded batch **`resolve_evidence_many`** under one frozen context with
    per-ref typed refusals (`get_many` partial-result style) to eliminate N
    round trips and N frozen-context validations per search.
  - Expose engine **search hints** (`no_results`, `degraded`, `more_results`)
    across Python, TypeScript, and Rust on `search`, `search_frozen`, and
    `EvidenceSearchRequestV1`.
  - Add explicit capability booleans on **`OpenReport`** (`reranker_compiled`,
    `cuda_compiled`, `rerank_available`).
  - Add SDK **readback of operational-collection kinds** and explicit mismatch
    behavior on re-registration.
- **Graduate `fathomdb-mcp` and ship lockstep `fathomdb-cli` PyPI wheels (Memex R10)**:
  - Expose a bounded **exact frozen-evidence search-and-resolve workflow**
    (exact bytes, revision identity, typed drift/erasure/truncation outcomes)
    aligned with a corresponding SDK-hosted helper workflow.
  - Enforce caller scope across all MCP tools and continuations.
  - Package and publish `fathomdb-mcp` binaries in the release workflow.
  - Publish a lockstep **`fathomdb-cli` PyPI binary wheel** (Maturin
    `bindings = "bin"`, optionally exposed via `fathomdb[cli]`) so Python-only
    consumers (`pip install` / `uv tool install`) get exact-version `fathomdb
    doctor`, `recompute-mean`, `recover`, and later `0.8.33`'s `migrate`
    without needing a Rust toolchain, while preserving the SDK-vs-CLI operator
    boundary.
  - *(Open HITL decision: whether governed/idempotent writes & checkpoints enter
    MCP in `0.8.31` or remain SDK-only.)*
- **Targeted contract doc fixes (Memex Graph Ask 1, Ask 4, Ask 5)**:
  - Fix stale `use_graph_arm` description in `docs/reference/python-api.md`,
    document PyPI wheel CUDA + cross-encoder reranker coverage in
    `docs/install/python.md`, and document mixed-`source_id`
    `Engine.write(batch)` single-transaction atomicity and zero-row rollback on
    `WriteValidationError`.

### 5. Operational durability and existing `0.8.31` quality backlog

- Formalize and verify **backup, restore, and full rebuild** workflows against
  an authoritative corpus (matching revisions, evidence spans, and graph
  results), keeping CLI operator verbs cleanly separated per
  [recovery-surface.md](../../../docs/positions/recovery-surface.md).
- Execute scoped items from [quality-backlog.md](quality-backlog.md):
  - **Power-loss / torn-write recovery proof** (beyond process `SIGKILL`) under
    a declared storage/durability fault model.
  - **Coverage-guided fuzzing** on query/filter, property predicate, cursor,
    codec, and recovery boundaries, retaining minimized regressions.

### 6. Non-negotiable invariants: erasure, metadata management, and zero latency regression

- **Erasure maintained by construction (defects only — no new erasure research
  campaigns)**:
  - Register every new derived property/index table in `ROW_OWNED_PROJECTIONS`
    (keyed by `write_cursor`) so `purge` and `erase_source` clean them
    automatically.
  - Extend `physical_erasure/admission.rs` and `physical_erasure.rs` byte
    accounting and scrubbing to cover node/edge property payloads across both
    Autonomous and ExplicitMaintenance modes.
  - Historical graph replay never resurrects purged/erased records; scope
    revocation and physical erasure stay distinct semantics.
  - Every new feature slice ships RED→GREEN `purge` and `erase_source` tests
    asserting zero rows, zero WAL/file remnants, and valid closure proofs.
  - **Targeted W12 erasure defect/ergonomics fixes (Memex Ask 3)**:
    distinguish a completing-retry `EraseReport` from already-erased, make BUSY
    checkpoint error text accurate (do not unconditionally blame a caller
    reader), and verify in Slice 0 whether checkpoint-only owed-closure
    write-fencing on unrelated writers can be narrowed without weakening
    closure durability.
- **Metadata management integrity**:
  - Add every new canonical, property, and scope table to the **read-visibility
    generation and coalescing trigger manifest** (`fathomdb-schema` step 37+
    pattern) so frozen reads and cursors always detect drift.
  - Keep typed property indexes synchronous (same-transaction) and async vector
    readiness truthful.
  - Add an automated **`sqlite_master` table-manifest completeness audit** test
    ensuring every table is classified across `CANONICAL_TABLES` /
    `ROW_OWNED_PROJECTIONS`, read-visibility triggers, erasure coverage,
    `doctor`, and Inspector.
- **No latency regression**:
  - **Pay-only-when-used fast paths**: keep `0.8.30`'s `hybrid_deferred_identity`
    and streamed FTS paths free of extra JOINs when no property/scope/history
    filter is active; keep `canonical_edges` partial active index for current
    traversal and use separate composite indexes for historical `as_of` replay;
    short-circuit single/default scope.
  - Store canonical properties inline (O(1) hydration) and index only declared
    filterable properties in typed B-trees; enforce `EXPLAIN QUERY PLAN`
    `SEARCH`-only (no `SCAN`) tests and preserve trigger coalescing.
  - Gate promotion on a **paired `v0.8.30` vs. `0.8.31` benchmark** (10k / 50k /
    100k) proving no statistically significant p50/p95 regression on existing
    unpredicated workloads.

### 7. Inspector brought in-line and into the release cycle

- Merge `inspector/dev/0.2.0` (`src/inspector/`, `dev/inspector/`,
  `docs/inspector/`) onto `main`.
- Upgrade Inspector from its `.28`/early-`.29` Engine baseline (`6edc1155f`,
  Candle 0.11 / cudarc 0.19.10) to the `0.8.31` Engine (schema 38+, Candle 0.13 /
  cudarc 0.21), executing the integration and qualification requirements from
  `compatibility-0.8.30-assessment.md` (on `inspector/dev/0.2.0`) plus UI views
  for **node/edge properties**, **historical graph inspection**, and
  **scope/readiness**.
- Integrate Inspector versioning, build, smoke tests, and release artifacts into
  the standard FathomDB release pipeline.

## Out of scope / explicitly deferred

- Graph query languages (Cypher, Gremlin, GQL, SPARQL).
- Database-owned abstention judgment, answer verification, or synthesis (host
  responsibility per [ROADMAP.md](../../ROADMAP.md)).
- Automatic schema migration from `0.8.30` (`37 -> 38+`) — scheduled in
  [0.8.33](../0.8.33/scope.md).
- Public OSS repo/doc overhaul and auto-doc redesign — scheduled in
  [0.8.32](../0.8.32/scope.md).
- Publishable competitive benchmark campaigns — scheduled in
  [0.8.34](../0.8.34/scope.md).
- **Not needed / superseded (Memex R2)**: projected-text indexing on the hybrid
  route is unnecessary once `0.8.31` stores typed node properties outside
  `canonical_nodes.body` and allows plain-text bodies with filtered properties.
- **Deferred (Memex E3)**: upgrading `ingest_with_extractor` to full
  `WriteProvenanceV1` / caller scope / multi-source dependencies is deferred
  (post-`0.8.33` / `0.9.0`); `0.8.32` will document it as legacy-only and steer
  consumers to `Engine.actuate`.
- **Recommended deferral (HITL needed)**: defer
  [cuda-delivery-plan.md](cuda-delivery-plan.md) (CUDA 13 + Tegra CUDA 13) out
  of `0.8.31` so `0.8.31` stays focused on property graph, production readiness,
  and Inspector integration.

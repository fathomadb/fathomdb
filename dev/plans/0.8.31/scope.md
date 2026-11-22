---
title: FathomDB 0.8.31 release scope — property graph core, strict writes, historical graph, Memex unblockers and Inspector
status: ACTIVE
target_release: 0.8.31
date: 2026-11-21
---

# FathomDB 0.8.31 release scope

## Theme

Deliver FathomDB's **property graph core** (typed properties on **both nodes
and edges** outside `body`, without adding a graph query language), **strict
endpoint write integrity**, **bitemporal historical graph reconstruction**,
**all core Memex `0.6.2` engine/CLI unblockers** (including a minimal
schema-37→38 upgrade helper), zero latency regression against `0.8.30`, and
**Inspector** merged into `main` and wired into the release cycle.

*(Split from [0.8.32](../0.8.32/scope.md) by owner ruling 2026-11-21 so caller
scope, read+write MCP, opt-in bulk writes, and deep durability/fuzzing execute
cleanly without overloading `0.8.31`.)*

## In scope

### 1. First-class property graph: inline typed properties + declared indexes (Option 1 — decided 2026-11-21)

- **Schema 38+ (`SCHEMA_VERSION` bump `37 -> 38+`)**:
  - Store arbitrary key-value typed properties (string, integer, float,
    boolean) in an **inline canonical JSON `properties` column on both
    `canonical_nodes` and `canonical_edges` — outside `body`** — so `body`
    remains clean plain text for FTS5 and 512-wordpiece embedding without
    envelope JSON keys, UUIDs, or hashes polluting retrieval.
  - Read properties back in O(1) on `SearchHit`, `GraphTargetV1`,
    `GraphOriginV1`, and resolved evidence with full SDK parity across Rust
    (`fathomdb-sdk`), Python, and TypeScript. *(Eliminates Memex ask R2 /
    projected-text hybrid indexing.)*
  - Project **declared filterable properties** (via `configure_projections`)
    into a **typed B-tree index table** (`text_val`, `num_val`, `bool_val`) and
    `vec0` pre-KNN metadata columns for corpus-wide `SearchFilter` / `Filter`
    queries. Undeclared keys remain cleanly rejected on corpus-wide
    `SearchFilter` (no silent full-corpus JSON scans), matching `FilterTerm::Json`.
  - Keep property updates strictly **immutable/revisioned** (new `write_cursor`,
    new `_fathomdb_artifact_revisions` entry, superseding the prior version) —
    never in-place mutation.
  - **Reserve `scope TEXT NOT NULL DEFAULT ''`** on canonical and operational
    tables in schema 38 (inactive / zero-cost in `0.8.31`) so `0.8.32`
    engine-enforced caller scope lands without breaking `0.8.31` / Memex stores.
- **App-declared vector-committable kinds (Memex R1)**:
  - Lift the hardcoded 7-kind dense node allowlist (`email`, `article`, `paper`,
    `meeting`, `note`, `todo`, `doc`) so applications (Memex `mms1:*`, Hermes,
    OpenClaw, domain apps) declare their own vector-indexed node kinds.
- **Operational-collection kind readback (Memex R12)**:
  - Add SDK readback of registered operational-collection kinds and explicit
    mismatch behavior on re-registration across Rust, Python, and TypeScript.

### 2. Typed traversal predicates, opt-in fused-hit seeding, and historical graph reconstruction

- **Ad-hoc + indexed property predicates on `graph.expand` and `use_graph_arm`**:
  - Extend `GraphExpandRequestV1` to accept closed, validated typed predicates
    (`=`, `!=`, `<`, `<=`, `>`, `>=`) on **any edge or target property**
    (declared or undeclared — evaluated inline in microseconds after indexed
    `(from_id, kind)` / `(to_id, kind)` lookup, and accelerated via declared
    typed B-trees when declared for high-fanout edges).
  - Return edge properties on `GraphOriginV1` / `GraphTargetV1` and prove
    ineligible edges cannot enter a traversal path or displace eligible results.
  - Add edge-kind/edge-property filtering on `use_graph_arm` and fix the
    mixed-edge `SEED_FTS_N` slot caveat so edges touching ineligible nodes
    cannot displace eligible seed slots (Memex Graph Ask 5).
- **Opt-in fused-hit graph seeding and seed provenance (Memex Graph Asks 2 & 3)**:
  - Add opt-in `graph_seed="fused_top_k" | "graph_fts" | "both"` and
    `graph_seed_k`, plus per-hit seed provenance (seed source + hop depth) in
    explanation and evidence, letting vector/rerank relevance drive graph
    expansion over body-less property-graph edges while keeping default
    `use_graph_arm=False` zero-cost.
- **Historical graph reconstruction (External Reviewer P2 — largest functional gap)**:
  - Support **bitemporal / revision-aware graph traversal** (`as_of` /
    historical edge view mode) rather than unconditionally skipping superseded
    edges in `graph_expand/execution.rs`.
  - Reconstruct the exact graph path before and after an edge is superseded,
    including when validity time (`t_valid` / `t_invalid`) and
    revision/supersession time differ, while remaining strictly subordinate to
    physical erasure.

### 3. Strict-only write endpoint integrity (decided 2026-11-21)

- Enforce **strict referential integrity on every write path** (`engine.write`
  and `actuate`): any edge referencing a missing or inactive endpoint causes
  atomic refusal with zero partial state after reopen.
- *(Pre-1.0 intentional breaking contract change: no loose/dangling bulk mode
  ships in `0.8.31`; opt-in bulk/non-strict import lands in
  [0.8.32](../0.8.32/scope.md). Existing dangling-edge tests are updated under
  the Slice 0 ADR.)*

### 4. Memex SDK/CLI unblockers and minimal schema-37→38 upgrade helper

- **SDK evidence, hints, and capability parity (Memex Asks 1, 2, 4)**:
  - Bounded batch **`resolve_evidence_many`** under one frozen context with
    per-ref typed refusals (`get_many` partial-result style).
  - Engine **search hints** (`no_results`, `degraded`, `more_results`) across
    Python, TypeScript, and Rust on `search`, `search_frozen`, and
    `EvidenceSearchRequestV1`.
  - Explicit **`OpenReport` capability booleans** (`reranker_compiled`,
    `cuda_compiled`, `rerank_available`).
- **Minimal schema-37→38 export/upgrade helper for Memex (decided 2026-11-21)**:
  - Provide a tested, verified CLI helper to upgrade a closed `0.8.30` (schema
    37) database to `0.8.31` (schema 38+) with pre-upgrade backup and
    post-upgrade `doctor full-integrity` verification so Memex can migrate live
    `.30` stores ahead of full multi-version migration in
    [0.8.34](../0.8.34/scope.md).
- **Lockstep `fathomdb-cli` PyPI binary wheel (Memex R10)**:
  - Publish lockstep `fathomdb-cli` binary wheels on PyPI (Maturin
    `bindings = "bin"`, optionally exposed via `fathomdb[cli]`) so Python-only
    users (`pip install` / `uv tool install`) get exact-version `fathomdb
    doctor`, `recompute-mean`, `recover`, and the 37→38 helper with zero Rust
    toolchain while preserving the SDK-vs-CLI operator boundary.
- **Targeted contract doc fixes (Memex Graph Ask 1, Ask 4, Ask 5)**:
  - Fix stale `use_graph_arm` text in `docs/reference/python-api.md`, document
    PyPI wheel CUDA + cross-encoder reranker coverage in
    `docs/install/python.md`, and document mixed-`source_id`
    `Engine.write(batch)` single-transaction atomicity and zero-row rollback on
    `WriteValidationError`.

### 5. Non-negotiable invariants: erasure, metadata management, and zero latency regression

- **Erasure maintained by construction (defects only — no new erasure research
  campaigns)**:
  - Register every new derived property index table in `ROW_OWNED_PROJECTIONS`
    (keyed by `write_cursor`) so `purge` and `erase_source` clean them
    automatically.
  - Extend `physical_erasure/admission.rs` and `physical_erasure.rs` byte
    accounting and scrubbing to cover inline node/edge `properties` across both
    Autonomous and ExplicitMaintenance modes.
  - Historical graph replay never resurrects purged/erased records.
  - **Targeted W12 erasure defect/ergonomics fixes (Memex Ask 3)**:
    distinguish a completing-retry `EraseReport` from already-erased, fix BUSY
    checkpoint error diagnostics, and evaluate in Slice 0 whether
    checkpoint-only owed-closure write-fencing on unrelated writers can be
    narrowed without weakening closure durability.
  - Every new feature slice ships RED→GREEN `purge` and `erase_source` tests
    asserting zero rows, zero WAL/file remnants, and valid closure proofs.
- **Metadata management integrity**:
  - Add every new table to the **read-visibility generation and coalescing
    trigger manifest** so frozen reads and cursors always detect property
    mutations.
  - Add an automated **`sqlite_master` table-manifest completeness audit** test
    asserting every table is classified across `CANONICAL_TABLES` /
    `ROW_OWNED_PROJECTIONS`, read-visibility triggers, erasure coverage,
    `doctor`, and Inspector.
- **No latency regression**:
  - **Pay-only-when-used fast paths**: keep `0.8.30`'s `hybrid_deferred_identity`
    and streamed FTS paths free of extra JOINs when no property or history
    filter is active; keep `canonical_edges` partial active index for current
    traversal and use separate composite indexes for historical `as_of` replay.
  - Enforce `EXPLAIN QUERY PLAN` `SEARCH`-only (no `SCAN`) tests and preserve
    transactional visibility trigger coalescing.
  - Gate promotion on a **paired `v0.8.30` vs. `0.8.31` benchmark** (10k / 50k /
    100k) proving no statistically significant p50/p95 regression on unpredicated
    workloads.

### 6. Inspector brought in-line, upgraded to `0.8.31`, and hardened for fast release cycles

- **Integrate completed Inspector `0.2.1` branch onto `main` / `0.8.31`**:
  - Source: **`origin/inspector/dev/0.2.1`** at
    `d6501cc5ac9b4fd1747be28f1896f7f7670ff39a` (based on `release/0.8.30`
    `72fe93634`, zero `src/rust/` / `.cargo/config.toml` diff, `Cargo.lock`
    `+17/-0` adding only `fathomdb-inspector 0.2.1`, already verified across
    `0.8.29`/`0.8.30`, 81/81 installed tests, and `mkdocs build --strict`).
  - Upgrade Inspector from its verified `0.8.30` baseline to `0.8.31` (schema
    38+), adding UI views for **outside-`body` node/edge properties** and
    **historical graph inspection (`as_of`)**.
  - Wire Inspector versioning, build, smoke qualification, and release
    artifacts into the standard FathomDB release pipeline.
- **Inspector & monorepo CI/bootstrap hardening (owner-requested from Inspector
  `0.2.1` retrospective)**:
  1. **Decouple `src/inspector/tools/local-ci.sh` from full-monorepo
     `scripts/agent-verify.sh` when `src/rust/`, `src/python/`, and `src/ts/`
     are unchanged**: default to an Inspector-scoped source gate
     (`agent-lint.sh` + `agent-typecheck.sh` + `cargo test -p fathomdb-inspector`
     bare & `cached-cpu` + Inspector unit/tool/browser suites, ~5 min vs. ~45
     min), retaining `--full-workspace` when shared core paths change.
  2. **Fix worktree Python/TS bootstrap (`scripts/bootstrap.sh` /
     `scripts/setup-python-owned.sh`)** to preflight everything
     `agent-verify.sh` assumes upfront: prefer system `/usr/bin/python3.12`
     over `uv` standalone Python 3.13 on Ubuntu 24.04 (fixing missing
     `os.memfd_create` in `src/inspector/tools/vector-fixture.py` and late
     `libpython3.13.so.1.0` failure in `cargo test -p fathomdb-py --lib`), and
     provision checkout-owned `src/ts/node_modules` alongside root
     `node_modules` so `test_tegra_napi_build_contract.sh` and
     `test_native_artifact_runtime_validation.sh` never fail mid-gate.
  3. **Add explicit per-command timeouts** to process-spawning, GUI, and
     `strace` qualification harnesses so cleanup hangs (such as parent-wrapped
     `strace` + detached child holding stdout open after a test failure) cannot
     stall unattended runs.
  4. **Retain the Docker-free native Ubuntu 24.04 (`glibc 2.39`) qualification
     pipeline (`D4 = B`)**: keep native `Xvfb -displayfd`, `/proc` browser PID
     discovery, CDP `Browser.close`, and parent-wrapped `strace -ff` (no
     Docker, `openbox`, `xdotool`, or `sudo`).

## Out of scope / explicitly placed elsewhere

- **Moved to [0.8.32](../0.8.32/scope.md)**: engine-enforced caller scope &
  write authority, production read+write `fathomdb-mcp` (frozen evidence +
  governed writes & checkpoints), opt-in bulk/non-strict write mode, and
  [quality-backlog.md](quality-backlog.md) (backup/restore/rebuild proof,
  power-loss recovery, coverage-guided fuzzing).
- **Deferred (decided 2026-11-21)**: [cuda-delivery-plan.md](cuda-delivery-plan.md)
  (CUDA 13 + Tegra CUDA 13) deferred out of `0.8.31` (re-slot after `0.8.35` or
  dedicated release).
- **Not needed / superseded (Memex R2)**: projected-text indexing on the hybrid
  route is eliminated by outside-`body` typed node properties.
- **Deferred (Memex E3)**: upgrading `ingest_with_extractor` to full provenance
  and scope (post-`0.8.34` / `0.9.0`; documented as legacy in `0.8.33`).
- Graph query languages (Cypher, Gremlin, GQL, SPARQL) and database-owned
  abstention judgment.

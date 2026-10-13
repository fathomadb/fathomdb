---
title: Rust SDK benchmark driver design
date: 2026-10-13
target_release: post-0.8.27 experiment infrastructure
desc: Dedicated fathomdb-sdk Rust binding for cross-SDK benchmarks
blast_radius: experiments/sdk_drivers; experiment artifacts; no SDK API change
status: PROPOSED
---

# Rust SDK benchmark driver

## Contract and shipped boundary

This design implements `BT-RS-R01`–`BT-RS-R04` and `BT-RS-AC01`–
`BT-RS-AC04` in the
[competitive testing requirements](../performance-benchmarking/competitive-testing-requirements.md),
plus the [overall driver contract](sdk-performance-driver.md). It proposes an
experiment executable; it does not change the shipped SDK.

The 0.8.27 public application SDK is the dedicated
[`fathomdb-sdk` crate](../../src/rust/crates/fathomdb-sdk/Cargo.toml), not the
lower-level `fathomdb` facade. Its `Engine`, `read`, `graph`, `admin`,
`rerank`, and `embed_batch_cls` surface follows the
[Rust SDK interface](../interfaces/rust-sdk.md). Calls are synchronous and
return `Result`; optional Python keywords or TS options become typed Rust
option structs. `Error::kind()` is the shared error category. The crate's
default feature set is empty; `default-embedder`, `default-reranker`, and
device features must be enabled deliberately. A build without the reranker
can take an identity path, so feature drift is a measurement error.

## Build artifact and process

Add a non-publishable, standalone Cargo binary under
`experiments/sdk_drivers/rust`, with its own pinned `Cargo.lock` and a path
dependency on `fathomdb-sdk`. It must not join the publishable SDK crate or
expose a new public API. Compile it with an exact manifest, locked
dependencies, target triple, profile, `rustc` version, `RUSTFLAGS`, and
declared feature list. Its Cargo root repeats the reviewed workspace
`[patch.crates-io]` sources for SQLite, cudarc, and the Candle crates because
the repository root's patches do not propagate to a standalone downstream
project. Hash the final executable and retain a build manifest that binds
the SDK/engine source revision, feature resolution, and resolved source/
checksum closure for shared engine/embedder dependencies. Disallow
direct driver dependencies on `fathomdb` or `fathomdb-engine`; measured calls
must be expressible using `fathomdb_sdk` re-exports. The coordinator launches
only this pinned binary in `capabilities` or `run-repetition` mode.

Cross-SDK eligibility checks common engine **source revision**, resolved
shared engine/embedder dependency closure, and effective runtime config and
feature/model/device treatment, not equality of three different native binary
hashes. The Rust binary hash remains its own arm identity. If a Python/TS
build's effective feature set or shared dependency closure cannot be verified,
the comparison stays descriptive. A FTS-only Rust binary may omit model
features for a separate Rust diagnostic, but cannot join a three-SDK paired
cell whose other arms include a different effective feature treatment.

## Repetition lifecycle and call boundary

1. Decode one closed `sdk.driver.v1` repetition request, validate fixture and
   binary/build hashes, operation/option versions, and declared feature and
   device requirements. Configure process-wide SQLite runtime using public
   `admin::configure_runtime` before the first `Engine::open`.
2. Open the coordinator-prepared fresh database with
   `Engine::open(path, OpenOptions { config, use_default_embedder })`. The
   setup helper's Python bootstrap/doctor revision must match the Rust arm's
   engine revision. Record public `open_report()` and readiness. An explicit
   open-latency case instead uses its own registered boundary.
3. Decode neutral inputs into re-exported typed DTOs and option structs
   **before** timing. FTS invokes `engine.search_text_only(&query,
   TextSearchOptions { view, limit })`. Other bindings use `SearchOptions`,
   `SearchFilterArg`, `ProjectedTextSearchOptions`, `FrozenSearchOptions`,
   `SearchExpandOptions`, `ListOptions`, `NeighborsOptions`, and typed V1
   requests as appropriate. The registry fills every default explicitly;
   it never equates `None` with a missing measurement setting or reaches for
   a Rust-only core overload. Temporal-capable bindings pass the fixed
   effective validity instant or read context; they cannot substitute a
   wall-clock-derived default for paired results. Translate `timeout_ms` to
   `engine.drain(timeout_ms)`.
4. Capture `std::time::Instant::now()` immediately before the public SDK call
   and stop it when the owned `Result<T>` returns. This includes the SDK
   wrapper and core call. DTO construction, result normalization, oracle
   comparison, JSON encoding, and disk I/O occur outside the clock. Store
   sample nanoseconds losslessly in the external artifact, then derive the
   registered millisecond aggregates. Do not turn an `Err` into a fast success.
5. Use a new binary process per independent repetition, a new process per
   cold singleton-model sample, and reconstructed database state per mutating
   sample. Call `engine.close()` before normal exit unless close is the timed
   operation. Report close/exit witnesses. The coordinator owns timeout,
   process-group termination, and provider-descendant reaping.

The driver emits one typed repetition response and hashed external timing/
normalized-evidence artifacts. The coordinator aggregates repetitions into
one `sdk.result.v1` and safe receipt for `(suite attempt, case, rust)`.
Answer-family scoring is the same external answer pipeline used for Python
and TS, linked to this arm's retrieval evidence digest.

## Dispatch, capabilities, and failures

Use a registry keyed by `(operation_id, semantic_version)` whose bindings
invoke only public SDK methods or namespace functions. Generate/check the
operation inventory against the
[governed parity map](../../src/conformance/governed-operation-parity.json)
and classify additional common members. Each advertised operation has a typed
constructor, output normalizer, setup/readiness predicate, and conformance
fixture. A model-dependent operation is absent/blocked in a binary lacking
its required feature; `rerank` returning identity is not proof that the
reranker treatment ran. Unknown IDs/options or unrepresentable neutral values
fail before any SDK call.

Map `fathomdb_sdk::Error::kind()` and typed payloads to the neutral failure
category. Do not parse `Display` text, catch/ignore an `Err`, or use private
core access. A Rust panic is a crashed driver process, which the coordinator
records as blocked; the SDK does not promise to translate panics into the
Python/TS error hierarchy. Include only safe category and payload summaries
in committed receipts.

## Acceptance trace

| Gate | Required proof |
| --- | --- |
| `BT-RS-AC01` | Locked build/runtime attestation, reviewed patch-set and dependency-closure comparison, and no-feature refusal prove exact binary/feature identity. |
| `BT-RS-AC02` | Real fresh-database FTS/read/graph fixture plus dependency/import guard prove use of `fathomdb_sdk` only. |
| `BT-RS-AC03` | Typed option/default, filter/frozen/graph, millisecond drain, and `Instant` boundary fixtures prove translation. |
| `BT-RS-AC04` | `Error::kind`, close/mutation/cold, feature mismatch, and timeout fixtures prove blocked outcomes without false samples. |
| `BT-AC01`–`BT-AC10` | The common suite/cell, safe receipt, three-scoreboard, and answer-stage gates apply unchanged. |

Implementation under `experiments/` must follow
[`experiments/AGENTS.md`](../../experiments/AGENTS.md). This design does not
authorize a build or benchmark run.

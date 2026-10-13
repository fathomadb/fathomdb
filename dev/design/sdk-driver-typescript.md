---
title: TypeScript SDK benchmark driver design
date: 2026-10-13
target_release: post-0.8.27 experiment infrastructure
desc: Public ESM and napi TypeScript SDK binding for cross-SDK benchmarks
blast_radius: experiments/sdk_drivers; experiment artifacts; no SDK API change
status: PROPOSED
---

# TypeScript SDK benchmark driver

## Contract and shipped boundary

This design implements `BT-TS-R01`–`BT-TS-R04` and `BT-TS-AC01`–
`BT-TS-AC04` in the
[competitive testing requirements](../performance-benchmarking/competitive-testing-requirements.md),
plus the [overall driver contract](sdk-performance-driver.md). The driver is
proposed; the public TypeScript SDK is shipped.

The 0.8.27 [`fathomdb` package](../../src/ts/package.json) is ESM, declares
Node `>=25 <26`, and ships a thin JavaScript package with a separately loaded
napi `.node` platform binary. The
[binding loader](../../src/ts/src/binding.ts) can prefer a local development
binary over a platform package. Public `Engine`, `read`, `graph`, `admin`,
`rerank`, and `embedBatchCls` come from
[`index.ts`](../../src/ts/src/index.ts). Most public operations are Promises.
`Engine.searchTextOnly` takes a **flat** `SearchOptions` object: `ReadView`
fields alongside `limit`; `{ view: ..., limit: ... }` is not its public shape.
These facts must drive the binding, not a Python-signature translation by name.

## Executable and loaded-binary identity

Add a compiled ESM executable under `experiments/sdk_drivers/typescript` with
fixed `capabilities` and `run-repetition` modes for `sdk.driver.v1`. Launch a
pinned Node executable and compiled driver JS; no `ts-node`, runtime source
transpilation, arbitrary import name, or per-case command string. The
coordinator expands the suite into cells, prepares the fresh database,
launches one Node process per repetition, and aggregates the repetition
responses into one cell result/receipt.

The arm manifest pins Node executable/version, main package version and
package bytes, lockfile digest, driver JS digest, platform triple, selected
native package/version or declared local dev build, and expected `.node`
digest. Its build manifest also attests the resolved shared engine/embedder
dependency source/checksum closure for comparison with Python and Rust;
unknown provenance leaves the arm descriptive. After public package import,
inspect Node's loaded native-module
cache and the loader's expected package roots to identify the **actual**
`fathomdb.<triple>.node` file. Require exactly one matching loaded binding and
hash its bytes. A manifest path alone is insufficient because the loader can
select a nearer local binary. Missing, ambiguous, mismatched, or undeclared
local binding blocks preflight. The main/package versions must agree; an
unsupported host yields a typed blocked result. The observed runtime report
also records `process.version`, platform/arch/libc, effective model/device,
and selected binary digest. No private native method is used for a measured
operation.

## Repetition lifecycle and timing

1. Decode a closed, one-repetition request and validate IDs, fixture hashes,
   capability/option versions, safe-integer ranges, package/binding identity,
   and runtime/device policy. Call public
   `admin.configureRuntime({ sqliteMode })` before any `Engine.open`.
2. Open the prepared database through public `await Engine.open(path,
   { engineConfig, useDefaultEmbedder })`. Record `openReport()` and readiness.
   For an open/startup case, use its registered timing boundary rather than
   silently including this setup open in query latency.
3. Build the public request object outside the timer. Convert canonical
   `snake_case` field names to documented camelCase. For FTS, flatten the
   neutral read-view fields into `SearchOptions` and call
   `await engine.searchTextOnly(query, { ...readViewFields, limit })`. The
   `splitSearchOptions` implementation in
   [`search.ts`](../../src/ts/src/search.ts) removes `limit` and passes the
   remaining fields as the view. Temporal-capable calls receive a fixed
   `validAsOf` or read context from the resolved case; an omitted field's
   implicit `now` cannot qualify for paired results. Pass explicit filter, projection, graph
   bounds, and frozen context only on the operations that support them; never
   route unsupported options to a superficially similar method.
4. Capture `process.hrtime.bigint()` immediately before invoking the public
   method and after its Promise resolves to the SDK-mapped JS value. That
   includes JS wrapper validation, napi work, and Promise completion. JSON
   parse/stringify, neutral normalization, oracle scoring, answer generation,
   and file I/O are outside the clock. For a declared concurrent treatment,
   record the Promise launch schedule, per-call latency, and batch wall time
   separately; serial is the first SDK-parity treatment.
5. Await `engine.close()` in `finally` unless close is the timed operation.
   Report both close and process-exit witnesses. A cold model sample uses a
   newly launched Node process with no earlier model call. Mutating samples
   use reconstructed initial state. The coordinator reaps the full process
   group and any provider child after timeout or crash.

Return a typed repetition response with requested/observed method, effective
options, sample count/errors, native identity, timing artifact hash, and
normalized-evidence artifact hash. The coordinator validates all repetitions
and writes one `sdk.result.v1`/safe receipt for `(suite attempt, case,
typescript)`. The shared answer stage, when requested, consumes the pinned
retrieval evidence artifact bound after the upstream same-attempt cell
completes; it is not implemented in this driver.

## Type conversion, dispatch, and failures

Dispatch `(operation_id, semantic_version)` through a table of public method
bindings seeded from the
[governed parity map](../../src/conformance/governed-operation-parity.json).
Each binding constructs the public option/request shape and normalizes results
by the registry rule. All common operations receive a disposition; supported
operations need a conformance fixture. New SDK methods extend this table and
capability manifest without changing coordinator flow.

The neutral protocol represents large unsigned IDs as decimal strings. Decode
to `number` only where the public TS field is numeric and `Number.isSafeInteger`
holds; preserve documented string-valued graph u64 fields as strings. Reject
unsafe values **before** invoking an SDK method. Record the normalized ID
space plus value so a node ID is not confused with a positional cursor.
Translate documented `FathomDbError` subclasses and host `TypeError`/
`RangeError` into typed failures. Rejected Promises are awaited and counted
as errors, never as fast successful samples; log only safe error category and
non-sensitive fields in the receipt.

## Acceptance trace

| Gate | Required proof |
| --- | --- |
| `BT-TS-AC01` | Loader test attests the actually selected native binary, including local-binary precedence and unsupported/version-mismatch refusal. |
| `BT-TS-AC02` | Real fresh-database FTS/read/graph fixture and delayed-Promise clock fixture prove public dispatch, await, and output parity. |
| `BT-TS-AC03` | Flat `SearchOptions`, camelCase, filter/graph, millisecond drain, and safe-integer fixtures prove lossless translation. |
| `BT-TS-AC04` | Promise rejection, close/timeout, and cold-process fixtures produce typed blocked results and no orphan child. |
| `BT-AC01`–`BT-AC10` | The suite/cell, schema, safe receipt, three-scoreboard, and shared-answer gates apply unchanged. |

Implementation under `experiments/` must follow
[`experiments/AGENTS.md`](../../experiments/AGENTS.md). This design does not
authorize a Node/native build or a benchmark run.

---
title: Python SDK benchmark driver design
date: 2026-10-13
target_release: post-0.8.27 experiment infrastructure
desc: Public Python SDK binding for the versioned cross-SDK benchmark protocol
blast_radius: experiments/sdk_drivers; experiment artifacts; no SDK API change
status: PROPOSED
---

# Python SDK benchmark driver

## Contract and shipped boundary

This design implements the Python-specific `BT-PY-R01`–`BT-PY-R04` and
`BT-PY-AC01`–`BT-PY-AC04` in the
[competitive testing requirements](../performance-benchmarking/competitive-testing-requirements.md),
plus the [overall driver contract](sdk-performance-driver.md). It is a
proposed experiment executable, not a shipped SDK feature.

At 0.8.27, `src/python/fathomdb` exports `Engine`, `read`, `graph`, `admin`,
`rerank`, and `embed_batch_cls` from the public package root. `Engine` is a
Python wrapper over a PyO3 extension; it converts native values into public
dataclasses. Package import runs the co-installation guard **before** loading
the extension. `Engine.search_text_only` is the explicit lexical method;
`Engine.search` is a separate hybrid method. `Engine.drain` takes seconds,
while TypeScript and Rust take milliseconds. These are observed API facts from
[`engine.py`](../../src/python/fathomdb/engine.py),
[`__init__.py`](../../src/python/fathomdb/__init__.py), and the
[Python interface](../interfaces/python.md).

## Executable and identity

Add a fixed `experiments.sdk_drivers.python_driver` module invoked only by the
coordinator's pinned `sys.executable -m` command. Its stdin/request file and
stdout/response shape are `sdk.driver.v1`; the only entry modes are
`capabilities` and `run-repetition`. It has no arbitrary method,
Python-module, or shell-command field. It reads one fully resolved cell
**repetition** request. The coordinator owns database creation, repetition
aggregation, attempt ID, process group, timeout, receipt, and suite manifest.

Preflight pins the Python executable/version, environment lock or wheel
manifest, driver source digest, SDK distribution/version and installed package
path, and the loaded `_fathomdb` extension path/digest. On import, the driver
lets `fathomdb._coinstall` reject conflicting distributions. It then checks
`fathomdb.__file__` and `fathomdb._fathomdb.__file__` against the expected
installed roots and hashes. An editable or checkout-local build is eligible
only when explicitly named and fully hashed in the arm manifest. A different
import origin blocks before timing; no ad hoc `sys.path` fallback selects a
convenient wheel. Identical engine source revision, resolved shared engine/
embedder dependency closure, and effective config/features are required for
an SDK comparison, while this arm's native extension digest remains distinct
from TS and Rust binary digests. The build manifest must attest that closure
or leave the arm descriptive.

## One-cell lifecycle

1. Validate the request, capability and operation versions, fixture hashes,
   interpreter/package/extension identity, and declared device/runtime policy.
   Configure `fathomdb.admin.configure_runtime(sqlite_mode=...)` before the
   first `Engine.open` in this process. The coordinator's setup helper runs
   separately and its bootstrap/doctor engine revision must match this arm.
2. Open the fresh prepared database through public `Engine.open`, using either
   a normalized `EngineConfig` or explicit keywords, never both. Record the
   requested config and public `open_report()` resolution. If this is an open
   latency case, use the registered open boundary instead of this setup step.
3. Construct public `SearchFilter`, `Filter`, `ReadView`, typed request
   dataclasses, and fixture arguments **before** each timed call. Dispatch the
   registry ID to a public `Engine`, `read`, `graph`, `admin`, `rerank`, or
   `embed_batch_cls` member. For FTS specifically call
   `engine.search_text_only(query, view=view, limit=limit)`. Translate a
   shared integer `timeout_ms` to a representable public `timeout_s` for
   `drain`. Before timing, verify that the shipped native conversion
   `int(timeout_s * 1000)` yields exactly the requested milliseconds; choose
   a nearby representable seconds value when necessary, or block if no exact
   round-trip exists. Record requested milliseconds, passed seconds, and
   effective native milliseconds. Do not use `engine.search` as an FTS
   fallback. For temporal-capable calls, supply the fixed effective
   `valid_as_of` or read context from the resolved case; never rely on the
   public `None`/omitted view's implicit current time for paired results.
4. Take `time.perf_counter_ns()` immediately before the public call and after
   the returned SDK value is materialized. The SDK's PyO3 call and Python
   dataclass conversion stay inside this boundary. Normalize IDs, ranks,
   source mapping, and errors **after** timing. Keep JSON decoding, oracle
   comparison, file writing, and answer generation outside it.
5. Follow the overall isolation rule: a separate interpreter process per
   independent repetition; a fresh process per cold singleton-model sample;
   fresh state per mutating sample. Close the Engine in `finally` except when
   `engine.close` is the timed operation, where the timed close itself is the
   closure witness. Report close failure and process exit separately. Provider
   children, when used, must come from a pinned setup manifest and be reaped
   by the coordinator on failure.

The driver writes external per-sample timing and normalized-evidence artifacts
and returns their hashes, counts, effective options, correctness digest, and
typed status. The coordinator aggregates and validates the repetitions, then
emits one `sdk.result.v1` plus one
safe common receipt for `(suite attempt, case, python)`; answer-family scoring
occurs later in the shared answer pipeline over the pinned evidence digest.

## Operation dispatch and expansion

Use a registry of small binding functions keyed by `(operation_id,
semantic_version)`, not a long `if` statement in the coordinator. Every entry
declares its Python spelling, argument constructor, output normalizer, and
setup/readiness oracle. Seed it from all shared operations in the
[governed parity map](../../src/conformance/governed-operation-parity.json)
and classify the additional common members. A newly shared SDK operation
requires a new binding and conformance fixture before the driver advertises
support. An operation needing external models/providers is explicit
`blocked_prerequisite` until its pinned artifact and oracle are present. A
supported operation that throws is an error, not an unsupported capability.

Map public `fathomdb.errors` classes by class identity and documented payload
fields. Host `TypeError`/`ValueError` from argument validation become a typed
invalid-request outcome. The driver's own decoder errors are preflight
failures. Do not parse exception messages, suppress an exception, or count a
failed call in a successful latency distribution. Write only category,
non-sensitive payload fields, and hashes to the safe result.

## Acceptance trace

| Gate | Required proof |
| --- | --- |
| `BT-PY-AC01` | Import-origin/co-install test proves the measured wheel or explicit local build and loaded extension match the arm manifest. |
| `BT-PY-AC02` | Real fresh-database FTS/read/graph fixture proves public dispatch and ordered-result parity, including the explicit FTS call. |
| `BT-PY-AC03` | Clock and 1001 ms native-round-trip fixtures prove the public-call boundary and exact timeout translation or pre-timing refusal. |
| `BT-PY-AC04` | Error, cold, mutation, close, and blocked-receipt fixtures prove isolation and safe results. |
| `BT-AC01`–`BT-AC10` | The coordinator's suite/cell, schema, receipt, quality, answer, and renderer gates apply unchanged; Python contributes only its typed driver artifacts. |

Implementation must follow [`experiments/AGENTS.md`](../../experiments/AGENTS.md)
when files are added there. This design neither runs nor authorizes a benchmark.

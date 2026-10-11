---
title: FathomDB 0.8.27 Slice 120 - TypeScript SDK decomposition design
status: REVIEWED
target_release: 0.8.27
---

# Slice 120 - TypeScript SDK decomposition design

## Boundary

The package root remains `dist/index.js` with `dist/index.d.ts`. `package.json`
declares no supported subpaths. `index.ts` explicitly re-exports the same value
and type names, without exporting cross-module private helpers or private module
paths. `Engine` remains the one class
identity and holds its private native handle, open/close lifecycle, and public
method signatures. Public methods delegate to domain functions with the native
handle and immutable configuration; no new object hierarchy or mutable registry
is introduced. `_native` remains the existing internal escape hatch used by
`read`, `graph`, and `admin`; no additional public handle is added.

## Ownership map

| Owner | Former `index.ts` responsibility |
| --- | --- |
| `core.ts` | `Engine` class, `EngineConfig` snapshot, open, close, drain, and thin instrumentation wiring. |
| `write.ts` | write and ingest types, provenance, source dependencies, closure/actuation response validation, lifecycle transition/purge/erase, and write/ingest/consolidate adapters. |
| `projection.ts` | projection specifications, deltas and readiness/status types, configuration adapter. |
| `search.ts` | search/filter types, validation and native result/explanation mapping, ranked and frozen search adapters, shared read/frozen-context conversion, and text/projected-text routes. |
| `embedding.ts` | Standalone rerank and CLS batch embedding types/adapters, plus `Engine.embed`. |
| `evidence.ts` | search/resolve evidence types and native response mapping, dependency-trace request/response validation and evidence adapters; it may import search's result mapper and frozen-context converter. |
| `graph.ts` | graph types, request/response validation, graph evidence and traversal adapters. |
| `admin.ts` | runtime and database admin namespaces, configuration types and validation. |
| `open.ts` | open-report, device, GPU witness and embedder-event types/mapping, subscriber event types, counter snapshot data, and the dense-disabled/equivalence status adapters. |
| `instrumentation.ts` | Telemetry and feedback adapters, `validateIdArray`, counters, profiling, slow threshold and subscriber callback forwarding. |
| `native-call.ts` | Private leaf error interception helpers `intercept` and `interceptSync` for moved root code; no domain or core imports. The existing `read.ts` interceptor remains in that already-owned module. |
| Existing modules | `binding`, `errors`, `evidence-validation`, `platform`, `read`, and `validation` keep their already-established owners. |

Small shared conversion helpers live with their canonical owner and are
imported privately by another domain. Runtime dependencies run from `core`
and namespaces to domain modules, from evidence/graph to search where needed,
and from every caller to the leaf `native-call`/binding/errors/validation
modules. No domain imports `core` at runtime; it may import `Engine` only as a
type and use the existing `_native` accessor where a namespace needs a handle.
Do not move `read` or `errors` code
back into the root. Preserve lexical validation order, native call arguments,
error translation through `intercept`/`interceptSync`, and method receiver
semantics. No object spread or JSON round-trip may replace a current direct
return unless already present in the original path.

## Proof and migration

The Slice 30 comparator's TypeScript declaration/package/runtime row methods
compare an exact pre-move capture with the candidate. The older immutable
Slice 30 baseline is retained, with Slice 110's accepted subscriber signature
and event-shape deltas explicitly accounted for; it is not rewritten to force
equality. The consumer fixture compiles against the package root (the complete
supported entrypoint set), and a runtime smoke uses an installed package in
an isolated consumer. Shared wire/error fixtures and owning `src/ts/tests`
suites protect validation precedence and data shape. The pre-move run is
recorded before movement. A plausible export or delegation defect is injected
to prove the consumer fixture is not vacuous; the defect is reverted before
the first production move. Source moves proceed in small batches with compile
and focused tests. The final candidate receives code review, independent
verification, and repository-required source gates. Slice 150 retains the
integrated release qualification matrix.

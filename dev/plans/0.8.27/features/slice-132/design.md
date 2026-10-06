---
title: FathomDB 0.8.27 Slice 132 — dedicated Rust SDK draft design
status: DRAFT
target_release: 0.8.27
planning_baseline: 2f217f573
---

# Slice 132 — dedicated Rust SDK draft design

## Decision sought

Publish a distinct `fathomdb-sdk` crate, imported as `fathomdb_sdk`, as the
application-facing Rust peer of the Python `fathomdb` package and TypeScript
`fathomdb` package. The user requested an SDK with an identical surface, not a
renaming of the present `fathomdb` engine facade. The new crate owns a closed
public export map. It uses `fathomdb-engine` internally and never publicly
reexports its `Engine` or a core-only provider/operator method. The existing
`fathomdb` crate stays available while its separate disposition is decided.

This design is a draft. Before changing public code, an ADR must supersede the
`BIND-RUST` exception in
`dev/adr/ADR-0.8.0-supersede-five-verb-surface-cap.md` and update
`dev/interfaces/{rust,python,typescript}.md` plus
`docs/positions/sdk-parity.md`. The successor must define the exact signed
surface and the existing-crate migration posture.

## What exists and what must be built

| Area | At the planning baseline | Slice 132 delta |
| --- | --- | --- |
| Rust package | `fathomdb` is a thin facade that reexports `fathomdb_engine::Engine` and selected types. | Add `fathomdb-sdk` with its own `Engine` wrapper, public modules, export allowlist, package metadata, examples, and installed-consumer tests. |
| Operation governance | `governed-operation-parity.json` maps 44 Python/TypeScript operations; it has no Rust mapping and omits documented CLS batch embedding. Rust has a separate type allowlist. | Extend the signed map to all three SDKs and include CLS; govern root, `Engine`, `read`, `graph`, and `admin` membership. |
| Read and graph | Rust core has `read_*` and graph methods; Python/TypeScript expose `read.*` and `graph.*`. | Add `read` and `graph` modules with functions taking `&Engine`, preserving namespace and operation placement. |
| Admin | Python/TypeScript expose engine-attached `configure` and process-level `configure_runtime`; Rust facade currently exports process-level `configure_runtime`. | Add both `admin::configure` and `admin::configure_runtime` with matching validation and return contracts. |
| Standalone utilities | Python/TypeScript root exports `rerank` and CLS batch embedding; Rust facade has no equivalent root exports. | Add both root operations and their complete public request/result/error types. |
| Named data and errors | Rust core has many underlying types but the facade omits `SearchHit`, `IdSpace`, and the core `IdSpaceKind` discriminator; Python/TypeScript publish broad root type/error vocabularies. | Build a signed three-way type/error/config mapping and explicit Rust exports or SDK DTOs. |
| Core-only capabilities | Whole-Engine reexport exposes Rust-specific provider and operational routes. | Keep them unreachable from `fathomdb_sdk`; audit existing crate/plugin compatibility separately. |

These are source-grounded observations, not a claim that matching core method
names already prove identical behavior. Every adapter requires a signature,
default, validation, error, and result-shape comparison against shipped Python
and TypeScript behavior.

## Public surface rule

The canonical contract is a signed semantic member set with one locator per
SDK. Each member has a Rust path, Python path, TypeScript path, input schema,
default rules, output schema, error categories, and feature refusal behavior.
The SDKs expose the same application operations and publicly nameable types.
Rust uses `snake_case`, references/ownership, `Result<T, E>`, and synchronous
calls; TypeScript uses camel-case and Promises. These are only language
translations. A Rust-only application option or a missing result field is a
parity failure. Documented Python/TypeScript differences must be reconciled
before treating either one as the source of truth.

The application root is intentionally small and recognizable:

```rust
use fathomdb_sdk::{admin, graph, read, embed_batch_cls, rerank, Engine};

let engine = Engine::open("./example.fdb", Default::default())?;
let hit = read::get(&engine, "note:1", None)?;
let neighbors = graph::neighbors(&engine, "note:1", 2, Default::default(), None)?;
engine.close()?;
```

The sketch shows namespace and ownership, not final signatures. The source
audit must fix the exact open result/report, option types, read-view argument,
graph direction, and close contract before interface ratification. In
particular, current core `Engine::open` returns `OpenedEngine`; Python and
TypeScript return an `Engine` with an `open_report` accessor. The SDK may adapt
that return only if it preserves the report, validation, and error contract.

### Operation placement

- Root: `Engine`, `EngineConfig`, `rerank`, `embed_batch_cls`, and all approved
  public DTO and error names.
- `Engine`: the canonical `engine.*` set in the shared map: open/close,
  write/actuate/lifecycle/dependency, ingest/consolidate, search/frozen/evidence,
  embed, and projection configuration. Methods use the same semantic names as
  Python, with Rust `snake_case`.
- `read`: all canonical `read.*` operations, including `get`, `get_many`,
  pagination, boundary crossing, projection status/readiness, and operational
  state. Each accepts `&Engine` first, as Python/TypeScript do conceptually.
- `graph`: `expand`, `neighbors`, and `search_expand`, accepting `&Engine`
  first. Graph request/result v1 types retain exact schema version and field
  semantics.
- `admin`: engine-attached `configure` and process-level
  `configure_runtime`.

This placement is part of parity. A core method such as `read_get` cannot
substitute for the SDK's `read::get`. No `Deref<Target = fathomdb_engine::Engine>`,
public `inner()`, wholesale `pub use`, or public core `Engine` field may bypass
the export map. Shared immutable DTO types may be selectively reexported only
after their fields and constructors pass the three-way contract review.

### Requests, results, errors, and configuration

The inventory starts from the Python package root and stub, the TypeScript
package root and declarations, and the current Rust engine/schema sources. It
must list every public request/result type, enum or tagged union, config field,
error family, and exported utility. For each:

- Match identity carriers, optional versus nullable fields, integer range,
  ordering, pagination cursor meaning, timestamps, schema versions, and
  serialized names. `SearchHit`, `IdSpace`, and the Rust representation of its
  space discriminator are explicit first-pass checks.
- Match validation order and error category, including closed-engine behavior,
  feature refusal, malformed frozen/evidence/graph requests, and invalid
  configuration. Rust `Result` variants may wrap core errors but must expose
  the same stable reason and field-path information where bindings do.
- Match defaults and accepted ranges for open/configuration, search/read views,
  filters, graph limits, runtime SQLite mode, and embedder settings. Do not
  invent a Rust-only config knob in the SDK.
- Preserve root `rerank` passage identities, ordering, scores, validation,
  default options, and refusal when unavailable. Preserve root CLS batch
  embedding cardinality, order, normalization, empty input, pooling mode, and
  refusal. Engine `embed` remains the distinct mean-pooled operation.
- Preserve `close` as an idempotent lifecycle boundary, including prompt
  release of the engine's embedder resources; an SDK wrapper must not retain a
  second owning reference after a successful close.

When the core type is too broad or its representation diverges, create a
private conversion to an SDK-owned DTO. Do not leak a core-only enum variant
through an otherwise approved type. External extraction/consolidation
callbacks need a separate three-way signature and error-propagation review.

## Internal architecture

`fathomdb-sdk` depends on `fathomdb-engine` and schema/embedder crates only as
needed internally. A private `Engine` field owns the core engine. Small
concern modules implement root engine methods, `read`, `graph`, `admin`,
embedding, and error/DTO conversion. Public `lib.rs` explicitly exports the
approved names. The SDK does not introduce a second database runtime or
duplicate database algorithms; adapters validate and translate at the boundary
then call the existing core. A mismatch that needs a core fix is tested and
fixed in the core before the SDK adapter is declared equivalent.

Features are also governed. The default feature set must match the default
model behavior of the other SDKs. Feature-disabled rerank/CLS/embed operations
return the same documented refusal category; they do not vanish from the
public type surface. No SDK feature forwards `operator`, custom-provider
injection, raw SQL, test hooks, or benchmark-only routes. Existing
`fathomdb`, `fathomdb-engine`, and `fathomdb-embedder-api` remain separate
published contracts pending an explicit compatibility decision.

## Verification design

1. Capture an exact installed-package inventory for Python, TypeScript, and
   current Rust at the baseline SHA. Expand the operation map with Rust
   locators and CLS. Add a signed type/error/config inventory; record any
   Python/TypeScript disagreements as decisions, not silent translations.
2. Add failing Rust consumer compile tests for root imports, every namespace
   call, all public DTO/error names, and package feature combinations. Add
   compile-fail tests that attempt to reach a core-only provider, recovery, or
   raw-SQL route through `fathomdb_sdk`.
3. Add human-authored shared behavior fixtures through installed SDKs, using
   a real database for integration tests. Cover defaults, validation order,
   result shape, not-found behavior, lifecycle/close, evidence/graph, rerank,
   CLS embedding, and feature refusal.
4. Make the parity checker fail closed on missing, extra, renamed, or
   shape-incompatible signed members. Adversarially mutate one Rust export,
   one extra operation, and one result/error shape to prove the checker fails.
   Keep the current Python/TypeScript oracle until its successor passes.
5. Run scoped checks during development, then strict full
   `./scripts/agent-verify.sh`, installed crate/wheel/npm consumer checks, and
   independent code/contract review on one candidate SHA. Record receipts in
   the Slice 132 status before advancing to Slice 135.

## Open design decisions

- Confirm the separate `fathomdb-sdk` package name and how the existing
  `fathomdb` crate is described or migrated for Rust users. The separate crate
  is the recommended design because it gives the SDK a closed public boundary.
- Disposition the published engine/embedder plugin contract after checking
  actual external use. This is distinct from the new SDK's parity contract.
- Reconcile any Python/TypeScript discrepancies exposed by the full signed
  inventory. The design does not authorize silently removing an existing
  binding capability to make the map easier to satisfy.

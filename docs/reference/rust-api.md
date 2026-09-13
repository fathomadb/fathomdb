# Rust API

Crate: `fathomdb` 0.8.25. The facade re-exports the supported application
surface from `fathomdb-engine`; generated item-level documentation is published
at [`docs.rs/fathomdb`](https://docs.rs/fathomdb/0.8.25/fathomdb/). The locked
[Rust interface](https://github.com/fathomadb/fathomdb/blob/main/dev/interfaces/rust.md)
owns the contract and feature-gating details.

## Surface model

Rust is not namespace-identical to the Python and TypeScript SDKs. Application
operations are inherent `Engine` methods and public carriers are facade
re-exports. The default feature set contains no recovery-named or raw-SQL
method. The `operator` feature enables the diagnostic and recovery seam used by
`fathomdb-cli`.

The established core remains `Engine::open`, `Engine::write`,
`Engine::search`, `Engine::close`, and schema configuration. `PreparedWrite`
is non-exhaustive; downstream matches require a wildcard arm.

## Process-start runtime configuration

Call `fathomdb::admin::configure_runtime(RuntimeSqliteMode)` before the first
Engine opens. `RuntimeSqliteMode` is `Performance` or `Diagnostics`; success
returns `RuntimeConfiguration`. Identical calls are idempotent, while late or
conflicting calls return `RuntimeConfigurationError`. With no explicit call,
the first open selects performance.

## 0.8.25 data-plane additions

| Capability | Engine method or facade entry point | Principal result |
| ---------- | ----------------------------------- | ---------------- |
| Atomic caller-decided batch | `Engine::actuate` | `ActuationReceiptV1` |
| Source-dependency registration | `Engine::register_source_dependency` | `SourceDependencyV1` |
| Dependencies for a source | `Engine::dependencies_for_source` | `DependencyListV1` |
| Dependency for a derived revision | `Engine::dependency_for_derived` | `Option<SourceDependencyV1>` |
| Keyed closure status | `Engine::read_dependency_closure` | `Option<ClosureStatusV1>` |
| Mint frozen authority | `Engine::freeze_read_context` | `FrozenReadContextV1` |
| Frozen retrieval | `Engine::search_frozen` | `SearchResult` |
| Frozen search plus expansion | `Engine::search_expand_frozen` | `SearchExpandResult` |
| Evidence-bearing retrieval | `Engine::search_with_evidence` | `EvidenceSearchResultV1` |
| Exact evidence resolution | `Engine::resolve_evidence` | `ResolvedEvidenceV1` |
| Dependency trace | `Engine::trace_dependency` | `DependencyTraceResultV1` |
| Canonical pagination | `Engine::read_canonical_page` | `PageV1<NodeRecord>` |
| Operational-state point read | `Engine::read_operational_state` | `Option<OperationalStateRecordV1>` |
| Operational-state pagination | `Engine::read_operational_state_page` | `PageV1<OperationalStateRecordV1>` |
| Projection-generation status | `Engine::read_projection_generation_status` | `ProjectionGenerationStatusV1` |
| Receipt-keyed mutation readiness | `Engine::read_mutation_projection_status` | `MutationProjectionStatusV1` |
| Constrained graph expansion | `Engine::graph_expand` | `GraphExpandResultV1` |

Versioned provenance is carried by `PreparedWrite::ProvenancedNode` and
`PreparedWrite::ProvenancedEdge`, using `WriteProvenanceV1`, immutable revision
IDs, `CanonicalHash`, and exact `SourceLocator` values. The legacy write
variants remain available.

The frozen-read, evidence, trace, page, projection-generation, graph-expansion,
dependency, closure, and actuation families are closed version-1 contracts.
Rust uses `u64` for native counters and boundaries; Python and TypeScript
serialize the corresponding values as canonical decimal strings.

Explained `search_frozen` and `search_with_evidence` results carry finalized,
non-empty correlation identities. See
[Frozen evidence](../guides/frozen-evidence.md) for the canonical evidence flow
and retry rules.

## Errors

Methods return `EngineError` or `EngineOpenError` and preserve the typed
sub-errors re-exported by the facade, including `ProvenanceError`,
`DependencyError`, `DependencyClosureError`, `ActuationError`,
`FrozenReadError`, `ProjectionGenerationError`, `PageError`, `EvidenceErrorV1`,
`DependencyTraceErrorV1`, and `GraphExpansionErrorV1`. Match non-exhaustive
engine errors with a wildcard arm.

## Operator boundary

Enable the `operator` feature only for diagnostics or recovery. It exposes
report types and methods used by `fathomdb doctor` and `fathomdb recover`,
including the 0.8.25 data-plane-integrity report. Python and TypeScript do not
expose an SDK equivalent of the doctor or recovery surface.

## See also

- [Install — Rust](../install/rust.md)
- [Python API](python-api.md)
- [TypeScript API](typescript-api.md)
- [CLI](cli.md)
- [Errors](errors.md)

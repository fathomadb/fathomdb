# SDK operation map

Generated from tracked source; do not edit this page by hand.
Regenerate with `python3 scripts/generate-public-docs.py --write`.

Sources: [src/conformance/governed-operation-parity.json](https://github.com/fathomadb/fathomdb/blob/main/src/conformance/governed-operation-parity.json).

This operation index lists governed names and owners. It does not specify
signatures, options, return shapes, or error behavior. Use the maintained
[Python API](../reference/python-api.md),
[TypeScript API](../reference/typescript-api.md), and
[Rust SDK reference](../reference/rust-sdk.md) for those contracts.

Governed operations: **44**. Each language cell is
`locator` / `spelling`; `engine_static` is on `Engine`,
`engine_instance` is on an open handle, `package` is standalone,
and `read`, `graph`, and `admin` name their respective surfaces.

| Operation | State | Python | TypeScript | Rust |
| --- | --- | --- | --- | --- |
| `engine.open` | live | `engine_static` / `open` | `engine_static` / `open` | `engine_static` / `open` |
| `admin.configure` | live | `admin` / `configure` | `admin` / `configure` | `admin` / `configure` |
| `engine.write` | live | `engine_instance` / `write` | `engine_instance` / `write` | `engine_instance` / `write` |
| `engine.actuate` | live | `engine_instance` / `actuate` | `engine_instance` / `actuate` | `engine_instance` / `actuate` |
| `engine.register_source_dependency` | live | `engine_instance` / `register_source_dependency` | `engine_instance` / `registerSourceDependency` | `engine_instance` / `register_source_dependency` |
| `engine.dependencies_for_source` | live | `engine_instance` / `dependencies_for_source` | `engine_instance` / `dependenciesForSource` | `engine_instance` / `dependencies_for_source` |
| `engine.dependency_for_derived` | live | `engine_instance` / `dependency_for_derived` | `engine_instance` / `dependencyForDerived` | `engine_instance` / `dependency_for_derived` |
| `engine.read_dependency_closure` | live | `engine_instance` / `read_dependency_closure` | `engine_instance` / `readDependencyClosure` | `engine_instance` / `read_dependency_closure` |
| `engine.transition` | live | `engine_instance` / `transition` | `engine_instance` / `transition` | `engine_instance` / `transition` |
| `engine.purge` | live | `engine_instance` / `purge` | `engine_instance` / `purge` | `engine_instance` / `purge` |
| `engine.erase_source` | live | `engine_instance` / `erase_source` | `engine_instance` / `eraseSource` | `engine_instance` / `erase_source` |
| `engine.search` | live | `engine_instance` / `search` | `engine_instance` / `search` | `engine_instance` / `search` |
| `engine.freeze_read_context` | live | `engine_instance` / `freeze_read_context` | `engine_instance` / `freezeReadContext` | `engine_instance` / `freeze_read_context` |
| `engine.search_frozen` | live | `engine_instance` / `search_frozen` | `engine_instance` / `searchFrozen` | `engine_instance` / `search_frozen` |
| `engine.search_expand_frozen` | live | `engine_instance` / `search_expand_frozen` | `engine_instance` / `searchExpandFrozen` | `engine_instance` / `search_expand_frozen` |
| `engine.search_text_only` | live | `engine_instance` / `search_text_only` | `engine_instance` / `searchTextOnly` | `engine_instance` / `search_text_only` |
| `engine.search_projected_text` | live | `engine_instance` / `search_projected_text` | `engine_instance` / `searchProjectedText` | `engine_instance` / `search_projected_text` |
| `engine.search_with_evidence` | live | `engine_instance` / `search_with_evidence` | `engine_instance` / `searchWithEvidence` | `engine_instance` / `search_with_evidence` |
| `engine.resolve_evidence` | live | `engine_instance` / `resolve_evidence` | `engine_instance` / `resolveEvidence` | `engine_instance` / `resolve_evidence` |
| `engine.resolve_graph_evidence` | live | `engine_instance` / `resolve_graph_evidence` | `engine_instance` / `resolveGraphEvidence` | `engine_instance` / `resolve_graph_evidence` |
| `engine.trace_dependency` | live | `engine_instance` / `trace_dependency` | `engine_instance` / `traceDependency` | `engine_instance` / `trace_dependency` |
| `engine.close` | live | `engine_instance` / `close` | `engine_instance` / `close` | `engine_instance` / `close` |
| `read.get` | live | `read` / `get` | `read` / `get` | `read` / `get` |
| `read.get_many` | live | `read` / `get_many` | `read` / `getMany` | `read` / `get_many` |
| `read.collection` | live | `read` / `collection` | `read` / `collection` | `read` / `collection` |
| `read.mutations` | live | `read` / `mutations` | `read` / `mutations` | `read` / `mutations` |
| `read.list` | live | `read` / `list` | `read` / `list` | `read` / `list` |
| `engine.ingest_with_extractor` | live | `engine_instance` / `ingest_with_extractor` | `engine_instance` / `ingestWithExtractor` | `engine_instance` / `ingest_with_extractor` |
| `engine.consolidate_with_provider` | live | `engine_instance` / `consolidate_with_provider` | `engine_instance` / `consolidateWithProvider` | `engine_instance` / `consolidate_with_provider` |
| `graph.expand` | live | `graph` / `expand` | `graph` / `expand` | `graph` / `expand` |
| `graph.neighbors` | live | `graph` / `neighbors` | `graph` / `neighbors` | `graph` / `neighbors` |
| `graph.search_expand` | live | `graph` / `search_expand` | `graph` / `searchExpand` | `graph` / `search_expand` |
| `rerank` | live | `package` / `rerank` | `package` / `rerank` | `package` / `rerank` |
| `engine.embed` | live | `engine_instance` / `embed` | `engine_instance` / `embed` | `engine_instance` / `embed` |
| `read.crossed_boundary_since` | live | `read` / `crossed_boundary_since` | `read` / `crossedBoundarySince` | `read` / `crossed_boundary_since` |
| `engine.configure_projections` | live | `engine_instance` / `configure_projections` | `engine_instance` / `configureProjections` | `engine_instance` / `configure_projections` |
| `read.projections` | live | `read` / `projections` | `read` / `projections` | `read` / `projections` |
| `read.projection_status` | live | `read` / `projection_status` | `read` / `projectionStatus` | `read` / `projection_status` |
| `read.embedding_readiness` | live | `read` / `embedding_readiness` | `read` / `embeddingReadiness` | `read` / `embedding_readiness` |
| `read.projection_generation_status` | live | `read` / `projection_generation_status` | `read` / `projectionGenerationStatus` | `read` / `projection_generation_status` |
| `read.mutation_projection_status` | live | `read` / `mutation_projection_status` | `read` / `mutationProjectionStatus` | `read` / `mutation_projection_status` |
| `read.canonical_page` | live | `read` / `canonical_page` | `read` / `canonicalPage` | `read` / `canonical_page` |
| `read.operational_state` | live | `read` / `operational_state` | `read` / `operationalState` | `read` / `operational_state` |
| `read.operational_state_page` | live | `read` / `operational_state_page` | `read` / `operationalStatePage` | `read` / `operational_state_page` |

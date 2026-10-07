//! The `graph` namespace: bounded expansion, neighbour traversal and
//! search-then-expand. Each function takes the [`Engine`] first.

use fathomdb_engine::{
    GraphExpandRequestV1, GraphExpandResultV1, NodeRecord, SearchExpandResult, SearchFilter,
};

use crate::error::{Error, Result};
use crate::guard;
use crate::options::{NeighborsOptions, SearchExpandOptions};
use crate::Engine;

/// Bounded graph expansion from explicit or query seeds.
pub fn expand(engine: &Engine, request: &GraphExpandRequestV1) -> Result<GraphExpandResultV1> {
    Ok(engine.core().graph_expand(request)?)
}

/// Records reachable from `logical_id` within `depth` hops.
pub fn neighbors(
    engine: &Engine,
    logical_id: &str,
    depth: u32,
    options: NeighborsOptions,
) -> Result<Vec<NodeRecord>> {
    guard::text(logical_id)?;
    let view = options.view.unwrap_or_default();
    Ok(engine.core().graph_neighbors(logical_id, depth, options.direction, &view)?)
}

/// Search, then expand each hit's neighbourhood by `depth` hops.
///
/// `filter.attributes` must be empty: neither the Python nor the TypeScript
/// SDK forwards attribute predicates on this path.
pub fn search_expand(
    engine: &Engine,
    query: &str,
    depth: u32,
    filter: Option<SearchFilter>,
    options: SearchExpandOptions,
) -> Result<SearchExpandResult> {
    guard::text(query)?;
    if let Some(filter) = &filter {
        if !filter.attributes.is_empty() {
            return Err(Error::invalid_argument(
                "graph.search_expand does not accept filter attributes",
            ));
        }
        guard::search_filter(filter)?;
    }
    Ok(engine.core().search_expand_with_limit(query, filter, depth, options.search_limit)?)
}

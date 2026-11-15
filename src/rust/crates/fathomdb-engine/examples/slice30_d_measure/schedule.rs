//! Ordinary D scheduling seam; observations and scientific admission are separate.
use fathomdb_embedder_api::Embedder;
use fathomdb_engine::{
    Engine, EngineConfig, ProjectionRole, ProjectionSpec, ProjectionVector, TestEmbedderChoice,
};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Held,
    Overlap,
}

pub(crate) struct OrdinaryCell<'a> {
    pub(crate) path: &'a Path,
    pub(crate) mode: Mode,
    pub(crate) queries: &'a [String],
    pub(crate) origin: Instant,
    pub(crate) deadline: Instant,
}

pub(crate) fn run_ordinary(
    cell: OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String> + Send,
    provider: Arc<dyn Embedder>,
) -> Result<Value, String> {
    let _ = (cell.mode, cell.queries, cell.origin, cell.deadline, bodies);
    let engine = Engine::open_with_test_choice_and_config_for_test(
        cell.path,
        TestEmbedderChoice::Caller(provider),
        EngineConfig::default(),
    )
    .map_err(|error| format!("{error:?}"))?
    .engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "doc".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector::default()),
                source: None,
            }],
            &[],
        )
        .map_err(|error| format!("{error:?}"))?;
    engine.close().map_err(|error| format!("{error:?}"))?;
    Ok(json!({
        "complete":false,"state":"not_run","reason":"ordinary D schedule is not implemented",
        "calls":[],"batches":[],"probes":[],"held":null,"final":null,
        "release_ns":null,"drain":null,"writer":null,"reader":null,
        "runtime":null,"close_succeeded":true,"closed_ns":null,
        "collection_enabled":false,"periodic_numeric_polls":0,"periodic_readiness_polls":0,
        "actual_overlap":"ACTUAL_OVERLAP_UNOBSERVED"
    }))
}

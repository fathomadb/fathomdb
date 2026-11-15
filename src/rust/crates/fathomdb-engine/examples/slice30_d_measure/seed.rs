//! Governed seeding seam; scheduling/drain/close stay with the owning driver.
use fathomdb_engine::{Engine, EngineError};

#[derive(Debug, serde::Serialize)]
pub(crate) struct SeedReport {
    pub(crate) committed_batches: Vec<usize>,
}

pub(crate) fn seed_rows(
    _engine: &Engine,
    _bodies: impl Iterator<Item = String>,
) -> Result<SeedReport, EngineError> {
    Ok(SeedReport { committed_batches: Vec::new() })
}

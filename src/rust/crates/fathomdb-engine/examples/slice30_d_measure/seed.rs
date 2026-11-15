//! Governed seeding seam; scheduling/drain/close stay with the owning driver.
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};

const BATCH_SIZE: usize = 1000;

#[derive(Debug, serde::Serialize)]
pub(crate) struct SeedReport {
    pub(crate) committed_batches: Vec<usize>,
}

pub(crate) fn seed_rows(
    engine: &Engine,
    mut bodies: impl Iterator<Item = String>,
) -> Result<SeedReport, EngineError> {
    let source_id = SourceId::new("test:fixture").expect("fixed synthetic source id");
    let mut report = SeedReport { committed_batches: Vec::new() };
    loop {
        let batch = bodies
            .by_ref()
            .take(BATCH_SIZE)
            .map(|body| PreparedWrite::Node {
                kind: "doc".into(),
                body,
                source_id: source_id.clone(),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        if batch.is_empty() {
            return Ok(report);
        }
        engine.write(&batch)?;
        report.committed_batches.push(batch.len());
    }
}

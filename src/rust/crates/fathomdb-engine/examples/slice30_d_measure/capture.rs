//! C capture entry seam; stream custody is implemented only after its semantic RED.
use super::schedule::{self, OrdinaryCell};
use fathomdb_embedder_api::Embedder;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

pub(crate) fn run_characterized(
    cell: OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String> + Send,
    provider: Arc<dyn Embedder>,
    identity: &Value,
    output: &Path,
) -> Result<Value, String> {
    let _ = output;
    let mut report = schedule::run_ordinary(cell, bodies, provider)?;
    report["cell"] = identity.clone();
    report["complete"] = Value::Bool(false);
    report["observation"] = Value::Null;
    report["reason"] = "characterization capture is not implemented".into();
    Ok(report)
}

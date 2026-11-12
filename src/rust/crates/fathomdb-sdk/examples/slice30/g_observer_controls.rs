//! Bounded Core error controls; source errors and unavailable reader routes stay explicit.

use super::{CoreObserver, GAdapter};
use fathomdb_engine::{
    EngineError, FrozenReadErrorReason, InitialState, PreparedWrite, ReadContextV1, ReadView,
    SearchFilter, SourceId,
};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Instant;

pub(super) fn run(config: &Value, out: &Path, store: &Path, _observer: Option<CoreObserver>) {
    let adapter = GAdapter::open("engine_hybrid", store);
    let report = adapter.report();
    assert_eq!(
        report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid.as_deref(),
        config["gpu_uuid"].as_str()
    );
    assert_eq!(report.default_embedder.dimension, 384);
    let GAdapter::Core(opened) = &adapter else { unreachable!() };
    let engine = &opened.engine;
    let started = Instant::now();
    let mut filter = SearchFilter::default();
    filter.attributes.push(("not_declared".into(), "alice".into()));
    let invalid = engine.search_filtered("alphaonly", Some(filter));
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let error =
        invalid.as_ref().expect_err("ordinary undeclared attribute must be typed InvalidFilter");
    assert!(matches!(error, EngineError::InvalidFilter { .. }));
    let invalid_row = json!({"operation":"invalid_filter","ok":false,"error_class":"InvalidFilter",
        "raw_error":format!("{error:?}"),"elapsed_ns":elapsed_ns});
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(100), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "observercontrolonly".into(),
            source_id: SourceId::new("test:slice30-observer-control").unwrap(),
            logical_id: Some("observer-context-change".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.drain(5000).unwrap();
    let started = Instant::now();
    let stale = engine.search_frozen("alphaonly", &context, 0, false, 0.3, 0, false, 10);
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let error = stale.as_ref().expect_err("governed commit must stale the frozen context");
    assert!(
        matches!(error, EngineError::FrozenRead(error) if error.reason == FrozenReadErrorReason::StateDrifted)
    );
    let frozen_row = json!({"operation":"stale_frozen","ok":false,"error_class":"FrozenRead",
        "raw_error":format!("{error:?}"),"elapsed_ns":elapsed_ns});
    adapter.close();
    let proof = json!({"controls":[invalid_row,frozen_row],"selected_cuda_uuid":report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid,
        "scope":"actual Core error APIs and argument construction; freeze/write/drain/observer transport excluded"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

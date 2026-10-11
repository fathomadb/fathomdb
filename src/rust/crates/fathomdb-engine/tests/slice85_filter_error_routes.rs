//! Slice 85 moved the snapshot filter error behind narrow per-owner mappings.
//! One undeclared filter attribute must keep its typed outcome on every route:
//! the frozen mint and search raise `InvalidFilter` with the validator's exact
//! reason, and graph expansion reports an invalid context.

use fathomdb_engine::{
    Engine, EngineError, GraphExpandRequestV1, GraphExpansionErrorReasonV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, InitialState, PreparedWrite, ReadContextV1, ReadView, SearchFilter,
    SourceId, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const UNDECLARED_REASON: &str = "filter attribute \"slice85_undeclared\" is not a declared \
     `filterable` projection; declare it via configure_projections before filtering on it";

fn undeclared_filter() -> SearchFilter {
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("slice85_undeclared".to_string(), "x".to_string())];
    filter
}

fn open(dir: &TempDir) -> fathomdb_engine::OpenedEngine {
    let opened = Engine::open(dir.path().join(format!("slice85-routes{SQLITE_SUFFIX}"))).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("root".into()),
            kind: "doc".into(),
            body: "slice85 routed needle".into(),
            source_id: SourceId::new("test:slice85").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.drain(5_000).unwrap();
    opened
}

fn invalid_filter_reason(result: Result<impl Sized, EngineError>) -> String {
    match result {
        Err(EngineError::InvalidFilter { reason }) => reason,
        Err(other) => panic!("expected InvalidFilter, got {other:?}"),
        Ok(_) => panic!("an undeclared filter attribute must be rejected"),
    }
}

#[test]
fn frozen_mint_and_search_share_the_validator_reason() {
    let dir = TempDir::new().unwrap();
    let opened = open(&dir);
    let context = ReadContextV1::new(ReadView::default(), undeclared_filter()).unwrap();
    assert_eq!(
        invalid_filter_reason(opened.engine.freeze_read_context(&context)),
        UNDECLARED_REASON
    );
    assert_eq!(
        invalid_filter_reason(opened.engine.search_filtered("needle", Some(undeclared_filter()))),
        UNDECLARED_REASON
    );
}

#[test]
fn graph_expansion_reports_an_undeclared_attribute_as_an_invalid_context() {
    let dir = TempDir::new().unwrap();
    let opened = open(&dir);
    let request = |eligibility: SearchFilter| GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), eligibility).unwrap(),
        },
        max_depth: 1,
        result_limit: 10,
        max_work_units: 1_000,
        include_explanation: false,
        include_evidence: false,
    };
    opened.engine.graph_expand(&request(SearchFilter::default())).expect("unfiltered control");
    match opened.engine.graph_expand(&request(undeclared_filter())) {
        Err(EngineError::GraphExpansion(error)) => {
            assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphContextInvalid);
            assert_eq!(error.field_path, "/context");
        }
        other => panic!("expected a graph context error, got {other:?}"),
    }
}

use std::collections::BTreeSet;

use fathomdb_engine::{
    Engine, EngineError, FrozenReadErrorReason, ProjectionRole, ProjectionSpec, ReadContextV1,
    ReadView, SearchFilter,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[test]
fn canonical_attribute_order_has_one_authenticated_encoding() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("ordering{SQLITE_SUFFIX}"))).unwrap();
    let roles = BTreeSet::from([ProjectionRole::Filterable]);
    opened
        .engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "a".to_string(),
                    roles: roles.clone(),
                    fts: None,
                    vector: None,
                    source: None,
                },
                ProjectionSpec {
                    name: "z".to_string(),
                    roles,
                    fts: None,
                    vector: None,
                    source: None,
                },
            ],
            &[],
        )
        .unwrap();

    let make = |attributes| {
        let mut eligibility = SearchFilter::default();
        eligibility.attributes = attributes;
        ReadContextV1::new(
            ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
            eligibility,
        )
        .unwrap()
    };
    let forward = opened
        .engine
        .freeze_read_context(&make(vec![
            ("a".to_string(), "first".to_string()),
            ("z".to_string(), "last".to_string()),
        ]))
        .unwrap();
    let reverse = opened
        .engine
        .freeze_read_context(&make(vec![
            ("z".to_string(), "last".to_string()),
            ("a".to_string(), "first".to_string()),
        ]))
        .unwrap();

    assert_eq!(forward.token, reverse.token);
}

#[test]
fn mint_rejects_undeclared_and_oversized_eligibility() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("bounds{SQLITE_SUFFIX}"))).unwrap();
    let mut undeclared = SearchFilter::default();
    undeclared.attributes = vec![("missing".to_string(), "value".to_string())];
    let undeclared = ReadContextV1::new(ReadView::default(), undeclared).unwrap();
    assert!(matches!(
        opened.engine.freeze_read_context(&undeclared),
        Err(EngineError::InvalidFilter { .. })
    ));

    let mut oversized = SearchFilter::default();
    oversized.attributes = vec![("field".to_string(), "x".repeat(65_536))];
    assert!(matches!(
        ReadContextV1::new(ReadView::default(), oversized),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::ContextInvalid
    ));
}

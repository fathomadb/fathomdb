//! Slice30 engine/session/operation ownership on real stores; no model workload.
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use std::panic::{catch_unwind, AssertUnwindSafe};

fn seed(path: &std::path::Path) -> Engine {
    let opened = Engine::open(path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "literal needle".into(),
            logical_id: Some("alpha".into()),
            source_id: SourceId::new("test:slice30-owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine
}

#[test]
fn observation_is_owned_bounded_and_transported_to_the_actual_reader() {
    let directory = tempfile::tempdir().unwrap();
    let a = seed(&directory.path().join("a.sqlite"));
    let b = seed(&directory.path().join("b.sqlite"));
    let guard = a.begin_slice30_observation_for_test(32).unwrap();
    guard.with_operation(41, || {
        assert_eq!(a.search_text_only("needle").unwrap().results[0].id.value, "alpha")
    });
    guard.with_operation(42, || {
        assert_eq!(b.search_text_only("needle").unwrap().results[0].id.value, "alpha")
    });
    let first = guard.take();
    assert!(
        first.records.iter().any(|r| r.phase == "api_text"),
        "actual engine A API interval is missing"
    );
    assert!(
        first.records.iter().any(|r| r.phase == "reader_transaction"),
        "reader must receive the actual operation token"
    );
    assert!(first.records.iter().all(|r| r.operation_id == 41));
    assert_eq!(first.dropped_records, 0);
    assert!(first.session_sequence > 0);
    assert!(matches!(
        a.begin_slice30_observation_for_test(2),
        Err(EngineError::InvalidArgument { .. })
    ));
    assert!(catch_unwind(AssertUnwindSafe(|| guard.with_operation(7, || {
        a.search_text_only("needle").unwrap();
        panic!("contained observation unwind");
    })))
    .is_err());
    a.search_text_only("needle").unwrap();
    guard.with_operation(8, || {
        a.search_text_only("needle").unwrap();
    });
    let after = guard.take();
    assert!(after.records.iter().any(|r| r.operation_id == 7));
    assert!(after.records.iter().any(|r| r.operation_id == 8));
    assert!(after.records.iter().all(|r| r.operation_id == 7 || r.operation_id == 8));
    drop(guard);
    assert!(matches!(
        a.begin_slice30_observation_for_test(0),
        Err(EngineError::InvalidArgument { .. })
    ));
    assert!(matches!(
        a.begin_slice30_observation_for_test(32769),
        Err(EngineError::InvalidArgument { .. })
    ));
    let bounded = a.begin_slice30_observation_for_test(2).unwrap();
    for id in 0..5 {
        bounded.with_operation(id, || {
            a.search_text_only("needle").unwrap();
        });
    }
    let batch = bounded.take();
    assert_eq!(batch.records.len(), 2);
    assert!(batch.dropped_records > 0);
    a.close().unwrap();
    drop(bounded);
    assert!(matches!(a.begin_slice30_observation_for_test(2), Err(EngineError::Closing)));
    b.close().unwrap();
}

use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};
use std::thread;

use fathomdb_engine::{
    arm_frozen_after_validation_hook_for_test, Engine, EngineError, InitialState, LifecycleState,
    PreparedWrite, ProjectionRole, ProjectionSpec, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Lifecycle,
    Erasure,
    DependencyClosure,
    Projection,
}

fn node() -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some("root".to_string()),
        kind: "doc".to_string(),
        body: r#"{"owner":"alice","text":"unique frozen needle"}"#.to_string(),
        source_id: SourceId::new("test:slice35-after-validation").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn mutate(engine: &Engine, mutation: Mutation) {
    match mutation {
        Mutation::Lifecycle => {
            engine.transition("root", LifecycleState::Deleted, None).unwrap();
        }
        Mutation::Erasure => {
            assert!(matches!(
                engine.erase_source("test:slice35-after-validation"),
                Err(EngineError::ErasureIncomplete { stage, .. })
                    if stage == "wal_checkpoint"
            ));
        }
        Mutation::DependencyClosure => {
            engine
                .execute_for_test(
                    "INSERT INTO _fathomdb_dependency_closures(
                       schema_version,closure_operation_id,root_kind,root_value,cause,
                       effective_at_epoch_s,admitted_write_boundary,
                       admitted_dependency_generation,closure_sequence,retry_fingerprint,
                       phase,affected_count
                     ) VALUES(
                       1,'slice35-race','source_revision','source-r1','soft_deleted',
                       0,0,0,1,
                       '0000000000000000000000000000000000000000000000000000000000000000',
                       'proving',1
                     )",
                )
                .unwrap();
        }
        Mutation::Projection => {
            let roles = BTreeSet::from([ProjectionRole::Filterable]);
            engine
                .configure_projections(
                    &[ProjectionSpec {
                        name: "owner".to_string(),
                        roles,
                        fts: None,
                        vector: None,
                        source: None,
                    }],
                    &[],
                )
                .unwrap();
        }
    }
}

fn assert_after_validation_mutation_uses_the_pinned_snapshot(mutation: Mutation) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("after-validation-{mutation:?}{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).unwrap();
    let engine = Arc::new(opened.engine);
    engine.write(&[node()]).unwrap();
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();

    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_frozen_after_validation_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));

    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || {
            engine.search_frozen("unique frozen needle", &frozen, 0, false, 0.3, 0, false, 10)
        })
    };
    ready.wait();
    mutate(&engine, mutation);
    release.wait();

    let result = worker.join().unwrap().unwrap();
    assert!(
        result.results.iter().any(|hit| hit.id.value == "root"),
        "the operation must remain on the snapshot pinned before {mutation:?}"
    );
    if matches!(mutation, Mutation::Erasure) {
        engine
            .erase_source("test:slice35-after-validation")
            .expect("erasure retry must close once the pinned snapshot is released");
    }
}

#[test]
fn lifecycle_erasure_dependency_and_projection_changes_after_validation_are_snapshot_isolated() {
    for mutation in
        [Mutation::Lifecycle, Mutation::Erasure, Mutation::DependencyClosure, Mutation::Projection]
    {
        assert_after_validation_mutation_uses_the_pinned_snapshot(mutation);
    }
}

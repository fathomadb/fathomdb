//! Tiny real-store projection custody controls; no scale or GPU qualification.
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineConfig, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SourceId, TestEmbedderChoice,
};
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;

struct FixedVector;
impl Embedder for FixedVector {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-d-custody-unit", "fixed", 768)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 768];
        vector[0] = 1.0;
        Ok(vector)
    }
}

fn open(path: &std::path::Path) -> Engine {
    let engine = Engine::open_with_test_choice_and_config_for_test(
        path,
        TestEmbedderChoice::Caller(Arc::new(FixedVector)),
        EngineConfig::default(),
    )
    .unwrap()
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
        .unwrap();
    engine
}

struct Frozen<'a>(&'a Engine);
impl<'a> Frozen<'a> {
    fn new(engine: &'a Engine) -> Self {
        engine.set_projection_scheduler_frozen_for_test(true);
        Self(engine)
    }
}
impl Drop for Frozen<'_> {
    fn drop(&mut self) {
        self.0.set_projection_scheduler_frozen_for_test(false);
    }
}

fn seed(engine: &Engine, prefix: &str, count: usize) -> Vec<u64> {
    let writes = (0..count)
        .map(|index| PreparedWrite::Node {
            kind: "doc".into(),
            body: format!("needle {prefix} {index}"),
            logical_id: Some(format!("{prefix}-{index}")),
            source_id: SourceId::new("test:slice30-d-custody").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect::<Vec<_>>();
    engine.write(&writes).unwrap().row_cursors
}

fn closed_dense_rows(path: &std::path::Path) -> (u64, u64, u64) {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM vector_default), \
             (SELECT COUNT(*) FROM _fathomdb_vector_rows), \
             (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap()
}

#[test]
fn background_observation_belongs_to_the_armed_engine_and_actual_jobs() {
    let directory = tempfile::tempdir().unwrap();
    let a_path = directory.path().join("a.sqlite");
    let b_path = directory.path().join("b.sqlite");
    let a = open(&a_path);
    let b = open(&b_path);
    let frozen = Frozen::new(&a);
    let origin = Instant::now();
    let observer = a.begin_slice30_projection_observation_for_test(512, origin).unwrap();
    let other = b.begin_slice30_observation(64).unwrap();
    let a_cursors = seed(&a, "a", 2);
    let pending = a.read_embedding_readiness().unwrap().pending_count;
    seed(&b, "b", 3);
    b.drain(5_000).unwrap();
    drop(frozen);
    a.drain(5_000).unwrap();
    let readiness = a.read_embedding_readiness().unwrap();
    let a_hits = observer.with_operation(11, || a.search("needle")).unwrap().results.len();
    observer.with_operation(42, || b.search("needle")).unwrap();
    other.with_operation(77, || b.search("needle")).unwrap();
    let batch = observer.take();
    let other_batch = other.take();
    a.close().unwrap();
    b.close().unwrap();
    drop(observer);
    drop(other);

    assert_eq!(pending, 2);
    assert_eq!(readiness.pending_count, 0);
    assert_eq!(a_hits, 2);
    assert_eq!(closed_dense_rows(&a_path), (2, 2, 2));
    assert_eq!(closed_dense_rows(&b_path), (3, 3, 3));
    assert_eq!(batch.dropped_records, 0);
    assert!(batch.issues.is_empty());
    let b_engine_id = other_batch.records.first().expect("actual B foreground record").engine_id;
    assert!(batch.records.iter().all(|record| record.engine_id != b_engine_id));
    assert!(batch.records.iter().all(|record| record.operation_id != 42));
    assert!(batch
        .records
        .iter()
        .any(|record| record.phase == "api_hybrid" && record.operation_id == 11));
    let expected = a_cursors.into_iter().collect::<BTreeSet<_>>();
    for phase in [
        "projection_scan",
        "projection_dequeue",
        "projection_processing",
        "projection_embed_dispatch_wall",
    ] {
        let records =
            batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert!(!records.is_empty(), "actual engine A background phase is missing: {phase}");
        assert!(records.iter().all(|record| record.operation_id >= 1_u64 << 63));
        let cursors = records
            .iter()
            .flat_map(|record| record.facts["cursors"].as_array().expect("bounded actual cursors"))
            .map(|value| value.as_u64().expect("actual numeric cursor"))
            .collect::<BTreeSet<_>>();
        assert_eq!(cursors, expected, "actual job coverage differs: {phase}");
        for record in records {
            if phase == "projection_dequeue" {
                assert!(record.started_ns.is_none() && record.ended_ns.is_none());
            } else {
                assert!(record
                    .started_ns
                    .zip(record.ended_ns)
                    .is_some_and(|(start, end)| start <= end));
            }
        }
    }
}

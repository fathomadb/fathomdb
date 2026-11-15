//! Real tiny governed writes; literal inputs never qualify primary synthetic cells.
#[path = "../examples/slice30_d_measure/provider.rs"]
mod provider;
#[path = "../examples/slice30_d_measure/seed.rs"]
mod seed;

use fathomdb_engine::{
    Engine, EngineConfig, ProjectionRole, ProjectionSpec, ProjectionVector, TestEmbedderChoice,
};
use provider::VaryingEmbedder;
use rusqlite::{Connection, OpenFlags};
use std::sync::Arc;

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

#[test]
fn seed_driver_commits_real_rows_in_1000_row_batches_without_interbatch_drain() {
    for (n, expected_batches) in [(2_usize, vec![2_usize]), (1001, vec![1000, 1])] {
        let directory = tempfile::tempdir().unwrap();
        let store = directory.path().join("seed.sqlite");
        let engine = Engine::open_with_test_choice_and_config_for_test(
            &store,
            TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
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
        let frozen = Frozen::new(&engine);
        engine.drain(5_000).unwrap();
        let bodies = (0..n).map(|index| format!("unit natural record {index}")).collect::<Vec<_>>();
        let report = seed::seed_rows(&engine, bodies.clone().into_iter());
        let held = engine.read_embedding_readiness();
        drop(frozen);
        let drained = engine.drain(10_000);
        let final_pending = engine.read_embedding_readiness();
        let closed = engine.close();

        closed.expect("real seed fixture must close before assertions");
        drained.expect("actual released projections must drain");
        let report = report.expect("real seed seam must return its actual commit report");
        assert_eq!(
            held.unwrap().pending_count as usize,
            n,
            "governed seed omitted real held pending rows"
        );
        assert_eq!(final_pending.unwrap().pending_count, 0);
        let connection =
            Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let counts: (usize, usize, usize) = connection.query_row(
            "SELECT (SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc'), (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
        assert_eq!(counts, (n, n, n), "seed omitted actual canonical/projected/terminal rows");
        assert_eq!(
            report.committed_batches, expected_batches,
            "actual successful write batch sizes differ"
        );
        let mut statement = connection
            .prepare("SELECT body FROM canonical_nodes WHERE kind='doc' ORDER BY write_cursor")
            .unwrap();
        let actual = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(actual, bodies, "seed changed the supplied natural body population");
    }
}

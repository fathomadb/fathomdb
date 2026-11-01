#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbeddingReadinessState, Engine, InitialState, PreparedWrite, ProjectionScanForTest, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::{params, Connection};
use std::sync::Arc;
use tempfile::TempDir;

const NOW: i64 = 2_000_000_000;
const SCANS: [ProjectionScanForTest; 2] =
    [ProjectionScanForTest::Scheduler, ProjectionScanForTest::CandidatePage];

#[derive(Debug)]
struct FixedEmbedder;
impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice50", "fixed", 8)
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text.contains("fail") {
            return Err(EmbedderError::Failed { message: "fixture failure".into() });
        }
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

struct Fixture {
    _dir: TempDir,
    engine: Engine,
    raw: Connection,
}
impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("scan{SQLITE_SUFFIX}"));
        let engine =
            Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap().engine;
        engine.configure_vector_kind_for_test("doc").unwrap();
        engine.configure_vector_kind_for_test("edge_fact").unwrap();
        engine.set_projection_scheduler_frozen_for_test(true);
        engine.set_projection_retry_delays_for_test(&[]);
        let raw = Connection::open(path).unwrap();
        Self { _dir: dir, engine, raw }
    }

    fn seed_raw(&mut self, kinds: &[u8], terminals: &[bool]) {
        let tx = self.raw.transaction().unwrap();
        tx.execute_batch("DELETE FROM canonical_nodes; DELETE FROM canonical_edges; DELETE FROM _fathomdb_projection_terminal;").unwrap();
        for (i, kind) in kinds.iter().enumerate() {
            let c = (i + 1) as i64;
            match kind % 10 {
                0..=3 => {
                    tx.execute("INSERT INTO canonical_nodes(write_cursor,kind,body,superseded_at,state,valid_from,valid_until) VALUES(?1,?2,'body',?3,?4,?5,?6)", params![c, if kind % 10 == 1 { "other" } else { "doc" }, if kind % 10 == 2 { Some(1) } else { None }, if kind % 10 == 3 { "deleted" } else { "active" }, Option::<i64>::None, Option::<i64>::None]).unwrap();
                }
                k => {
                    tx.execute("INSERT INTO canonical_edges(write_cursor,kind,from_id,to_id,body,superseded_at,t_invalid) VALUES(?1,'relates_to','a','b',?2,?3,?4)", params![c, if k == 5 { None } else { Some("edge body") }, if k == 6 { Some(1) } else { None }, match k { 7 => Some(NOW-1), 8 => Some(NOW+1), _ => None }]).unwrap();
                }
            }
            if terminals.get(i).copied().unwrap_or(false) {
                tx.execute(
                    "INSERT INTO _fathomdb_projection_terminal VALUES(?1,?2)",
                    params![c, if i % 2 == 0 { "failed" } else { "up_to_date" }],
                )
                .unwrap();
            }
        }
        tx.commit().unwrap();
    }
}

// Frozen source from 630af0bfc: the shared edge fragment plus its expanded
// default-view temporal/dependency predicates. It does not call a production
// SQL builder, so changes to the bounded query cannot alter this oracle.
fn baseline_edge(now: usize) -> String {
    format!(
        r#"FROM canonical_edges ce
         JOIN _fathomdb_vector_kinds
           ON _fathomdb_vector_kinds.kind = 'edge_fact'
         LEFT JOIN _fathomdb_projection_terminal pt
           ON pt.write_cursor = ce.write_cursor
         WHERE ce.body IS NOT NULL
           AND ce.superseded_at IS NULL
           AND (ce.t_invalid IS NULL OR ce.t_invalid > ?{now})
           AND NOT EXISTS(
             SELECT 1 FROM _fathomdb_artifact_revisions closure_owner
             JOIN _fathomdb_source_dependencies closure_dep ON closure_dep.derived_revision_id=closure_owner.revision_id
             JOIN _fathomdb_source_links closure_link ON closure_link.artifact_revision_id=closure_owner.revision_id
             JOIN _fathomdb_source_links closure_source ON closure_source.artifact_revision_id=closure_link.source_revision_id
             JOIN _fathomdb_dependency_closures closure_op ON closure_op.phase!='complete' AND (
               (closure_op.root_kind='source_revision' AND closure_op.root_value=closure_link.source_revision_id) OR
               (closure_op.root_kind='source_bucket' AND closure_op.root_value=closure_source.source_id))
             WHERE closure_owner.write_cursor=ce.write_cursor)
           AND NOT EXISTS(
             SELECT 1 FROM _fathomdb_artifact_revisions source_owner
             JOIN _fathomdb_source_dependencies source_dep ON source_dep.derived_revision_id=source_owner.revision_id
             JOIN _fathomdb_source_links derived_link ON derived_link.artifact_revision_id=source_owner.revision_id
             WHERE source_owner.write_cursor=ce.write_cursor AND NOT EXISTS(
               SELECT 1 FROM _fathomdb_artifact_revisions source_revision
               JOIN canonical_nodes source_node ON source_node.write_cursor=source_revision.write_cursor
               WHERE source_revision.revision_id=derived_link.source_revision_id
                 AND source_revision.artifact_class='node' AND source_revision.artifact_role='canonical_source'
                 AND source_revision.completeness='complete' AND source_node.superseded_at IS NULL
                 AND source_node.state='active'
                 AND (source_node.valid_from IS NULL OR source_node.valid_from <= ?{now})
                 AND (source_node.valid_until IS NULL OR source_node.valid_until > ?{now})))
           AND pt.write_cursor IS NULL"#
    )
}

fn baseline_sql(scan: ProjectionScanForTest) -> String {
    match scan {
        ProjectionScanForTest::Scheduler => format!(
            r#"SELECT pending.write_cursor, pending.kind, pending.body, current.generation_id FROM (
             SELECT canonical_nodes.write_cursor AS write_cursor,
                    canonical_nodes.kind AS kind,
                    canonical_nodes.body AS body
             FROM canonical_nodes
             JOIN _fathomdb_vector_kinds ON _fathomdb_vector_kinds.kind = canonical_nodes.kind
             LEFT JOIN _fathomdb_projection_terminal ON _fathomdb_projection_terminal.write_cursor = canonical_nodes.write_cursor
             WHERE canonical_nodes.write_cursor > ?1 AND _fathomdb_projection_terminal.write_cursor IS NULL
             UNION ALL
             SELECT ce.write_cursor AS write_cursor, 'edge_fact' AS kind, ce.body AS body
             {} AND ce.write_cursor > ?1
         ) AS pending CROSS JOIN _fathomdb_projection_generation_current AS current
         WHERE current.singleton=1 ORDER BY pending.write_cursor LIMIT 256"#,
            baseline_edge(2)
        ),
        ProjectionScanForTest::CandidatePage => format!(
            r#"SELECT pending.write_cursor,pending.kind FROM (
           SELECT n.write_cursor,n.kind FROM canonical_nodes n
           JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind
           LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=n.write_cursor
           WHERE n.write_cursor>?1 AND (?2 IS NULL OR n.write_cursor<=?2) AND pt.write_cursor IS NULL
           UNION ALL SELECT ce.write_cursor,'edge_fact'
           {} AND ce.write_cursor>?1 AND (?2 IS NULL OR ce.write_cursor<=?2)
         ) AS pending ORDER BY pending.write_cursor LIMIT 256"#,
            baseline_edge(3)
        ),
    }
}

fn assert_page(f: &Fixture, scan: ProjectionScanForTest, after: u64, through: Option<u64>) {
    let actual = f.engine.projection_scan_page_for_test(scan, after, through, NOW).unwrap();
    let sql = baseline_sql(scan);
    let mut stmt = f.raw.prepare(&sql).unwrap();
    let mut values = vec![rusqlite::types::Value::from(after as i64)];
    if matches!(scan, ProjectionScanForTest::CandidatePage) {
        values.push(through.map(|v| (v as i64).into()).unwrap_or(rusqlite::types::Value::Null));
    }
    values.push(NOW.into());
    let expected = stmt
        .query_map(rusqlite::params_from_iter(values), |r| {
            Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(actual.rows, expected, "{scan:?} after={after} through={through:?}");
}

#[test]
fn slice50_scan_hook_reports_production_pages() {
    let mut f = Fixture::new();
    f.seed_raw(&(0..1200).map(|i| if i % 4 == 0 { 4 } else { 0 }).collect::<Vec<_>>(), &[]);
    for scan in SCANS {
        let page = f.engine.projection_scan_page_for_test(scan, 0, None, NOW).unwrap();
        assert_eq!(page.rows.len(), 256);
        assert!(page.rows.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(page.vm_steps > 0 && !page.plan.is_empty());
        assert!(page.sql.contains("ORDER BY pending.write_cursor"));
        for after in (0..1200).step_by(50) {
            assert_page(&f, scan, after, None);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn slice50_bounded_scans_match_shipped_scan(
        kinds in prop::collection::vec(0u8..10, 1..700),
        terminals in prop::collection::vec(any::<bool>(), 0..700),
        span in 0u64..750,
    ) {
        let mut f = Fixture::new();
        f.seed_raw(&kinds,&terminals);
        for after in 0..=kinds.len() as u64 {
            for scan in SCANS {
                assert_page(&f,scan,after,None);
                if matches!(scan, ProjectionScanForTest::CandidatePage) { assert_page(&f,scan,after,Some(after+span)); }
            }
        }
    }
}

fn node(i: usize, fail: bool) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("{} node {i}", if fail { "fail" } else { "ok" }),
        source_id: SourceId::new("test:slice50").unwrap(),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn edge(i: usize, fail: bool) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "relates_to".into(),
        from: format!("a-{i}"),
        to: "b".into(),
        source_id: SourceId::new("test:slice50").unwrap(),
        logical_id: None,
        body: Some(format!("{} edge {i}", if fail { "fail" } else { "ok" })),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

#[test]
fn slice50_node_edge_ordering_cases() {
    for layout in 0..3 {
        let f = Fixture::new();
        let writes: Vec<_> = (0..1200)
            .map(|i| {
                let is_edge = match layout {
                    0 => i < 600,
                    1 => i >= 600,
                    _ => i % 2 == 0,
                };
                let fail = layout == 2 && i % 71 == 0;
                if is_edge {
                    edge(i, fail)
                } else {
                    node(i, fail)
                }
            })
            .collect();
        f.engine.write(&writes).unwrap();
        if layout == 2 {
            f.raw.execute("UPDATE canonical_nodes SET superseded_at=1 WHERE write_cursor=(SELECT MIN(write_cursor) FROM canonical_nodes)",[]).unwrap();
            f.raw.execute("UPDATE canonical_edges SET superseded_at=1 WHERE write_cursor=(SELECT MIN(write_cursor) FROM canonical_edges)",[]).unwrap();
            f.raw.execute("UPDATE canonical_edges SET t_invalid=1 WHERE write_cursor=(SELECT MAX(write_cursor) FROM canonical_edges)",[]).unwrap();
        }
        for after in (0..1200).step_by(50) {
            for scan in SCANS {
                assert_page(&f, scan, after, None);
            }
        }
        // Ordinary canonical nodes remain projection members when superseded;
        // query-time eligibility is separate. Edges filter supersession here.
        let expected = f.raw.prepare("SELECT write_cursor FROM canonical_nodes WHERE body NOT LIKE '%fail%' UNION ALL SELECT write_cursor FROM canonical_edges WHERE superseded_at IS NULL AND t_invalid IS NULL AND body NOT LIKE '%fail%' ORDER BY write_cursor").unwrap().query_map([],|r|r.get::<_,u64>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
        f.engine.set_projection_scheduler_frozen_for_test(false);
        f.engine.drain(30_000).unwrap();
        let actual = f
            .raw
            .prepare("SELECT write_cursor FROM _fathomdb_vector_rows ORDER BY write_cursor")
            .unwrap()
            .query_map([], |r| r.get::<_, u64>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(actual, expected, "layout {layout}");
        let readiness = f.engine.read_embedding_readiness().unwrap();
        assert_eq!(readiness.state, EmbeddingReadinessState::Ready);
        assert_eq!(readiness.pending_count, 0);
        if layout == 2 {
            assert!(
                f.raw
                    .query_row(
                        "SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='failed'",
                        [],
                        |r| r.get::<_, u64>(0)
                    )
                    .unwrap()
                    > 0
            );
        }
    }
}

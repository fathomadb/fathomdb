//! Independently authored object-state and CPU durability composition.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::*;
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};

struct AuthoredVectors;
impl Embedder for AuthoredVectors {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("cohesion", "v1", 8)
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        Ok(if text.contains("revised") {
            vec![0., 1., 0., 0., 0., 0., 0., 0.]
        } else {
            vec![1., 0., 0., 0., 0., 0., 0., 0.]
        })
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../../conformance/slice55-behavior.json")).unwrap()
}
fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn open(path: &Path) -> OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(AuthoredVectors)).unwrap()
}
fn seed(engine: &Engine) -> Vec<u64> {
    let f = fixture();
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "group".into(),
                roles: [ProjectionRole::Filterable].into(),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine.set_projection_scheduler_frozen_for_test(true);
    let writes = f["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, n)| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: f["body"].as_str().unwrap().into(),
                source_id: SourceId::new(n["source"].as_str().unwrap()).unwrap(),
                logical_id: Some(n["id"].as_str().unwrap().into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(if i == 0 { 1_700_000_000 } else { 2_000_000_000 }),
                valid_until: if i == 0 { Some(2_000_000_001) } else { None },
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(n["revision"].as_str().unwrap()).unwrap(),
                    SourceVersionId::new(n["version"].as_str().unwrap()).unwrap(),
                ),
            })
        })
        .collect::<Vec<_>>();
    let receipt = engine.write(&writes).unwrap();
    let hash = Sha256::digest(f["body"].as_str().unwrap().as_bytes())
        .iter()
        .map(|v| format!("{v:02x}"))
        .collect::<String>();
    engine
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "claim".into(),
            body: "authored wombat claim".into(),
            source_id: SourceId::new("cohesion:source-a").unwrap(),
            logical_id: Some("cohesion:claim".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("cohesion:claim-r1").unwrap(),
                SourceVersionId::new("cohesion:a-v1").unwrap(),
                SourceRevisionId::new("cohesion:a-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(hash).unwrap(),
            ),
        })])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "cohesion:dep",
                "cohesion:a-r1",
                "cohesion:claim-r1",
            )
            .unwrap(),
        )
        .unwrap();
    engine
        .write(&[PreparedWrite::Edge {
            kind: "supports".into(),
            from: "cohesion:b".into(),
            to: "cohesion:a".into(),
            body: None,
            source_id: SourceId::new("cohesion:source-b").unwrap(),
            logical_id: Some("cohesion:edge".into()),
            t_valid: Some(2_000_000_100),
            t_invalid: Some(2_000_000_002),
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    receipt.row_cursors
}
fn view(at: i64) -> ReadView {
    ReadView { valid_as_of: Some(at), ..ReadView::default() }
}
fn text_ids(engine: &Engine, at: i64) -> Vec<String> {
    let result = engine.search_text_only_view("wombat", &view(at)).unwrap();
    let mut ids = result
        .results
        .into_iter()
        .filter(|hit| hit.kind == "doc")
        .map(|hit| hit.id.value)
        .collect::<Vec<_>>();
    ids.sort();
    ids
}
fn graph_ids(engine: &Engine, at: i64, include_out_of_window: bool) -> Vec<String> {
    let mut requested = view(at);
    requested.include_out_of_window = include_out_of_window;
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("cohesion:b")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["doc".into()],
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(requested, SearchFilter::default()).unwrap(),
        },
        max_depth: 1,
        result_limit: 1,
        max_work_units: 10,
        include_explanation: false,
        include_evidence: false,
    };
    let result = engine.graph_expand(&request).unwrap();
    for target in &result.targets {
        assert_eq!(target.write_cursor, 1);
        assert_eq!(target.body, fixture()["body"].as_str().unwrap());
        assert_eq!(target.origin.seed_logical_id, "cohesion:b");
        assert_eq!(target.origin.predecessor_logical_id, "cohesion:b");
        assert_eq!(target.origin.target_logical_id, "cohesion:a");
        assert_eq!(target.origin.hop_count, 1);
        assert_eq!(target.origin.terminal_edge_kind, "supports");
        assert_eq!(target.origin.terminal_direction, TraversalDirection::Outgoing);
    }
    result.targets.into_iter().map(|target| target.logical_id).collect()
}
fn raw_counts(path: &Path) -> (i64, i64, i64, i64) {
    Connection::open(path).unwrap().query_row("SELECT (SELECT count(*) FROM canonical_nodes), (SELECT count(*) FROM _fathomdb_vector_rows), (SELECT count(*) FROM _fathomdb_projection_terminal), (SELECT count(*) FROM _fathomdb_source_dependencies)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap()
}
fn assert_initial(engine: &Engine, vectors: bool) {
    let f = fixture();
    for (i, n) in f["nodes"].as_array().unwrap().iter().enumerate() {
        let row =
            engine.read_get(n["id"].as_str().unwrap(), &view(2_000_000_000)).unwrap().unwrap();
        assert_eq!(row.body, f["body"].as_str().unwrap());
        let hits = engine.search_text_only_view("wombat", &view(2_000_000_000)).unwrap();
        assert_eq!(
            hits.results
                .iter()
                .find(|hit| hit.write_cursor == row.write_cursor)
                .unwrap()
                .source_id
                .as_deref(),
            n["source"].as_str()
        );
        assert_eq!(
            engine.has_vector_row_for_cursor_for_test(row.write_cursor).unwrap(),
            vectors,
            "record {i}"
        );
    }
    let generation_before_windows =
        engine.read_projection_generation_status().unwrap().generation_id;
    assert_eq!(
        text_ids(engine, 1_999_999_999),
        vec!["cohesion:a"],
        "future b is excluded before its start"
    );
    assert_eq!(text_ids(engine, 2_000_000_000), vec!["cohesion:a", "cohesion:b"]);
    assert_eq!(text_ids(engine, 2_000_000_001), vec!["cohesion:b"]);
    assert_eq!(
        graph_ids(engine, 2_000_000_000, false),
        vec!["cohesion:a"],
        "future edge t_valid does not gate traversal"
    );
    assert!(graph_ids(engine, 2_000_000_001, false).is_empty(), "node end is exclusive");
    assert_eq!(
        engine.read_projection_generation_status().unwrap().generation_id,
        generation_before_windows
    );
    let before_edge = engine.read_projection_generation_status().unwrap().generation_id;
    assert_eq!(
        graph_ids(engine, 2_000_000_001, true),
        vec!["cohesion:a"],
        "relaxed node windows leave the edge eligible before expiry"
    );
    assert!(
        graph_ids(engine, 2_000_000_002, true).is_empty(),
        "node relaxation never admits an edge at its expiry"
    );
    assert_eq!(engine.read_projection_generation_status().unwrap().generation_id, before_edge);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(view(2_000_000_000), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let request = DependencyTraceRequestV1::new(
        "cohesion:claim-r1",
        DependencyTraceDirectionV1::ToSource,
        frozen.clone(),
    )
    .unwrap();
    let result = engine.trace_dependency(request).unwrap();
    assert_eq!(
        result.nodes.iter().map(|n| n.artifact_revision_id.as_str()).collect::<Vec<_>>(),
        vec!["cohesion:claim-r1", "cohesion:a-r1"]
    );
    assert_eq!(result.dependency_edges.len(), 1);
    assert_eq!(result.dependency_edges[0].dependency_id, "cohesion:dep");
    assert_eq!(result.dependency_edges[0].source_revision_id, "cohesion:a-r1");
    assert_eq!(result.dependency_edges[0].derived_revision_id, "cohesion:claim-r1");
    let evidence = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: "authored".into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: false,
            limit: 10,
        })
        .unwrap();
    let claim =
        evidence.evidence.iter().find(|e| e.artifact_revision_id == "cohesion:claim-r1").unwrap();
    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: claim.evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.source_revision_id, "cohesion:a-r1");
    assert_eq!(resolved.canonical_source_body, f["body"].as_str().unwrap());
    let generation = engine.read_projection_generation_status().unwrap().generation_id;
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".into());
    filter.attributes = vec![("group".into(), "kept".into())];
    assert_eq!(
        engine
            .search_text_only_filtered("wombat", Some(filter), &view(2_000_000_001), 1)
            .unwrap()
            .results[0]
            .id
            .value,
        "cohesion:b"
    );
    let mut excluded = SearchFilter::default();
    excluded.attributes = vec![("group".into(), "absent".into())];
    assert!(engine
        .search_text_only_filtered("wombat", Some(excluded), &view(2_000_000_000), 10)
        .unwrap()
        .results
        .is_empty());
    assert_eq!(engine.read_projection_generation_status().unwrap().generation_id, generation);
    engine.set_vector_stage_only_for_test(true);
    let pending = engine.search_view("wombat", &view(2_000_000_000)).unwrap();
    if !vectors {
        assert!(pending.results.is_empty());
        assert_eq!(pending.soft_fallback.unwrap().branch, SoftFallbackBranch::Vector);
    }
    engine.set_vector_stage_only_for_test(false);
    if vectors {
        engine.set_vector_stage_only_for_test(true);
        let result = engine.search_view("wombat", &view(2_000_000_000)).unwrap();
        let mut ids = result.results.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>();
        ids.sort();
        assert_eq!(ids, vec!["cohesion:a", "cohesion:b"]);
        engine.set_vector_stage_only_for_test(false);
        let result = engine.search_view("wombat", &view(2_000_000_001)).unwrap();
        assert_eq!(
            result.results.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>(),
            vec!["cohesion:b"]
        );
        assert_eq!(
            result.soft_fallback.unwrap().branch,
            SoftFallbackBranch::Vector,
            "expired dependency source truthfully disables dense arm"
        );
    }
}

fn assert_durable(path: &Path, updated: bool, projected: bool) {
    let f = fixture();
    let conn = Connection::open(path).unwrap();
    let mut stmt=conn.prepare("SELECT n.logical_id,n.write_cursor,n.body,n.source_id,n.superseded_at IS NULL,ar.revision_id,sl.source_revision_id,sl.source_version_id FROM canonical_nodes n JOIN _fathomdb_artifact_revisions ar ON ar.write_cursor=n.write_cursor AND ar.artifact_class='node' JOIN _fathomdb_source_links sl ON sl.artifact_revision_id=ar.revision_id WHERE n.kind='doc' ORDER BY n.write_cursor").unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let mut expected = vec![
        (
            "cohesion:a".into(),
            1,
            f["body"].as_str().unwrap().into(),
            "cohesion:source-a".into(),
            true,
            "cohesion:a-r1".into(),
            "cohesion:a-r1".into(),
            "cohesion:a-v1".into(),
        ),
        (
            "cohesion:b".into(),
            2,
            f["body"].as_str().unwrap().into(),
            "cohesion:source-b".into(),
            !updated,
            "cohesion:b-r1".into(),
            "cohesion:b-r1".into(),
            "cohesion:b-v1".into(),
        ),
    ];
    if updated {
        expected.push((
            "cohesion:b".into(),
            5,
            f["updatedBody"].as_str().unwrap().into(),
            "cohesion:source-b".into(),
            true,
            "cohesion:b-r2".into(),
            "cohesion:b-r2".into(),
            "cohesion:b-v2".into(),
        ));
    }
    assert_eq!(rows, expected, "exact retained/current revision/source ownership");
    for row in &rows {
        let lexical: String = conn
            .query_row("SELECT body FROM search_index WHERE write_cursor=?1", [row.1], |r| r.get(0))
            .unwrap();
        assert_eq!(lexical, row.2);
        let terminal = conn.query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [row.1],
            |r| r.get::<_, String>(0),
        );
        let mut statement=conn.prepare("SELECT m.rowid,m.write_cursor,v.embedding FROM _fathomdb_vector_rows m JOIN vector_default v ON v.rowid=m.rowid WHERE m.write_cursor=?1").unwrap();
        let vectors = statement
            .query_map([row.1], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, Vec<u8>>(2)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        if projected {
            assert_eq!(terminal.unwrap(), "up_to_date");
            let mut vector = [0_f32; 8];
            vector[if row.1 == 5 { 1 } else { 0 }] = 1.;
            let bytes = vector.into_iter().flat_map(f32::to_le_bytes).collect::<Vec<_>>();
            assert_eq!(vectors, vec![(row.1, row.1, bytes)]);
        } else {
            assert!(matches!(terminal, Err(rusqlite::Error::QueryReturnedNoRows)));
            assert!(vectors.is_empty());
        }
    }
    assert_eq!(
        conn.query_row("SELECT count(*) FROM vector_default", [], |r| r.get::<_, i64>(0)).unwrap(),
        if projected {
            if updated {
                3
            } else {
                2
            }
        } else {
            0
        }
    );
    let dependencies=conn.prepare("SELECT d.dependency_id,d.derived_revision_id,l.source_revision_id FROM _fathomdb_source_dependencies d JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id ORDER BY d.dependency_id").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
    assert_eq!(
        dependencies,
        vec![("cohesion:dep".into(), "cohesion:claim-r1".into(), "cohesion:a-r1".into())]
    );
    let claim:(String,String,String)=conn.query_row("SELECT n.body,n.source_id,t.state FROM canonical_nodes n JOIN _fathomdb_projection_terminal t ON t.write_cursor=n.write_cursor WHERE n.logical_id='cohesion:claim'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(
        claim,
        ("authored wombat claim".into(), "cohesion:source-a".into(), "up_to_date".into())
    );
    let edge: (String, String, String, String, i64, i64) = conn
        .query_row(
            "SELECT logical_id,from_id,to_id,kind,t_valid,t_invalid FROM canonical_edges",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .unwrap();
    assert_eq!(
        edge,
        (
            "cohesion:edge".into(),
            "cohesion:b".into(),
            "cohesion:a".into(),
            "supports".into(),
            2_000_000_100,
            2_000_000_002
        )
    );
}

#[test]
fn object_cohesion_lag_drain_supersede_rollback_and_fresh_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cohesion.sqlite");
    let opened = open(&path);
    let engine = &opened.engine;
    let cursors = seed(engine);
    assert_initial(engine, false);
    assert_durable(&path, false, false);
    assert_eq!(engine.projection_status_for_test("doc").unwrap(), ProjectionStatus::Pending);
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(10_000).unwrap();
    assert_initial(engine, true);
    assert_durable(&path, false, true);
    let f = fixture();
    let updated = engine
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind: "doc".into(),
            body: f["updatedBody"].as_str().unwrap().into(),
            source_id: SourceId::new("cohesion:source-b").unwrap(),
            logical_id: Some("cohesion:b".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("cohesion:b-r2").unwrap(),
                SourceVersionId::new("cohesion:b-v2").unwrap(),
            ),
        })])
        .unwrap();
    engine.drain(10_000).unwrap();
    assert_ne!(updated.cursor, cursors[1]);
    let raw = Connection::open(&path).unwrap();
    let revision: String = raw
        .query_row(
            "SELECT revision_id FROM _fathomdb_artifact_revisions WHERE write_cursor=?1",
            [updated.cursor],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(revision, "cohesion:b-r2");
    drop(raw);
    let before = raw_counts(&path);
    let mut bad = node("rejected", "cohesion:source-a", "must not persist");
    if let PreparedWrite::Node { kind, .. } = &mut bad {
        *kind = "".into();
    }
    assert!(engine
        .write(&[node("partial", "cohesion:source-a", "must not persist"), bad])
        .is_err());
    assert_eq!(raw_counts(&path), before);
    assert!(engine.read_get("partial", &ReadView::default()).unwrap().is_none());
    assert_eq!(
        engine.read_get("cohesion:b", &ReadView::default()).unwrap().unwrap().body,
        f["updatedBody"]
    );
    let history = ReadView { include_superseded: true, ..view(2_000_000_000) };
    let result = engine.search_text_only_view("wombat", &history).unwrap();
    assert_eq!(result.results.iter().filter(|hit| hit.kind == "doc").count(), 3);
    assert_durable(&path, true, true);
    engine.close().unwrap();
    drop(opened);
    run_child(&path, "verify");
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn child(path: &Path, mode: &str) -> OwnedChild {
    OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cohesion_child", "--ignored", "--nocapture"])
            .env("COHESION_PATH", path)
            .env("COHESION_MODE", mode)
            .stdout(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
fn run_child(path: &Path, mode: &str) {
    let mut child = child(path, mode);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(exit) = child.0.try_wait().unwrap() {
            assert!(exit.success(), "child {mode}: {exit}");
            assert_eq!(
                std::fs::read_to_string(format!("{}.verified-{mode}", path.display())).unwrap(),
                mode
            );
            break;
        }
        assert!(Instant::now() < deadline, "child watchdog");
        std::thread::sleep(Duration::from_millis(10));
    }
}
#[test]
#[ignore = "supervised subprocess helper"]
fn cohesion_child() {
    let path = std::env::var("COHESION_PATH").unwrap();
    let mode = std::env::var("COHESION_MODE").unwrap();
    let opened = open(Path::new(&path));
    if mode == "crash" {
        seed(&opened.engine);
        std::fs::write(format!("{path}.ready"), b"committed-pending").unwrap();
        loop {
            std::thread::park_timeout(Duration::from_secs(1));
        }
    }
    opened.engine.drain(10_000).unwrap();
    if mode == "retry" {
        assert_initial(&opened.engine, true);
    } else {
        let f = fixture();
        assert_eq!(
            opened.engine.read_get("cohesion:b", &ReadView::default()).unwrap().unwrap().body,
            f["updatedBody"]
        );
        assert_eq!(
            opened.engine.read_get("cohesion:a", &view(2_000_000_000)).unwrap().unwrap().body,
            f["body"]
        );
        assert!(opened.engine.read_get("partial", &ReadView::default()).unwrap().is_none());
    }
    assert_durable(Path::new(&path), mode != "retry", true);
    opened.engine.close().unwrap();
    std::fs::write(format!("{path}.verified-{mode}"), mode).unwrap();
}
#[test]
fn delivered_process_kill_retains_pending_canonical_dependency_then_retries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.sqlite");
    let register = Engine::open(dir.path().join("registration-only.sqlite")).unwrap();
    register.engine.close().unwrap();
    drop(register);
    let mut child = child(&path, "crash");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !path.with_extension("sqlite.ready").exists() {
        assert!(child.0.try_wait().unwrap().is_none(), "child died before delivery");
        assert!(Instant::now() < deadline, "ready watchdog");
        std::thread::sleep(Duration::from_millis(10));
    }
    child.0.kill().unwrap();
    let status = child.0.wait().unwrap();
    assert!(!status.success());
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(9));
    }
    let copy = dir.path().join("sacrificial.sqlite");
    std::fs::copy(&path, &copy).unwrap();
    std::fs::copy(format!("{}-wal", path.display()), format!("{}-wal", copy.display())).unwrap();
    assert_eq!(raw_counts(&copy), (3, 0, 2, 1), "before normal reopen can dispatch pending work");
    assert_durable(&copy, false, false);
    let raw = Connection::open(&copy).unwrap();
    assert_eq!(raw.query_row("SELECT count(*) FROM canonical_nodes WHERE logical_id IN ('cohesion:a','cohesion:b') AND body=?1",[fixture()["body"].as_str().unwrap()],|r|r.get::<_,i64>(0)).unwrap(),2);
    drop(raw);
    run_child(&path, "retry");
    assert_eq!(raw_counts(&path).1, 2);
}

#[test]
fn bounded_seeded_operation_model_uses_independent_current_and_history_state() {
    for seed in 0_u64..16 {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.sqlite");
        let mut opened = open(&path);
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        let mut expected = BTreeMap::<String, String>::new();
        let mut revisions = 0;
        let mut rng = 0x55_0830 ^ seed;
        for step in 0..10 {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            let id = format!("model:{}", (rng >> 32) % 3);
            match rng % 5 {
                0 => {
                    opened.engine.close().unwrap();
                    drop(opened);
                    opened = open(&path);
                }
                1 => {
                    let before = raw_counts(&path).0;
                    let mut bad = node("invalid", "model-source", "bad");
                    if let PreparedWrite::Node { kind, .. } = &mut bad {
                        *kind = "".into();
                    }
                    assert!(opened
                        .engine
                        .write(&[node("rejected", "model-source", "rejected"), bad])
                        .is_err());
                    assert_eq!(raw_counts(&path).0, before);
                }
                2 => {
                    opened.engine.drain(10_000).unwrap();
                }
                _ => {
                    let body = format!("modelneedle seed{seed} step{step} café");
                    opened.engine.write(&[node(&id, "model-source", &body)]).unwrap();
                    expected.insert(id, body);
                    revisions += 1;
                }
            }
            let observed = opened
                .engine
                .search_text_only("modelneedle")
                .unwrap()
                .results
                .into_iter()
                .map(|hit| (hit.id.value, hit.body))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(observed, expected, "seed={seed} shortest failing prefix={}", step + 1);
            for (id, body) in &expected {
                assert_eq!(
                    &opened.engine.read_get(id, &ReadView::default()).unwrap().unwrap().body,
                    body,
                    "seed={seed} prefix={}",
                    step + 1
                );
            }
            let history = opened
                .engine
                .search_text_only_view(
                    "modelneedle",
                    &ReadView { include_superseded: true, ..ReadView::default() },
                )
                .unwrap();
            assert_eq!(history.results.len(), revisions, "seed={seed} prefix={}", step + 1);
        }
        opened.engine.drain(10_000).unwrap();
        for id in expected.keys() {
            let row = opened.engine.read_get(id, &ReadView::default()).unwrap().unwrap();
            assert!(opened.engine.has_vector_row_for_cursor_for_test(row.write_cursor).unwrap());
        }
        opened.engine.close().unwrap();
    }
}

#[test]
fn held_reader_does_useful_work_as_wal_grows_then_checkpoint_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("held.sqlite");
    let opened = open(&path);
    opened.engine.write(&[node("control", "source", "independent control")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let checkpoint = Connection::open(&path).unwrap();
    checkpoint.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    let mut reader = Connection::open(&path).unwrap();
    let tx = reader.transaction().unwrap();
    assert_eq!(
        tx.query_row("SELECT body FROM canonical_nodes WHERE logical_id='control'", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "independent control"
    );
    let before = std::fs::metadata(format!("{}-wal", path.display())).unwrap().len();
    for i in 0..8 {
        opened
            .engine
            .write(&[node(
                &format!("wal:{i}"),
                "source",
                &format!("useful persisted {i} {}", "é".repeat(4096)),
            )])
            .unwrap();
        assert_eq!(
            tx.query_row("SELECT count(*) FROM canonical_nodes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    assert!(std::fs::metadata(format!("{}-wal", path.display())).unwrap().len() > before);
    checkpoint.busy_timeout(Duration::ZERO).unwrap();
    let busy: i64 =
        checkpoint.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get(0)).unwrap();
    tx.commit().unwrap();
    drop(reader);
    assert_eq!(busy, 1);
    opened.engine.close().unwrap();
    drop(opened);
    assert_eq!(
        checkpoint
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(checkpoint);
    let reopened = open(&path);
    assert_eq!(raw_counts(&path).0, 9);
    for i in 0..8 {
        assert_eq!(
            reopened
                .engine
                .read_get(&format!("wal:{i}"), &ReadView::default())
                .unwrap()
                .unwrap()
                .body,
            format!("useful persisted {i} {}", "é".repeat(4096))
        );
    }
    reopened.engine.close().unwrap();
}

use fathomdb_engine::*;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path, sync::Arc};

pub(super) fn node(id: &str, source: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body,
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}
struct Fixed;
impl fathomdb_embedder_api::Embedder for Fixed {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("slice85", "fixed", 8)
    }
    fn embed(&self, _: &str) -> Result<Vec<f32>, fathomdb_embedder_api::EmbedderError> {
        Ok(vec![1.; 8])
    }
}
pub(super) const MARKER: &str = "qzxvphysicalscopeeightyfive";
pub(super) const TABLES: [&str; 4] =
    ["search_index", "search_index_v2", "search_index_edges", "property_search_index"];
pub(super) fn rich_fixture(path: &std::path::Path, targets: bool) -> Arc<Engine> {
    use sha2::Digest;
    let engine = Arc::new(
        Engine::open_with_test_choice_and_config_for_test(
            path,
            TestEmbedderChoice::Caller(Arc::new(Fixed)),
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "text".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .write(&[node("left", "kept", "left".into()), node("right", "kept", "right".into())])
        .unwrap();
    if targets {
        let body = serde_json::json!({"text":format!("{MARKER} canonical")}).to_string();
        let hash = sha2::Sha256::digest(body.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let provenance_node = |id: &str, source: &str, body: String, provenance| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body,
                source_id: SourceId::new(source).unwrap(),
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
                provenance,
            })
        };
        engine
            .write(&[
                provenance_node(
                    "root",
                    "target",
                    body,
                    WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("source-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                    ),
                ),
                provenance_node(
                    "dependent",
                    "target",
                    serde_json::json!({"text":format!("{MARKER} dependent")}).to_string(),
                    WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("derived-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(hash).unwrap(),
                    ),
                ),
            ])
            .unwrap();
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1")
                    .unwrap(),
            )
            .unwrap();
    }
    for version in 0..3 {
        let mut batch = vec![node(
            "kept-history",
            "kept",
            serde_json::json!({"text":format!("survivorneedle history{version}")}).to_string(),
        )];
        if targets {
            batch.push(node(
                "target-history",
                "target",
                serde_json::json!({"text":format!("{MARKER} history{version}")}).to_string(),
            ));
            batch.push(PreparedWrite::Edge {
                kind: "rel".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("target").unwrap(),
                logical_id: Some("target-edge".into()),
                body: Some(format!("{MARKER} edge{version}")),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            });
        }
        engine.write(&batch).unwrap();
    }
    engine.drain(10000).unwrap();
    engine
}

pub(super) fn minimal_fixture(path: &Path) -> Arc<Engine> {
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine.write(&[node("root", "target", MARKER.into())]).unwrap();
    for revision in 0..3 {
        engine
            .write(&[node("kept-history", "kept", format!("survivorneedle history{revision}"))])
            .unwrap();
    }
    engine
}

pub(super) fn rich(path: &Path, targets: bool) -> Arc<Engine> {
    let engine = rich_fixture(path, targets);
    for revision in 0..3 {
        engine
            .write(&[PreparedWrite::Edge {
                kind: "rel".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("kept").unwrap(),
                logical_id: Some("kept-edge".into()),
                body: Some(format!("survivorneedle edge{revision}")),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            }])
            .unwrap();
    }
    engine.drain(10000).unwrap();

    engine
}

pub(super) fn count(c: &Connection, sql: &str) -> i64 {
    c.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn integers(c: &Connection, sql: &str, arg: &str) -> Vec<i64> {
    let mut statement = c.prepare(sql).unwrap();
    let result =
        statement.query_map([arg], |r| r.get(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    result
}

fn rows(c: &Connection, sql: &str, arg: &str) -> Vec<Value> {
    use rusqlite::types::ValueRef;
    let mut statement = c.prepare(sql).unwrap();
    let columns = statement.column_count();
    let result = statement
        .query_map([arg], |row| {
            let mut values = Vec::new();
            for i in 0..columns {
                values.push(match row.get_ref(i)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(n) => json!({"integer": n}),
                    ValueRef::Real(n) => json!({"real_bits": n.to_bits()}),
                    ValueRef::Text(t) => json!({"text": std::str::from_utf8(t).unwrap()}),
                    ValueRef::Blob(b) => json!({"blob": b}),
                });
            }
            Ok(Value::Array(values))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    result
}

pub(super) fn scores(engine: &Engine) -> Vec<(String, u64)> {
    engine
        .search_explained("survivorneedle", None, 0, false, 0.3, 0)
        .unwrap()
        .results
        .into_iter()
        .map(|hit| (hit.id.to_prefixed(), hit.score.to_bits()))
        .collect()
}

#[derive(Serialize, Deserialize)]
pub(super) struct Preimage {
    pub nonce: String,
    pub ticket: String,
    pub rich: bool,
    pub request: Value,
    pub targets: Vec<i64>,
    pub vectors: Vec<i64>,
    pub survivors: Vec<i64>,
    pub survivor_rows: Vec<(String, Vec<Value>)>,
    pub target_rows: Vec<(String, Vec<Value>)>,
    pub intact_scores_pre_stress: Vec<(String, u64)>,
    pub non_head_stress: bool,
    pub survivor_only_scores: Vec<(String, u64)>,
}

fn rows_for_cursors(c: &Connection, cursors: &[i64]) -> Vec<(String, Vec<Value>)> {
    let encoded = serde_json::to_string(cursors).unwrap();
    let mut result = Vec::new();
    for table in [
        "canonical_nodes",
        "canonical_edges",
        "canonical_attributes",
        "search_index",
        "search_index_v2",
        "search_index_edges",
        "property_search_index",
        "_fathomdb_vector_rows",
        "_fathomdb_vector_spans",
    ] {
        let sql = format!("SELECT rowid,* FROM {table} WHERE write_cursor IN (SELECT value FROM json_each(?1)) ORDER BY rowid");
        result.push((table.into(), rows(c, &sql, &encoded)));
    }
    let vectors = integers(c, "SELECT rowid FROM _fathomdb_vector_rows WHERE write_cursor IN (SELECT value FROM json_each(?1)) ORDER BY rowid", &encoded);
    if !vectors.is_empty() {
        result.push(("vector_default".into(), rows(c, "SELECT rowid,* FROM vector_default WHERE rowid IN (SELECT value FROM json_each(?1)) ORDER BY rowid", &serde_json::to_string(&vectors).unwrap())));
    }
    result
}

pub(super) fn request(c: &Connection, ticket: &str) -> Value {
    let raw: String = c
        .query_row(
            "SELECT payload FROM _fathomdb_erasure_requests WHERE ticket=?1",
            [ticket],
            |r| r.get(0),
        )
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}

pub(super) fn root(c: &Connection) -> Option<Value> {
    use rusqlite::OptionalExtension;
    let raw: Option<String> = c
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'",
            [],
            |r| r.get(0),
        )
        .optional()
        .unwrap();
    raw.map(|s| serde_json::from_str(&s).unwrap())
}

pub(super) fn capture(
    engine: &Engine,
    nonce: String,
    ticket: String,
    rich: bool,
    baseline: Vec<(String, u64)>,
    intact_scores_pre_stress: Vec<(String, u64)>,
    non_head_stress: bool,
) -> Preimage {
    let c = Connection::open(engine.path()).unwrap();
    assert_eq!(
        count(&c, "SELECT count(*) FROM canonical_nodes WHERE source_id='target'"),
        if rich { 5 } else { 1 }
    );
    assert_eq!(
        count(&c, "SELECT count(*) FROM canonical_edges WHERE source_id='target'"),
        if rich { 3 } else { 0 }
    );
    assert_eq!(
        count(&c, "SELECT count(*) FROM canonical_nodes WHERE source_id='kept'"),
        if rich { 5 } else { 3 }
    );
    if rich {
        for table in TABLES {
            let positive: i64 = c
                .query_row(
                    &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                    [MARKER],
                    |r| r.get(0),
                )
                .unwrap();
            assert!(positive > 0, "positive target family {table}");
        }
    }
    let targets = integers(&c, "SELECT write_cursor FROM canonical_nodes WHERE source_id=?1 UNION SELECT write_cursor FROM canonical_edges WHERE source_id=?1 ORDER BY write_cursor", "target");
    let survivors = integers(&c, "SELECT write_cursor FROM canonical_nodes WHERE source_id=?1 UNION SELECT write_cursor FROM canonical_edges WHERE source_id=?1 ORDER BY write_cursor", "kept");
    let encoded = serde_json::to_string(&targets).unwrap();
    let vectors = integers(&c, "SELECT rowid FROM _fathomdb_vector_rows WHERE write_cursor IN (SELECT value FROM json_each(?1)) ORDER BY rowid", &encoded);
    if rich {
        assert!(!vectors.is_empty(), "positive mapped head-vector population even without stress");
        let spans: i64 = c.query_row("SELECT count(*) FROM _fathomdb_vector_spans WHERE write_cursor IN (SELECT value FROM json_each(?1))", [&encoded], |r| r.get(0)).unwrap();
        assert!(spans > 0, "positive target span population");
        if !non_head_stress {
            let invalid: i64 = c.query_row("SELECT count(*) FROM _fathomdb_vector_rows r LEFT JOIN _fathomdb_vector_spans s ON s.vector_rowid=r.rowid WHERE r.write_cursor IN (SELECT value FROM json_each(?1)) AND (s.vector_rowid IS NULL OR r.rowid!=r.write_cursor OR s.write_cursor!=r.write_cursor OR s.window_number!=0 OR s.byte_start!=0)", [&encoded], |r| r.get(0)).unwrap();
            assert_eq!(invalid, 0, "rollback retains positive valid head-vector/map/span coverage");
        }
    }
    assert_eq!(vectors.contains(&900000), non_head_stress);
    if non_head_stress {
        assert_eq!(count(&c, "SELECT count(*) FROM _fathomdb_vector_spans WHERE vector_rowid=900000 AND window_number=1"), 1);
    }

    let mut marker = false;
    let mut survivor = false;
    for suffix in ["", "-wal"] {
        let bytes =
            std::fs::read(format!("{}{suffix}", engine.path().display())).unwrap_or_default();
        marker |= bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes());
        survivor |= bytes.windows("survivorneedle".len()).any(|w| w == b"survivorneedle");
    }
    assert!(marker && survivor, "positive raw population before fault");
    Preimage {
        nonce,
        request: request(&c, &ticket),
        ticket,
        rich,
        target_rows: rows_for_cursors(&c, &targets),
        targets,
        vectors,
        survivor_rows: rows_for_cursors(&c, &survivors),
        survivors,
        intact_scores_pre_stress,
        non_head_stress,
        survivor_only_scores: baseline,
    }
}

pub(super) fn read_only(path: &Path) -> Connection {
    let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    c.pragma_update(None, "query_only", true).unwrap();
    c
}

pub(super) fn assert_survivors(c: &Connection, pre: &Preimage) {
    assert_eq!(
        rows_for_cursors(c, &pre.survivors),
        pre.survivor_rows,
        "exact survivor row/cursor/history identity"
    );
}

pub(super) fn assert_target_partition(c: &Connection, pre: &Preimage, present: bool) {
    let encoded = serde_json::to_string(&pre.targets).unwrap();
    let found = integers(c, "SELECT write_cursor FROM canonical_nodes WHERE source_id=?1 UNION SELECT write_cursor FROM canonical_edges WHERE source_id=?1 ORDER BY write_cursor", "target");
    assert_eq!(found, if present { pre.targets.clone() } else { Vec::new() });
    if present {
        assert_eq!(
            rows_for_cursors(c, &pre.targets),
            pre.target_rows,
            "rollback preserves every populated target row/span/vector"
        );
    }
    if !present {
        for table in [
            "canonical_attributes",
            "search_index",
            "search_index_v2",
            "search_index_edges",
            "property_search_index",
            "_fathomdb_vector_rows",
            "_fathomdb_vector_spans",
        ] {
            let n: i64 = c.query_row(&format!("SELECT count(*) FROM {table} WHERE write_cursor IN (SELECT value FROM json_each(?1))"), [&encoded], |r| r.get(0)).unwrap();
            assert_eq!(n, 0, "target cursor zero in {table}");
        }
        if pre.rich {
            for table in ["vector_default", "_fathomdb_vector_rows"] {
                let n: i64 = c.query_row(&format!("SELECT count(*) FROM {table} WHERE rowid IN (SELECT value FROM json_each(?1))"), [serde_json::to_string(&pre.vectors).unwrap()], |r| r.get(0)).unwrap();
                assert_eq!(n, 0, "captured target vector IDs in {table}");
            }
            let n: i64 = c.query_row("SELECT count(*) FROM _fathomdb_vector_spans WHERE vector_rowid IN (SELECT value FROM json_each(?1))", [serde_json::to_string(&pre.vectors).unwrap()], |r| r.get(0)).unwrap();
            assert_eq!(n, 0, "explicit non-head/span zero");
        }
    }
}

pub(super) fn assert_phase(c: &Connection, pre: &Preimage, phase: &str) {
    let req = request(c, &pre.ticket);
    assert_eq!(req["status"]["phase"], phase);
    assert_eq!(
        req["status"]["logical_state"],
        if phase == "queued" { "unchanged_by_this_request" } else { "removed" }
    );
    assert_eq!(
        count(c, "SELECT coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1"),
        0
    );
    if matches!(phase, "sanitizing" | "finalizing" | "complete") {
        let state: String = c
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_fts_sanitation'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let state: Value = serde_json::from_str(&state).unwrap();
        assert_eq!(state["phase"], if phase == "complete" { "clean" } else { "finalizing" });
        assert_eq!(state["family"], TABLES.len());
    }
    if phase == "queued" {
        assert!(root(c).is_none());
    } else if phase == "complete" {
        assert!(root(c).is_none());
        assert_eq!(count(c, "SELECT count(*) FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased')"), 0);
    } else {
        let debt = root(c).unwrap();
        assert_eq!(debt["ticket"], pre.ticket);
        if phase == "finalizing" {
            assert_eq!(debt["redacted"], true);
            assert_eq!(debt["retry"], "");
            for field in ["cursors", "vectors", "denied_sources", "denied_identities"] {
                assert_eq!(debt[field], json!([]), "identity-free root {field}");
            }
        } else {
            assert_eq!(debt["redacted"], false);
            assert!(!debt["retry"].as_str().unwrap().is_empty());
            assert_eq!(debt["cursors"], json!(pre.targets));
        }
    }
    if matches!(phase, "finalizing" | "complete") {
        assert!(req["selector"].is_null());
        assert_eq!(req["cut"], 0);
        for field in ["cursors", "sources", "identities"] {
            assert_eq!(req["scope"][field], json!([]));
        }
        assert_eq!(req["originals"], json!([]));
    } else {
        for field in ["selector", "cut", "scope", "originals", "key", "ticket"] {
            assert_eq!(req[field], pre.request[field], "immutable accepted {field}");
        }
    }
}

pub(super) fn complete(engine: &Arc<Engine>, pre: &Preimage, sink: Option<&Path>, output: &Path) {
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    if let Some(sink) = sink {
        handle.reattach_original_erasure_sink(&pre.ticket, sink.to_str().unwrap()).unwrap();
    }
    assert!(matches!(
        handle.complete_physical_erasure(&pre.ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    let c = read_only(engine.path());
    assert_phase(&c, pre, "complete");
    assert_survivors(&c, pre);
    assert_target_partition(&c, pre, false);
    assert_eq!(
        count(&c, "SELECT count(*) FROM _fathomdb_erasure_requests WHERE phase='complete'"),
        1
    );
    if pre.rich {
        assert_eq!(scores(engine), pre.survivor_only_scores);
    }
    drop(c);
    std::fs::create_dir(output).unwrap();
    let mut retained = false;
    for suffix in ["", "-wal"] {
        let path = format!("{}{suffix}", engine.path().display());
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if suffix == "-wal" && e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => panic!("raw proof unavailable: {e}"),
        };
        assert!(
            !bytes.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
            "target bytes before close: {suffix}"
        );
        retained |= bytes.windows("survivorneedle".len()).any(|w| w == b"survivorneedle");
        std::fs::write(output.join(if suffix.is_empty() { "main" } else { "wal" }), bytes).unwrap();
    }
    assert!(retained);
}

pub(super) fn reopen(path: &Path, rich: bool) -> Arc<Engine> {
    let config = EngineConfig {
        physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
        ..Default::default()
    };
    if rich {
        Arc::new(
            Engine::open_with_test_choice_and_config_for_test(
                path,
                TestEmbedderChoice::Caller(Arc::new(Fixed)),
                config,
            )
            .unwrap()
            .engine,
        )
    } else {
        Arc::new(
            Engine::open_with_choice_and_config(path, EmbedderChoice::None, config).unwrap().engine,
        )
    }
}

pub(super) fn inject_non_head(path: &Path) {
    let c = Connection::open(path).unwrap();
    let head: i64 = c.query_row("SELECT v.rowid FROM _fathomdb_vector_rows v JOIN canonical_nodes n ON n.write_cursor=v.write_cursor WHERE n.source_id='target' ORDER BY v.rowid LIMIT 1", [], |r| r.get(0)).unwrap();
    c.execute_batch(&format!("CREATE TEMP TABLE scope_vector_copy AS SELECT * FROM vector_default WHERE rowid={head}; INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status,lifecycle,valid_from,valid_until) SELECT 900000,embedding,vec_bit(embedding_bin),source_type,kind,created_at,status,lifecycle,valid_from,valid_until FROM scope_vector_copy; DROP TABLE scope_vector_copy; INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) SELECT 900000,kind,write_cursor FROM _fathomdb_vector_rows WHERE rowid={head}; INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length) SELECT 900000,write_cursor,(SELECT COALESCE(MAX(window_number),0)+1 FROM _fathomdb_vector_spans v WHERE v.write_cursor=s.write_cursor),byte_start,byte_end,body_length FROM _fathomdb_vector_spans s WHERE vector_rowid={head};")).unwrap();
}

use super::*;
use crate::fusion::RecordKey;
use crate::stored_coverage::StoredHeadCoverageUnavailableReason as Reason;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use rusqlite::hooks::{AuthAction, AuthContext, Authorization, TransactionOperation};
use rusqlite::trace::{TraceEvent, TraceEventCodes};
use std::collections::BTreeMap;
use std::time::Instant;

struct Provider(Option<usize>);
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("stored-coverage", "test", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
    fn truncation_offset(&self, _: &str) -> Result<Option<usize>, EmbedderError> {
        Ok(self.0)
    }
}

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        logical_id: Some(id.into()),
        source_id: SourceId::new("test:stored-coverage").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn fixture(count: usize) -> (tempfile::TempDir, OpenedEngine) {
    let directory = tempfile::tempdir().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        directory.path().join("coverage.sqlite"),
        Arc::new(Provider(Some(4))),
    )
    .unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    if count > 0 {
        opened
            .engine
            .write(&(0..count).map(|i| node(&format!("n{i}"), "abcdefghij")).collect::<Vec<_>>())
            .unwrap();
        opened.engine.drain(10_000).unwrap();
    }
    (directory, opened)
}

fn unavailable(engine: &Engine, reason: Reason) {
    assert!(matches!(engine.read_projection_status().unwrap().vector_coverage,
        StoredHeadCoverageV1::Unavailable{reason:actual,..} if actual==reason));
}

fn available(engine: &Engine, records: u64) {
    match engine.read_projection_status().unwrap().vector_coverage {
        StoredHeadCoverageV1::Available { counts, .. } => assert_eq!(counts.records, records),
        other => panic!("expected committed coverage, got {other:?}"),
    }
}

#[test]
fn stored_coverage_cache_detects_local_rollback_external_schema_and_profile_changes() {
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    engine.read_projection_generation_status().unwrap();
    available(engine, 1);
    {
        let mut guard = engine.connection.lock().unwrap();
        let tx = guard.as_mut().unwrap().transaction().unwrap();
        tx.execute(
            "UPDATE _fathomdb_open_state SET value=value WHERE key='_fathomdb_database_id'",
            [],
        )
        .unwrap();
        tx.rollback().unwrap();
    }
    unavailable(engine, Reason::StateChanged);
    let external = Connection::open(&engine.path).unwrap();
    external
        .execute("UPDATE canonical_nodes SET reason='external' WHERE logical_id='n0'", [])
        .unwrap();
    unavailable(engine, Reason::StateChanged);
    engine.read_projection_generation_status().unwrap();
    available(engine, 1);
    external.execute_batch("CREATE TABLE coverage_probe(value INTEGER)").unwrap();
    unavailable(engine, Reason::StateChanged);
    external
        .execute(
            "UPDATE _fathomdb_embedder_profiles SET revision='different' WHERE profile='default'",
            [],
        )
        .unwrap();
    unavailable(engine, Reason::StateChanged);
    external.execute_batch("PRAGMA user_version=36").unwrap();
    assert!(
        engine.read_projection_status().is_err(),
        "unsupported token domain is error, not availability"
    );
    external.execute_batch("PRAGMA user_version=37").unwrap();
}

#[test]
fn stored_coverage_failed_commit_cannot_publish_a_staged_capture() {
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    engine.read_projection_generation_status().unwrap();
    let original = engine.stored_head_coverage.lock().unwrap().clone().unwrap();
    {
        let mut guard = engine.connection.lock().unwrap();
        let connection = guard.as_mut().unwrap();
        connection
            .execute("UPDATE canonical_nodes SET reason='new snapshot' WHERE logical_id='n0'", [])
            .unwrap();
        connection.authorizer(Some(|context: AuthContext<'_>| match context.action {
            AuthAction::Transaction { operation: TransactionOperation::Unknown } => {
                Authorization::Deny
            }
            _ => Authorization::Allow,
        }));
    }
    assert!(engine.read_projection_generation_status().is_err());
    {
        let mut guard = engine.connection.lock().unwrap();
        guard.as_mut().unwrap().authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    }
    assert_eq!(engine.stored_head_coverage.lock().unwrap().as_ref().unwrap().token, original.token);
    unavailable(engine, Reason::StateChanged);
}

#[test]
fn stored_coverage_reopen_refresh_and_cache_poison_remain_distinct() {
    let (_directory, opened) = fixture(0);
    let engine = &opened.engine;
    unavailable(engine, Reason::CacheCold);
    engine.read_projection_generation_status().unwrap();
    available(engine, 0);
    let fingerprint = |engine: &Engine| -> Option<String> {
        let guard = engine.connection.lock().unwrap();
        guard.as_ref().unwrap().query_row("SELECT value FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'", [], |row| row.get(0)).optional().unwrap()
    };
    assert!(fingerprint(engine).is_none());
    let path = engine.path.clone();
    engine.close().unwrap();
    let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider(Some(4)))).unwrap();
    // The first registered open initializes the existing provider probe cache.
    assert!(fingerprint(&reopened.engine).is_some());
    unavailable(&reopened.engine, Reason::CacheCold);
    reopened.engine.read_projection_generation_status().unwrap();
    available(&reopened.engine, 0);
    reopened.engine.close().unwrap();
    let stable = Engine::open_with_embedder_for_test(&path, Arc::new(Provider(Some(4)))).unwrap();
    available(&stable.engine, 0);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = stable.engine.stored_head_coverage.lock().unwrap();
        panic!("poison only this fixture's coverage cache");
    }));
    assert!(stable.engine.read_projection_status().is_err());
}

#[test]
fn stored_coverage_attribution_exclusions_never_hide_original_corruption() {
    let (_directory, opened) = fixture(1);
    let path = opened.engine.path.clone();
    let cursor = opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "link".into(),
            from: "n0".into(),
            to: "n0".into(),
            source_id: SourceId::new("test:stored-coverage").unwrap(),
            logical_id: Some("bodyless".into()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap()
        .row_cursors[0];
    opened.engine.close().unwrap();
    let connection = Connection::open(path).unwrap();
    connection.execute("DELETE FROM vector_default", []).unwrap();
    assert_eq!(
        crate::vector_spans::capture(&connection).unwrap().records,
        1,
        "aggregate is not physical-vector health"
    );
    for key in [cursor, 9999] {
        connection.execute("INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(?1,'edge_fact',?1)",[key]).unwrap();
        connection.execute("INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,body_length,byte_end) VALUES(?1,?1,0,0,0,NULL)",[key]).unwrap();
    }
    let counts = crate::vector_spans::capture(&connection).unwrap();
    assert_eq!((counts.records, counts.unattributed_heads), (1, 2));
    connection.execute("UPDATE canonical_edges SET write_cursor=(SELECT write_cursor FROM canonical_nodes WHERE logical_id='n0') WHERE logical_id='bodyless'",[]).unwrap();
    let counts = crate::vector_spans::capture(&connection).unwrap();
    assert_eq!((counts.records, counts.unattributed_heads), (0, 3));
    connection
        .execute("UPDATE _fathomdb_vector_spans SET body_length='bad' WHERE vector_rowid=9999", [])
        .unwrap();
    assert!(crate::vector_spans::capture(&connection).is_err());
    connection
        .execute("UPDATE _fathomdb_vector_spans SET body_length=0 WHERE vector_rowid=9999", [])
        .unwrap();
    connection.execute("INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,body_length,byte_end) VALUES(8888,8888,0,0,0,NULL)",[]).unwrap();
    assert!(crate::vector_spans::capture(&connection).is_err());
}

#[test]
fn scored_extent_refuses_missing_misplaced_extra_and_malformed_tuples() {
    let _serial = SEARCH_TRACE_LOCK.lock().unwrap();
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    let cursor = engine.read_projection_generation_status().unwrap().observed_boundary;
    let guard = engine.connection.lock().unwrap();
    let connection = guard.as_ref().unwrap();
    let key = RecordKey::CanonicalNode(cursor);
    assert_eq!(crate::vector_spans::scored_extent(connection, key, "abcdefghij").unwrap(), Some(4));
    assert!(crate::vector_spans::scored_extent(connection, key, "different!").is_err());
    assert!(crate::vector_spans::scored_extent(
        connection,
        RecordKey::CanonicalEdge(cursor),
        "abcdefghij"
    )
    .is_err());
    assert_eq!(
        crate::vector_spans::scored_extent(
            connection,
            RecordKey::OwnerlessNodeFts(cursor as i64),
            "abcdefghij"
        )
        .unwrap(),
        None
    );
    for sql in [
        "DELETE FROM _fathomdb_vector_rows",
        "DELETE FROM _fathomdb_vector_spans",
        "DELETE FROM vector_default",
        "UPDATE _fathomdb_vector_spans SET vector_rowid=9999",
        "UPDATE _fathomdb_vector_spans SET write_cursor=9999",
        "UPDATE _fathomdb_vector_spans SET window_number=1",
        "UPDATE _fathomdb_vector_spans SET byte_start=1",
        "UPDATE _fathomdb_vector_spans SET body_length=11",
        "UPDATE _fathomdb_vector_spans SET byte_end='bad'",
        "UPDATE _fathomdb_vector_spans SET byte_end=-1",
        "UPDATE _fathomdb_vector_spans SET body_length=10.5",
        "INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) SELECT 9999,kind,write_cursor FROM _fathomdb_vector_rows",
        "INSERT INTO _fathomdb_vector_spans(vector_rowid,write_cursor,window_number,byte_start,body_length,byte_end) SELECT 9999,write_cursor,1,0,body_length,byte_end FROM _fathomdb_vector_spans",
    ] {
        connection.execute_batch("PRAGMA ignore_check_constraints=ON; SAVEPOINT corrupt_tuple").unwrap();
        connection.execute_batch(sql).unwrap();
        assert!(crate::vector_spans::scored_extent(connection,key,"abcdefghij").is_err(),"{sql}");
        connection.execute_batch("ROLLBACK TO corrupt_tuple; RELEASE corrupt_tuple; PRAGMA ignore_check_constraints=OFF").unwrap();
    }
    connection.execute("UPDATE _fathomdb_vector_spans SET byte_end=NULL", []).unwrap();
    assert_eq!(crate::vector_spans::scored_extent(connection, key, "abcdefghij").unwrap(), None);
    connection.execute("DELETE FROM vector_default", []).unwrap();
    assert!(
        crate::vector_spans::scored_extent(connection, key, "abcdefghij").is_err(),
        "NULL cannot hide a missing physical vector"
    );
}

#[derive(Default)]
struct Trace {
    starts: BTreeMap<String, i32>,
    rows: BTreeMap<String, usize>,
    completed: Vec<(String, i32, usize)>,
}

fn start_trace(engine: &Engine) -> Arc<Mutex<Trace>> {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let recorded = Arc::clone(&trace);
    let mut guard = engine.connection.lock().unwrap();
    guard
        .as_mut()
        .unwrap()
        .trace_v2(
            TraceEventCodes::SQLITE_TRACE_STMT
                | TraceEventCodes::SQLITE_TRACE_PROFILE
                | TraceEventCodes::SQLITE_TRACE_ROW,
            Some(move |event: TraceEvent<'_>| {
                let mut trace = recorded.lock().unwrap();
                match event {
                    TraceEvent::Stmt(statement, _) => {
                        trace.starts.insert(
                            statement.sql().into_owned(),
                            statement.get_status(rusqlite::StatementStatus::VmStep),
                        );
                    }
                    TraceEvent::Row(statement) => {
                        *trace.rows.entry(statement.sql().into_owned()).or_default() += 1;
                    }
                    TraceEvent::Profile(statement, _) => {
                        let sql = statement.sql().into_owned();
                        let start = trace.starts.remove(&sql).unwrap_or(0);
                        let rows = trace.rows.remove(&sql).unwrap_or(0);
                        trace.completed.push((
                            sql,
                            statement.get_status(rusqlite::StatementStatus::VmStep) - start,
                            rows,
                        ));
                    }
                    _ => {}
                }
            }),
        )
        .unwrap();
    trace
}

#[test]
fn stored_coverage_work_is_fixed_on_status_and_streamed_in_existing_validation() {
    let sizes: Vec<usize> = std::env::var("FATHOMDB_COVERAGE_WORK_SIZES")
        .unwrap_or_else(|_| "8,64".into())
        .split(',')
        .map(|n| n.parse().unwrap())
        .collect();
    assert!(
        sizes.len() >= 2 && sizes.iter().all(|n| *n > 0) && sizes.windows(2).all(|n| n[0] < n[1])
    );
    let mut status_cells = Vec::new();
    for count in sizes {
        let (_directory, opened) = fixture(count);
        let engine = &opened.engine;
        engine.read_projection_generation_status().unwrap();
        let retained = engine.stored_head_coverage.lock().unwrap().clone();
        let trace = start_trace(engine);
        for cold in [true, false] {
            *engine.stored_head_coverage.lock().unwrap() =
                if cold { None } else { retained.clone() };
            engine.read_projection_status().unwrap();
            let observations = std::mem::take(&mut trace.lock().unwrap().completed);
            assert!(observations.iter().all(|(sql, _, _)| !sql.contains("canonical_")
                && !sql.contains("vector_default")
                && !sql.contains("_fathomdb_vector_spans")));
            let vm: i32 = observations.iter().map(|(_, vm, _)| vm).sum();
            let rows: usize = observations.iter().map(|(_, _, rows)| rows).sum();
            assert!(vm > 0);
            status_cells.push((observations.len(), vm, rows));
            eprintln!("coverage_status records={count} cold={cold} statements={} vm_steps={vm} rows={rows}",observations.len());
        }
        let guard = engine.connection.lock().unwrap();
        let connection = guard.as_ref().unwrap();
        let old = Instant::now();
        crate::vector_spans::validate(connection, None).unwrap();
        let old_us = old.elapsed().as_micros();
        let baseline = std::mem::take(&mut trace.lock().unwrap().completed);
        let began = Instant::now();
        let counts = crate::vector_spans::capture(connection).unwrap();
        let capture_us = began.elapsed().as_micros();
        let captured = std::mem::take(&mut trace.lock().unwrap().completed);
        assert_eq!(counts.records, count as u64);
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].2, count);
        assert!(!captured[0].0.contains("vector_default"));
        eprintln!("coverage_capture records={count} statements={} rows={} vm_steps={} baseline_vm_steps={} instrumented_capture_us={capture_us} instrumented_baseline_us={old_us}",captured.len(),captured[0].2,captured[0].1,baseline.iter().map(|(_,vm,_)|vm).sum::<i32>());
    }
    assert!(
        status_cells.windows(2).all(|cells| cells[0] == cells[1]),
        "fixed-work status cells: {status_cells:?}"
    );
}

static SEARCH_TRACE_LOCK: Mutex<()> = Mutex::new(());

fn search(engine: &Engine, query: &str, graph: bool, explain: bool, snippet: bool) -> SearchResult {
    engine
        .search_reranked_view_with_snippets(
            query,
            None,
            0,
            graph,
            0.3,
            0,
            explain,
            &ReadView::default(),
            10,
            snippet.then_some(&SnippetOptions::default()),
        )
        .unwrap()
}

fn extent_queries() -> usize {
    take_slice71_search_statement_trace_for_test()
        .iter()
        .filter(|event| event.as_str() == "snippet_scored_extent")
        .count()
}

#[test]
fn scored_extent_equal_bodies_and_repeated_public_refs_keep_exact_record_endpoints() {
    let _serial = SEARCH_TRACE_LOCK.lock().unwrap();
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    let receipt = engine
        .write(&[
            node("shared", "abcdefghij"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "shared".into(),
                to: "n0".into(),
                body: Some("abcdefghij".into()),
                logical_id: Some("shared".into()),
                source_id: SourceId::new("test:stored-coverage").unwrap(),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    {
        let guard = engine.connection.lock().unwrap();
        guard
            .as_ref()
            .unwrap()
            .execute(
                "UPDATE _fathomdb_vector_spans SET byte_end=9 WHERE write_cursor=?1",
                [receipt.row_cursors[1]],
            )
            .unwrap();
    }
    take_slice71_search_statement_trace_for_test();
    let result = search(engine, "abcdefghij", false, true, true);
    assert_eq!(result.results.iter().filter(|hit| hit.id.value == "shared").count(), 2);
    for ((hit, snippet), detail) in result
        .results
        .iter()
        .zip(result.snippets.as_ref().unwrap())
        .zip(&result.explanation.as_ref().unwrap().per_hit)
    {
        assert!(detail.vector_rank.is_some());
        assert_eq!(
            snippet.scored_bytes,
            Some(if hit.write_cursor == receipt.row_cursors[1] { 9 } else { 4 })
        );
    }
    assert_eq!(extent_queries(), result.results.len());
    let result = search(engine, "abcdefghij", false, false, true);
    for (hit, snippet) in result.results.iter().zip(result.snippets.as_ref().unwrap()) {
        assert_eq!(
            snippet.scored_bytes,
            Some(if hit.write_cursor == receipt.row_cursors[1] { 9 } else { 4 })
        );
    }
}

#[test]
fn scored_extent_pinned_reader_cannot_take_a_later_endpoint() {
    let _serial = SEARCH_TRACE_LOCK.lock().unwrap();
    let (_directory, opened) = fixture(1);
    let engine = Arc::new(opened.engine);
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    arm_frozen_after_validation_hook_for_test(
        &engine,
        Box::new(move || {
            ready_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        }),
    );
    let candidate = Arc::clone(&engine);
    let pinned = frozen.clone();
    let worker = std::thread::spawn(move || {
        candidate.search_frozen_with_snippets(
            "query",
            &pinned,
            0,
            false,
            0.3,
            0,
            false,
            10,
            Some(&SnippetOptions::default()),
        )
    });
    ready_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let external = Connection::open(&engine.path).unwrap();
    let changed = external.execute("UPDATE _fathomdb_vector_spans SET byte_end=9", []);
    release_tx.send(()).unwrap();
    changed.unwrap();
    match worker.join().unwrap() {
        Ok(result) => {
            assert_eq!(result.results.len(), 1);
            assert_eq!(result.snippets.unwrap()[0].scored_bytes, Some(4));
        }
        Err(error) => assert!(matches!(error, EngineError::FrozenRead(_)), "{error:?}"),
    }
    assert!(matches!(
        engine.search_frozen_with_snippets(
            "query",
            &frozen,
            0,
            false,
            0.3,
            0,
            false,
            10,
            Some(&SnippetOptions::default())
        ),
        Err(EngineError::FrozenRead(_))
    ));
    assert_eq!(
        search(&engine, "query", false, false, true).snippets.unwrap()[0].scored_bytes,
        Some(9)
    );
}

#[test]
fn scored_extent_ownerless_collision_and_opt_out_make_no_extra_reads() {
    let _serial = SEARCH_TRACE_LOCK.lock().unwrap();
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    {
        let guard = engine.connection.lock().unwrap();
        let connection = guard.as_ref().unwrap();
        let cursor: u64 = connection
            .query_row(
                "SELECT write_cursor FROM canonical_nodes WHERE logical_id='n0'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        connection
            .execute("UPDATE search_index SET rowid=1000 WHERE write_cursor=?1", [cursor])
            .unwrap();
        connection.execute("INSERT INTO search_index(rowid,body,kind,write_cursor) VALUES(?1,'ownerlessneedle','doc',9999)",[cursor]).unwrap();
    }
    take_slice71_search_statement_trace_for_test();
    // Structural explanations require a canonical lifecycle owner, so the
    // existing ownerless route is exercised without that separate sidecar.
    let hybrid = search(engine, "ownerlessneedle", false, false, true);
    let at = hybrid.results.iter().position(|hit| hit.body == "ownerlessneedle").unwrap();
    assert_eq!(hybrid.snippets.as_ref().unwrap()[at].scored_bytes, None);
    let canonical = hybrid.results.iter().position(|hit| hit.id.value == "n0").unwrap();
    assert_eq!(hybrid.snippets.as_ref().unwrap()[canonical].scored_bytes, Some(4));
    assert_eq!(hybrid.results.len(), 2);
    let participants = 1;
    assert_eq!(extent_queries(), participants);
    let off = search(engine, "ownerlessneedle", false, false, false);
    assert!(off.snippets.is_none());
    assert_eq!(extent_queries(), 0);
    let text = engine
        .search_text_only_view_with_snippets(
            "ownerlessneedle",
            &ReadView::default(),
            10,
            Some(&SnippetOptions::default()),
        )
        .unwrap();
    assert_eq!(text.results.len(), 1);
    assert_eq!(text.snippets.unwrap()[0].scored_bytes, None);
    assert_eq!(extent_queries(), 0);
    assert_eq!(snippet_for("ownerlessneedle", &[], &SnippetOptions::default()).scored_bytes, None);
    assert_eq!(extent_queries(), 0);
}

struct DirectionalProvider;
impl Embedder for DirectionalProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("scored-graph", "test", 8)
    }
    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        let mut value = vec![-1.0; 8];
        value[usize::from(!(input == "zanzibar" || input.contains("vectorhit")))] = 1.0;
        Ok(value)
    }
    fn truncation_offset(&self, _: &str) -> Result<Option<usize>, EmbedderError> {
        Ok(Some(4))
    }
}

#[test]
fn scored_extent_nested_graph_cannot_manufacture_vector_participation() {
    let _serial = SEARCH_TRACE_LOCK.lock().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        directory.path().join("graph.sqlite"),
        Arc::new(DirectionalProvider),
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let mut writes: Vec<_> = (0..256)
        .map(|i| {
            node(
                &if i == 0 { "seed".into() } else { format!("near{i}") },
                &format!("vectorhit near {i}"),
            )
        })
        .collect();
    writes.push(node("lexical", "zanzibar lexical only"));
    writes.push(node("graph", "distant graph node"));
    for target in ["lexical", "graph"] {
        writes.push(PreparedWrite::Edge {
            kind: "link".into(),
            from: "seed".into(),
            to: target.into(),
            body: Some(format!("zanzibar bridge {target}")),
            logical_id: Some(format!("edge-{target}")),
            source_id: SourceId::new("test:stored-coverage").unwrap(),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        });
    }
    engine.write(&writes).unwrap();
    engine.drain(10_000).unwrap();
    take_slice71_search_statement_trace_for_test();
    let result = search(engine, "zanzibar", true, true, true);
    for name in ["lexical", "graph"] {
        let at = result
            .results
            .iter()
            .position(|hit| hit.id.value == name)
            .unwrap_or_else(|| panic!("{name} not reached: {:?}", result.results));
        let detail = &result.explanation.as_ref().unwrap().per_hit[at];
        assert_eq!(
            detail.vector_rank, None,
            "fixture must exclude {name} from original vector arm"
        );
        if name == "graph" {
            assert!(detail.graph_rank.is_some());
        } else {
            assert!(detail.text_rank.is_some());
        }
        assert_eq!(
            result.snippets.as_ref().unwrap()[at].scored_bytes,
            None,
            "nested fusion cannot lend participation"
        );
        let guard = engine.connection.lock().unwrap();
        assert_eq!(
            crate::vector_spans::scored_extent(
                guard.as_ref().unwrap(),
                RecordKey::CanonicalNode(result.results[at].write_cursor),
                &result.results[at].body
            )
            .unwrap(),
            Some(4),
            "negative control actually has a known stored vector head"
        );
    }
    let actual = extent_queries();
    let participants = result
        .explanation
        .as_ref()
        .unwrap()
        .per_hit
        .iter()
        .filter(|hit| hit.vector_rank.is_some())
        .count();
    assert_eq!(
        actual,
        participants + 2,
        "two explicit fixture checks are separate from the search's participant reads"
    );
    assert!(participants <= result.results.len());
}

#[test]
fn stored_coverage_rebuild_and_erasure_invalidate_committed_facts() {
    let (_directory, opened) = fixture(1);
    let engine = &opened.engine;
    engine.read_projection_generation_status().unwrap();
    available(engine, 1);
    engine.rebuild_vec0().unwrap();
    unavailable(engine, Reason::StateChanged);
    engine.read_projection_generation_status().unwrap();
    // The temporary test-only kind is absent from the persistent replay schema.
    available(engine, 0);
    engine.erase_source("test:stored-coverage").unwrap();
    unavailable(engine, Reason::StateChanged);
    engine.read_projection_generation_status().unwrap();
    available(engine, 0);
}

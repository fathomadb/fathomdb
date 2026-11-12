use fathomdb_engine::{EmbedderChoice, Engine};
use proptest::prelude::*;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn no_wal(path: &Path) {
    let wal = PathBuf::from(format!("{}-wal", path.display()));
    assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
}

fn inspect(path: &Path, records: &[Value]) -> Value {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let rows: Vec<(i64, String, String, String)> = connection
        .prepare("SELECT write_cursor,logical_id,kind,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows.len(), records.len());
    let mut kinds = serde_json::Map::new();
    let mut expected = Vec::new();
    for ((cursor, id, kind, body), record) in rows.iter().zip(records) {
        assert_eq!(id, record["id"].as_str().unwrap());
        assert_eq!(kind, record["kind"].as_str().unwrap());
        assert_eq!(body, record["body"].as_str().unwrap());
        let count = kinds.entry(kind.clone()).or_insert(json!(0));
        *count = json!(count.as_u64().unwrap() + 1);
        let (rowid, bytes, window, start, end, length): (i64, i64, i64, i64, Option<i64>, i64) = connection.query_row(
            "SELECT v.rowid,length(d.embedding),s.window_number,s.byte_start,s.byte_end,s.body_length FROM _fathomdb_vector_rows v JOIN vector_default d ON d.rowid=v.rowid JOIN _fathomdb_vector_spans s ON s.vector_rowid=v.rowid AND s.write_cursor=v.write_cursor WHERE v.write_cursor=?1 AND v.kind=?2",
            rusqlite::params![cursor, kind], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).unwrap();
        assert_eq!(
            (rowid, bytes, window, start, end, length),
            (*cursor, 3072, 0, 0, None, i64::try_from(body.len()).unwrap())
        );
        expected.push(json!({"id":id,"kind":kind,"write_cursor":cursor,"body_bytes":body.len()}));
    }
    for table in ["vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"] {
        let count: usize = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, records.len());
    }
    let profile: (String, String, u32) = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(profile, ("varying".into(), "perf-gates-dense".into(), 768));
    json!({"records":expected,"kind_counts":kinds,"physical_vectors":records.len(),"span_rows":records.len(),"pending_count":0,"profile":{"name":"varying","revision":"perf-gates-dense","dimension":768},"span_end":"unknown"})
}

fn verify(records: Vec<Value>) -> (Value, PathBuf) {
    let root = tempfile::Builder::new().prefix("s30-template-").tempdir().unwrap().keep();
    println!("template proof: {}", root.display());
    // SQLite runtime/vec registration must precede any independent connection.
    Engine::open_with_choice(root.join("registration.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary SHA256 before runtime: {}", digest(&binary));
    let config = root.join("config.json");
    std::fs::write(
        &config,
        serde_json::to_vec(&json!({"family":"D_TEMPLATE","records":records})).unwrap(),
    )
    .unwrap();
    let out = root.join("out");
    let mut child = Command::new(binary)
        .arg(config)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(30) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("template fixture timeout, NOT_RED");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "fixture failure NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let proof: Value =
        serde_json::from_slice(&std::fs::read(out.join("api-proof.json")).unwrap()).unwrap();
    assert_eq!(proof["api_complete"], true);
    assert_eq!(proof["write_calls"], 1);
    assert_eq!(proof["write_rows"], records.len());
    assert_eq!(proof["pending"], 0);
    assert_eq!(proof["scheduler_runtime_threads"], 2);
    assert_eq!(proof["embedder_pool_size"], 5);
    let store = out.join("template.sqlite");
    let expected = inspect(&store, &records);
    no_wal(&store);
    let hash = digest(&store);
    let copy_dir = root.join("fresh-writable-copy");
    std::fs::create_dir(&copy_dir).unwrap();
    let copy = copy_dir.join("template.sqlite");
    std::fs::copy(&store, &copy).unwrap();
    assert_eq!(digest(&copy), hash);
    let reopened = Engine::open_with_choice(&copy, EmbedderChoice::None).unwrap().engine;
    assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
    assert_eq!(inspect(&copy, &records), expected);
    reopened.close().unwrap();
    no_wal(&copy);
    assert_eq!(digest(&store), hash);
    println!("existing governed API, physical rows, spans, closed copy and reopen controls passed");
    let metadata_path = out.join("template.json");
    let metadata: Value = if metadata_path.exists() {
        serde_json::from_slice(&std::fs::read(metadata_path).unwrap()).unwrap()
    } else {
        Value::Null
    };
    assert_eq!(metadata["summary"], expected, "missing closed-template metadata");
    assert_eq!(metadata["closed_main_sha256"], hash);
    (metadata, store)
}

#[test]
fn literal_closed_doc_note_template_metadata() {
    verify(vec![
        json!({"id":"alpha","kind":"doc","body":"alpha only evidence"}),
        json!({"id":"beta","kind":"note","body":"βeta note evidence"}),
        json!({"id":"gamma","kind":"doc","body":"gamma document"}),
        json!({"id":"delta","kind":"note","body":"delta note"}),
    ]);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, ..ProptestConfig::default() })]
    #[test]
    fn completed_cohort_preserves_counts_order_and_utf8_spans(rows in prop::collection::vec((any::<bool>(), 1usize..12), 1..7)) {
        let records = rows.iter().enumerate().map(|(index,(note,repeats))| json!({"id":format!("row-{index}"),"kind":if *note {"note"} else {"doc"},"body":format!("row {index} {}", "évidence ".repeat(*repeats))})).collect();
        verify(records);
    }
}

fn detailed_coverage(records: Vec<Value>) {
    let (metadata, store) = verify(records.clone());
    let connection = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut kinds = std::collections::BTreeMap::<String, Vec<i64>>::new();
    let mut expected_rows = Vec::new();
    let mut physical_ids = std::collections::BTreeSet::new();
    for record in &records {
        let id = record["id"].as_str().unwrap();
        let kind = record["kind"].as_str().unwrap();
        let body = record["body"].as_str().unwrap();
        let cursors: Vec<i64> = connection.prepare("SELECT write_cursor FROM canonical_nodes WHERE logical_id=?1 AND kind=?2 AND superseded_at IS NULL").unwrap().query_map(rusqlite::params![id,kind], |row| row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(cursors.len(), 1);
        let cursor = cursors[0];
        let mapping: Vec<i64> = connection
            .prepare("SELECT rowid FROM _fathomdb_vector_rows WHERE kind=?1 AND write_cursor=?2")
            .unwrap()
            .query_map(rusqlite::params![kind, cursor], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(mapping.len(), 1);
        let rowid = mapping[0];
        assert!(physical_ids.insert(rowid));
        let (physical_kind, bytes): (String, u64) = connection
            .query_row(
                "SELECT kind,length(embedding) FROM vector_default WHERE rowid=?1",
                [rowid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((physical_kind.as_str(), bytes), (kind, 3072));
        let spans: Vec<(i64,i64,Option<i64>,u64)> = connection.prepare("SELECT window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans WHERE vector_rowid=?1 AND write_cursor=?2").unwrap().query_map(rusqlite::params![rowid,cursor], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(spans, [(0, 0, None, u64::try_from(body.len()).unwrap())]);
        kinds.entry(kind.to_owned()).or_default().push(cursor);
        expected_rows.push(json!({"logical_id":id,"kind":kind,"write_cursor":cursor,"vector_rowid":rowid,"dimension":768,"window_number":0,"byte_start":0,"byte_end":null,"body_length":body.len()}));
    }
    for table in
        ["canonical_nodes", "_fathomdb_vector_rows", "vector_default", "_fathomdb_vector_spans"]
    {
        let count: usize = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, records.len());
    }
    let by_kind: Vec<_> = kinds.into_iter().map(|(kind,mut cursors)| {
        cursors.sort_unstable();
        let count=cursors.len();
        json!({"kind":kind,"cursors":cursors,"canonical_rows":count,"mapping_rows":count,"physical_vectors":count,"span_rows":count,"known_ends":0,"unknown_ends":count})
    }).collect();
    drop(connection);
    println!("independent exact per-kind canonical/mapping/physical/span cardinality and D NULL controls passed");
    let expected = json!({"verified":true,"by_kind":by_kind,"rows":expected_rows,"profile":{"name":"varying","revision":"perf-gates-dense","dimension":768}});
    assert_eq!(metadata["coverage"], expected, "missing tool-wired detailed completeness report");
}

#[test]
fn literal_tool_report_covers_each_doc_note_cursor_and_physical_span() {
    detailed_coverage(vec![
        json!({"id":"coverage-doc","kind":"doc","body":"literal document evidence"}),
        json!({"id":"coverage-note","kind":"note","body":"literal note évidence"}),
        json!({"id":"coverage-doc-2","kind":"doc","body":"second document"}),
    ]);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 4, max_shrink_iters: 0, ..ProptestConfig::default() })]
    #[test]
    fn generated_tool_report_preserves_per_kind_cursor_span_cardinality(rows in prop::collection::vec((any::<bool>(),1usize..8),1..7)) {
        let records=rows.iter().enumerate().map(|(index,(note,repeats))|json!({"id":format!("coverage-{index}"),"kind":if *note {"note"}else{"doc"},"body":format!("record {index} {}","évidence ".repeat(*repeats))})).collect();
        detailed_coverage(records);
    }
}

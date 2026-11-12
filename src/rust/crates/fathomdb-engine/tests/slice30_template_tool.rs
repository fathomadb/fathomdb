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

fn verify(records: Vec<Value>) {
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

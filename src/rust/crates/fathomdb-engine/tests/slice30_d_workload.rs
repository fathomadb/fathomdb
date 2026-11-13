use fathomdb_engine::{EmbedderChoice, Engine};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn digest(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|b| format!("{b:02x}")).collect()
}

fn verify(count: usize, warmups: usize, samples: usize) {
    let root = tempfile::Builder::new().prefix("s30-d-query-").tempdir().unwrap().keep();
    println!("query fixture {} rows={count} warmups={warmups} samples={samples}", root.display());
    Engine::open_with_choice(root.join("register.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let records:Vec<_>=(0..count).map(|i|json!({"id":format!("row-{i}"),"kind":if i%2==0 {"doc"}else{"note"},"body":format!("record {i} évidence 🐟")})).collect();
    let config = root.join("cell.json");
    std::fs::write(&config,serde_json::to_vec(&json!({"family":"D_QUERY_FIXTURE","records":records,"warmups":warmups,"samples":samples})).unwrap()).unwrap();
    let out = root.join("out");
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary before runtime SHA256 {}", digest(&binary));
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg(config)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(150) {
            let _ = child.kill();
            while started.elapsed() < Duration::from_secs(180) {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("owned fixture timeout NOT_RED {}", root.display());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        status.success(),
        "API/setup failed NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let complete: Value =
        serde_json::from_slice(&std::fs::read(out.join("complete.json")).unwrap()).unwrap();
    assert_eq!(complete["complete"], true);
    let proof: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/api-proof.json")).unwrap())
            .unwrap();
    assert_eq!(proof["api_complete"], true);
    assert_eq!(proof["pending"], 0);
    let metadata: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/template.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["coverage"]["verified"], true);
    let source = out.join("template/template.sqlite");
    assert_eq!(metadata["closed_main_sha256"], digest(&source));
    for store in [&source, &out.join("query.sqlite")] {
        let wal = PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |m| m.len() == 0));
        let db = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        for table in
            ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
        {
            assert_eq!(
                db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, usize>(0))
                    .unwrap(),
                count
            );
        }
        let profile:(String,String,u32)=db.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(profile, ("varying".into(), "perf-gates-dense".into(), 768));
        drop(db);
        let reopened = Engine::open_with_choice(store, EmbedderChoice::None).unwrap().engine;
        assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
        reopened.close().unwrap();
    }
    let rows: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert!(!rows.is_empty(), "missing successful ordinary API control NOT_RED");
    let expected: BTreeSet<_> = (0..count).map(|i| format!("l:row-{i}")).collect();
    for row in &rows {
        assert_eq!(row["ok"], true);
        assert_eq!(row["query"], "record");
        let ids: BTreeSet<_> = row["ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(ids, expected, "actual literal API results NOT_RED");
        assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
    }
    println!("actual query and closed/copy/profile/vector controls passed");
    let actual_warmups = rows.iter().filter(|r| r["phase"] == "warmup").count();
    let actual_samples = rows.iter().filter(|r| r["phase"] == "timed").count();
    assert_eq!(
        (actual_warmups, actual_samples),
        (warmups, samples),
        "missing actual D warmup/timed query loop"
    );
    let mut previous = 0;
    for (index, row) in rows.iter().enumerate() {
        let (phase, sample) =
            if index < warmups { ("warmup", index) } else { ("timed", index - warmups) };
        assert_eq!(row["phase"], phase);
        assert_eq!(row["sample_index"], sample);
        let start = row["start_ns"].as_u64().unwrap();
        let end = row["end_ns"].as_u64().unwrap();
        assert!(previous <= start && start < end);
        assert!(row["elapsed_ns"].as_u64().unwrap() <= end - start);
        previous = end;
    }
}

#[test]
fn timed_queries_repeat_without_warmups() {
    verify(3, 0, 4);
}
#[test]
fn warmup_queries_precede_timed_queries() {
    verify(2, 2, 3);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301301),..ProptestConfig::default()})]
    #[test]
    fn bounded_populations_preserve_query_counts_order_and_values(rows in 1usize..6,warmups in 0usize..4,samples in 2usize..7){verify(rows,warmups,samples);}
}

fn writer_fixture(appended: usize, cohort: usize, warmups: usize) {
    let root = tempfile::Builder::new().prefix("s30-d-writer-").tempdir().unwrap().keep();
    println!(
        "writer fixture {} appended={appended} cohort={cohort} warmups={warmups}",
        root.display()
    );
    Engine::open_with_choice(root.join("register.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let records = vec![
        json!({"id":"seed-a","kind":"doc","body":"record alpha"}),
        json!({"id":"seed-b","kind":"note","body":"record beta"}),
    ];
    let appended_records:Vec<_>=(0..appended).map(|i|json!({"id":format!("tail-{i}"),"kind":"note","body":format!("append evidence {i} é")})).collect();
    let config = root.join("cell.json");
    std::fs::write(&config,serde_json::to_vec(&json!({"family":"D_QUERY_FIXTURE","records":records,"write_records":appended_records,"cohort":cohort,"warmups":warmups,"samples":4})).unwrap()).unwrap();
    let binary = PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").unwrap())
        .join("release/examples/slice30_gd_diagnostic");
    println!("binary before runtime SHA256 {}", digest(&binary));
    let out = root.join("out");
    let started = Instant::now();
    let mut child = Command::new(binary)
        .arg(config)
        .arg(&out)
        .stdout(Stdio::from(std::fs::File::create(root.join("stdout.log")).unwrap()))
        .stderr(Stdio::from(std::fs::File::create(root.join("stderr.log")).unwrap()))
        .spawn()
        .unwrap();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(150) {
            let _ = child.kill();
            while started.elapsed() < Duration::from_secs(180) {
                if child.try_wait().unwrap().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            panic!("owned writer fixture timeout NOT_RED {}", root.display());
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(
        status.success(),
        "API/setup failed NOT_RED: {}",
        std::fs::read_to_string(root.join("stderr.log")).unwrap()
    );
    let metadata: Value =
        serde_json::from_slice(&std::fs::read(out.join("template/template.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["coverage"]["verified"], true);
    assert_eq!(metadata["closed_main_sha256"], digest(&out.join("template/template.sqlite")));
    let rows: Vec<Value> = std::fs::read_to_string(out.join("queries.jsonl"))
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(rows.len(), warmups + 4, "existing real query loops NOT_RED");
    let permitted: BTreeSet<_> = ["l:seed-a".to_owned(), "l:seed-b".to_owned()]
        .into_iter()
        .chain((0..appended).map(|i| format!("l:tail-{i}")))
        .collect();
    for row in &rows {
        assert_eq!(row["ok"], true);
        assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
        let ids = row["ids"].as_array().unwrap();
        assert!(!ids.is_empty());
        assert!(ids.iter().all(|id| permitted.contains(id.as_str().unwrap())));
    }
    let path = out.join("writes.json");
    let writes: Vec<Value> = if path.exists() {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    } else {
        Vec::new()
    };
    println!(
        "existing real query/template controls passed; actual governed cohorts {}",
        writes.len()
    );
    assert_eq!(
        writes.len(),
        appended.div_ceil(cohort),
        "missing actual concurrent D governed writer work"
    );
    let first_timed =
        rows.iter().find(|r| r["phase"] == "timed").unwrap()["start_ns"].as_u64().unwrap();
    let last_warm = rows
        .iter()
        .filter(|r| r["phase"] == "warmup")
        .map(|r| r["end_ns"].as_u64().unwrap())
        .max()
        .unwrap_or(0);
    let mut previous = last_warm;
    let mut done = 0;
    let mut cursors = Vec::new();
    for write in &writes {
        let start = write["start_ns"].as_u64().unwrap();
        let end = write["end_ns"].as_u64().unwrap();
        assert!(previous <= start && start < end);
        assert!(
            write["launch_ns"].as_u64().unwrap() <= first_timed
                && write["launch_ns"].as_u64().unwrap() <= start
        );
        assert_eq!(write["rows"], cohort.min(appended - done));
        let current = write["row_cursors"].as_array().unwrap();
        assert_eq!(current.len(), cohort.min(appended - done));
        cursors.extend(current.iter().map(|v| v.as_u64().unwrap()));
        assert_eq!(write["cursor"], *current.last().unwrap());
        done += current.len();
        previous = end;
    }
    assert_eq!(done, appended);
    let db =
        Connection::open_with_flags(out.join("query.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let actual:Vec<(String,u64)>=db.prepare("SELECT logical_id,write_cursor FROM canonical_nodes WHERE logical_id LIKE 'tail-%' ORDER BY write_cursor").unwrap().query_map([],|r|Ok((r.get(0)?,r.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(
        actual,
        (0..appended).map(|i| (format!("tail-{i}"), cursors[i])).collect::<Vec<_>>()
    );
    for table in
        ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
    {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get::<_, usize>(0))
                .unwrap(),
            2 + appended
        );
    }
    let wal = PathBuf::from(format!("{}-wal", out.join("query.sqlite").display()));
    assert!(std::fs::metadata(wal).map_or(true, |m| m.len() == 0));
}

#[test]
fn governed_writer_runs_beside_timed_queries_after_warmups() {
    writer_fixture(7, 3, 2);
}
#[test]
fn zero_warmup_writer_keeps_partial_final_cohort() {
    writer_fixture(4, 3, 0);
}
proptest! {
    #![proptest_config(ProptestConfig{cases:4,max_shrink_iters:0,failure_persistence:None,rng_seed:RngSeed::Fixed(0x301302),..ProptestConfig::default()})]
    #[test]
    fn bounded_writer_cohorts_preserve_receipts_and_completed_tail(rows in 1usize..9,cohort in 1usize..5,warmups in 0usize..3){writer_fixture(rows,cohort,warmups);}
}

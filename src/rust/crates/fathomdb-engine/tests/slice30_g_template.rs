use fathomdb_engine::{EmbedderChoice, Engine};
use proptest::prelude::*;
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const GPU: &str = "GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b";

fn hash(path: &Path) -> String {
    Sha256::digest(std::fs::read(path).unwrap()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn no_wal(path: &Path) {
    let wal = PathBuf::from(format!("{}-wal", path.display()));
    assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
}

fn literal() -> Vec<Value> {
    (0..65).map(|index|json!({"id":format!("literal-{index}"),"kind":if index%2==0 {"doc"}else{"note"},"body":if index==0 {"évidence ".repeat(1600)}else{format!("short natural record {index} évidence")}})).collect()
}

fn independent_coverage(store: &Path, records: &[Value]) -> Value {
    let connection = Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let profile:(String,String,u32)=connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    assert_eq!(
        profile,
        (
            "fathomdb-bge-small-en-v1.5".into(),
            "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a".into(),
            384
        )
    );
    let actual_ids:Vec<String>=connection.prepare("SELECT logical_id FROM canonical_nodes WHERE state='active' AND superseded_at IS NULL ORDER BY write_cursor").unwrap().query_map([],|row|row.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    assert_eq!(
        actual_ids,
        records.iter().map(|row| row["id"].as_str().unwrap()).collect::<Vec<_>>()
    );
    let mut kinds = BTreeMap::<String, Vec<i64>>::new();
    let mut rowids = BTreeSet::new();
    let mut expected_rows = Vec::new();
    for record in records {
        let id = record["id"].as_str().unwrap();
        let kind = record["kind"].as_str().unwrap();
        let body = record["body"].as_str().unwrap();
        let (cursor,actual_body):(i64,String)=connection.query_row("SELECT write_cursor,body FROM canonical_nodes WHERE logical_id=?1 AND kind=?2 AND state='active' AND superseded_at IS NULL",rusqlite::params![id,kind],|row|Ok((row.get(0)?,row.get(1)?))).unwrap();
        assert_eq!(actual_body, body);
        let mapping: Vec<i64> = connection
            .prepare("SELECT rowid FROM _fathomdb_vector_rows WHERE kind=?1 AND write_cursor=?2")
            .unwrap()
            .query_map(rusqlite::params![kind, cursor], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(mapping.len(), 1);
        let rowid = mapping[0];
        assert!(rowids.insert(rowid));
        let (physical_kind, vector): (String, Vec<u8>) = connection
            .query_row("SELECT kind,embedding FROM vector_default WHERE rowid=?1", [rowid], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(physical_kind, kind);
        assert_eq!(vector.len(), 384 * 4);
        assert!(vector
            .chunks_exact(4)
            .all(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()).is_finite()));
        let spans:Vec<(i64,i64,Option<usize>,usize)>=connection.prepare("SELECT window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans WHERE vector_rowid=?1 AND write_cursor=?2").unwrap().query_map(rusqlite::params![rowid,cursor],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?))).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(spans.len(), 1);
        let (window, start, end, length) = spans[0];
        assert_eq!((window, start, length), (0, 0, body.len()));
        let end = end.expect("pinned BGE successful head span must not be NULL, NOT_RED");
        assert!(end <= body.len() && body.is_char_boundary(end));
        if id == "literal-0" {
            assert!(end < body.len(), "overlength Unicode must be truncated");
        } else {
            assert_eq!(end, body.len(), "short input must have full known head end");
        }
        kinds.entry(kind.to_owned()).or_default().push(cursor);
        expected_rows.push(json!({"logical_id":id,"kind":kind,"write_cursor":cursor,"vector_rowid":rowid,"dimension":384,"window_number":0,"byte_start":0,"byte_end":end,"body_length":length}));
    }
    for table in
        ["canonical_nodes", "_fathomdb_vector_rows", "vector_default", "_fathomdb_vector_spans"]
    {
        let count: usize = connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, records.len());
    }
    let by_kind:Vec<_>=kinds.into_iter().map(|(kind,mut cursors)| {cursors.sort_unstable();let count=cursors.len();json!({"kind":kind,"canonical_rows":count,"cursors":cursors,"mapping_rows":count,"physical_vectors":count,"span_rows":count,"known_ends":count,"unknown_ends":0})}).collect();
    json!({"verified":true,"by_kind":by_kind,"rows":expected_rows,"profile":{"name":profile.0,"revision":profile.1,"dimension":profile.2}})
}

fn series(cases: Vec<Vec<Value>>) {
    assert!(!cases.is_empty() && cases.len() <= 5);
    let root = tempfile::Builder::new().prefix("s30-bge-template-").tempdir().unwrap().keep();
    println!("normal BGE proof: {}", root.display());
    Engine::open_with_choice(root.join("registration.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine
        .close()
        .unwrap();
    let mut inputs = Vec::new();
    for (index, records) in cases.iter().enumerate() {
        let path = root.join(format!("records-{index}.jsonl"));
        let mut file = std::fs::File::create(&path).unwrap();
        for record in records {
            serde_json::to_writer(&mut file, record).unwrap();
            file.write_all(b"\n").unwrap();
        }
        inputs.push(json!({"records":path}));
    }
    let config = root.join("cell.json");
    std::fs::write(
        &config,
        serde_json::to_vec_pretty(
            &json!({"family":"G_TEMPLATE","gpu_uuid":GPU,"templates":inputs}),
        )
        .unwrap(),
    )
    .unwrap();
    let oracle = json!({"test_source_sha256":hash(&PathBuf::from(std::env::var_os("S30_GD_SOURCE").unwrap()).join(file!())),"test_executable":std::env::current_exe().unwrap(),"test_executable_sha256":hash(&std::env::current_exe().unwrap()),"config_sha256":hash(&config)});
    std::fs::write(root.join("oracle-identity.json"), serde_json::to_vec_pretty(&oracle).unwrap())
        .unwrap();
    println!("oracle before runtime: {oracle}");
    let source = PathBuf::from(std::env::var_os("S30_GD_SOURCE").unwrap());
    let log = std::fs::File::create(root.join("controller.log")).unwrap();
    let mut child = Command::new(source.join(".venv/bin/python"))
        .arg(
            source.join("dev/experiments/0.8.29-slice30-measurements/native/g_template_fixture.py"),
        )
        .arg(&config)
        .arg(&root)
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() > Duration::from_secs(180) {
            child.kill().unwrap();
            let cleaning = Instant::now();
            while child.try_wait().unwrap().is_none() && cleaning.elapsed() < Duration::from_secs(5)
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("bounded template controller exceeded180s, NOT_RED; owned group guard retains cleanup");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "CUDA/setup/preparation failure NOT_RED: {}",
        std::fs::read_to_string(root.join("controller.log")).unwrap()
    );
    let mut expected = Vec::new();
    for (index, records) in cases.iter().enumerate() {
        let out = root.join("out").join(format!("case-{index}"));
        let proof: Value =
            serde_json::from_slice(&std::fs::read(out.join("api-proof.json")).unwrap()).unwrap();
        assert_eq!(proof["api_complete"], true);
        assert_eq!(proof["write_rows"], records.len());
        assert_eq!(proof["write_calls"], records.len().div_ceil(64));
        assert_eq!(proof["pending"], 0);
        assert_eq!(proof["selected_cuda_uuid"], GPU);
        assert_eq!(proof["cuda_ordinal"], 0);
        assert_eq!(proof["cuda_compiled"], true);
        assert_eq!(
            proof["input_ids"],
            json!(records.iter().map(|row| row["id"].as_str().unwrap()).collect::<Vec<_>>())
        );
        for (batch, cohort) in proof["cohorts"].as_array().unwrap().iter().enumerate() {
            assert_eq!(cohort["first_input_index"], batch * 64);
            assert_eq!(cohort["rows"], (records.len() - batch * 64).min(64));
            assert!(cohort["write_ns"].as_u64().unwrap() > 0);
        }
        for timer in
            ["open_ns", "configure_ns", "seed_ns", "drain_ns", "close_ns", "verification_ns"]
        {
            assert!(proof[timer].as_u64().unwrap() > 0);
        }
        let store = out.join("template.sqlite");
        let coverage = independent_coverage(&store, records);
        assert_eq!(proof["coverage"], coverage);
        no_wal(&store);
        let digest = hash(&store);
        assert_eq!(proof["closed_main_sha256"], digest);
        let copy_dir = root.join(format!("fresh-copy-{index}"));
        std::fs::create_dir(&copy_dir).unwrap();
        let copy = copy_dir.join("template.sqlite");
        std::fs::copy(&store, &copy).unwrap();
        assert_eq!(hash(&copy), digest);
        let reopened = Engine::open_with_choice(&copy, EmbedderChoice::None).unwrap().engine;
        assert_eq!(reopened.read_embedding_readiness().unwrap().pending_count, 0);
        assert_eq!(independent_coverage(&copy, records), coverage);
        reopened.close().unwrap();
        no_wal(&copy);
        assert_eq!(hash(&store), digest);
        expected.push((
            out,
            json!({"coverage":coverage,"closed_main_sha256":digest,"preparation":proof}),
        ));
    }
    println!("all actual CUDA/API/cohort/physical/profile/nonNULL short+long spans/closed-copy/reopen controls passed for {} templates",cases.len());
    for (out, expected) in expected {
        let path = out.join("template.json");
        let report: Value = if path.exists() {
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        } else {
            Value::Null
        };
        assert_eq!(report, expected, "missing normal BGE closed-template report");
    }
}

#[test]
fn literal_normal_bge_template_reports_65_row_boundary_and_head_spans() {
    series(vec![literal()]);
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2, max_shrink_iters: 0, failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn generated_normal_bge_templates_preserve_order_utf8_and_coverage(cohorts in prop::collection::vec(prop::collection::vec((any::<bool>(),1usize..8),1..8),4)) {
        let mut cases=vec![literal()];
        cases.extend(cohorts.iter().enumerate().map(|(case,rows)| rows.iter().enumerate().map(|(index,(note,repeats))|json!({"id":format!("case-{case}-row-{index}"),"kind":if *note {"note"}else{"doc"},"body":format!("record {index} {}","évidence ".repeat(*repeats))})).collect()));
        series(cases);
    }
}

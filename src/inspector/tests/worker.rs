#![cfg(target_os = "linux")]
#![allow(dead_code)]
#[path = "fixtures/mod.rs"]
mod fixtures;
#[path = "../src/process.rs"]
mod process;
use process::OwnedWorker;
use serde_json::json;
use std::path::Path;
#[test]
fn real_cleared_session_worker_owns_lock_and_preserves_header_under_hostile_parent() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let c = rusqlite::Connection::open(&path).unwrap();
    c.pragma_update(None, "application_id", 0x1462).unwrap();
    drop(c);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let pid = worker.pid;
    let environment = std::fs::read(format!("/proc/{pid}/environ")).unwrap();
    let actual: std::collections::BTreeSet<_> = environment
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).unwrap())
        .collect();
    let temporary = actual.iter().find_map(|s| s.strip_prefix("TMPDIR=")).unwrap();
    let work = Path::new(temporary);
    assert!(work.starts_with(scratch.path()));
    assert_eq!(work.file_name().unwrap(), "work");
    assert_eq!(
        actual,
        std::collections::BTreeSet::from([
            format!("TMPDIR={temporary}"),
            format!("XDG_CACHE_HOME={}", work.parent().unwrap().join("view").display()),
            "LANG=C.UTF-8".into(),
            "LC_ALL=C.UTF-8".into(),
            "FATHOMDB_EMBED_DEVICE=cpu".into(),
            "RAYON_NUM_THREADS=1".into(),
        ])
    );
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("{}.lock", path.display()))
        .unwrap();
    assert!(lock.try_lock().is_err());
    let catalog = worker.call(json!({"op":"catalog"})).unwrap();
    assert_eq!(catalog["counts"]["node"]["revisions"], "2");
    worker.close().unwrap();
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    assert!(lock.try_lock().is_ok());
    drop(lock);
    let c = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        c.pragma_query_value(None, "application_id", |r| r.get::<_, i64>(0)).unwrap(),
        0x1462
    );
}

#[path = "../src/failure.rs"]
mod failure;

#[test]
#[cfg(feature = "fixture-control")]
fn private_fixture_control_uses_the_owned_real_engine_but_read_requests_cannot_invoke_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":true,"show_erased":false,"show_payloads":false}})).unwrap();
    worker.fixture(json!({"control":"transition","identity":"alpha","deleted":true})).unwrap();
    let page=worker.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"alpha"}})).unwrap();
    assert!(page["items"].as_array().unwrap().is_empty());
    assert!(worker
        .call(json!({"op":"fixture","control":"transition","identity":"alpha","deleted":false}))
        .is_err());
    worker.fixture(json!({"control":"transition","identity":"alpha","deleted":false})).unwrap();
    let page=worker.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"alpha"}})).unwrap();
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    worker.close().unwrap();
}

#[test]
#[ignore = "mandatory F-06 scale qualification; run explicitly with --ignored"]
fn scale_inventory_bounds_all_enumerations_and_real_worker_memory() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_scale(&path);
    let check = rusqlite::Connection::open(&path).unwrap();
    for (table, expected) in
        [("canonical_nodes", 100003), ("operational_mutations", 100000), ("canonical_edges", 10000)]
    {
        let count: i64 =
            check.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap();
        assert_eq!(count, expected);
    }
    drop(check);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let memory = || {
        let text = std::fs::read_to_string(format!("/proc/{}/status", worker.pid)).unwrap();
        let number = |label: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(label))
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                * 1024
        };
        (number("VmRSS:"), number("VmHWM:"))
    };
    let (idle, _) = memory();
    let began = std::time::Instant::now();
    let catalog = worker.call(json!({"op":"catalog"})).unwrap();
    let elapsed = began.elapsed();
    assert_eq!(catalog["status"], "partial");
    assert!(elapsed <= std::time::Duration::from_secs(30));
    let scope = &catalog["scope"];
    let metadata = scope["metadata_examined"]
        .as_u64()
        .expect("metadata enumerations belong to the same aggregate counter");
    assert!(metadata > 0);
    let examined = scope["examined"].as_u64().unwrap();
    assert!(examined <= 100000);
    let records: u64 = catalog["counts"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v["examined"].as_u64().unwrap())
        .sum();
    assert_eq!(examined, records + metadata);
    for class in ["node", "edge", "log"] {
        let page = worker
            .call(json!({"op":"page","class":class,"after":"0","limit":200,"filter":{}}))
            .unwrap();
        assert_eq!(page["items"].as_array().unwrap().len(), 200);
        assert_eq!(page["status"], "partial");
        assert!(serde_json::to_vec(&page).unwrap().len() <= 2 * 1024 * 1024);
    }
    let text = std::fs::read_to_string(format!("/proc/{}/status", worker.pid)).unwrap();
    let number = |label: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    let peak = number("VmHWM:");
    assert!(peak.saturating_sub(idle) <= 1024 * 1024 * 1024);
    println!("F-06 actual worker: idle_rss={idle} peak_rss={peak} growth={} catalog_ms={} examined={examined}",peak.saturating_sub(idle),elapsed.as_millis());
    worker.close().unwrap();
}

#[test]
fn deterministic_content_sample_reports_only_actual_fields_and_never_schema_absence() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_related(&path);
    let long = "long-key-é".repeat(600);
    let body = serde_json::json!({long.clone():"actual value","optional":null}).to_string();
    let opened =
        fathomdb_engine::Engine::open_with_choice(&path, fathomdb_engine::EmbedderChoice::None)
            .unwrap();
    opened.engine.write(&[fixtures::node("long-key-record", &body)]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let sample = worker
        .call(json!({"op":"sample","class":"node","after":"0","filter":{}}))
        .expect("real bounded field sample operation");
    assert_eq!(sample["scope"]["sample"]["selection"], "first_matching_retained_records");
    assert_eq!(sample["scope"]["sample"]["absence_proof"], false);
    assert!(sample["items"].as_array().unwrap().len() <= 10);
    let records = sample["items"].as_array().unwrap();
    let derived = records.iter().find(|r| r["logical_id"] == "derived").unwrap();
    assert_eq!(derived["field_observation"]["root_kind"], "object");
    assert_eq!(derived["field_observation"]["names"], json!(["title", "text"]));
    let long_record = records.iter().find(|r| r["logical_id"] == "long-key-record").unwrap();
    let names = long_record["field_observation"]["names"].as_array().unwrap();
    assert!(names.iter().any(|v| v == &long));
    assert!(!names.iter().any(|v| v == "oversize key (copy exact range)"));
    let exact = worker.call(json!({"op":"detail","handle":derived["handle"]})).unwrap();
    assert_eq!(exact["record"]["revision_id"], "derived-r1");
    assert_eq!(exact["context"]["attachment"], sample["context"]["attachment"]);
    worker.close().unwrap();
}

#[test]
#[ignore = "allocation qualification uses a real128MiB payload; run explicitly with --ignored"]
fn payload_windows_do_not_materialize_the_whole_retained_value() {
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let payload_bytes = 128 * 1024 * 1024;
    let body = "x".repeat(payload_bytes);
    let opened =
        fathomdb_engine::Engine::open_with_choice(&path, fathomdb_engine::EmbedderChoice::None)
            .unwrap();
    opened.engine.write(&[fixtures::node("allocation-large", &body)]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    drop(body);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let page=worker.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"allocation-large"}})).unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let pid = worker.pid;
    let rss = move || {
        let text = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
        text.lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    let idle = rss();
    let peak = Arc::new(AtomicU64::new(idle));
    let stop = Arc::new(AtomicBool::new(false));
    let observed = peak.clone();
    let done = stop.clone();
    let monitor = std::thread::spawn(move || {
        while !done.load(Ordering::SeqCst) {
            observed.fetch_max(rss(), Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
    let response = worker
        .call(json!({"op":"chunk","handle":handle,"field":"body","offset":"0","limit":262144}));
    stop.store(true, Ordering::SeqCst);
    monitor.join().unwrap();
    let response = response.unwrap();
    assert_eq!(response["text"].as_str().unwrap().len(), 262144);
    assert_eq!(response["total_bytes"], payload_bytes.to_string());
    let peak = peak.load(Ordering::SeqCst);
    let growth = peak.saturating_sub(idle);
    println!("window allocation: payload_bytes={payload_bytes} idle_rss={idle} observed_peak_rss={peak} growth={growth}");
    assert!(growth<(payload_bytes as u64)/2,"bounded window must not preload an entire retained payload; generous allowance is less than half the payload");
    worker.close().unwrap();
}

#[test]
#[ignore = "allocation qualification uses a real128MiB payload; run explicitly with --ignored"]
fn metadata_previews_do_not_materialize_the_whole_retained_identity() {
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    };
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let payload_bytes = 128 * 1024 * 1024;
    let body = "x".repeat(payload_bytes);
    let opened =
        fathomdb_engine::Engine::open_with_choice(&path, fathomdb_engine::EmbedderChoice::None)
            .unwrap();
    opened.engine.write(&[fixtures::node(&body, "metadata-only body")]).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    drop(body);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let pid = worker.pid;
    let rss = move || {
        let text = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap();
        text.lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            * 1024
    };
    let idle = rss();
    let peak = Arc::new(AtomicU64::new(idle));
    let stop = Arc::new(AtomicBool::new(false));
    let observed = peak.clone();
    let done = stop.clone();
    let monitor = std::thread::spawn(move || {
        while !done.load(Ordering::SeqCst) {
            observed.fetch_max(rss(), Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
    let response =
        worker.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{}}));
    stop.store(true, Ordering::SeqCst);
    monitor.join().unwrap();
    let response = response.unwrap();
    let record = response["items"].as_array().unwrap().last().unwrap();
    assert_eq!(record["logical_id_bytes"], payload_bytes);
    assert!(record["logical_id"].as_str().unwrap().len() <= 4096);
    let peak = peak.load(Ordering::SeqCst);
    let growth = peak.saturating_sub(idle);
    println!("metadata preview allocation: payload_bytes={payload_bytes} idle_rss={idle} observed_peak_rss={peak} growth={growth}");
    assert!(growth<(payload_bytes as u64)/2,"metadata preview must not preload an entire retained identity; generous allowance is less than half the payload");
    worker.close().unwrap();
}

#[test]
fn cached_workspace_checks_all_exact_references_in_one_bounded_request() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_related(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker = OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")), scratch.path(), json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let page = worker.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"derived"}})).unwrap();
    let handle = page["items"][0]["handle"].clone();
    let associations = worker.call(json!({"op":"associations","handle":handle})).unwrap();
    let refs = vec![
        handle.clone(),
        associations["attributes"][0]["handle"].clone(),
        associations["links"][0]["targets"][0]["handle"].clone(),
    ];
    let check = worker
        .call(json!({"op":"check","handles":refs}))
        .expect("all cached content owners must be validated as one bounded read");
    assert_eq!(check["scope"]["validated"], 3);
    assert_eq!(check["scope"]["snapshot"], "this_request_only");
    assert!(
        worker.call(json!({"op":"check","handles":[handle,"guessed-handle"]})).is_err(),
        "a permitted anchor cannot authorize a foreign target"
    );
    assert_eq!(
        worker.call(json!({"op":"check","handles":vec!["x";202]})).unwrap_err().error,
        "invalid_range"
    );
    worker.close().unwrap();
}

#[test]
fn cached_metadata_validation_does_not_require_permission_to_disclose_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let scratch = tempfile::tempdir().unwrap();
    let mut worker = OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")), scratch.path(), json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let page = worker
        .call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{}}))
        .unwrap();
    let handle = page["items"][0]["handle"].clone();
    let result = worker
        .call(json!({"op":"check","metadata_handles":[handle]}))
        .expect("structural page metadata has its own bounded validation scope");
    assert_eq!(result["scope"]["validated"], 1);
    worker.close().unwrap();
}

#[test]
fn lexical_response_caps_the_entire_envelope_and_keeps_the_real_worker_usable() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let request = fixtures::add_lexical_envelope(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let outcome = worker.call(request);
    assert!(outcome.is_ok(),"near-cap valid lexical request must return scoped partial without disconnecting worker: {outcome:?}");
    let result = outcome.unwrap();
    assert_eq!(result["status"], "partial");
    assert!(serde_json::to_vec(&result).unwrap().len() <= 2 * 1024 * 1024);
    assert!(!result["items"].as_array().unwrap().is_empty());
    assert!(result["items"].as_array().unwrap().len() < 100);
    let next = worker
        .call(
            json!({"op":"page","class":"node","after":"0","limit":1,"filter":{"identity":"alpha"}}),
        )
        .unwrap();
    assert_eq!(next["items"][0]["logical_id"], "alpha");
    worker.close().unwrap();
}

#[test]
#[ignore = "mandatory F06 lexical admission/RSS qualification; run explicitly with --ignored"]
fn scale_lexical_refusal_stays_bounded_and_preserves_usable_exact_reads() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_scale(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let worker_pid = worker.pid;
    let rss = || {
        let status = std::fs::read_to_string(format!("/proc/{worker_pid}/status")).unwrap();
        let read = |label: &str| {
            status
                .lines()
                .find_map(|v| v.strip_prefix(label))
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                * 1024
        };
        (read("VmRSS:"), read("VmHWM:"))
    };
    let idle = rss().0;
    let began = std::time::Instant::now();
    let refusal = worker.call(
        json!({"op":"lexical","route":"body","query":"scale","view":{},"filter":{},"limit":100}),
    );
    assert!(matches!(refusal,Err(ref error) if error.error=="budget_exhausted"));
    assert!(began.elapsed() < std::time::Duration::from_secs(30));
    let page = worker
        .call(
            json!({"op":"page","class":"node","after":"0","limit":1,"filter":{"identity":"alpha"}}),
        )
        .unwrap();
    assert_eq!(page["items"][0]["logical_id"], "alpha");
    let peak = rss().1;
    assert!(peak.saturating_sub(idle) <= 1024 * 1024 * 1024);
    println!(
        "F06 lexical admission: elapsed_ms={} idle_rss={idle} peak_rss={peak} growth={}",
        began.elapsed().as_millis(),
        peak.saturating_sub(idle)
    );
    worker.close().unwrap();
}

#[test]
#[ignore = "mandatory actual cached CPU positive; requires owned venv and pinned local assets"]
fn cached_cpu_default_serves_real_dense_query_without_cache_or_embedding_writes() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let output = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "fixture failed: {}", String::from_utf8_lossy(&output.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    let before = std::fs::read(&path).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let catalog = worker.call(json!({"op":"catalog"})).unwrap();
    assert_eq!(
        catalog["database"]["model_runtime"], "available_cached_cpu",
        "a valid cached CPU store must be admitted with production Default"
    );
    let result = worker.call(json!({"op":"serving","query":"puppy","limit":10})).unwrap();
    assert!(result["query_context"]["vector_hits"].as_u64().unwrap() > 0);
    assert_eq!(result["query_context"]["text_hits"], 0);
    let hits = result["items"].as_array().unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0]["native_id"]["value"], "dog");
    assert!(hits.iter().all(|v| v["evidence"]["vector_rank"].is_u64()));
    assert_eq!(hits[0]["evidence"]["vector_rank"], 0);
    assert_eq!(result["scope"]["score_family"], "engine_rrf");
    assert_eq!(result["scope"]["dense_stage_distances"], "not_exposed");
    worker.close().unwrap();
    let after = std::fs::read(&path).unwrap();
    assert!(!before.is_empty() && !after.is_empty());
    let c = rusqlite::Connection::open(&path).unwrap();
    let count: i64 =
        c.query_row("SELECT count(*) FROM _fathomdb_embed_probe", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 45);
    assert!(std::fs::read_dir(dir.path().join("snapshot/fathomdb/embedders/0b2926f8a9b1"))
        .unwrap()
        .all(|e| !e.unwrap().file_name().to_string_lossy().ends_with("partial")));
}

#[test]
fn all_owned_workers_initialize_with_closed_cpu_allowlist() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let scratch = tempfile::tempdir().unwrap();
    let worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let environment = std::fs::read(format!("/proc/{}/environ", worker.pid)).unwrap();
    let actual: std::collections::BTreeMap<_, _> = environment
        .split(|v| *v == 0)
        .filter(|v| !v.is_empty())
        .map(|v| {
            String::from_utf8(v.to_vec())
                .unwrap()
                .split_once('=')
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .unwrap()
        })
        .collect();
    assert_eq!(actual.len(),6,"every session child needs its fixed CPU/cache environment before dependency initialization");
    assert_eq!(
        actual.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![
            "FATHOMDB_EMBED_DEVICE",
            "LANG",
            "LC_ALL",
            "RAYON_NUM_THREADS",
            "TMPDIR",
            "XDG_CACHE_HOME"
        ]
    );
    assert_eq!(actual["FATHOMDB_EMBED_DEVICE"], "cpu");
    assert_eq!(actual["RAYON_NUM_THREADS"], "1");
    let work = Path::new(&actual["TMPDIR"]);
    let view = Path::new(&actual["XDG_CACHE_HOME"]);
    assert_eq!(work.parent(), view.parent());
    assert!(work.starts_with(scratch.path()));
    assert_eq!(work.file_name().unwrap(), "work");
    assert_eq!(view.file_name().unwrap(), "view");
    worker.close().unwrap();
}

#[test]
#[ignore = "mandatory cached CPU retained-export positive; requires pinned cache and owned wheel"]
fn confined_cached_worker_exports_only_authorized_retained_content_outside_its_write_roots() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .arg("queued")
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(fixture.stdout).unwrap().trim());
    let scratch = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":true,"show_payloads":false}})).unwrap();
    assert_eq!(
        worker.call(json!({"op":"catalog"})).unwrap()["database"]["model_runtime"],
        "available_cached_cpu"
    );
    let page = worker
        .call(
            json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"dog"}}),
        )
        .unwrap();
    let handle = page["items"][0]["handle"].as_str().unwrap();
    let output = destination.path().join("retained.txt");
    let exported = worker.call(json!({"op":"export","handle":handle,"destination":output}));
    assert!(
        exported.is_ok(),
        "authorized retained export must survive model confinement: {exported:?}"
    );
    let bytes = std::fs::read(&output).unwrap();
    let split = bytes.iter().position(|b| *b == b'\n').unwrap();
    assert_eq!(&bytes[split + 1..], b"{\"summary\":\"a domestic dog playing with a ball\"}");
    assert!(
        worker.call(json!({"op":"export","handle":handle,"destination":output})).is_err(),
        "existing files cannot be overwritten"
    );
    worker.close().unwrap();
}

#[test]
fn worker_teardown_removes_only_its_private_session_resources() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    let scratch = tempfile::tempdir().unwrap();
    let canary = scratch.path().join("keep-parent-owned");
    std::fs::write(&canary, b"keep").unwrap();
    let worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let resources = std::fs::read_dir(scratch.path())
        .unwrap()
        .map(|v| v.unwrap().path())
        .filter(|p| p.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(resources.len(), 1);
    worker.close().unwrap();
    assert!(!resources[0].exists(), "closed worker must remove its private work/view resources");
    assert_eq!(std::fs::read(canary).unwrap(), b"keep");
}

#[test]
#[ignore = "mandatory cached CPU canonical/symlink/relative path qualification"]
fn cached_cpu_admission_preserves_symlink_and_relative_database_selection() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(fixture.stdout).unwrap().trim());
    let alias = tempfile::tempdir().unwrap();
    let link = alias.path().join("shortcut.sqlite");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    for selection in [link, std::path::PathBuf::from("fixture.sqlite")] {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cached_alias_case_child", "--ignored", "--nocapture"])
            .current_dir(path.parent().unwrap())
            .env("INSPECTOR_ALIAS_DB", &selection)
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
    }
}
#[test]
#[ignore = "owned path-selection subprocess only"]
fn cached_alias_case_child() {
    let path = std::path::PathBuf::from(std::env::var_os("INSPECTOR_ALIAS_DB").unwrap());
    let scratch = tempfile::tempdir().unwrap();
    let worker = OwnedWorker::spawn(
        Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),
        scratch.path(),
        json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}}),
    );
    assert!(worker.is_ok(), "valid cached alias path must attach: {:?}", worker.as_ref().err());
    let mut worker = worker.unwrap();
    assert_eq!(
        worker.call(json!({"op":"catalog"})).unwrap()["database"]["model_runtime"],
        "available_cached_cpu"
    );
    assert!(
        worker.call(json!({"op":"serving","query":"puppy","limit":10})).unwrap()["query_context"]
            ["vector_hits"]
            .as_u64()
            .unwrap()
            > 0
    );
    worker.close().unwrap();
}

#[test]
#[ignore = "mandatory cached source-alias export exclusion"]
fn cached_source_alias_cannot_export_into_its_canonical_cache_root() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .arg("queued")
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(fixture.stdout).unwrap().trim());
    let cache = tempfile::tempdir().unwrap();
    let aliases = tempfile::tempdir().unwrap();
    let alias = aliases.path().join("cache-alias");
    std::os::unix::fs::symlink(cache.path(), &alias).unwrap();
    let relative = Path::new("fathomdb/embedders/0b2926f8a9b1");
    let target = cache.path().join(relative);
    std::fs::create_dir_all(&target).unwrap();
    let source = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache")
        })
        .join(relative);
    for name in ["config.json", "tokenizer.json", "model.safetensors"] {
        std::os::unix::fs::symlink(source.join(name), target.join(name)).unwrap();
    }
    let destination = cache.path().join("forbidden-retained-export");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cache_export_alias_child", "--ignored", "--nocapture"])
        .env("XDG_CACHE_HOME", &alias)
        .env("INSPECTOR_ALIAS_DB", &path)
        .env("INSPECTOR_ALIAS_EXPORT", &destination)
        .env("INSPECTOR_ALIAS_EXPORT_ALIAS", alias.join("forbidden-retained-export"))
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert!(!destination.exists());
}
#[test]
#[ignore = "owned cache-export alias subprocess only"]
fn cache_export_alias_child() {
    let path = std::path::PathBuf::from(std::env::var_os("INSPECTOR_ALIAS_DB").unwrap());
    let destination = std::path::PathBuf::from(std::env::var_os("INSPECTOR_ALIAS_EXPORT").unwrap());
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":true,"show_payloads":false}})).unwrap();
    assert_eq!(
        worker.call(json!({"op":"catalog"})).unwrap()["database"]["model_runtime"],
        "available_cached_cpu"
    );
    let page = worker
        .call(
            json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"dog"}}),
        )
        .unwrap();
    for destination in [
        destination,
        std::path::PathBuf::from(std::env::var_os("INSPECTOR_ALIAS_EXPORT_ALIAS").unwrap()),
    ] {
        let exported = worker.call(
            json!({"op":"export","handle":page["items"][0]["handle"],"destination":destination}),
        );
        assert!(
            exported.is_err(),
            "canonical source cache root must reject export, got {exported:?}"
        );
    }
    worker.close().unwrap();
}

fn cache_inventory(root: &Path) -> std::collections::BTreeMap<String, String> {
    use std::os::unix::fs::MetadataExt;
    fn walk(root: &Path, path: &Path, out: &mut std::collections::BTreeMap<String, String>) {
        let m = std::fs::symlink_metadata(path).unwrap();
        let kind = if m.is_dir() {
            "dir"
        } else if m.file_type().is_symlink() {
            "link"
        } else {
            "file"
        };
        let data = if m.file_type().is_symlink() {
            std::fs::read_link(path).unwrap().to_string_lossy().to_string()
        } else if m.is_file() {
            format!("{:?}", std::fs::read(path).unwrap())
        } else {
            String::new()
        };
        out.insert(
            path.strip_prefix(root).unwrap().to_string_lossy().to_string(),
            format!(
                "{kind}:{},{},{},{},{},{},{},{},{},{}:{data}",
                m.dev(),
                m.ino(),
                m.mode(),
                m.len(),
                m.mtime(),
                m.mtime_nsec(),
                m.uid(),
                m.gid(),
                m.ctime(),
                m.ctime_nsec()
            ),
        );
        if m.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                walk(root, &entry.unwrap().path(), out);
            }
        }
    }
    let mut result = std::collections::BTreeMap::new();
    walk(root, root, &mut result);
    result
}
#[test]
#[ignore = "mandatory valid/missing/partial/corrupt/replaced/racing cached assets qualification"]
fn actual_cached_assets_fail_closed_without_cache_mutation_or_lexical_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = std::path::PathBuf::from(String::from_utf8(fixture.stdout).unwrap().trim());
    let relative = Path::new("fathomdb/embedders/0b2926f8a9b1");
    let source = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache")
        })
        .join(relative);
    let pinned = std::fs::read(source.join("config.json")).unwrap();
    for case in ["valid", "missing", "partial", "corrupt", "replaced", "racing"] {
        let cache = tempfile::tempdir().unwrap();
        let target = cache.path().join(relative);
        std::fs::create_dir_all(&target).unwrap();
        for name in ["config.json", "tokenizer.json", "model.safetensors"] {
            if case == "missing" || (case == "partial" && name != "config.json") {
                continue;
            }
            std::os::unix::fs::symlink(source.join(name), target.join(name)).unwrap();
        }
        if case == "corrupt" || case == "replaced" {
            let replacement = target.join("owned-replacement");
            std::fs::write(
                &replacement,
                if case == "corrupt" {
                    b"partial".as_slice()
                } else {
                    b"{\"hidden_size\":384,\"model_type\":\"bert\"}".as_slice()
                },
            )
            .unwrap();
            std::fs::rename(replacement, target.join("config.json")).unwrap();
        }
        let before = cache_inventory(cache.path());
        let cue = dir.path().join(format!("startup-cue-{case}"));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut racer = None;
        if case == "racing" {
            let stop = stop.clone();
            let target = target.clone();
            let cue = cue.clone();
            let pinned = pinned.clone();
            racer = Some(std::thread::spawn(move || {
                while !cue.exists() && !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                let mut index = 0;
                while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                    let replacement = target.join("owned-race-replacement");
                    std::fs::write(
                        &replacement,
                        if index % 2 == 0 {
                            pinned.as_slice()
                        } else {
                            b"{\"hidden_size\":384,\"model_type\":\"bert\"}"
                        },
                    )
                    .unwrap();
                    std::fs::rename(replacement, target.join("config.json")).unwrap();
                    index += 1;
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }));
        }
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "cached_assets_case_child", "--ignored", "--nocapture"])
            .env("XDG_CACHE_HOME", cache.path())
            .env("INSPECTOR_CASE_DB", &path)
            .env(
                "INSPECTOR_CASE_MODE",
                if case == "valid" {
                    "valid"
                } else if case == "racing" {
                    "either"
                } else {
                    "none"
                },
            )
            .env("INSPECTOR_CASE_CUE", &cue)
            .output()
            .unwrap();
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(racer) = racer {
            racer.join().unwrap();
        }
        assert!(
            child.status.success(),
            "case {case}: {}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
        std::fs::remove_file(&cue).unwrap();
        if case != "racing" {
            assert_eq!(cache_inventory(cache.path()), before, "cache invariant for {case}");
        } else {
            let mut names = std::fs::read_dir(&target)
                .unwrap()
                .map(|v| v.unwrap().file_name().to_string_lossy().to_string())
                .collect::<Vec<_>>();
            names.sort();
            assert_eq!(names, ["config.json", "model.safetensors", "tokenizer.json"]);
        }
    }
}
#[test]
#[ignore = "owned cached-assets subprocess only"]
fn cached_assets_case_child() {
    let path = std::path::PathBuf::from(std::env::var_os("INSPECTOR_CASE_DB").unwrap());
    let mode = std::env::var("INSPECTOR_CASE_MODE").unwrap();
    let cue = std::path::PathBuf::from(std::env::var_os("INSPECTOR_CASE_CUE").unwrap());
    let scratch = tempfile::tempdir().unwrap();
    let mut worker=OwnedWorker::spawn_registered(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}}),|_|{std::fs::write(cue,b"ready").unwrap();}).unwrap();
    let catalog = worker.call(json!({"op":"catalog"})).unwrap();
    let available = catalog["database"]["model_runtime"] == "available_cached_cpu";
    if mode == "valid" {
        assert!(available);
    } else if mode == "none" {
        assert!(!available);
    }
    let serving = worker.call(json!({"op":"serving","query":"puppy","limit":10}));
    if available {
        let data = serving.unwrap();
        assert!(data["query_context"]["vector_hits"].as_u64().unwrap() > 0);
        assert_eq!(data["query_context"]["text_hits"], 0);
        assert!(data["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|v| v["evidence"]["vector_rank"].is_u64()));
    } else {
        assert_eq!(serving.unwrap_err().error, "model_unavailable");
    }
    let lexical=worker.call(json!({"op":"lexical","route":"body","query":"domestic","property":null,"filter":{},"view":{},"limit":10})).unwrap();
    assert!(!lexical["items"].as_array().unwrap().is_empty());
    worker.close().unwrap();
}

#[test]
#[cfg(not(feature = "cached-cpu"))]
#[ignore = "mandatory feature-off admission with a real cached-vector store; requires owned wheel and pinned assets"]
fn feature_off_build_never_probes_cached_assets() {
    let dir = tempfile::tempdir().unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
    let fixture = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("src/inspector/tools/vector-fixture.py"))
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(fixture.status.success(), "{}", String::from_utf8_lossy(&fixture.stderr));
    let path = String::from_utf8(fixture.stdout).unwrap().trim().to_owned();
    let cache = dir.path().join("unreadable-cache");
    let asset = cache.join("fathomdb/embedders/0b2926f8a9b1/config.json");
    std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
    let fifo = std::ffi::CString::new(asset.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "feature_off_cache_probe_helper", "--nocapture"])
        .env("INSPECTOR_FEATURE_OFF_DATABASE", &path)
        .env("XDG_CACHE_HOME", cache)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[cfg(not(feature = "cached-cpu"))]
#[ignore = "subprocess helper for feature_off_build_never_probes_cached_assets"]
fn feature_off_cache_probe_helper() {
    let path =
        std::path::PathBuf::from(std::env::var_os("INSPECTOR_FEATURE_OFF_DATABASE").unwrap());
    let scratch = tempfile::tempdir().unwrap();
    let mut worker = OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")), scratch.path(), json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let catalog = worker.call(json!({"op":"catalog"})).unwrap();
    assert_eq!(catalog["database"]["model_support"], "not_compiled");
    assert_eq!(catalog["database"]["model_runtime"], "unavailable_no_model_loaded");
    assert_eq!(catalog["counts"]["node"]["revisions"], "2");
    let view =
        std::fs::read_dir(scratch.path()).unwrap().next().unwrap().unwrap().path().join("view");
    assert_eq!(std::fs::read_dir(view).unwrap().count(), 0);
    worker.close().unwrap();
}

#[test]
fn stored_graph_has_exact_directed_multiedges_and_ambiguous_retained_endpoints() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut w = OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let page=w.call(json!({"op":"page","class":"node","after":"0","limit":20,"filter":{"identity":"derived"}})).unwrap();
    let h = page["items"][1]["handle"].as_str().unwrap();
    let result = w.call(
        json!({"op":"graph","handle":h,"direction":"both","kind":null,"depth":3,"expanded":[h]}),
    );
    assert!(result.is_ok(), "stored directed graph must be supported: {result:?}");
    let g = result.unwrap();
    assert_eq!(g["status"], "complete");
    let mut ids = g["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["logical_id"].as_str().unwrap())
        .collect::<Vec<_>>();
    ids.sort();
    assert_eq!(ids, vec!["cycle", "missing", "parallel-a", "parallel-b", "self"]);
    let edge = g["edges"].as_array().unwrap().iter().find(|v| v["logical_id"] == "self").unwrap();
    assert_eq!(edge["from"]["resolution"], "many");
    assert_eq!(
        edge["from"]["targets"].as_array().unwrap().len(),
        2,
        "never choose the current revision for a logical endpoint"
    );
    let missing =
        g["edges"].as_array().unwrap().iter().find(|v| v["logical_id"] == "missing").unwrap();
    assert_eq!(missing["to"]["resolution"], "zero");
    let long =
        g["edges"].as_array().unwrap().iter().find(|v| v["logical_id"] == "parallel-a").unwrap();
    let mut body = String::new();
    let mut offset = "0".to_string();
    loop {
        let chunk=w.call(json!({"op":"chunk","handle":long["handle"],"field":"body","offset":offset,"limit":262144})).unwrap();
        body.push_str(chunk["text"].as_str().unwrap());
        if let Some(n) = chunk["continuation"].as_str() {
            offset = n.into();
        } else {
            break;
        }
    }
    assert_eq!(body, "full edge é🦀<img src=https://evil.invalid/x>".repeat(5000));
    for (direction, expected) in [("outgoing", 4), ("incoming", 2)] {
        let f=w.call(json!({"op":"graph","handle":h,"direction":direction,"kind":null,"depth":1,"expanded":[h]})).unwrap();
        assert_eq!(f["edges"].as_array().unwrap().len(), expected);
    }
    let f=w.call(json!({"op":"graph","handle":h,"direction":"both","kind":"supports","depth":1,"expanded":[h]})).unwrap();
    assert_eq!(f["edges"].as_array().unwrap().len(), 3);
    assert_eq!(f["scope"]["absence_cause"], "undetermined");
    w.close().unwrap();
}

#[test]
fn investigation_keeps_registered_dependencies_and_class_qualified_revision_history_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut w=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let p=w.call(json!({"op":"page","class":"node","after":"0","limit":10,"filter":{"identity":"derived"}})).unwrap();
    let h = p["items"][0]["handle"].as_str().unwrap();
    let result = w.call(json!({"op":"investigate","handle":h}));
    assert!(
        result.is_ok(),
        "retained-history and registered-dependency investigation is required: {result:?}"
    );
    let a = result.unwrap();
    assert_eq!(
        a["revisions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["revision_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["derived-r1", "derived-r2"]
    );
    assert!(a["revisions"].as_array().unwrap().iter().all(|v| v["class"] == "node"));
    let source =
        a["links"].as_array().unwrap().iter().find(|v| v["kind"] == "recorded_source").unwrap();
    assert_eq!(source["target"]["revision_id"], "source-r1");
    assert_eq!(source["metadata"]["locator_kind"], "whole_body");
    let dependency = a["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["kind"] == "registered_dependency_to_source")
        .unwrap();
    assert_eq!(dependency["target"]["revision_id"], "source-r1");
    assert_eq!(dependency["metadata"]["dependency_id"], "graph-dependency");
    let source_h = source["target"]["handle"].as_str().unwrap();
    let reverse = w.call(json!({"op":"investigate","handle":source_h})).unwrap();
    let dependents = reverse["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == "registered_dependency_to_dependents")
        .collect::<Vec<_>>();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0]["target"]["revision_id"], "derived-r1");
    let detail = w.call(json!({"op":"detail","handle":a["revisions"][0]["handle"]})).unwrap();
    assert!(detail["content"]["text"].as_str().unwrap().contains("derived body"));
    assert_ne!(a["context"]["operation"], detail["context"]["operation"]);
    assert_eq!(a["scope"]["retrieval_absence_cause"], "undetermined");
    w.close().unwrap();
}

#[test]
fn native_graph_evidence_records_actual_frozen_target_and_terminal_edge_separately() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_graph(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut w=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let p=w.call(json!({"op":"page","class":"node","after":"0","limit":20,"filter":{"identity":"derived"}})).unwrap();
    let result=w.call(json!({"op":"graph_serving","handle":p["items"][1]["handle"],"direction":"outgoing","kind":"supports","depth":1}));
    assert!(result.is_ok(), "actual frozen graph evidence must be supported: {result:?}");
    let g = result.unwrap();
    assert_eq!(g["items"].as_array().unwrap().len(), 1);
    let hit = &g["items"][0];
    assert_eq!(hit["target"]["revision_id"], "alpha-r1");
    assert_eq!(hit["payload"]["text"],"native graph alpha target","native graph must preserve the original observed payload separately from later record hydration");
    assert_eq!(hit["payload"]["scope"], "native_frozen_graph_expand");
    assert_eq!(hit["terminal_edge"]["revision_id"], "parallel-a-r1");
    assert!(hit["evidence"]["target_reference"].as_str().unwrap().starts_with("fdbgev1."));
    assert!(hit["evidence"]["edge_reference"].as_str().unwrap().starts_with("fdbgev1."));
    assert_eq!(g["query_context"]["route"], "engine_frozen_graph_expand");
    assert_eq!(g["scope"]["hydration_snapshot"], "separate_later_policy_read");
    assert_eq!(g["scope"]["absence_cause"], "undetermined");
    assert!(g["admission"]["charged_bytes"].as_u64().unwrap() <= 64 * 1024 * 1024);
    w.close().unwrap();
}

#[test]
fn native_graph_unavailable_provenance_is_a_named_refusal_and_keeps_the_worker_usable() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixtures::database(dir.path());
    fixtures::add_nullable(&path);
    let scratch = tempfile::tempdir().unwrap();
    let mut w=OwnedWorker::spawn(Path::new(env!("CARGO_BIN_EXE_fathomdb-inspector")),scratch.path(),json!({"path":path,"policy":{"no_show_inactive":false,"show_erased":false,"show_payloads":false}})).unwrap();
    let p = w
        .call(
            json!({"op":"page","class":"node","after":"0","limit":1,"filter":{"identity":"alpha"}}),
        )
        .unwrap();
    let error=w.call(json!({"op":"graph_serving","handle":p["items"][0]["handle"],"direction":"outgoing","kind":null,"depth":1})).unwrap_err();
    assert_eq!(
        error.error, "native_query_refused",
        "missing evidence must not become a malformed transport reply"
    );
    assert!(w.call(json!({"op":"detail","handle":p["items"][0]["handle"]})).is_ok());
    w.close().unwrap();
}

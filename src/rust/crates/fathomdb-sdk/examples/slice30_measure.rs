//! Provider-free Slice30 direct-text measurement adapter. See the committed
//! dev/experiments/0.8.29-slice30-measurements/README.md for commands and scope.

use fathomdb_engine::{EmbedderChoice, Engine as CoreEngine, ReadView, SearchFilter};
use fathomdb_sdk::{
    Engine as SdkEngine, InitialState, OpenOptions, PreparedWrite, SourceId, TextSearchOptions,
};
use serde_json::{json, Value};
use std::path::Path;

enum Adapter {
    Core(Box<CoreEngine>),
    Sdk(Box<SdkEngine>),
}

impl Adapter {
    fn open(adapter: &str, path: &Path) -> Self {
        match adapter {
            "engine" => Self::Core(Box::new(
                CoreEngine::open_with_choice(path, EmbedderChoice::None)
                    .expect("open core without provider")
                    .engine,
            )),
            "sdk" => Self::Sdk(Box::new(
                SdkEngine::open(path, OpenOptions::default()).expect("open SDK without provider"),
            )),
            _ => panic!("unknown adapter"),
        }
    }

    fn write(&self, batch: &[PreparedWrite]) -> u64 {
        match self {
            Self::Core(engine) => engine.write(batch).expect("core write").cursor,
            Self::Sdk(engine) => engine.write(batch).expect("SDK write").cursor,
        }
    }

    fn close(&self) {
        match self {
            Self::Core(engine) => engine.close().expect("core close"),
            Self::Sdk(engine) => engine.close().expect("SDK close"),
        }
    }
}

fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new("test:slice30").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn observe(engine: &Adapter, query: &str, kind: Option<&str>) -> Value {
    let filter = kind.map(|kind| {
        let mut filter = SearchFilter::default();
        filter.kind = Some(kind.into());
        filter
    });
    let started = std::time::Instant::now();
    let result = match engine {
        Adapter::Core(engine) => engine
            .search_text_only_filtered_with_snippets(query, filter, &ReadView::default(), 10, None)
            .map_err(|error| format!("{error:?}")),
        Adapter::Sdk(engine) => {
            let mut options = TextSearchOptions::default();
            options.filter = filter;
            engine.search_text_only(query, options).map_err(|error| format!("{error:?}"))
        }
    };
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).expect("bounded sample duration");
    match result {
        Ok(result) => json!({"ok": true, "elapsed_ns": elapsed_ns,
            "ids": result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(),
            "bodies": result.results.iter().map(|hit| &hit.body).collect::<Vec<_>>(),
            "returned_hits": result.results.len(), "projection_cursor": result.projection_cursor,
            "soft_fallback": format!("{:?}", result.soft_fallback), "degradations": format!("{:?}", result.degradations),
            "search_snapshot": null, "snapshot_observation": "unavailable"}),
        Err(error) => json!({"ok": false, "elapsed_ns": elapsed_ns, "ids": [], "error": error}),
    }
}

struct Schedule {
    searches: Vec<Value>,
    batches: Vec<Value>,
    probes: Vec<Value>,
}

#[cfg(test)]
fn run_schedule(
    engine: &Adapter,
    queries: &[String],
    samples: usize,
    batch_size: usize,
    searches_per_batch: usize,
) -> Schedule {
    run_schedule_observed(engine, queries, samples, batch_size, searches_per_batch, &mut |_, _| {})
}

fn run_schedule_observed(
    engine: &Adapter,
    queries: &[String],
    samples: usize,
    batch_size: usize,
    searches_per_batch: usize,
    observer: &mut dyn FnMut(&str, &Value),
) -> Schedule {
    assert!(!queries.is_empty() && searches_per_batch > 0);
    assert!(batch_size == 1 || batch_size == 10);
    let mut output = Schedule { searches: vec![], batches: vec![], probes: vec![] };
    for sample in 0..samples {
        if sample % searches_per_batch == 0 {
            let batch = sample / searches_per_batch;
            let slots: Vec<usize> =
                if batch_size == 1 { vec![batch % 10] } else { (0..10).collect() };
            let writes: Vec<_> = slots
                .iter()
                .map(|slot| {
                    node(
                        &format!("s30marker-{slot}"),
                        "note",
                        &format!("s30markercurrent{slot} revision{batch}"),
                    )
                })
                .collect();
            let start = std::time::Instant::now();
            let cursor = engine.write(&writes);
            output.batches.push(json!({"before_sample": sample, "rows": writes.len(),
                "write_ns": u64::try_from(start.elapsed().as_nanos()).unwrap(),
                "after_write_cursor": cursor, "visibility_generation": null,
                "authority_observation": "write receipt only; search snapshot unavailable"}));
            observer("batches", output.batches.last().unwrap());
            for slot in slots {
                let mut probe = observe(engine, &format!("s30markercurrent{slot}"), Some("note"));
                probe["marker_current"] = json!(
                    probe["ids"] == json!([format!("l:s30marker-{slot}")])
                        && probe["bodies"]
                            == json!([format!("s30markercurrent{slot} revision{batch}")])
                );
                probe["before_sample"] = json!(sample);
                observer("marker-probes", &probe);
                output.probes.push(probe);
            }
        }
        let mut result = observe(engine, &queries[sample % queries.len()], Some("doc"));
        result["sample_index"] = json!(sample);
        result["query_index"] = json!(sample % queries.len());
        observer("samples", &result);
        output.searches.push(result);
    }
    output
}

fn raw_match_count(path: &Path, query: &str, kind: Option<&str>) -> u64 {
    let expression = fathomdb_query::compile_text_query(query).match_expression;
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open own fixture for untimed match counts");
    if let Some(kind) = kind {
        connection
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1 AND kind=?2",
                rusqlite::params![expression, kind],
                |row| row.get(0),
            )
            .expect("filtered raw FTS count")
    } else {
        connection
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1",
                [expression],
                |row| row.get(0),
            )
            .expect("raw FTS count")
    }
}

fn read_jsonl(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("read owned JSONL input")
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON string line"))
        .collect()
}

fn write_jsonl(path: &Path, rows: &[Value]) {
    use std::io::Write;
    let mut output =
        std::io::BufWriter::new(std::fs::File::create(path).expect("create owned output"));
    for row in rows {
        serde_json::to_writer(&mut output, row).expect("write record");
        output.write_all(b"\n").expect("write newline");
    }
    output.flush().expect("flush records");
}

fn run_cell(config: &Value, out: &Path) -> Value {
    run_cell_observed(config, out, &mut |_, _| {})
}

fn run_cell_observed(config: &Value, out: &Path, observer: &mut dyn FnMut(&str, &Value)) -> Value {
    let number =
        |key: &str| usize::try_from(config[key].as_u64().expect(key)).expect("usize bound");
    let n = number("n");
    let warmup = number("warmup");
    let samples = number("samples");
    let corpus = read_jsonl(config["corpus"].as_str().expect("corpus path"));
    let queries = read_jsonl(config["queries"].as_str().expect("queries path"));
    assert!(n > 0 && corpus.len() >= n && !queries.is_empty() && samples > 0);
    let filtered = config["family"] == "P2";
    assert!(filtered || config["family"] == "P1");
    std::fs::create_dir_all(out).expect("create owned cell output");
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "measurement store must be fresh");
    let open_started = std::time::Instant::now();
    let seed = Adapter::open("sdk", &store);
    let seed_open_ns = u64::try_from(open_started.elapsed().as_nanos()).unwrap();
    let write_started = std::time::Instant::now();
    for start in (0..n).step_by(1024) {
        let batch: Vec<_> = corpus[start..n.min(start + 1024)]
            .iter()
            .enumerate()
            .map(|(offset, body)| node(&format!("s30doc-{}", start + offset), "doc", body))
            .collect();
        seed.write(&batch);
    }
    if filtered {
        let markers: Vec<_> = (0..10)
            .map(|slot| node(&format!("s30marker-{slot}"), "note", "initial marker"))
            .collect();
        seed.write(&markers);
    }
    let seed_write_ns = u64::try_from(write_started.elapsed().as_nanos()).unwrap();
    seed.close();
    drop(seed);
    let mut rows = Vec::new();
    let mut warmups = Vec::new();
    let mut batches = Vec::new();
    let mut probes = Vec::new();
    let mut passes = Vec::new();
    for adapter in config["adapters"].as_array().expect("adapter list") {
        let name = adapter.as_str().expect("adapter name");
        let opening = std::time::Instant::now();
        let engine = Adapter::open(name, &store);
        let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
        for sample in 0..warmup {
            let mut row =
                observe(&engine, &queries[sample % queries.len()], filtered.then_some("doc"));
            row["adapter"] = json!(name);
            row["query_index"] = json!(sample % queries.len());
            observer("warmups", &row);
            warmups.push(row);
        }
        let loop_started = std::time::Instant::now();
        let mut measured = if filtered {
            let schedule = run_schedule_observed(
                &engine,
                &queries,
                samples,
                number("batch_size"),
                number("searches_per_batch"),
                observer,
            );
            batches.extend(schedule.batches);
            probes.extend(schedule.probes);
            schedule.searches
        } else {
            (0..samples)
                .map(|sample| {
                    let row = observe(&engine, &queries[sample % queries.len()], None);
                    observer("samples", &row);
                    row
                })
                .collect()
        };
        let wall_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
        for (sample, row) in measured.iter_mut().enumerate() {
            row["adapter"] = json!(name);
            row["repeat"] = config["repeat"].clone();
            row["sample_index"] = json!(sample);
            row["query_index"] = json!(sample % queries.len());
        }
        rows.extend(measured);
        let closing = std::time::Instant::now();
        engine.close();
        drop(engine);
        passes.push(json!({"adapter":name,"wall_ns":wall_ns,"open_ns":open_ns,
            "close_ns":u64::try_from(closing.elapsed().as_nanos()).unwrap(),
            "throughput_scope":if filtered {"natural searches over write/probe/search loop"} else {"natural search/record loop"}}));
    }
    let matches: Vec<_> = queries
        .iter()
        .enumerate()
        .map(|(index, query)| {
            json!({"query_index":index,
        "raw_fts_matches":raw_match_count(&store, query, filtered.then_some("doc"))})
        })
        .collect();
    let connection =
        rusqlite::Connection::open_with_flags(&store, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read own post-cell counts");
    let count = |sql| connection.query_row(sql, [], |row| row.get::<_, u64>(0)).unwrap();
    let natural =
        count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL");
    let markers =
        count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL");
    let vectors = count("SELECT COUNT(*) FROM _fathomdb_vector_rows");
    let format: u32 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    write_jsonl(&out.join("samples.jsonl"), &rows);
    write_jsonl(&out.join("warmups.jsonl"), &warmups);
    write_jsonl(&out.join("batches.jsonl"), &batches);
    write_jsonl(&out.join("marker-probes.jsonl"), &probes);
    write_jsonl(&out.join("matches.jsonl"), &matches);
    let complete = natural == n as u64
        && vectors == 0
        && format == 37
        && markers == if filtered { 10 } else { 0 }
        && rows.iter().chain(&warmups).all(|row| row["ok"] == true)
        && probes.iter().all(|row| row["marker_current"] == true);
    let result = json!({"complete":complete,"sample_count":rows.len(),"warmup_count":warmups.len(),
        "filtered":filtered,"natural_records":natural,"marker_records":markers,"vector_rows":vectors,
        "format":format,"query_count":queries.len(),"batch_count":batches.len(),"marker_probe_count":probes.len(),
        "seed_open_ns":seed_open_ns,"seed_write_ns":seed_write_ns,"passes":passes,
        "snapshot_observation":"unavailable; write receipts are not search snapshot authority",
        "cache_note":"P2 marker proof calls intervene between each write and natural searches; OS caches uncontrolled"});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30_measure cell.json owned-output-directory");
    let config: Value = serde_json::from_slice(&std::fs::read(&args[1]).expect("read frozen cell"))
        .expect("cell JSON");
    let result = run_cell(&config, Path::new(&args[2]));
    println!("{result}");
    if result["complete"] != true {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_cell_retains_completed_raw_checkpoints() {
        const CHILD_DIRECTORY: &str = "FATHOMDB_SLICE30_RETENTION_CHILD_DIRECTORY";
        if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
            let directory = std::path::PathBuf::from(directory);
            let config: Value =
                serde_json::from_slice(&std::fs::read(directory.join("cell.json")).unwrap())
                    .unwrap();
            let mut observed = 0;
            run_cell_observed(&config, &directory.join("out"), &mut |stream, _| {
                if stream == "samples" {
                    observed += 1;
                    if observed == 16 {
                        std::fs::write(directory.join("ready"), b"16 completed observations")
                            .unwrap();
                        std::thread::sleep(std::time::Duration::from_secs(30));
                        panic!("parent did not stop its retention child");
                    }
                }
            });
            panic!("retention child unexpectedly completed");
        }
        struct OwnedChild(std::process::Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                self.0.wait().expect("reap only owned retention child");
            }
        }
        let records = |path: &Path| -> Vec<Value> {
            std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        };
        for family in ["P1", "P2"] {
            let directory = tempfile::tempdir().unwrap();
            let corpus = directory.path().join("corpus.jsonl");
            let queries = directory.path().join("queries.jsonl");
            std::fs::write(&corpus, "\"alphaonly evidence\"\n\"betaonly evidence\"\n").unwrap();
            std::fs::write(&queries, "\"alphaonly\"\n\"betaonly\"\n").unwrap();
            let config = json!({"family":family,"n":2,"adapters":["sdk"],"warmup":4,"samples":64,
                "batch_size":1,"searches_per_batch":1,"repeat":0,"corpus":corpus,"queries":queries});
            std::fs::write(
                directory.path().join("cell.json"),
                serde_json::to_vec(&config).unwrap(),
            )
            .unwrap();
            let log = std::fs::File::create(directory.path().join("child.log")).unwrap();
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tests::interrupted_cell_retains_completed_raw_checkpoints",
                    "--nocapture",
                ])
                .env(CHILD_DIRECTORY, directory.path())
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap();
            let child = OwnedChild(child);
            let started = std::time::Instant::now();
            let ready = directory.path().join("ready");
            while !ready.exists() && started.elapsed() < std::time::Duration::from_secs(20) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let acknowledged = ready.exists();
            drop(child);
            assert!(
                acknowledged,
                "actual 16-observation rendezvous absent: {}",
                std::fs::read_to_string(directory.path().join("child.log")).unwrap()
            );
            let out = directory.path().join("out");
            let samples = records(&out.join("samples.jsonl"));
            assert_eq!(samples.len(), 16, "{family}: completed raw checkpoint lost");
            assert_eq!(records(&out.join("warmups.jsonl")).len(), 4);
            assert!(!out.join("result.json").exists());
            for (index, row) in samples.iter().enumerate() {
                assert_eq!(row["ids"], json!([format!("l:s30doc-{}", index % 2)]));
                assert_eq!(row["adapter"], "sdk");
                assert_eq!(row["query_index"], index % 2);
                assert_eq!(row["sample_index"], index);
                assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
            }
            if family == "P2" {
                assert_eq!(records(&out.join("batches.jsonl")).len(), 16);
                let probes = records(&out.join("marker-probes.jsonl"));
                assert_eq!(probes.len(), 16);
                assert!(probes.iter().all(|row| row["marker_current"] == true));
            }
        }
    }

    #[test]
    fn observation_records_real_text_identity_across_reopen() {
        for name in ["engine", "sdk"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("literal.sqlite");
            let engine = Adapter::open(name, &path);
            engine.write(&[
                node("alpha", "doc", "alphaonly evidence"),
                node("beta", "doc", "betaonly evidence"),
            ]);
            let result = observe(&engine, "alphaonly", None);
            assert_eq!(result["ids"], json!(["l:alpha"]), "{name}: {result}");
            assert_eq!(result["ok"], true);
            assert!(result["elapsed_ns"].as_u64().unwrap() > 0);
            engine.close();
            drop(engine);
            let reopened = Adapter::open(name, &path);
            assert_eq!(observe(&reopened, "betaonly", None)["ids"], json!(["l:beta"]));
            reopened.close();
        }
    }

    #[test]
    fn schedule_observes_commits_search_counts_and_current_marker_bodies() {
        let cases = [
            (1, 1, 4, vec![0, 1, 2, 3], 4, 3, "s30markercurrent3 revision3"),
            (10, 10, 20, vec![0, 10], 20, 9, "s30markercurrent9 revision1"),
            (1, 100, 200, vec![0, 100], 2, 1, "s30markercurrent1 revision1"),
        ];
        for (batch_size, ratio, samples, starts, probe_count, last_slot, expected_body) in cases {
            let directory = tempfile::tempdir().unwrap();
            let engine = Adapter::open("sdk", &directory.path().join("schedule.sqlite"));
            engine.write(&[node("natural", "doc", "naturalalpha")]);
            for slot in 0..10 {
                engine.write(&[node(&format!("s30marker-{slot}"), "note", "initial marker")]);
            }
            let result =
                run_schedule(&engine, &["naturalalpha".into()], samples, batch_size, ratio);
            assert_eq!(result.searches.len(), samples);
            assert!(result.searches.iter().all(|row| row["ids"] == json!(["l:natural"])));
            assert_eq!(
                result
                    .batches
                    .iter()
                    .map(|row| row["before_sample"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                starts
            );
            assert_eq!(result.probes.len(), probe_count);
            assert!(result.probes.iter().all(|row| row["ok"] == true));
            assert_eq!(
                observe(&engine, &format!("s30markercurrent{last_slot}"), Some("note"))["bodies"],
                json!([expected_body])
            );
            engine.close();
        }
    }

    #[test]
    fn match_observer_uses_product_compiler_and_keeps_raw_count_separate() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("matches.sqlite");
        let engine = Adapter::open("sdk", &path);
        for index in 0..12 {
            engine.write(&[node(
                &format!("alpha-{index}"),
                "doc",
                &format!("alpha evidence{index}"),
            )]);
        }
        engine
            .write(&[node("marker", "note", "alpha marker"), node("stopword", "doc", "the only")]);
        let query = "The ALPHA, alpha?";
        assert_eq!(raw_match_count(&path, query, None), 13);
        assert_eq!(raw_match_count(&path, query, Some("doc")), 12);
        let result = observe(&engine, query, Some("doc"));
        assert_eq!(result["returned_hits"], 10);
        assert!(result["ids"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id.as_str().unwrap().starts_with("l:alpha-")));
        engine.close();
    }

    #[test]
    fn cell_keeps_warmups_separate_and_seeds_one_fresh_natural_store() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("corpus.jsonl");
        let queries = directory.path().join("queries.jsonl");
        std::fs::write(&corpus, "\"alphaonly evidence\"\n\"betaonly evidence\"\n").unwrap();
        std::fs::write(&queries, "\"alphaonly\"\n\"betaonly\"\n").unwrap();
        let out = directory.path().join("out");
        let config = json!({"family":"P1", "n":2, "adapters":["engine","sdk"],
            "warmup":3, "samples":4, "repeat":0, "corpus":corpus, "queries":queries});
        let result = run_cell(&config, &out);
        assert_eq!(result["sample_count"], 8);
        assert_eq!(result["warmup_count"], 6);
        assert_eq!(result["filtered"], false);
        assert_eq!(result["natural_records"], 2);
        let rows: Vec<Value> = std::fs::read_to_string(out.join("samples.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 8);
        assert_eq!(
            rows.iter().map(|row| row["ids"].clone()).collect::<Vec<_>>(),
            vec![
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"])
            ]
        );
        let connection = rusqlite::Connection::open_with_flags(
            out.join("store.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, u64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row
                    .get::<_, u64>(0))
                .unwrap(),
            0
        );
        drop(connection);
        assert!(std::panic::catch_unwind(|| run_cell(&config, &out)).is_err());
    }

    #[test]
    fn cell_labels_filtered_marker_schedule_separately_from_p1() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("corpus.jsonl");
        let queries = directory.path().join("queries.jsonl");
        std::fs::write(&corpus, "\"naturalalpha\"\n\"naturalbeta\"\n").unwrap();
        std::fs::write(&queries, "\"naturalalpha\"\n").unwrap();
        let out = directory.path().join("out");
        let config = json!({"family":"P2", "n":2, "adapters":["sdk"], "batch_size":1,
            "searches_per_batch":1, "warmup":3, "samples":4, "repeat":0,
            "corpus":corpus, "queries":queries});
        let result = run_cell(&config, &out);
        assert_eq!(result["sample_count"], 4);
        assert_eq!(result["warmup_count"], 3);
        assert_eq!(result["filtered"], true);
        assert_eq!(result["natural_records"], 2);
        assert_eq!(result["marker_records"], 10);
        assert_eq!(result["batch_count"], 4);
        assert_eq!(result["marker_probe_count"], 4);
        assert_eq!(result["complete"], true);
    }
}

//! Deterministic-provider query caller for owned D source fixtures.

use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ReadView, SourceId, TestEmbedderChoice,
};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dense_template;
use super::varying_embedder::VaryingEmbedder;

pub(super) fn run(config: &Value, out: &Path) {
    let origin = Instant::now();
    dense_template::run(config, &out.join("template"));
    let source = out.join("template/template.sqlite");
    let store = out.join("query.sqlite");
    assert!(!store.exists());
    std::fs::copy(&source, &store).unwrap();
    let opened = Engine::open_with_test_choice_for_test(
        &store,
        TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
    )
    .unwrap();
    let engine = &opened.engine;
    assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
    let mut output = std::fs::File::create(out.join("queries.jsonl")).unwrap();
    let warmups = usize::try_from(config["warmups"].as_u64().unwrap()).unwrap();
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    for sample in 0..warmups {
        write_row(&mut output, &query_row(engine, origin, "warmup", sample));
    }
    if let Some(records) = config["write_records"].as_array() {
        let cohort = usize::try_from(config["cohort"].as_u64().unwrap()).unwrap();
        assert!(cohort > 0);
        let writes = std::thread::scope(|scope| {
            let (start_readiness, readiness_ready) = std::sync::mpsc::sync_channel(1);
            let (stop_readiness, readiness_stop) = std::sync::mpsc::sync_channel(1);
            let readiness = if config["readiness"].as_bool().unwrap_or(false) {
                Some(scope.spawn(move || {
                    readiness_ready.recv_timeout(Duration::from_secs(5)).unwrap();
                    let mut reads = Vec::new();
                    loop {
                        let started = Instant::now();
                        let start_ns = u64::try_from(started.duration_since(origin).as_nanos()).unwrap();
                        let result = engine.read_embedding_readiness();
                        let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
                        reads.push(match result {
                            Ok(readiness) => json!({"ok":true,"start_ns":start_ns,"end_ns":end_ns,
                                "pending_count":readiness.pending_count,"usable_embedder":readiness.usable_embedder,
                                "state":format!("{:?}",readiness.state)}),
                            Err(error) => json!({"ok":false,"start_ns":start_ns,"end_ns":end_ns,
                                "pending_count":null,"error":format!("{error:?}")}),
                        });
                        let until_next = (started + Duration::from_millis(100)).saturating_duration_since(Instant::now());
                        match readiness_stop.recv_timeout(until_next) {
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
                            _ => break,
                        }
                    }
                    reads
                }))
            } else {
                None
            };
            let (release, ready) = std::sync::mpsc::sync_channel(1);
            let writer = scope.spawn(move || {
                let launch_ns: u64 = ready.recv_timeout(Duration::from_secs(5)).unwrap();
                let mut writes = Vec::new();
                for rows in records.chunks(cohort) {
                    let batch: Vec<_> = rows.iter().map(|row| PreparedWrite::Node {
                        kind: row["kind"].as_str().unwrap().into(),
                        body: row["body"].as_str().unwrap().into(),
                        source_id: SourceId::new("test:slice30-d-overlap").unwrap(),
                        logical_id: Some(row["id"].as_str().unwrap().into()),
                        state: InitialState::Active,
                        reason: None, valid_from: None, valid_until: None,
                    }).collect();
                    let start_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
                    let receipt = engine.write(&batch).unwrap();
                    let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
                    writes.push(json!({"ok":true,"launch_ns":launch_ns,"start_ns":start_ns,"end_ns":end_ns,
                        "rows":batch.len(),"cursor":receipt.cursor,"row_cursors":receipt.row_cursors}));
                }
                writes
            });
            let launch_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
            if readiness.is_some() {
                start_readiness.send(()).unwrap();
            }
            release.send(launch_ns).unwrap();
            for sample in 0..samples {
                write_row(&mut output, &query_row(engine, origin, "timed", sample));
            }
            let writes = writer.join().unwrap();
            if let Some(readiness) = readiness {
                let _ = stop_readiness.send(());
                let reads = readiness.join().unwrap();
                std::fs::write(
                    out.join("readiness.json"),
                    serde_json::to_vec_pretty(&reads).unwrap(),
                )
                .unwrap();
            }
            writes
        });
        std::fs::write(out.join("writes.json"), serde_json::to_vec_pretty(&writes).unwrap())
            .unwrap();
        engine.drain(5000).unwrap();
    } else {
        for sample in 0..samples {
            write_row(&mut output, &query_row(engine, origin, "timed", sample));
        }
    }
    output.flush().unwrap();
    engine.close().unwrap();
    std::fs::write(out.join("complete.json"), b"{\"complete\":true}\n").unwrap();
}

fn write_row(output: &mut std::fs::File, row: &Value) {
    serde_json::to_writer(&mut *output, row).unwrap();
    output.write_all(b"\n").unwrap();
}

fn query_row(engine: &Engine, origin: Instant, phase: &str, sample: usize) -> Value {
    let start_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
    let started = Instant::now();
    let result = engine
        .search_reranked_view_with_limit(
            "record",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
        )
        .unwrap();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
    json!({"phase":phase,"sample_index":sample,"query":"record",
    "start_ns":start_ns,"end_ns":end_ns,"elapsed_ns":elapsed_ns,"ok":true,
    "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
    "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations)})
}

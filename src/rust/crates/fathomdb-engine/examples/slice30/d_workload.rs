//! Deterministic-provider query caller for owned D source fixtures.

use fathomdb_engine::{Engine, ReadView, TestEmbedderChoice};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

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
    let row = json!({"phase":"timed","sample_index":0,"query":"record",
        "start_ns":start_ns,"end_ns":end_ns,"elapsed_ns":elapsed_ns,"ok":true,
        "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
        "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations)});
    serde_json::to_writer(&mut output, &row).unwrap();
    output.write_all(b"\n").unwrap();
    output.flush().unwrap();
    engine.close().unwrap();
    std::fs::write(out.join("complete.json"), b"{\"complete\":true}\n").unwrap();
}

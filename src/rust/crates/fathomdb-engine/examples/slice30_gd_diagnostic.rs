//! Bounded Slice30 G/D diagnostic driver. See the indexed experiment README.

use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, ReadView, SourceId};
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

#[cfg(feature = "test-hooks")]
#[path = "slice30/dense_template.rs"]
mod dense_template;

fn strings(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("read owned input")
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON string"))
        .collect()
}

fn run(config: &Value, out: &Path) {
    if config["family"] == "D_TEMPLATE" {
        #[cfg(feature = "test-hooks")]
        return dense_template::run(config, out);
        #[cfg(not(feature = "test-hooks"))]
        panic!("D fixture requires its explicit test-hooks artifact");
    }
    assert_eq!(config["family"], "P1");
    let n = usize::try_from(config["n"].as_u64().unwrap()).unwrap();
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    let bodies = strings(config["corpus"].as_str().unwrap());
    let queries = strings(config["queries"].as_str().unwrap());
    assert!(n > 0 && bodies.len() >= n && !queries.is_empty());
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "diagnostic store must be fresh");
    let opened = Engine::open_with_choice(store, EmbedderChoice::None).unwrap();
    let engine = &opened.engine;
    let batch: Vec<_> = bodies[..n]
        .iter()
        .enumerate()
        .map(|(index, body)| PreparedWrite::Node {
            kind: "doc".into(),
            body: body.clone(),
            source_id: SourceId::new("test:slice30-gd").unwrap(),
            logical_id: Some(format!("s30doc-{index}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&batch).unwrap();
    #[cfg(feature = "slice30-measurement")]
    engine.slice30_observe_queries_for_test(true).unwrap();
    let mut output = std::fs::File::create(out.join("samples.jsonl")).unwrap();
    for sample in 0..samples {
        let started = std::time::Instant::now();
        let result = engine
            .search_text_only_filtered_with_snippets(
                &queries[sample % queries.len()],
                None,
                &ReadView::default(),
                10,
                None,
            )
            .unwrap();
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        let row = json!({"ok":true,"elapsed_ns":elapsed_ns,
            "ids":result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>()});
        serde_json::to_writer(&mut output, &row).unwrap();
        output.write_all(b"\n").unwrap();
    }
    output.flush().unwrap();
    #[cfg(feature = "slice30-measurement")]
    {
        let mut observations = std::fs::File::create(out.join("observer.jsonl")).unwrap();
        for record in engine.take_slice30_measurements_for_test().unwrap() {
            serde_json::to_writer(&mut observations, &record).unwrap();
            observations.write_all(b"\n").unwrap();
        }
        observations.flush().unwrap();
    }
    engine.close().unwrap();
    std::fs::write(out.join("result.json"), b"{\"complete\":true}\n").unwrap();
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30_gd_diagnostic cell.json owned-output");
    let config = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    run(&config, Path::new(&args[2]));
}

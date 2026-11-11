// Post-code manual acceptance driver, never a regression test or product helper.
use fathomdb_sdk::{
    read, EffectiveEmbedDevice, EmbedderEvent, Engine, InitialState, OpenOptions, PreparedWrite,
    ProjectionRole, ProjectionSpec, ProjectionVector, SearchFilter, SoftFallbackBranch, SourceId,
    TextSearchOptions,
};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        logical_id: Some(id.into()),
        source_id: SourceId::new("s60-manual").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn edge() -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".into(),
        from: "a".into(),
        to: "b".into(),
        source_id: SourceId::new("s60-manual").unwrap(),
        logical_id: Some("edge".into()),
        body: Some(r#"{"text":"s60metaedge","status":"open","created_at":0}"#.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}
fn produce(root: &Path) {
    let uuid = std::env::var("S60_GPU_UUID").expect("S60_GPU_UUID is required");
    assert!(cfg!(feature = "cuda-producer"), "produce mode requires explicit CUDA build");
    assert_eq!(std::env::var("CUDA_VISIBLE_DEVICES").unwrap(), uuid);
    assert_eq!(std::env::var("FATHOMDB_EMBED_DEVICE").unwrap(), "cuda:0");
    let path = root.join("store.sqlite");
    assert!(!path.exists(), "fresh owned fixture only");
    let producer =
        Engine::open(&path, OpenOptions { use_default_embedder: true, ..Default::default() })
            .unwrap();
    let report = producer.open_report();
    let resolution = report.embedder_device_resolution.as_ref().unwrap();
    assert!(resolution.cuda_compiled);
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), Some(uuid.as_str()));
    let device = match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(device) => device,
        _ => panic!("CPU fallback forbidden"),
    };
    assert_eq!(device.ordinal, 0);
    assert_eq!(device.uuid.as_deref(), Some(uuid.as_str()));
    assert_eq!(resolution.visible_cuda_devices.len(), 1);
    assert_eq!(resolution.visible_cuda_devices[0].visible_ordinal, 0);
    assert_eq!(resolution.visible_cuda_devices[0].uuid, uuid);
    assert!(report.reranker_device_resolution.is_none());
    assert!(report.embedder_gpu_allocation_witness.is_none());
    assert!(report.embedder_download_ms.is_none());
    assert!(report.embedder_mean_centering_required);
    let mut assets = BTreeSet::new();
    for event in &report.embedder_events {
        match event {
            EmbedderEvent::DefaultEmbedderDownload { .. } => panic!("download forbidden"),
            EmbedderEvent::DefaultEmbedderCacheHit { file, cache_path, .. } => {
                assert!(cache_path.starts_with(std::env::var("XDG_CACHE_HOME").unwrap()));
                assets.insert(file.as_str());
            }
            _ => {}
        }
    }
    assert_eq!(assets, BTreeSet::from(["config.json", "tokenizer.json", "model.safetensors"]));
    eprintln!("actual producer open report: {report:?}");
    producer
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "memory".into(),
                    roles: [ProjectionRole::Searchable].into(),
                    fts: None,
                    vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                    source: None,
                },
                ProjectionSpec {
                    name: "group".into(),
                    roles: [ProjectionRole::Filterable].into(),
                    fts: None,
                    vector: None,
                    source: None,
                },
                ProjectionSpec {
                    name: "level".into(),
                    roles: [ProjectionRole::Filterable].into(),
                    fts: None,
                    vector: None,
                    source: None,
                },
            ],
            &[],
        )
        .unwrap();
    producer.write(&[node("a",r#"{"text":"s60metanode","group":"wanted","level":2,"status":"open","created_at":0}"#),node("b","s60 endpoint"),edge()]).unwrap();
    producer.drain(60_000).unwrap();
    let ready = read::embedding_readiness(&producer).unwrap();
    assert_eq!(ready.state.as_str(), "ready");
    assert_eq!(ready.pending_count, 0);
    assert!(ready.usable_embedder);
    let stored = {
        let connection = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get::<_, u64>(0))
                .unwrap(),
            3
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row
                    .get::<_, u64>(0))
                .unwrap(),
            3
        );
        let mut statement=connection.prepare("SELECT c.logical_id,v.created_at,v.status FROM canonical_nodes c JOIN vector_default v ON v.rowid=c.write_cursor WHERE c.logical_id='a' UNION ALL SELECT c.logical_id,v.created_at,v.status FROM canonical_edges c JOIN vector_default v ON v.rowid=c.write_cursor WHERE c.logical_id='edge' ORDER BY 1").unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    };
    assert_eq!(stored.len(), 2);
    assert!(stored.iter().all(|(_, time, status)| *time > 0 && status.is_empty()));
    producer.close().unwrap();
    let manifest = json!({"database":path,"producer_pid":std::process::id(),"selected_uuid":uuid,"visible_ordinal":0,"cuda_compiled":true,"model_downloads":0,"rows":stored,"vector_rows":3});
    std::fs::write(root.join("producer.json"), serde_json::to_vec_pretty(&manifest).unwrap())
        .unwrap();
    println!("{manifest}");
}
fn predicate(args: &Value) -> SearchFilter {
    let mut out = SearchFilter::default();
    out.kind = args["kind"].as_str().map(str::to_owned);
    out.source_type = args["source_type"].as_str().map(str::to_owned);
    out.status = args["status"].as_str().map(str::to_owned);
    out.created_after = args["created_after"].as_i64();
    if let Some(attributes) = args["attributes"].as_object() {
        out.attributes = attributes
            .iter()
            .map(|(key, value)| (key.clone(), value.as_str().unwrap().to_owned()))
            .collect();
    }
    out
}
fn controls(root: &Path) {
    assert!(!cfg!(feature = "cuda-producer"), "CPU/None control must have no CUDA feature");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(root.join("producer.json")).unwrap()).unwrap();
    let path = root.join("store.sqlite");
    assert_eq!(manifest["database"], path.to_str().unwrap());
    let reader = Engine::open(&path, OpenOptions::default()).unwrap();
    assert!(reader.open_report().embedder_device_resolution.is_none());
    let status = read::projection_status(&reader).unwrap();
    assert!(!status.runtime_embedder_available);
    assert_eq!(status.runtime_unavailability_reason.as_str(), "no_runtime");
    for row in manifest["rows"].as_array().unwrap() {
        let id = row[0].as_str().unwrap();
        let time = row[1].as_i64().unwrap();
        assert_eq!(row[2], "");
        let (query, kind, source, reference) = match id {
            "a" => ("s60metanode", "doc", "article", "l:a"),
            "edge" => ("s60metaedge", "link", "edge_fact", "l:edge"),
            _ => panic!("unknown authored id"),
        };
        for (mut args, hit) in [
            (json!({"status":""}), true),
            (json!({"created_after":time}), true),
            (json!({"created_after":time+1}), false),
            (json!({"status":"open"}), false),
            (json!({"status":"","created_after":time,"kind":kind,"source_type":source}), true),
            (
                json!({"status":"","created_after":time,"kind":kind,"source_type":source,"attributes":{"group":"wanted","level":"2"}}),
                id == "a",
            ),
            (
                json!({"status":"","created_after":time,"kind":kind,"source_type":source,"attributes":{"group":"wanted","level":"3"}}),
                false,
            ),
        ] {
            let mut options = TextSearchOptions::default();
            options.filter = Some(predicate(&args));
            options.view.valid_as_of = Some(1_900_000_000);
            options.limit = 100;
            let control = reader.search_text_only(query, options).unwrap();
            let expected = if hit { vec![reference] } else { vec![] };
            assert_eq!(
                control.results.iter().map(|result| result.id.to_prefixed()).collect::<Vec<_>>(),
                expected,
                "authored predicate {query}/{args}"
            );
            let results=control.results.iter().map(|result|json!({"ref":result.id.to_prefixed(),"score":result.score,"display_score":format!("{:.3}",result.score),"kind":kind,"source_id":"s60-manual","body":if id=="a" {r#"{"text":"s60metanode","group":"wanted","level":2,"status":"open","created_at":0}"#} else {r#"{"text":"s60metaedge","status":"open","created_at":0}"#},"arm":match result.branch {SoftFallbackBranch::Text=>"text",SoftFallbackBranch::TextEdge=>"text_edge",_=>panic!("nonlexical arm")}})).collect::<Vec<_>>();
            args["query"] = json!(query);
            args["mode"] = json!("text");
            args["as_of"] = json!(1_900_000_000);
            args["limit"] = json!(100);
            args["hints"] = json!(false);
            println!("{}", json!({"args":args,"expected_refs":expected,"sdk_results":results}));
        }
    }
    reader.close().unwrap();
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    assert_eq!(args.len(), 3);
    let root = std::fs::canonicalize(&args[2]).unwrap();
    assert!(root.starts_with(
        std::path::PathBuf::from(std::env::var("S60_TASK").expect("S60_TASK is required"))
            .join("gpu-metadata")
    ));
    match args[1].as_str() {
        "produce" => produce(&root),
        "controls" => controls(&root),
        _ => panic!("produce or controls required"),
    }
}

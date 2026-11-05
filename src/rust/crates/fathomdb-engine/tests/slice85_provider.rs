use fathomdb_engine::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
fn fixture() -> (tempfile::TempDir, Arc<Engine>) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("provider.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    (dir, engine)
}
#[test]
fn maintenance_refuses_provider_newcomers_before_starting_process() {
    let (dir, engine) = fixture();
    let handle = engine.begin_erasure_maintenance(1000).unwrap();
    let mut results = Vec::new();
    for task in ["extract", "consolidate"] {
        let marker = dir.path().join(task);
        let code = format!(
            "from pathlib import Path;Path({}).write_text('started')",
            serde_json::to_string(marker.to_str().unwrap()).unwrap()
        );
        let cmd = ["python3", "-c", code.as_str()];
        let result = if task == "extract" {
            engine.ingest_with_extractor(&cmd, &[]).map(|_| ())
        } else {
            engine.consolidate_with_provider(&cmd, &[]).map(|_| ())
        };
        results.push((
            task,
            matches!(result, Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))),
            marker.exists(),
        ));
    }
    drop(handle);
    assert!(
        results.iter().all(|(_, busy, created)| *busy && !*created),
        "provider newcomer performed work before admission: {results:?}"
    );
}
#[test]
fn admitted_extractor_remains_owned_through_external_payload_and_result() {
    let (dir, engine) = fixture();
    let reached = dir.path().join("reached");
    let release = dir.path().join("release");
    let code=format!("import sys,json,time\nfrom pathlib import Path\nh=json.loads(sys.stdin.readline())\nprint(json.dumps(dict(type='ready',protocol=h['protocol'],schema_version=1)),flush=True)\nr=json.loads(sys.stdin.readline())\nPath({}).write_text('payload-owned')\nwhile not Path({}).exists(): time.sleep(0.01)\nprint(json.dumps(dict(type='result',request_id=r['request_id'],entities=[],edges=[])),flush=True)\n",serde_json::to_string(reached.to_str().unwrap()).unwrap(),serde_json::to_string(release.to_str().unwrap()).unwrap());
    let provider = {
        let engine = Arc::clone(&engine);
        std::thread::spawn(move || {
            engine.ingest_with_extractor(
                &["python3", "-c", &code],
                &[ExtractDocument {
                    source_doc_id: "input".into(),
                    body: "caller-owned payload".into(),
                }],
            )
        })
    };
    let deadline = Instant::now() + Duration::from_secs(3);
    while !reached.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let reached_ok = reached.exists();
    let acquired = engine.begin_erasure_maintenance(0);
    let busy = matches!(acquired, Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy)));
    drop(acquired);
    std::fs::write(release, "release").unwrap();
    provider.join().unwrap().unwrap();
    assert!(reached_ok, "actual provider must receive payload");
    assert!(busy, "maintenance must drain the admitted provider exchange");
}

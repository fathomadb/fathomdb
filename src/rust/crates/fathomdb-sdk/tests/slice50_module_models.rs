//! Explicit module ownership controls. The ignored native case requires the
//! selected, guarded release-verification GPU venue and actual cached models.

#[test]
fn empty_release_is_repeatable_without_model_features() {
    fathomdb_sdk::release_models().unwrap();
    fathomdb_sdk::release_models().unwrap();
}

#[cfg(all(feature = "embed-cuda", feature = "rerank-cuda"))]
#[test]
#[ignore = "requires a guarded CUDA venue and actual pinned model files"]
fn native_release_reload_and_busy_preserve_engine_ownership() {
    use fathomdb_sdk::{
        Engine, ErrorKind, InitialState, OpenOptions, PreparedWrite, RerankOptions, RerankPassage,
        SourceId, TextSearchOptions,
    };
    use std::sync::mpsc;
    use std::time::Duration;
    assert_eq!(std::env::var("FATHOMDB_REQUIRE_LIVE").as_deref(), Ok("1"));
    let directory = tempfile::tempdir().unwrap();
    fathomdb_sdk::release_models().unwrap();
    let engine = Engine::open(
        directory.path().join("models.fdb"),
        OpenOptions { use_default_embedder: true, ..OpenOptions::default() },
    )
    .unwrap();
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "durable module release canary".into(),
            source_id: SourceId::new("release-canary").unwrap(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .unwrap();
    engine.drain(30_000).unwrap();
    let text = "the cat sat on the mat";
    let mean = engine.embed(text).unwrap();
    assert_eq!(mean.len(), 384);
    assert!(mean.iter().all(|value| value.is_finite()));
    let cls = fathomdb_sdk::embed_batch_cls(&[text]).unwrap();
    assert_eq!(cls.len(), 1);
    assert_eq!(cls[0].len(), 384);
    let passages = [RerankPassage { id: 1, body: text.into(), score: 0.5 }];
    let ce = fathomdb_sdk::rerank(text, &passages, 1, RerankOptions::default()).unwrap();
    assert!(ce[0].ce_score.is_some_and(f64::is_finite));
    fathomdb_sdk::release_models().unwrap();
    assert_eq!(engine.embed(text).unwrap(), mean);
    assert_eq!(fathomdb_sdk::embed_batch_cls(&[text]).unwrap(), cls);
    assert_eq!(fathomdb_sdk::rerank(text, &passages, 1, RerankOptions::default()).unwrap(), ce);
    let (started, reached) = mpsc::channel();
    let (finish, released) = mpsc::channel();
    let expected_ce = ce.clone();
    let worker = std::thread::spawn(move || {
        fathomdb_embedder::module_models::with_reranker(|_| {
            started.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(fathomdb_sdk::rerank(text, &passages, 1, RerankOptions::default()).unwrap())
        })
        .unwrap()
        .unwrap()
    });
    reached.recv_timeout(Duration::from_secs(10)).unwrap();
    let refusal = fathomdb_sdk::release_models();
    finish.send(()).unwrap();
    let actual = worker.join().unwrap();
    assert_eq!(refusal.unwrap_err().kind(), ErrorKind::Overloaded);
    assert_eq!(actual, expected_ce);
    fathomdb_sdk::release_models().unwrap();
    assert_eq!(fathomdb_sdk::embed_batch_cls(&[text]).unwrap(), cls);
    fathomdb_sdk::release_models().unwrap();
    let found = engine.search_text_only("canary", TextSearchOptions::default()).unwrap();
    assert_eq!(found.results.len(), 1);
    assert_eq!(found.results[0].body, "durable module release canary");
    engine.close().unwrap();
    let reopened =
        Engine::open(directory.path().join("models.fdb"), OpenOptions::default()).unwrap();
    assert_eq!(
        reopened.search_text_only("canary", TextSearchOptions::default()).unwrap().results.len(),
        1
    );
    reopened.close().unwrap();
    println!("RUST_MODULE_RELEASE_HEALTHY_BUSY_OK");
}

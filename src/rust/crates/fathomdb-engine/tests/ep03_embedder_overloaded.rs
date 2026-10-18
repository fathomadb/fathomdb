//! EP-03 `embedder_overloaded`: a search whose query embedding is refused by a
//! full embed queue, or expires in that queue, still runs on its other arms and
//! reports the skipped vector arm (queue-admission.md § 9 test 30).
//!
//! Saturation uses a real store and the real embed dispatcher: a caller embedder
//! that blocks in `embed` until the test opens its gate, with a one-worker pool
//! (waiting capacity 4).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArmDegradation, DegradationReason, EmbedderChoice, Engine, EngineConfig, InitialState,
    PreparedWrite, SearchResult, SoftFallbackBranch, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const QUERY: &str = "ep03needle";

#[derive(Debug)]
struct GatedEmbedder {
    open: Mutex<bool>,
    changed: Condvar,
    blocked: AtomicUsize,
}

impl GatedEmbedder {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            open: Mutex::new(true),
            changed: Condvar::new(),
            blocked: AtomicUsize::new(0),
        })
    }

    fn close(&self) {
        *self.open.lock().unwrap() = false;
    }

    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

impl Embedder for GatedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("ep03-gated", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let open = self.open.lock().unwrap();
        if !*open {
            self.blocked.fetch_add(1, Ordering::SeqCst);
            let _open = self
                .changed
                .wait_timeout_while(open, Duration::from_secs(10), |open| !*open)
                .unwrap();
        }
        Ok(vec![1.0; 8])
    }
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(2));
    }
}

fn open_engine(dir: &TempDir, embedder: Arc<GatedEmbedder>, timeout_ms: u64) -> Arc<Engine> {
    let path = dir.path().join(format!("ep03{SQLITE_SUFFIX}"));
    let engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder),
        EngineConfig {
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(timeout_ms),
            ..EngineConfig::default()
        },
    )
    .expect("open")
    .engine;
    engine
        .write(&[note("ep03-a", "ep03needle first note"), note("ep03-b", "ep03needle second note")])
        .expect("write fixture");
    engine.drain(30_000).expect("drain");
    Arc::new(engine)
}

fn note(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".to_owned(),
        body: body.to_owned(),
        source_id: SourceId::new("test:ep03").expect("source id"),
        logical_id: Some(logical_id.to_owned()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn overloaded() -> Vec<ArmDegradation> {
    vec![ArmDegradation::new(SoftFallbackBranch::Vector, DegradationReason::EmbedderOverloaded)]
}

fn assert_text_only_with_overload(result: &SearchResult) {
    assert_eq!(result.degradations, overloaded());
    assert!(!result.results.is_empty(), "the text arm still answers");
    assert!(
        result.results.iter().all(|hit| hit.branch == SoftFallbackBranch::Text),
        "no vector hit can appear without a query vector: {:?}",
        result.results
    );
}

#[test]
fn a_saturated_embed_queue_is_reported_as_embedder_overloaded() {
    let dir = TempDir::new().unwrap();
    let embedder = GatedEmbedder::new();
    let engine = open_engine(&dir, Arc::clone(&embedder), 30_000);
    engine.begin_d27_observation_for_test(Instant::now());
    embedder.close();

    let spawn_search = |engine: &Arc<Engine>| {
        let engine = Arc::clone(engine);
        thread::spawn(move || engine.search(QUERY))
    };
    let mut held = vec![spawn_search(&engine)];
    wait_until("the active embed", || embedder.blocked.load(Ordering::SeqCst) == 1);
    held.extend((0..4).map(|_| spawn_search(&engine)));
    wait_until("four queued embeds", || {
        let events = engine.d27_observation_for_test().unwrap().embed_dispatch_events;
        events.iter().filter(|event| event.queued && event.started_ns.is_none()).count() == 4
    });

    let result = engine.search(QUERY).expect("a saturated queue degrades, it does not fail");
    let events = engine.d27_observation_for_test().unwrap().embed_dispatch_events;
    let last = events.last().expect("the refused request is recorded");
    assert!(!last.queued, "the sixth request was refused at admission (Saturated)");

    embedder.open();
    for search in held {
        let healthy = search.join().unwrap().expect("held searches complete");
        assert!(healthy.degradations.is_empty(), "served embeds report nothing");
    }
    assert_text_only_with_overload(&result);
    Arc::try_unwrap(engine).ok().expect("sole owner").close().expect("close");
}

#[test]
fn a_queued_embed_request_past_its_deadline_is_reported_as_embedder_overloaded() {
    let dir = TempDir::new().unwrap();
    let embedder = GatedEmbedder::new();
    let engine = open_engine(&dir, Arc::clone(&embedder), 300);
    engine.begin_d27_observation_for_test(Instant::now());
    embedder.close();

    let holder = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.search(QUERY))
    };
    wait_until("the active embed", || embedder.blocked.load(Ordering::SeqCst) == 1);

    let result = engine.search(QUERY).expect("an expired queued embed degrades");
    let events = engine.d27_observation_for_test().unwrap().embed_dispatch_events;
    let last = events.last().expect("the expired request is recorded");
    assert!(last.queued && last.started_ns.is_none(), "it expired while queued (QueuedExpired)");

    embedder.open();
    let started_timeout = holder.join().unwrap().expect("the started request's search completes");
    assert!(
        started_timeout.degradations.is_empty(),
        "a started-then-timed-out embed is not embedder_overloaded"
    );
    assert_text_only_with_overload(&result);
    Arc::try_unwrap(engine).ok().expect("sole owner").close().expect("close");
}

fn digest(body: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A provenanced fixture (evidence search needs full source provenance) on an
/// engine whose embed deadline is short, so a queued request expires.
fn open_provenanced_engine(dir: &TempDir, embedder: Arc<GatedEmbedder>) -> Arc<Engine> {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, ProvenancedNodeV1, SourceLocator, SourceRevisionId,
        SourceVersionId, WriteProvenanceV1,
    };
    let path = dir.path().join(format!("ep03-frozen{SQLITE_SUFFIX}"));
    let engine = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder),
        EngineConfig {
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(300),
            ..EngineConfig::default()
        },
    )
    .expect("open")
    .engine;
    let source_body = "ep03 canonical source bytes";
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: source_body.into(),
                source_id: SourceId::new("ep03-source").unwrap(),
                logical_id: Some("ep03-source-logical".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("ep03-source-r1").unwrap(),
                    SourceVersionId::new("ep03-source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "claim".into(),
                body: "ep03needle claim".into(),
                source_id: SourceId::new("ep03-source").unwrap(),
                logical_id: Some("ep03-claim-logical".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("ep03-claim-r1").unwrap(),
                    SourceVersionId::new("ep03-source-v1").unwrap(),
                    SourceRevisionId::new("ep03-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(source_body)).unwrap(),
                ),
            }),
        ])
        .expect("write provenanced fixture");
    engine.drain(30_000).expect("drain");
    Arc::new(engine)
}

/// Occupy the only provider worker until the gate opens. The holder's own
/// call ends in a started timeout; every request submitted meanwhile queues
/// and expires (QueuedExpired).
fn hold_the_only_worker(
    engine: &Arc<Engine>,
    embedder: &GatedEmbedder,
) -> thread::JoinHandle<Result<Vec<f32>, fathomdb_engine::EngineError>> {
    embedder.close();
    let holder = {
        let engine = Arc::clone(engine);
        thread::spawn(move || engine.embed_text("hold the worker"))
    };
    wait_until("the active embed", || embedder.blocked.load(Ordering::SeqCst) == 1);
    holder
}

#[test]
fn a_frozen_search_reports_embedder_overloaded() {
    use fathomdb_engine::{EvidenceSearchRequestV1, ReadContextV1, ReadView, SearchFilter};
    let dir = TempDir::new().unwrap();
    let embedder = GatedEmbedder::new();
    let engine = open_provenanced_engine(&dir, Arc::clone(&embedder));
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let holder = hold_the_only_worker(&engine, &embedder);

    let plain = engine
        .search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10)
        .expect("frozen search degrades");
    let evidence = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: QUERY.into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            include_explanation: false,
            limit: 10,
        })
        .expect("evidence search degrades");

    embedder.open();
    assert!(holder.join().unwrap().is_err(), "the holder's started call timed out");
    assert_text_only_with_overload(&plain);
    assert_text_only_with_overload(&evidence.search_result);
    assert_eq!(evidence.evidence.len(), evidence.search_result.results.len());

    let healthy = engine
        .search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10)
        .expect("healthy frozen search");
    assert!(healthy.degradations.is_empty(), "a served frozen embed reports nothing");
    Arc::try_unwrap(engine).ok().expect("sole owner").close().expect("close");
}

#[test]
fn search_expand_reports_embedder_overloaded() {
    use fathomdb_engine::{ReadContextV1, ReadView, SearchFilter};
    let dir = TempDir::new().unwrap();
    let embedder = GatedEmbedder::new();
    let engine = open_provenanced_engine(&dir, Arc::clone(&embedder));
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let holder = hold_the_only_worker(&engine, &embedder);

    let expanded = engine.search_expand(QUERY, None, 1).expect("search_expand degrades");
    let expanded_frozen =
        engine.search_expand_frozen(QUERY, &frozen, 1, 10).expect("frozen expand degrades");

    embedder.open();
    assert!(holder.join().unwrap().is_err(), "the holder's started call timed out");
    for result in [&expanded, &expanded_frozen] {
        assert_eq!(result.degradations, overloaded());
        assert!(!result.search_hits.is_empty(), "the text arm still answers");
        assert!(result.search_hits.iter().all(|hit| hit.branch == SoftFallbackBranch::Text));
    }

    let healthy = engine.search_expand(QUERY, None, 1).expect("healthy search_expand");
    assert!(healthy.degradations.is_empty(), "a served embed reports nothing");
    Arc::try_unwrap(engine).ok().expect("sole owner").close().expect("close");
}

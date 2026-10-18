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

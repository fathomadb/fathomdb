//! Per-call missing-provider feedback through each real search route.
use fathomdb_engine::{
    ArmDegradation, ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine,
    EvidenceSearchRequestV1, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1,
    ReadView, SearchFilter, SearchHit, SoftFallbackBranch, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};

const QUERY: &str = "sliceseventyneedle";

fn seed(engine: &Engine) {
    let body = "original source bytes";
    let hash =
        Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>();
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: body.into(),
                source_id: SourceId::new("s70-source").unwrap(),
                logical_id: Some("s70-source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("s70-source-r1").unwrap(),
                    SourceVersionId::new("s70-v1").unwrap(),
                ),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "claim".into(),
                body: format!("{QUERY} claim"),
                source_id: SourceId::new("s70-source").unwrap(),
                logical_id: Some("s70-claim".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("s70-claim-r1").unwrap(),
                    SourceVersionId::new("s70-v1").unwrap(),
                    SourceRevisionId::new("s70-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(hash).unwrap(),
                ),
                properties: None,
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
}

fn absent_route(route: &str) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open_with_choice(dir.path().join("absence.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    seed(&engine);
    let text = engine.search_text_only(QUERY).unwrap();
    assert!(text.degradations.is_empty(), "explicit text-only requests no vector arm");
    assert_eq!(text.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["s70-claim"]);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let (hits, reasons): (Vec<SearchHit>, Vec<ArmDegradation>) = match route {
        "ordinary" => {
            let r = engine.search(QUERY).unwrap();
            (r.results, r.degradations)
        }
        "frozen" => {
            let r = engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10).unwrap();
            (r.results, r.degradations)
        }
        "evidence" => {
            let r = engine
                .search_with_evidence(&EvidenceSearchRequestV1::new(QUERY.into(), frozen.clone()))
                .unwrap();
            assert_eq!(r.evidence.len(), 1);
            (r.search_result.results, r.search_result.degradations)
        }
        "expand" => {
            let r = engine.search_expand(QUERY, None, 1).unwrap();
            (r.search_hits, r.degradations)
        }
        "expand_frozen" => {
            let r = engine.search_expand_frozen(QUERY, &frozen, 1, 10).unwrap();
            (r.search_hits, r.degradations)
        }
        _ => unreachable!(),
    };
    engine.close().unwrap();
    assert_eq!(
        reasons.iter().map(|e| (e.arm, e.reason.as_str())).collect::<Vec<_>>(),
        [(SoftFallbackBranch::Vector, "embedder_absent")],
        "{route}: per-call absence must survive this route"
    );
    assert_eq!(hits.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["s70-claim"]);
    assert!(hits.iter().all(|h| h.branch == SoftFallbackBranch::Text));
    // One lexical rank contributes the accepted text weight 3 / (30 + 1).
    assert_eq!(hits[0].score, 3.0 / 31.0);
}

macro_rules! absence_case {
    ($name:ident,$route:literal) => {
        #[test]
        fn $name() {
            absent_route($route);
        }
    };
}
absence_case!(ordinary_reports_per_call_absence, "ordinary");
absence_case!(frozen_reports_per_call_absence, "frozen");
absence_case!(evidence_reports_per_call_absence, "evidence");
absence_case!(expand_reports_per_call_absence, "expand");
absence_case!(frozen_expand_reports_per_call_absence, "expand_frozen");

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EngineConfig, EngineError, TestEmbedderChoice};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct QueryGate {
    armed: AtomicBool,
    query_calls: AtomicUsize,
    reached: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for QueryGate {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("s70-query-gate", "r1", 8)
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text == QUERY {
            self.query_calls.fetch_add(1, Ordering::SeqCst);
            if self.armed.swap(false, Ordering::SeqCst) {
                let _ = self.reached.send(());
                // A finite rescue bounds cleanup even if the test's release owner fails.
                let _ = self.release.lock().unwrap().recv_timeout(Duration::from_secs(5));
            }
        }
        Ok(vec![1.0; 8])
    }
}

struct ReleaseOnDrop(mpsc::Sender<()>);
impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn timeout_route(route: &str) {
    let dir = tempfile::tempdir().unwrap();
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(QueryGate {
        armed: AtomicBool::new(false),
        query_calls: AtomicUsize::new(0),
        reached: reached_tx,
        release: Mutex::new(release_rx),
    });
    let engine = Engine::open_with_test_choice_and_config_for_test(
        dir.path().join("timeout.sqlite"),
        TestEmbedderChoice::Caller(provider.clone()),
        EngineConfig {
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(300),
            ..EngineConfig::default()
        },
    )
    .unwrap()
    .engine;
    let release = ReleaseOnDrop(release_tx);
    seed(&engine);
    let text = engine.search_text_only(QUERY).unwrap();
    let calls_after_text = provider.query_calls.load(Ordering::SeqCst);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    engine.begin_d27_observation_for_test(Instant::now());
    provider.armed.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let (hits, reasons) = match route {
        "ordinary" => {
            let r = engine.search(QUERY).unwrap();
            (r.results, r.degradations)
        }
        "frozen" => {
            let r = engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10).unwrap();
            (r.results, r.degradations)
        }
        "evidence" => {
            let r = engine
                .search_with_evidence(&EvidenceSearchRequestV1::new(QUERY.into(), frozen.clone()))
                .unwrap();
            (r.search_result.results, r.search_result.degradations)
        }
        "expand" => {
            let r = engine.search_expand(QUERY, None, 1).unwrap();
            (r.search_hits, r.degradations)
        }
        "expand_frozen" => {
            let r = engine.search_expand_frozen(QUERY, &frozen, 1, 10).unwrap();
            (r.search_hits, r.degradations)
        }
        _ => unreachable!(),
    };
    let elapsed = started.elapsed();
    let reached = reached_rx.recv_timeout(Duration::from_millis(100));
    let query_events = engine.d27_observation_for_test().unwrap().embed_dispatch_events;
    // This second request must expire queued: the timed-out provider still owns its slot.
    let probe = engine.embed_text("s70 queued probe");
    let held_events = engine.d27_observation_for_test().unwrap().embed_dispatch_events;
    drop(release);
    let healthy = engine.search(QUERY).unwrap();
    engine.close().unwrap();
    assert!(reached.is_ok(), "the query itself must reach its provider");
    assert_eq!(query_events.len(), 1);
    assert!(query_events[0].started_ns.is_some() && query_events[0].terminal_ns.is_some());
    assert!(elapsed < Duration::from_secs(2), "the waiter must not await provider rescue");
    assert!(
        matches!(probe, Err(EngineError::Overloaded)),
        "held worker must retain its slot: {probe:?}"
    );
    assert_eq!(held_events.len(), 2);
    assert!(held_events[1].queued && held_events[1].started_ns.is_none());
    assert_eq!(calls_after_text, 0, "text-only must never invoke the provider");
    assert!(text.degradations.is_empty());
    assert!(healthy.degradations.is_empty(), "released worker remains reusable");
    assert_eq!(
        healthy.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
        ["s70-claim"]
    );
    assert_eq!(
        reasons.iter().map(|e| (e.arm, e.reason.as_str())).collect::<Vec<_>>(),
        [(SoftFallbackBranch::Vector, "embedder_timeout")],
        "{route}: started timeout is not queue overload"
    );
    assert_eq!(hits.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["s70-claim"]);
    assert!(hits.iter().all(|h| h.branch == SoftFallbackBranch::Text));
    assert_eq!(hits[0].score, 3.0 / 31.0);
}

macro_rules! timeout_case {
    ($name:ident,$route:literal) => {
        #[test]
        fn $name() {
            timeout_route($route);
        }
    };
}
timeout_case!(ordinary_reports_started_timeout, "ordinary");
timeout_case!(frozen_reports_started_timeout, "frozen");
timeout_case!(evidence_reports_started_timeout, "evidence");
timeout_case!(expand_reports_started_timeout, "expand");
timeout_case!(frozen_expand_reports_started_timeout, "expand_frozen");

#[derive(Debug)]
struct QueryFailure(&'static str);
impl Embedder for QueryFailure {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("s70-provider-error", "r1", 8)
    }
    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text != QUERY {
            return Ok(vec![1.0; 8]);
        }
        match self.0 {
            "timeout" => Err(EmbedderError::Timeout),
            "failed" => Err(EmbedderError::Failed { message: "provider returned failure".into() }),
            "invalid" => Ok(vec![f32::NAN; 8]),
            _ => unreachable!(),
        }
    }
}

#[test]
fn provider_errors_and_invalid_vectors_retain_unlabeled_sparse_fallback() {
    for word in ["timeout", "failed", "invalid"] {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open_with_test_choice_and_config_for_test(
            dir.path().join("provider-errors.sqlite"),
            TestEmbedderChoice::Caller(Arc::new(QueryFailure(word))),
            EngineConfig::default(),
        )
        .unwrap()
        .engine;
        seed(&engine);
        let frozen = engine
            .freeze_read_context(
                &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
            )
            .unwrap();
        let results = [
            engine.search(QUERY).unwrap(),
            engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10).unwrap(),
        ];
        engine.close().unwrap();
        for result in results {
            assert!(
                result.degradations.is_empty(),
                "provider-returned {word} is not a dispatcher deadline"
            );
            assert_eq!(
                result.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(),
                ["s70-claim"]
            );
        }
    }
}

//! Executor-only tests for the bounded engine-owned embed dispatcher.

#[path = "../src/embed_dispatch.rs"]
mod embed_dispatch;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use embed_dispatch::{DispatchError, EmbedDispatcher, EmbedOutput};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};

const DIMENSION: u32 = 2;

struct Invocation {
    release: mpsc::Sender<()>,
}

impl Invocation {
    fn release(self) {
        self.release.send(()).expect("release provider call");
    }
}

struct ActiveCall<'a>(&'a AtomicUsize);

impl Drop for ActiveCall<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

struct RendezvousEmbedder {
    started: mpsc::Sender<Invocation>,
    calls: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
}

impl RendezvousEmbedder {
    fn new() -> (Arc<Self>, mpsc::Receiver<Invocation>) {
        let (started, receiver) = mpsc::channel();
        (
            Arc::new(Self {
                started,
                calls: AtomicUsize::new(0),
                active: AtomicUsize::new(0),
                peak: AtomicUsize::new(0),
            }),
            receiver,
        )
    }

    fn invoke(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        let _guard = ActiveCall(&self.active);
        let (release, wait) = mpsc::channel();
        self.started.send(Invocation { release }).expect("report provider entry");
        wait.recv().expect("test must release provider call");
        if text == "panic" {
            panic!("controlled provider panic");
        }
        if text == "error" {
            return Err(EmbedderError::Failed { message: "controlled".to_owned() });
        }
        if text == "short" {
            return Ok(vec![1.0]);
        }
        if text == "nan" {
            return Ok(vec![f32::NAN, 0.0]);
        }
        Ok(vec![1.0, 0.0])
    }
}

impl Embedder for RendezvousEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("rendezvous", "rev-a", DIMENSION)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.invoke(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        let first = texts.first().copied().expect("nonempty test batch");
        let vector = self.invoke(first)?;
        Ok(vec![vector; texts.len()])
    }
}

fn entered(receiver: &mpsc::Receiver<Invocation>) -> Invocation {
    receiver.recv_timeout(Duration::from_secs(2)).expect("provider call must start")
}

fn join(dispatcher: &EmbedDispatcher) {
    assert!(
        dispatcher.join_until(Instant::now() + Duration::from_secs(2)),
        "released workers must join"
    );
}

#[test]
fn absent_provider_allocates_no_queue_or_worker() {
    let dispatcher = EmbedDispatcher::new(None, 4, Duration::from_secs(1)).expect("construct");
    let snapshot = dispatcher.snapshot();
    assert_eq!(snapshot.queue_capacity, 0);
    assert_eq!(snapshot.live_workers, 0);
    assert!(dispatcher.accounting().is_none());
    assert!(matches!(dispatcher.submit_text("x".to_owned()), Err(DispatchError::NotConfigured)));
    dispatcher.close();
    join(&dispatcher);
}

#[test]
fn exact_waiting_capacity_and_provider_overlap_for_one_two_and_four_workers() {
    for size in [1, 2, 4] {
        let (provider, entered_calls) = RendezvousEmbedder::new();
        let dispatcher = EmbedDispatcher::new(Some(provider.clone()), size, Duration::from_secs(5))
            .expect("construct");
        assert_eq!(dispatcher.snapshot().live_workers, size);
        assert_eq!(dispatcher.snapshot().queue_capacity, 4 * size);

        let mut running = Vec::new();
        for index in 0..size {
            running
                .push(dispatcher.submit_text(format!("running-{index}")).expect("admit running"));
        }
        let calls: Vec<_> = (0..size).map(|_| entered(&entered_calls)).collect();
        assert_eq!(provider.peak.load(Ordering::SeqCst), size);

        let mut queued = Vec::new();
        for index in 0..4 * size {
            queued.push(dispatcher.submit_text(format!("queued-{index}")).expect("admit queue"));
        }
        assert_eq!(dispatcher.snapshot().queued, 4 * size);
        assert!(matches!(
            dispatcher.submit_text("overflow".to_owned()),
            Err(DispatchError::Saturated)
        ));

        dispatcher.close();
        for reply in running.into_iter().chain(queued) {
            assert!(matches!(reply.wait(), Err(DispatchError::Closing)));
        }
        for call in calls {
            call.release();
        }
        join(&dispatcher);
        assert_eq!(provider.calls.load(Ordering::SeqCst), size);
        assert_eq!(dispatcher.snapshot().live_workers, 0);
    }
}

#[test]
fn queued_deadline_expires_without_invoking_provider() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher =
        EmbedDispatcher::new(Some(provider.clone()), 1, Duration::from_millis(100)).unwrap();
    let running = dispatcher.submit_text("running".to_owned()).unwrap();
    let call = entered(&entered_calls);
    let queued = dispatcher.submit_text("queued".to_owned()).unwrap();
    assert!(matches!(queued.wait(), Err(DispatchError::QueuedExpired)));
    assert!(matches!(running.wait(), Err(DispatchError::StartedTimeout)));
    call.release();
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn explicit_queued_cancellation_does_not_invoke_provider_or_renew_deadline() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher =
        EmbedDispatcher::new(Some(provider.clone()), 1, Duration::from_secs(2)).unwrap();
    let running = dispatcher.submit_text("running".to_owned()).unwrap();
    let call = entered(&entered_calls);
    let queued = dispatcher.submit_text("queued".to_owned()).unwrap();
    let deadline = queued.deadline();
    assert!(deadline > Instant::now());
    queued.cancel();
    assert_eq!(queued.deadline(), deadline, "polling/cancellation never resets the request clock");
    assert!(matches!(queued.wait(), Err(DispatchError::Cancelled)));
    call.release();
    assert!(matches!(running.wait(), Ok(EmbedOutput::One(_))));
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn started_timeout_keeps_slot_occupied_then_discards_late_result() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher =
        EmbedDispatcher::new(Some(provider.clone()), 1, Duration::from_millis(150)).unwrap();
    let first = dispatcher.submit_text("first".to_owned()).unwrap();
    let first_call = entered(&entered_calls);
    assert!(matches!(first.wait(), Err(DispatchError::StartedTimeout)));
    assert_eq!(dispatcher.snapshot().active, 1);
    assert_eq!(dispatcher.snapshot().live_workers, 1);

    let second = dispatcher.submit_text("second".to_owned()).unwrap();
    assert_eq!(dispatcher.snapshot().queued, 1);
    assert_eq!(provider.peak.load(Ordering::SeqCst), 1);
    first_call.release();
    entered(&entered_calls).release();
    assert!(matches!(second.wait(), Ok(EmbedOutput::One(_))));
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(dispatcher.snapshot().late_results, 1);
    assert_eq!(provider.peak.load(Ordering::SeqCst), 1);
}

#[test]
fn completion_after_deadline_is_late_even_before_first_wait() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher = EmbedDispatcher::new(Some(provider), 1, Duration::from_millis(100)).unwrap();
    let reply = dispatcher.submit_text("late".to_owned()).unwrap();
    let call = entered(&entered_calls);
    let (_never_send, elapsed) = mpsc::channel::<()>();
    assert!(
        elapsed
            .recv_timeout(
                reply.deadline().saturating_duration_since(Instant::now())
                    + Duration::from_millis(20)
            )
            .is_err(),
        "test must cross the original request deadline without polling the reply"
    );
    call.release();
    let started = Instant::now();
    while dispatcher.snapshot().active != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider completion must settle");
        std::thread::yield_now();
    }
    assert!(matches!(reply.wait(), Err(DispatchError::StartedTimeout)));
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(dispatcher.snapshot().late_results, 1);
}

#[test]
fn batch_uses_one_deadline_instead_of_row_count_times_timeout() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher =
        Arc::new(EmbedDispatcher::new(Some(provider), 1, Duration::from_millis(100)).unwrap());
    let batch = dispatcher.submit_batch(vec!["row".to_owned(); 8]).unwrap();
    let call = entered(&entered_calls);
    let (finished, received) = mpsc::channel();
    std::thread::spawn(move || finished.send(batch.wait()).expect("report batch result"));
    let outcome =
        received.recv_timeout(Duration::from_millis(500)).expect("one fixed batch deadline");
    assert!(matches!(outcome, Err(DispatchError::StartedTimeout)));
    call.release();
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(dispatcher.snapshot().late_results, 1);
}

#[test]
fn timely_error_and_panic_transport_leave_fixed_worker_reusable() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher = EmbedDispatcher::new(Some(provider), 1, Duration::from_secs(2)).unwrap();
    let error = dispatcher.submit_text("error".to_owned()).unwrap();
    entered(&entered_calls).release();
    assert!(matches!(error.wait(), Err(DispatchError::Provider(_))));

    let panic = dispatcher.submit_text("panic".to_owned()).unwrap();
    entered(&entered_calls).release();
    match panic.wait() {
        Err(DispatchError::Panic(payload)) => {
            assert!(
                payload.is::<&str>() || payload.is::<String>(),
                "panic payload must be preserved"
            );
        }
        other => panic!("expected transported provider panic, got {other:?}"),
    }

    for invalid in ["short", "nan"] {
        let malformed = dispatcher.submit_text(invalid.to_owned()).unwrap();
        entered(&entered_calls).release();
        assert!(matches!(malformed.wait(), Err(DispatchError::InvalidOutput)));
    }

    let healthy = dispatcher.submit_text("healthy".to_owned()).unwrap();
    entered(&entered_calls).release();
    assert!(matches!(healthy.wait(), Ok(EmbedOutput::One(_))));
    dispatcher.close();
    join(&dispatcher);
    assert_eq!(dispatcher.snapshot().live_workers, 0);
}

#[test]
fn late_panic_is_discarded_and_retained_worker_accounting_survives_front_end_drop() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher = EmbedDispatcher::new(Some(provider), 1, Duration::from_millis(100)).unwrap();
    let accounting = dispatcher.accounting().expect("provider accounting");
    let panic = dispatcher.submit_text("panic".to_owned()).unwrap();
    let call = entered(&entered_calls);
    assert!(matches!(panic.wait(), Err(DispatchError::StartedTimeout)));
    dispatcher.close();
    assert!(!dispatcher.join_until(Instant::now() + Duration::from_millis(20)));
    assert_eq!(accounting.snapshot().live_workers, 1);
    drop(dispatcher);
    assert_eq!(accounting.snapshot().live_workers, 1);
    call.release();
    let started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "retained worker must exit");
        std::thread::yield_now();
    }
    assert_eq!(accounting.snapshot().late_panics, 1);
}

#[test]
fn worker_drain_uses_one_absolute_budget_across_four_retained_slots() {
    let (provider, entered_calls) = RendezvousEmbedder::new();
    let dispatcher =
        Arc::new(EmbedDispatcher::new(Some(provider), 4, Duration::from_secs(2)).unwrap());
    let replies: Vec<_> =
        (0..4).map(|index| dispatcher.submit_text(format!("held-{index}")).unwrap()).collect();
    let calls: Vec<_> = (0..4).map(|_| entered(&entered_calls)).collect();
    dispatcher.close();
    for reply in replies {
        assert!(matches!(reply.wait(), Err(DispatchError::Closing)));
    }

    let (finished, received) = mpsc::channel();
    let draining = Arc::clone(&dispatcher);
    std::thread::spawn(move || {
        finished
            .send(draining.join_until(Instant::now() + Duration::from_millis(100)))
            .expect("report bounded drain");
    });
    assert_eq!(
        received.recv_timeout(Duration::from_millis(250)).expect("one shared drain budget"),
        false
    );
    assert_eq!(dispatcher.snapshot().live_workers, 4);
    for call in calls {
        call.release();
    }
    // The first absolute budget is spent. A second join must not wait under
    // a renewed deadline; success is valid only after the workers have exited.
    let started = Instant::now();
    while dispatcher.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "released workers must exit");
        std::thread::yield_now();
    }
    join(&dispatcher);
}

#[test]
fn independent_executors_keep_queues_and_workers_separate() {
    let (provider_one, calls_one) = RendezvousEmbedder::new();
    let (provider_two, calls_two) = RendezvousEmbedder::new();
    let one = EmbedDispatcher::new(Some(provider_one), 1, Duration::from_secs(5)).unwrap();
    let two = EmbedDispatcher::new(Some(provider_two), 2, Duration::from_secs(5)).unwrap();
    let one_active = one.submit_text("one".to_owned()).unwrap();
    let one_call = entered(&calls_one);
    let two_active = [
        two.submit_text("two-a".to_owned()).unwrap(),
        two.submit_text("two-b".to_owned()).unwrap(),
    ];
    let two_calls = [entered(&calls_two), entered(&calls_two)];
    assert_eq!(one.snapshot().queue_capacity, 4);
    assert_eq!(two.snapshot().queue_capacity, 8);
    assert_eq!(one.snapshot().active, 1);
    assert_eq!(two.snapshot().active, 2);
    one.close();
    assert!(matches!(one_active.wait(), Err(DispatchError::Closing)));
    assert_eq!(two.snapshot().active, 2, "closing one engine must not cancel another");
    two.close();
    for reply in two_active {
        assert!(matches!(reply.wait(), Err(DispatchError::Closing)));
    }
    one_call.release();
    for call in two_calls {
        call.release();
    }
    join(&one);
    join(&two);
}

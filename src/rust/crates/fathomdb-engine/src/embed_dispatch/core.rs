//! Fixed, engine-owned provider dispatch. This module owns no database state.

use std::any::Any;
use std::collections::VecDeque;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[cfg(feature = "test-hooks")]
use super::d27_observation;
#[cfg(feature = "test-hooks")]
use d27_observation::Collector;
use fathomdb_embedder_api::{Embedder, EmbedderError, Vector};

pub(crate) enum DispatchError {
    NotConfigured,
    Saturated,
    QueuedExpired,
    StartedTimeout,
    Provider(EmbedderError),
    InvalidOutput,
    Panic(Box<dyn Any + Send>),
    Closing,
    Cancelled,
}

impl std::fmt::Debug for DispatchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => formatter.write_str("NotConfigured"),
            Self::Saturated => formatter.write_str("Saturated"),
            Self::QueuedExpired => formatter.write_str("QueuedExpired"),
            Self::StartedTimeout => formatter.write_str("StartedTimeout"),
            Self::Provider(error) => formatter.debug_tuple("Provider").field(error).finish(),
            Self::InvalidOutput => formatter.write_str("InvalidOutput"),
            Self::Panic(_) => formatter.write_str("Panic"),
            Self::Closing => formatter.write_str("Closing"),
            Self::Cancelled => formatter.write_str("Cancelled"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum EmbedOutput {
    One(Vector),
    Batch(Vec<Vector>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct DispatchSnapshot {
    pub(crate) queue_capacity: usize,
    pub(crate) queued: usize,
    pub(crate) active: usize,
    pub(crate) live_workers: usize,
    pub(crate) late_results: usize,
    pub(crate) late_panics: usize,
}

#[derive(Clone)]
pub(crate) struct DispatchAccounting {
    shared: Arc<Shared>,
}

impl DispatchAccounting {
    pub(crate) fn snapshot(&self) -> DispatchSnapshot {
        self.shared.snapshot()
    }
}

enum RequestBody {
    One(String),
    Batch(Vec<String>),
}

struct Request {
    body: RequestBody,
    reply: Arc<ReplyState>,
}

enum ReplyPhase {
    Queued,
    Started,
    Finished,
}

struct ReplyValue {
    phase: ReplyPhase,
    outcome: Option<Result<EmbedOutput, DispatchError>>,
}

struct ReplyState {
    deadline: Instant,
    value: Mutex<ReplyValue>,
    ready: Condvar,
    #[cfg(feature = "test-hooks")]
    observation: Option<(Arc<Collector>, u64)>,
    #[cfg(test)]
    start_pause: Mutex<Option<StartPause>>,
}

#[cfg(test)]
struct StartPause {
    reached: mpsc::Sender<()>,
    resume: mpsc::Receiver<()>,
    completed: mpsc::Sender<()>,
}

impl ReplyState {
    fn new(
        deadline: Instant,
        #[cfg(feature = "test-hooks")] observation: Option<(Arc<Collector>, u64)>,
    ) -> Self {
        Self {
            deadline,
            value: Mutex::new(ReplyValue { phase: ReplyPhase::Queued, outcome: None }),
            ready: Condvar::new(),
            #[cfg(feature = "test-hooks")]
            observation,
            #[cfg(test)]
            start_pause: Mutex::new(None),
        }
    }

    #[cfg(feature = "test-hooks")]
    fn observe_start(&self) {
        if let Some((collector, id)) = &self.observation {
            collector.start(*id);
        }
    }

    #[cfg(feature = "test-hooks")]
    fn observe_queued(&self) {
        if let Some((collector, id)) = &self.observation {
            collector.queued(*id);
        }
    }

    #[cfg(feature = "test-hooks")]
    fn observe_terminal(&self) {
        if let Some((collector, id)) = &self.observation {
            collector.terminal(*id);
        }
    }

    fn resolve(&self, outcome: Result<EmbedOutput, DispatchError>) -> bool {
        let mut value = self.value.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if matches!(value.phase, ReplyPhase::Finished) {
            return false;
        }
        if Instant::now() >= self.deadline {
            let error = if matches!(value.phase, ReplyPhase::Queued) {
                DispatchError::QueuedExpired
            } else {
                DispatchError::StartedTimeout
            };
            value.phase = ReplyPhase::Finished;
            value.outcome = Some(Err(error));
            #[cfg(feature = "test-hooks")]
            self.observe_terminal();
            self.ready.notify_all();
            return false;
        }
        value.phase = ReplyPhase::Finished;
        value.outcome = Some(outcome);
        #[cfg(feature = "test-hooks")]
        self.observe_terminal();
        self.ready.notify_all();
        true
    }

    fn start(&self) -> bool {
        #[cfg(test)]
        let pause = self.start_pause.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        #[cfg(test)]
        if let Some(gate) = &pause {
            gate.reached.send(()).expect("report pre-lock start handoff");
            gate.resume.recv_timeout(Duration::from_secs(2)).expect("release start handoff");
        }
        let started = {
            let mut value = self.value.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let now = Instant::now();
            if !matches!(value.phase, ReplyPhase::Queued) {
                false
            } else if now >= self.deadline {
                value.phase = ReplyPhase::Finished;
                value.outcome = Some(Err(DispatchError::QueuedExpired));
                #[cfg(feature = "test-hooks")]
                self.observe_terminal();
                self.ready.notify_all();
                false
            } else {
                value.phase = ReplyPhase::Started;
                #[cfg(feature = "test-hooks")]
                self.observe_start();
                true
            }
        };
        #[cfg(test)]
        if let Some(gate) = pause {
            gate.completed.send(()).expect("report completed start handoff");
        }
        started
    }

    fn expired_or_finished(&self, now: Instant) -> bool {
        let mut value = self.value.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if matches!(value.phase, ReplyPhase::Finished) {
            return true;
        }
        if now >= self.deadline && matches!(value.phase, ReplyPhase::Queued) {
            value.phase = ReplyPhase::Finished;
            value.outcome = Some(Err(DispatchError::QueuedExpired));
            #[cfg(feature = "test-hooks")]
            self.observe_terminal();
            self.ready.notify_all();
            return true;
        }
        false
    }

    fn wait(&self) -> Result<EmbedOutput, DispatchError> {
        let mut value = self.value.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(outcome) = value.outcome.take() {
                return outcome;
            }
            let now = Instant::now();
            if now >= self.deadline && !matches!(value.phase, ReplyPhase::Finished) {
                let error = if matches!(value.phase, ReplyPhase::Queued) {
                    DispatchError::QueuedExpired
                } else {
                    DispatchError::StartedTimeout
                };
                value.phase = ReplyPhase::Finished;
                #[cfg(feature = "test-hooks")]
                self.observe_terminal();
                self.ready.notify_all();
                return Err(error);
            }
            let remaining = self.deadline.saturating_duration_since(now);
            value = self
                .ready
                .wait_timeout(value, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
    }
}

pub(crate) struct EmbedReply {
    reply: Arc<ReplyState>,
}

impl EmbedReply {
    pub(crate) fn deadline(&self) -> Instant {
        self.reply.deadline
    }

    pub(crate) fn wait(self) -> Result<EmbedOutput, DispatchError> {
        self.reply.wait()
    }

    pub(crate) fn cancel(&self) {
        self.reply.resolve(Err(DispatchError::Cancelled));
    }

    #[cfg(test)]
    pub(crate) fn pause_before_start_for_test(
        &self,
    ) -> (mpsc::Receiver<()>, mpsc::Sender<()>, mpsc::Receiver<()>) {
        let (reached, reached_reply) = mpsc::channel();
        let (resume, resume_reply) = mpsc::channel();
        let (completed, completed_reply) = mpsc::channel();
        let gate = StartPause { reached, resume: resume_reply, completed };
        *self.reply.start_pause.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) =
            Some(gate);
        (reached_reply, resume, completed_reply)
    }
}

impl Drop for EmbedReply {
    fn drop(&mut self) {
        self.cancel();
    }
}

struct QueueState {
    waiting: VecDeque<Request>,
    active: Vec<Arc<ReplyState>>,
    closing: bool,
    live_workers: usize,
    late_results: usize,
    late_panics: usize,
}

struct Shared {
    provider: Mutex<Option<Arc<dyn Embedder>>>,
    dimension: usize,
    queue_capacity: usize,
    timeout_ms: AtomicU64,
    state: Mutex<QueueState>,
    changed: Condvar,
    #[cfg(feature = "test-hooks")]
    observation: Mutex<Option<Arc<Collector>>>,
}

impl Shared {
    fn snapshot(&self) -> DispatchSnapshot {
        let state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        DispatchSnapshot {
            queue_capacity: self.queue_capacity,
            queued: state.waiting.len(),
            active: state.active.len(),
            live_workers: state.live_workers,
            late_results: state.late_results,
            late_panics: state.late_panics,
        }
    }
}

pub(crate) struct EmbedDispatcher {
    shared: Option<Arc<Shared>>,
    handles: Mutex<Vec<JoinHandle<()>>>,
    drain_deadline: Mutex<Option<Instant>>,
    #[cfg(test)]
    #[allow(dead_code)] // Standalone core tests include this module without engine lifecycle.
    drain_budget_ms: AtomicU64,
}

impl EmbedDispatcher {
    pub(crate) fn new(
        provider: Option<Arc<dyn Embedder>>,
        pool_size: usize,
        timeout: Duration,
    ) -> io::Result<Self> {
        let Some(provider) = provider else {
            return Ok(Self {
                shared: None,
                handles: Mutex::new(Vec::new()),
                drain_deadline: Mutex::new(None),
                #[cfg(test)]
                drain_budget_ms: AtomicU64::new(30_000),
            });
        };
        if !(1..=64).contains(&pool_size) || timeout.is_zero() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid embed pool settings"));
        }
        let queue_capacity = pool_size
            .checked_mul(4)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "embed queue overflow"))?;
        let dimension = provider.identity().dimension as usize;
        let shared = Arc::new(Shared {
            provider: Mutex::new(Some(provider)),
            dimension,
            queue_capacity,
            timeout_ms: AtomicU64::new(timeout.as_millis() as u64),
            state: Mutex::new(QueueState {
                waiting: VecDeque::with_capacity(queue_capacity),
                active: Vec::with_capacity(pool_size),
                closing: false,
                live_workers: 0,
                late_results: 0,
                late_panics: 0,
            }),
            changed: Condvar::new(),
            #[cfg(feature = "test-hooks")]
            observation: Mutex::new(None),
        });
        let mut handles = Vec::with_capacity(pool_size);
        for index in 0..pool_size {
            let worker_shared = Arc::clone(&shared);
            match thread::Builder::new()
                .name(format!("fathomdb-embed-{index}"))
                .spawn(move || worker_loop(worker_shared))
            {
                Ok(handle) => handles.push(handle),
                Err(error) => {
                    stop(&shared);
                    for handle in handles {
                        let _ = handle.join();
                    }
                    return Err(error);
                }
            }
        }
        let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while state.live_workers != pool_size {
            state = shared.changed.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        drop(state);
        Ok(Self {
            shared: Some(shared),
            handles: Mutex::new(handles),
            drain_deadline: Mutex::new(None),
            #[cfg(test)]
            drain_budget_ms: AtomicU64::new(30_000),
        })
    }

    pub(crate) fn submit_text(&self, text: String) -> Result<EmbedReply, DispatchError> {
        self.submit(RequestBody::One(text))
    }

    pub(crate) fn submit_batch(&self, texts: Vec<String>) -> Result<EmbedReply, DispatchError> {
        self.submit(RequestBody::Batch(texts))
    }

    fn submit(&self, body: RequestBody) -> Result<EmbedReply, DispatchError> {
        let Some(shared) = &self.shared else {
            return Err(DispatchError::NotConfigured);
        };
        let deadline =
            Instant::now() + Duration::from_millis(shared.timeout_ms.load(Ordering::Relaxed));
        let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        #[cfg(feature = "test-hooks")]
        let observation =
            shared.observation.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(
                |collector| {
                    (Arc::clone(collector), collector.admit(d27_observation::current_owner()))
                },
            );
        let reply = Arc::new(ReplyState::new(
            deadline,
            #[cfg(feature = "test-hooks")]
            observation,
        ));
        if state.closing {
            #[cfg(feature = "test-hooks")]
            reply.resolve(Err(DispatchError::Closing));
            return Err(DispatchError::Closing);
        }
        let now = Instant::now();
        state.waiting.retain(|request| !request.reply.expired_or_finished(now));
        if state.waiting.len() == shared.queue_capacity {
            #[cfg(feature = "test-hooks")]
            reply.resolve(Err(DispatchError::Saturated));
            return Err(DispatchError::Saturated);
        }
        #[cfg(feature = "test-hooks")]
        reply.observe_queued();
        state.waiting.push_back(Request { body, reply: Arc::clone(&reply) });
        shared.changed.notify_one();
        Ok(EmbedReply { reply })
    }

    #[cfg(feature = "test-hooks")]
    #[allow(dead_code)] // Standalone dispatcher tests include this module without Engine.
    pub(crate) fn begin_d27_observation(&self, origin: Instant) {
        if let Some(shared) = &self.shared {
            *shared.observation.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(Arc::new(Collector::new(origin)));
        }
    }

    #[cfg(feature = "test-hooks")]
    #[allow(dead_code)] // Standalone dispatcher tests include this module without Engine.
    pub(crate) fn d27_observation(
        &self,
        scheduler: usize,
        embedder: usize,
    ) -> Option<d27_observation::D27Observation> {
        self.shared
            .as_ref()?
            .observation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|collector| collector.snapshot(scheduler, embedder))
    }

    #[cfg(feature = "test-hooks")]
    #[allow(dead_code)] // Standalone dispatcher tests include this module without Engine.
    pub(crate) fn observe_projection_admission(&self, count: usize) {
        if let Some(shared) = &self.shared {
            if let Some(collector) =
                shared.observation.lock().unwrap_or_else(|e| e.into_inner()).as_ref()
            {
                collector.projection_admitted(count);
            }
        }
    }

    pub(crate) fn close(&self) {
        if let Some(shared) = &self.shared {
            stop(shared);
        }
    }

    #[cfg(test)]
    #[allow(dead_code)] // Standalone core tests include this module without engine lifecycle.
    pub(crate) fn set_drain_budget_ms_for_test(&self, budget_ms: u64) {
        self.drain_budget_ms.store(budget_ms, Ordering::Relaxed);
    }

    #[allow(dead_code)] // Standalone core tests include this module without engine lifecycle.
    pub(crate) fn join_after_quiescence(&self) -> bool {
        #[cfg(test)]
        let budget = Duration::from_millis(self.drain_budget_ms.load(Ordering::Relaxed));
        #[cfg(not(test))]
        let budget = Duration::from_secs(30);
        self.join_until(Instant::now() + budget)
    }

    #[allow(dead_code)] // Standalone core tests include this module without calling the engine test seam.
    pub(crate) fn set_timeout_ms_for_test(&self, timeout_ms: u64) {
        if let Some(shared) = &self.shared {
            shared.timeout_ms.store(timeout_ms, Ordering::Relaxed);
        }
    }

    pub(crate) fn join_until(&self, deadline: Instant) -> bool {
        let Some(shared) = &self.shared else {
            return true;
        };
        let deadline = {
            let mut first =
                self.drain_deadline.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            *first.get_or_insert(deadline)
        };
        let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        while state.live_workers != 0 {
            let now = Instant::now();
            if now >= deadline {
                return false;
            }
            state = shared
                .changed
                .wait_timeout(state, deadline.saturating_duration_since(now))
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        drop(state);
        let mut handles = self.handles.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        for handle in handles.drain(..) {
            let _ = handle.join();
        }
        shared.provider.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
        true
    }

    pub(crate) fn snapshot(&self) -> DispatchSnapshot {
        self.shared.as_ref().map_or_else(DispatchSnapshot::default, |shared| shared.snapshot())
    }

    pub(crate) fn accounting(&self) -> Option<DispatchAccounting> {
        self.shared.as_ref().map(|shared| DispatchAccounting { shared: Arc::clone(shared) })
    }
}

impl Drop for EmbedDispatcher {
    fn drop(&mut self) {
        self.close();
    }
}

fn stop(shared: &Shared) {
    let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.closing {
        return;
    }
    state.closing = true;
    for request in state.waiting.drain(..) {
        request.reply.resolve(Err(DispatchError::Closing));
    }
    for reply in &state.active {
        reply.resolve(Err(DispatchError::Closing));
    }
    shared.changed.notify_all();
}

fn valid(vector: &Vector, dimension: usize) -> bool {
    vector.len() == dimension && vector.iter().all(|value| value.is_finite())
}

fn invoke(shared: &Shared, body: RequestBody) -> Result<EmbedOutput, DispatchError> {
    let provider = shared
        .provider
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .as_ref()
        .cloned()
        .ok_or(DispatchError::Closing)?;
    let expected_rows = match &body {
        RequestBody::One(_) => None,
        RequestBody::Batch(texts) => Some(texts.len()),
    };
    let outcome = catch_unwind(AssertUnwindSafe(|| match body {
        RequestBody::One(text) => provider.embed(&text).map(EmbedOutput::One),
        RequestBody::Batch(texts) => {
            let inputs: Vec<&str> = texts.iter().map(String::as_str).collect();
            provider.embed_batch(&inputs).map(EmbedOutput::Batch)
        }
    }));
    match outcome {
        Err(payload) => Err(DispatchError::Panic(payload)),
        Ok(Err(error)) => Err(DispatchError::Provider(error)),
        Ok(Ok(EmbedOutput::One(vector))) if valid(&vector, shared.dimension) => {
            Ok(EmbedOutput::One(vector))
        }
        Ok(Ok(EmbedOutput::Batch(vectors)))
            if Some(vectors.len()) == expected_rows
                && !vectors.is_empty()
                && vectors.iter().all(|vector| valid(vector, shared.dimension)) =>
        {
            Ok(EmbedOutput::Batch(vectors))
        }
        Ok(Ok(_)) => Err(DispatchError::InvalidOutput),
    }
}

fn worker_loop(shared: Arc<Shared>) {
    {
        let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.live_workers += 1;
        shared.changed.notify_all();
    }
    loop {
        let request = {
            let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            loop {
                if state.closing {
                    break None;
                }
                if let Some(request) = state.waiting.pop_front() {
                    if request.reply.start() {
                        state.active.push(Arc::clone(&request.reply));
                        break Some(request);
                    }
                    continue;
                }
                state = shared.changed.wait(state).unwrap_or_else(|poisoned| poisoned.into_inner());
            }
        };
        let Some(request) = request else {
            break;
        };
        let result = invoke(&shared, request.body);
        let is_panic = matches!(result, Err(DispatchError::Panic(_)));
        let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.active.retain(|reply| !Arc::ptr_eq(reply, &request.reply));
        if !request.reply.resolve(result) {
            if is_panic {
                state.late_panics += 1;
            } else {
                state.late_results += 1;
            }
        }
        shared.changed.notify_all();
    }
    let mut state = shared.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    state.live_workers -= 1;
    shared.changed.notify_all();
}

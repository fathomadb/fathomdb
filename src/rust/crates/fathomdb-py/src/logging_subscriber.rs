//! Bounded Python logging adapter for native lifecycle diagnostics.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use fathomdb_engine::lifecycle::{
    Event, EventCategory, EventSource, Phase, ProfileRecord, SlowStatement, StressFailureContext,
    Subscriber, Subscription,
};
use fathomdb_engine::Engine as RustEngine;
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::{ClosingError, InvalidArgumentError, OverloadedError};

const CAPACITY: usize = 4096;

thread_local! {
    static IN_LOGGER: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn reject_reentry() -> PyResult<()> {
    IN_LOGGER.with(|flag| {
        if flag.get() {
            Err(InvalidArgumentError::new_err(
                "database work is not allowed from a FathomDB logging callback",
            ))
        } else {
            Ok(())
        }
    })
}

struct LoggerCallGuard;

impl LoggerCallGuard {
    fn enter() -> Self {
        IN_LOGGER.with(|flag| flag.set(true));
        Self
    }
}

impl Drop for LoggerCallGuard {
    fn drop(&mut self) {
        IN_LOGGER.with(|flag| flag.set(false));
    }
}

enum Record {
    Event(Event),
    Profile(ProfileRecord),
    Slow(SlowStatement),
    Stress(StressFailureContext),
    Dropped(u64),
}

enum Message {
    Record(Record),
    Stop,
}

struct QueueState {
    enabled: bool,
    receiver: Receiver<Message>,
    reported_drop_last: bool,
}

struct Queue {
    state: Mutex<QueueState>,
    sender: SyncSender<Message>,
    enabled: AtomicBool,
    dropped: AtomicU64,
    exited: AtomicBool,
}

impl Queue {
    fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        Self {
            state: Mutex::new(QueueState { enabled: true, receiver, reported_drop_last: false }),
            sender,
            enabled: AtomicBool::new(true),
            dropped: AtomicU64::new(0),
            exited: AtomicBool::new(false),
        }
    }

    fn add_dropped(&self, count: u64) {
        let _ = self.dropped.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            Some(current.saturating_add(count))
        });
    }

    fn push(&self, record: Record) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        // Consumer bookkeeping cannot turn spare queue capacity into event loss.
        match self.sender.try_send(Message::Record(record)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => self.add_dropped(1),
            Err(TrySendError::Disconnected(_)) => self.enabled.store(false, Ordering::Release),
        }
    }

    fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
        // Wake an idle receiver before waiting for its bookkeeping guard.
        // A full channel is already ready; Stop itself is never counted as loss.
        let _ = self.sender.try_send(Message::Stop);
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.enabled = false;
        while state.receiver.try_recv().is_ok() {}
        self.dropped.store(0, Ordering::Relaxed);
    }

    fn next(&self) -> Option<Record> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                self.enabled.store(false, Ordering::Release);
                return None;
            }
        };
        if !self.enabled.load(Ordering::Acquire) || !state.enabled {
            return None;
        }
        let dropped = self.dropped.swap(0, Ordering::Relaxed);
        if dropped > 0 && !state.reported_drop_last {
            state.reported_drop_last = true;
            return Some(Record::Dropped(dropped));
        }
        let message = match state.receiver.try_recv() {
            Ok(message) => Some(message),
            Err(TryRecvError::Empty) if dropped > 0 => {
                state.reported_drop_last = true;
                return Some(Record::Dropped(dropped));
            }
            Err(TryRecvError::Empty) => match state.receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(message) => Some(message),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => {
                    state.enabled = false;
                    self.enabled.store(false, Ordering::Release);
                    None
                }
            },
            Err(TryRecvError::Disconnected) => {
                state.enabled = false;
                self.enabled.store(false, Ordering::Release);
                None
            }
        };
        if !self.enabled.load(Ordering::Acquire) {
            return None;
        }
        match message {
            Some(Message::Record(record)) => {
                // Producers may add real overflow while this loss report is deferred.
                self.add_dropped(dropped);
                state.reported_drop_last = false;
                Some(record)
            }
            Some(Message::Stop) => {
                state.enabled = false;
                self.enabled.store(false, Ordering::Release);
                None
            }
            None => None,
        }
    }
}

struct PythonSubscriber {
    queue: Arc<Queue>,
}

impl Subscriber for PythonSubscriber {
    fn on_event(&self, event: &Event) {
        if self.queue.enabled.load(Ordering::Acquire) {
            self.queue.push(Record::Event(event.clone()));
        }
    }

    fn on_profile(&self, record: &ProfileRecord) {
        if self.queue.enabled.load(Ordering::Acquire) {
            self.queue.push(Record::Profile(*record));
        }
    }

    fn on_slow_statement(&self, signal: &SlowStatement) {
        if self.queue.enabled.load(Ordering::Acquire) {
            self.queue.push(Record::Slow(signal.clone()));
        }
    }

    fn on_stress_failure(&self, context: &StressFailureContext) {
        if self.queue.enabled.load(Ordering::Acquire) {
            self.queue.push(Record::Stress(context.clone()));
        }
    }
}

fn emit(py: Python<'_>, logger: &Bound<'_, PyAny>, record: Record) -> PyResult<()> {
    let payload = PyDict::new(py);
    let (level, message) = match record {
        Record::Event(event) => {
            let phase = match event.phase {
                Phase::Started => "started",
                Phase::Slow => "slow",
                Phase::Heartbeat => "heartbeat",
                Phase::Finished => "finished",
                Phase::Failed => "failed",
            };
            let source = match event.source {
                EventSource::Engine => "engine",
                EventSource::SqliteInternal => "sqlite_internal",
            };
            let category = match event.category {
                EventCategory::Writer => "writer",
                EventCategory::Search => "search",
                EventCategory::Admin => "admin",
                EventCategory::Error => "error",
                EventCategory::Corruption => "corruption",
                EventCategory::Recovery => "recovery",
                EventCategory::Io => "io",
            };
            payload.set_item("phase", phase)?;
            payload.set_item("source", source)?;
            payload.set_item("category", category)?;
            if let Some(code) = event.code {
                payload.set_item("code", code)?;
            }
            let level = if matches!(event.phase, Phase::Slow | Phase::Failed) { 30 } else { 20 };
            (level, "fathomdb.event")
        }
        Record::Profile(record) => {
            let profile = PyDict::new(py);
            profile.set_item("wall_clock_ms", record.wall_clock_ms)?;
            profile.set_item("step_count", record.step_count)?;
            profile.set_item("cache_delta", record.cache_delta)?;
            payload.set_item("profile_record", profile)?;
            (20, "fathomdb.profile")
        }
        Record::Slow(signal) => {
            let slow = PyDict::new(py);
            slow.set_item("statement", signal.statement)?;
            slow.set_item("wall_clock_ms", signal.wall_clock_ms)?;
            payload.set_item("slow_statement", slow)?;
            (30, "fathomdb.slow_statement")
        }
        Record::Stress(context) => {
            let stress = PyDict::new(py);
            stress.set_item("thread_group_id", context.thread_group_id)?;
            stress.set_item("op_kind", context.op_kind)?;
            stress.set_item("last_error_chain", context.last_error_chain)?;
            stress.set_item("projection_state", context.projection_state)?;
            payload.set_item("stress_failure", stress)?;
            (30, "fathomdb.stress_failure")
        }
        Record::Dropped(count) => {
            payload.set_item("dropped_records", count)?;
            (30, "fathomdb.dropped_records")
        }
    };
    let extra = PyDict::new(py);
    extra.set_item("fathomdb", payload)?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("extra", extra)?;
    let _guard = LoggerCallGuard::enter();
    logger.call_method("log", (level, message), Some(&kwargs))?;
    Ok(())
}

fn run_worker(
    queue: Arc<Queue>,
    weak_logger: Py<PyAny>,
    subscription: Arc<Mutex<Option<Subscription>>>,
) {
    loop {
        let record = queue.next();
        if !queue.enabled.load(Ordering::Acquire) {
            break;
        }
        let alive = Python::try_attach(|py| -> PyResult<bool> {
            let logger = weak_logger.bind(py).call0()?;
            if logger.is_none() {
                return Ok(false);
            }
            if let Some(record) = record {
                // A user handler may fail; database execution and later
                // diagnostics remain independent of that Python exception.
                let _ = emit(py, &logger, record);
            }
            Ok(true)
        });
        if !matches!(alive, Some(Ok(true))) {
            queue.disable();
            break;
        }
        if !queue.enabled.load(Ordering::Acquire) {
            break;
        }
    }
    if let Ok(mut handle) = subscription.lock() {
        handle.take();
    }
    queue.exited.store(true, Ordering::Release);
}

struct Attachment {
    queue: Arc<Queue>,
    subscription: Arc<Mutex<Option<Subscription>>>,
}

impl Attachment {
    fn detach(&self) {
        if let Ok(mut handle) = self.subscription.lock() {
            handle.take();
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PhaseState {
    Open,
    Closing,
    Closed,
}

struct SlotState {
    phase: PhaseState,
    active: Option<Attachment>,
    retiring: Option<Arc<Queue>>,
}

/// Owns the single active Python logger subscription for one Engine.
pub(crate) struct LoggingSlot {
    state: Mutex<SlotState>,
}

impl LoggingSlot {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(SlotState { phase: PhaseState::Open, active: None, retiring: None }),
        }
    }

    pub(crate) fn attach(
        &self,
        py: Python<'_>,
        engine: &Arc<RustEngine>,
        logger: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        reject_reentry()?;
        if !logger.getattr("log")?.is_callable() {
            return Err(pyo3::exceptions::PyTypeError::new_err("logger.log must be callable"));
        }
        let weakref = py.import("weakref")?.getattr("ref")?.call1((logger,))?.unbind();
        let mut slot =
            self.state.lock().map_err(|_| ClosingError::new_err("logger slot unavailable"))?;
        if slot.phase != PhaseState::Open {
            return Err(ClosingError::new_err("engine is closing"));
        }
        if slot.retiring.as_ref().is_some_and(|queue| !queue.exited.load(Ordering::Acquire)) {
            return Err(OverloadedError::new_err("previous logger callback is still running"));
        }
        let queue = Arc::new(Queue::new());
        let subscription = Arc::new(Mutex::new(Some(
            engine.subscribe(Arc::new(PythonSubscriber { queue: Arc::clone(&queue) })),
        )));
        let worker_queue = Arc::clone(&queue);
        let worker_subscription = Arc::clone(&subscription);
        thread::Builder::new()
            .name("fathomdb-python-logger".into())
            .spawn(move || run_worker(worker_queue, weakref, worker_subscription))
            .map_err(|error| OverloadedError::new_err(error.to_string()))?;
        let old = slot.active.take();
        if let Some(ref attachment) = old {
            attachment.queue.disable();
            slot.retiring = Some(Arc::clone(&attachment.queue));
        } else {
            slot.retiring = None;
        }
        slot.active = Some(Attachment { queue, subscription });
        drop(slot);
        if let Some(ref attachment) = old {
            attachment.detach();
        }
        drop(old);
        Ok(())
    }

    pub(crate) fn close_start(&self) {
        let old = if let Ok(mut slot) = self.state.lock() {
            slot.phase = PhaseState::Closing;
            let old = slot.active.take();
            if let Some(ref attachment) = old {
                attachment.queue.disable();
                slot.retiring = Some(Arc::clone(&attachment.queue));
            }
            old
        } else {
            None
        };
        if let Some(ref attachment) = old {
            attachment.detach();
        }
    }

    pub(crate) fn close_finish(&self) {
        if let Ok(mut slot) = self.state.lock() {
            slot.phase = PhaseState::Closed;
        }
    }
}

impl Drop for LoggingSlot {
    fn drop(&mut self) {
        self.close_start();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> Record {
        Record::Event(Event {
            phase: Phase::Started,
            source: EventSource::Engine,
            category: EventCategory::Writer,
            code: None,
        })
    }

    #[test]
    fn bookkeeping_contention_preserves_non_overloaded_event_pairs() {
        let queue = Arc::new(Queue::new());
        let state = queue.state.lock().unwrap();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let producer_queue = Arc::clone(&queue);
        let producer = thread::spawn(move || {
            producer_queue.push(event());
            producer_queue.push(Record::Event(Event {
                phase: Phase::Finished,
                source: EventSource::Engine,
                category: EventCategory::Writer,
                code: None,
            }));
            done_tx.send(()).unwrap();
        });
        let completed_while_locked = done_rx.recv_timeout(Duration::from_secs(1)).is_ok();
        drop(state);
        producer.join().unwrap();
        let describe = |record| match record {
            Some(Record::Event(Event { phase, .. })) => format!("{phase:?}"),
            Some(Record::Dropped(count)) => format!("Dropped({count})"),
            Some(_) => "other record".into(),
            None => "no record".into(),
        };
        let observed = [describe(queue.next()), describe(queue.next())];
        queue.disable();
        assert!(completed_while_locked, "producers must not wait for consumer bookkeeping");
        assert_eq!(
            observed,
            ["Started", "Finished"],
            "spare capacity must preserve exact FIFO pairing without loss"
        );
    }

    #[test]
    fn overload_is_bounded_reported_and_disabled_atomically() {
        let queue = Queue::new();
        for _ in 0..CAPACITY {
            queue.push(event());
        }
        queue.push(event());
        assert!(matches!(queue.next(), Some(Record::Dropped(1))));
        queue.push(event());
        assert!(matches!(queue.next(), Some(Record::Event(_))));
        assert!(matches!(queue.next(), Some(Record::Dropped(1))));
        for _ in 1..CAPACITY {
            assert!(matches!(queue.next(), Some(Record::Event(_))));
        }
        assert!(matches!(
            queue.state.lock().unwrap().receiver.try_recv(),
            Err(TryRecvError::Empty)
        ));
        queue.disable();
        queue.push(event());
        assert!(matches!(
            queue.state.lock().unwrap().receiver.try_recv(),
            Err(TryRecvError::Empty)
        ));
        assert!(queue.next().is_none());
    }

    #[test]
    fn overflow_loss_saturates_and_remains_fair_while_loss_is_deferred() {
        let queue = Queue::new();
        for _ in 0..CAPACITY {
            queue.push(event());
        }
        queue.dropped.store(u64::MAX - 1, Ordering::Relaxed);
        queue.push(event());
        queue.push(event());
        assert!(matches!(queue.next(), Some(Record::Dropped(u64::MAX))));
        queue.push(event());
        assert!(matches!(queue.next(), Some(Record::Event(_))));
        assert_eq!(queue.dropped.load(Ordering::Relaxed), 1);
        queue.push(event());
        queue.push(event());
        assert!(matches!(queue.next(), Some(Record::Dropped(2))));
        queue.disable();
        assert_eq!(queue.dropped.load(Ordering::Relaxed), 0);
        assert!(queue.next().is_none());
    }

    #[test]
    fn disabling_wakes_an_idle_consumer_before_its_poll_timeout() {
        let queue = Arc::new(Queue::new());
        let (started_tx, started_rx) = mpsc::channel();
        let (woke_tx, woke_rx) = mpsc::channel();
        let consumer_queue = Arc::clone(&queue);
        let consumer = thread::spawn(move || {
            started_tx.send(()).unwrap();
            woke_tx.send(consumer_queue.next().is_none()).unwrap();
        });
        let started = started_rx.recv_timeout(Duration::from_secs(1)).is_ok();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let waiting = loop {
            match queue.state.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break true,
                Err(_) => break false,
                Ok(state) => drop(state),
            }
            if std::time::Instant::now() >= deadline {
                break false;
            }
            thread::yield_now();
        };
        let (disabled_tx, disabled_rx) = mpsc::channel();
        let disabling_queue = Arc::clone(&queue);
        let disabler = thread::spawn(move || {
            disabling_queue.disable();
            disabled_tx.send(()).unwrap();
        });
        let woke = woke_rx.recv_timeout(Duration::from_millis(750)).ok();
        let disabled = disabled_rx.recv_timeout(Duration::from_millis(750)).is_ok();
        consumer.join().unwrap();
        disabler.join().unwrap();
        assert!(started && waiting, "idle consumer must own receiver bookkeeping");
        assert_eq!(woke, Some(true), "Stop must wake the receiver before its one-second poll");
        assert!(disabled, "disable must not wait out the receiver's poll timeout");
        assert!(!queue.enabled.load(Ordering::Acquire));
        assert_eq!(queue.dropped.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn poison_and_disconnect_end_delivery_without_an_enabled_dead_consumer() {
        let queue = Arc::new(Queue::new());
        let poisoning_queue = Arc::clone(&queue);
        let poisoned = thread::spawn(move || {
            let _state = poisoning_queue.state.lock().unwrap();
            panic!("poison receiver bookkeeping");
        });
        assert!(poisoned.join().is_err());
        assert!(queue.next().is_none());
        assert!(!queue.enabled.load(Ordering::Acquire));
        queue.disable();
        assert!(!queue.state.lock().err().unwrap().into_inner().enabled);
        for through_push in [false, true] {
            let queue = Queue::new();
            let (sender, receiver) = mpsc::sync_channel(CAPACITY);
            queue.state.lock().unwrap().receiver = receiver;
            drop(sender);
            if through_push {
                queue.push(event());
            }
            assert!(queue.next().is_none());
            assert!(!queue.enabled.load(Ordering::Acquire));
            assert_eq!(queue.dropped.load(Ordering::Relaxed), 0);
        }
    }

    #[test]
    fn stress_failure_reaches_python_logger_with_typed_context() -> PyResult<()> {
        Python::initialize();
        Python::attach(|py| {
            let fixture = PyModule::from_code(
                py,
                c"captured = []\nclass Logger:\n    def log(self, level, message, *, extra):\n        captured.append((level, message, extra))\nlogger = Logger()\n",
                c"fixture.py",
                c"fixture",
            )?;
            let logger = fixture.getattr("logger")?;
            let context = StressFailureContext {
                thread_group_id: 7,
                op_kind: "projection".into(),
                last_error_chain: vec!["sqlite busy".into()],
                projection_state: "stalled".into(),
            };
            emit(py, &logger, Record::Stress(context))?;
            let captured = fixture.getattr("captured")?;
            let entry = captured.get_item(0)?;
            assert_eq!(entry.get_item(0)?.extract::<u8>()?, 30);
            assert_eq!(entry.get_item(1)?.extract::<String>()?, "fathomdb.stress_failure");
            let payload = entry.get_item(2)?.get_item("fathomdb")?.get_item("stress_failure")?;
            assert_eq!(payload.get_item("thread_group_id")?.extract::<u64>()?, 7);
            assert_eq!(payload.get_item("op_kind")?.extract::<String>()?, "projection");
            assert_eq!(
                payload.get_item("last_error_chain")?.extract::<Vec<String>>()?,
                vec!["sqlite busy"]
            );
            assert_eq!(payload.get_item("projection_state")?.extract::<String>()?, "stalled");
            Ok(())
        })
    }
}

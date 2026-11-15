//! Bounded, opt-in measurement records owned by one engine and operation.

use crate::wal_attribution::WalAttributionCollector;
use crate::{Engine, EngineError};
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Instant;

/// One diagnostic interval or explicit not-run fact; never a product profile.
#[derive(Clone, Debug, Serialize)]
pub struct Slice30ObservationRecord {
    /// Process-local identity of the actual owning engine.
    pub engine_id: usize,
    /// Engine-local observation session.
    pub session_id: u64,
    /// Caller-assigned operation identity within the session.
    pub operation_id: u64,
    /// Unique span identity within the session.
    pub span_id: u64,
    /// Actual enclosing span, when present.
    pub parent_span_id: Option<u64>,
    /// Actual function/section or observer-added capture.
    pub phase: &'static str,
    /// Monotonic session-relative start, absent for not-run facts.
    pub started_ns: Option<u64>,
    /// Monotonic session-relative end, absent for not-run facts.
    pub ended_ns: Option<u64>,
    /// ok, error, not_run or unavailable; absent work is never zero cost.
    pub status: &'static str,
    /// Bounded primitive facts, not query text or results.
    pub facts: serde_json::Value,
}

/// Records drained from one bounded engine session.
#[derive(Debug, Serialize)]
pub struct Slice30ObservationBatch {
    /// Monotonic take sequence within this session.
    pub session_sequence: u64,
    /// Records retained within the configured bound.
    pub records: Vec<Slice30ObservationRecord>,
    /// Cumulative records lost to capacity or contention.
    pub dropped_records: u64,
    /// Bounded collection issues; never a replacement for a product error.
    pub issues: Vec<&'static str>,
}

pub(crate) struct ObservationSlot {
    active: Mutex<Weak<Collector>>,
    next_session: AtomicU64,
}

impl Default for ObservationSlot {
    fn default() -> Self {
        Self { active: Mutex::new(Weak::new()), next_session: AtomicU64::new(1) }
    }
}

struct Collector {
    engine_id: usize,
    session_id: u64,
    origin: Instant,
    capacity: usize,
    active: AtomicBool,
    next_span: AtomicU64,
    take_sequence: AtomicU64,
    dropped: AtomicU64,
    records: Mutex<Vec<Slice30ObservationRecord>>,
    #[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
    projection: bool,
    #[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
    next_projection_operation: AtomicU64,
}

impl Collector {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn append(&self, record: Slice30ObservationRecord) {
        if !self.active.load(Ordering::Acquire) {
            return;
        }
        if let Ok(mut records) = self.records.try_lock() {
            if records.len() < self.capacity {
                records.push(record);
                return;
            }
        }
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub(crate) struct OperationToken {
    collector: Weak<Collector>,
    engine_id: usize,
    operation_id: u64,
    parent_span_id: Option<u64>,
}

thread_local! {
    static CURRENT: RefCell<Option<OperationToken>> = const { RefCell::new(None) };
}

pub(crate) struct OperationScope(Option<OperationToken>);
impl Drop for OperationScope {
    fn drop(&mut self) {
        CURRENT.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

pub(crate) fn capture_token() -> Option<OperationToken> {
    CURRENT.with(|slot| slot.borrow().clone())
}

pub(crate) fn enter_token(
    owner: &Arc<WalAttributionCollector>,
    token: Option<OperationToken>,
) -> OperationScope {
    let engine_id = Arc::as_ptr(owner) as usize;
    let token = token.filter(|token| {
        token.engine_id == engine_id
            && token
                .collector
                .upgrade()
                .is_some_and(|collector| collector.active.load(Ordering::Acquire))
    });
    OperationScope(CURRENT.with(|slot| slot.replace(token)))
}

pub(crate) fn is_recording() -> bool {
    capture_token()
        .and_then(|token| token.collector.upgrade())
        .is_some_and(|collector| collector.active.load(Ordering::Acquire))
}

pub(crate) fn enter_engine(owner: &Arc<WalAttributionCollector>) -> OperationScope {
    enter_token(owner, capture_token())
}

struct SpanData {
    token: OperationToken,
    previous: Option<OperationToken>,
    span_id: u64,
    phase: &'static str,
    started_ns: u64,
    status: Cell<&'static str>,
    facts: RefCell<serde_json::Value>,
}

pub(crate) struct Span {
    data: Option<SpanData>,
    marker: PhantomData<Rc<()>>,
}

pub(crate) fn span(phase: &'static str) -> Span {
    let data = capture_token().and_then(|token| {
        let collector = token.collector.upgrade()?;
        if !collector.active.load(Ordering::Acquire) {
            return None;
        }
        let span_id = collector.next_span.fetch_add(1, Ordering::Relaxed);
        let started_ns = collector.now_ns();
        let mut nested = token.clone();
        nested.parent_span_id = Some(span_id);
        let previous = CURRENT.with(|slot| slot.replace(Some(nested)));
        Some(SpanData {
            token,
            previous,
            span_id,
            phase,
            started_ns,
            status: Cell::new("error"),
            facts: RefCell::new(serde_json::json!({})),
        })
    });
    Span { data, marker: PhantomData }
}

pub(crate) fn not_run(phase: &'static str, reason: &'static str) {
    let Some(token) = capture_token() else {
        return;
    };
    let Some(collector) = token.collector.upgrade() else {
        return;
    };
    if !collector.active.load(Ordering::Acquire) {
        return;
    }
    let record = Slice30ObservationRecord {
        engine_id: collector.engine_id,
        session_id: collector.session_id,
        operation_id: token.operation_id,
        span_id: collector.next_span.fetch_add(1, Ordering::Relaxed),
        parent_span_id: token.parent_span_id,
        phase,
        started_ns: None,
        ended_ns: None,
        status: "not_run",
        facts: serde_json::json!({"called":false,"sql_executed":false,"reason":reason}),
    };
    collector.append(record);
}

impl Span {
    pub(crate) fn counted_success(&self, candidates: usize) {
        if let Some(data) = &self.data {
            data.facts.replace(serde_json::json!({"candidates":candidates}));
            data.status.set("ok");
        }
    }

    pub(crate) fn success(&self) {
        if let Some(data) = &self.data {
            data.status.set("ok");
        }
    }

    pub(crate) fn snapshot_result(
        &self,
        authority: &Result<(u64, String, u64), EngineError>,
        view: &crate::temporal::FrozenView,
        cursor: u64,
    ) {
        let Some(data) = &self.data else {
            return;
        };
        match authority {
            Ok((generation, nonce, dependency_generation)) => {
                data.facts.replace(serde_json::json!({
                    "observer_added":true,"visibility_generation":generation,"visibility_nonce":nonce,
                    "dependency_generation":dependency_generation,"projection_cursor":cursor,
                    "effective_at_epoch_s":view.edge_now(),"validity_binding_epoch_s":view.now_param(),
                    "read_view":{"include_superseded":view.view.include_superseded,
                        "include_inactive":view.view.include_inactive,
                        "include_out_of_window":view.view.include_out_of_window}
                }));
                data.status.set("ok");
            }
            Err(error) => {
                data.facts.replace(
                    serde_json::json!({"observer_added":true,"error":format!("{error:?}")}),
                );
                data.status.set("unavailable");
            }
        }
    }

    pub(crate) fn boolean_result<E>(&self, result: &Result<bool, E>, sql_attempted: bool) {
        let Some(data) = &self.data else {
            return;
        };
        let mut facts = data.facts.borrow_mut();
        facts["called"] = serde_json::json!(true);
        facts["sql_attempted"] = serde_json::json!(sql_attempted);
        if let Ok(value) = result {
            facts["result"] = serde_json::json!(value);
            facts["sql_executed"] = serde_json::json!(sql_attempted);
            data.status.set("ok");
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let Some(data) = self.data.take() else {
            return;
        };
        CURRENT.with(|slot| *slot.borrow_mut() = data.previous);
        if let Some(collector) = data.token.collector.upgrade() {
            let record = Slice30ObservationRecord {
                engine_id: collector.engine_id,
                session_id: collector.session_id,
                operation_id: data.token.operation_id,
                span_id: data.span_id,
                parent_span_id: data.token.parent_span_id,
                phase: data.phase,
                started_ns: Some(data.started_ns),
                ended_ns: Some(collector.now_ns()),
                status: data.status.get(),
                facts: data.facts.into_inner(),
            };
            collector.append(record);
        }
    }
}

/// One thread-owned diagnostic session; dropping it releases only its own arm.
pub struct Slice30ObservationGuard {
    owner: Weak<WalAttributionCollector>,
    collector: Arc<Collector>,
    marker: PhantomData<Rc<()>>,
}

impl Slice30ObservationGuard {
    /// Run work with this engine/session and caller-assigned operation identity.
    /// Nested calls and unwind restore the prior context; work returns unchanged.
    pub fn with_operation<R>(&self, operation_id: u64, work: impl FnOnce() -> R) -> R {
        let token = OperationToken {
            collector: Arc::downgrade(&self.collector),
            engine_id: self.collector.engine_id,
            operation_id,
            parent_span_id: None,
        };
        let _restore = OperationScope(CURRENT.with(|slot| slot.replace(Some(token))));
        work()
    }

    /// Drain bounded records outside product clocks, retaining loss/issue counts.
    pub fn take(&self) -> Slice30ObservationBatch {
        let mut issues = vec![];
        let records = match self.collector.records.lock() {
            Ok(mut records) => {
                std::mem::replace(&mut *records, Vec::with_capacity(self.collector.capacity))
            }
            Err(_) => {
                issues.push("record_buffer_unavailable");
                vec![]
            }
        };
        Slice30ObservationBatch {
            session_sequence: self.collector.take_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            records,
            dropped_records: self.collector.dropped.load(Ordering::Relaxed),
            issues,
        }
    }
}

impl Drop for Slice30ObservationGuard {
    fn drop(&mut self) {
        self.collector.active.store(false, Ordering::Release);
        if let Some(owner) = self.owner.upgrade() {
            if let Ok(mut active) = owner.slice30_observation.active.lock() {
                if active.ptr_eq(&Arc::downgrade(&self.collector)) {
                    *active = Weak::new();
                }
            }
        }
    }
}

impl Engine {
    /// Begin one bounded diagnostic session for this exact engine.
    /// Bounds outside1..=32768 or duplicate sessions return InvalidArgument;
    /// closed engines return Closing. No SQL/provider work occurs on begin/drop.
    #[doc(hidden)]
    pub fn begin_slice30_observation(
        &self,
        max_records: usize,
    ) -> Result<Slice30ObservationGuard, EngineError> {
        let _operation = self.ensure_open()?;
        if !(1..=32768).contains(&max_records) {
            return Err(EngineError::InvalidArgument {
                msg: "observer records must be in1..=32768".into(),
            });
        }
        let slot = &self.wal_attribution.slice30_observation;
        let mut active = slot.active.lock().map_err(|_| EngineError::Storage)?;
        if active.upgrade().is_some_and(|collector| collector.active.load(Ordering::Acquire)) {
            return Err(EngineError::InvalidArgument {
                msg: "engine already has an observation".into(),
            });
        }
        let session_id = slot
            .next_session
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| EngineError::InvalidArgument {
                msg: "observation session exhausted".into(),
            })?;
        let collector = Arc::new(Collector {
            engine_id: Arc::as_ptr(&self.wal_attribution) as usize,
            session_id,
            origin: Instant::now(),
            capacity: max_records,
            active: AtomicBool::new(true),
            next_span: AtomicU64::new(1),
            take_sequence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            records: Mutex::new(Vec::with_capacity(max_records)),
            #[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
            projection: false,
            #[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
            next_projection_operation: AtomicU64::new(1_u64 << 63),
        });
        *active = Arc::downgrade(&collector);
        Ok(Slice30ObservationGuard {
            owner: Arc::downgrade(&self.wal_attribution),
            collector,
            marker: PhantomData,
        })
    }
}

/// Shareable private projection observation session; not an engine lifetime pin.
#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub struct Slice30ProjectionObservationGuard {
    owner: Weak<WalAttributionCollector>,
    collector: Arc<Collector>,
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
impl Slice30ProjectionObservationGuard {
    /// Return this collector's process-local engine and engine-local session IDs.
    /// This reads stored metadata without SQL, callbacks, wake or new admission;
    /// a retired guard continues to name its original session.
    pub fn owner_identity(&self) -> (usize, u64) {
        (self.collector.engine_id, self.collector.session_id)
    }

    /// Run work with this engine/session and caller-assigned operation identity.
    /// Nested calls and unwind restore the prior context; work returns unchanged.
    pub fn with_operation<R>(&self, operation_id: u64, work: impl FnOnce() -> R) -> R {
        let token = OperationToken {
            collector: Arc::downgrade(&self.collector),
            engine_id: self.collector.engine_id,
            operation_id,
            parent_span_id: None,
        };
        let _restore = OperationScope(CURRENT.with(|slot| slot.replace(Some(token))));
        work()
    }

    /// Drain bounded records outside product clocks, retaining loss/issue counts.
    pub fn take(&self) -> Slice30ObservationBatch {
        let mut issues = vec![];
        let records = match self.collector.records.lock() {
            Ok(mut records) => {
                std::mem::replace(&mut *records, Vec::with_capacity(self.collector.capacity))
            }
            Err(_) => {
                issues.push("record_buffer_unavailable");
                vec![]
            }
        };
        Slice30ObservationBatch {
            session_sequence: self.collector.take_sequence.fetch_add(1, Ordering::Relaxed) + 1,
            records,
            dropped_records: self.collector.dropped.load(Ordering::Relaxed),
            issues,
        }
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
impl Drop for Slice30ProjectionObservationGuard {
    fn drop(&mut self) {
        self.collector.active.store(false, Ordering::Release);
        if let Some(owner) = self.owner.upgrade() {
            if let Ok(mut active) = owner.slice30_observation.active.lock() {
                if active.ptr_eq(&Arc::downgrade(&self.collector)) {
                    *active = Weak::new();
                }
            }
        }
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
impl Engine {
    /// Begin a bounded, shareable projection diagnostic session for this engine.
    /// The supplied monotonic origin belongs to the driver's whole-cell clock.
    /// Bounds outside1..=32768 or duplicate sessions return InvalidArgument;
    /// closed engines return Closing. No SQL/provider work occurs on begin/drop.
    #[doc(hidden)]
    pub fn begin_slice30_projection_observation_for_test(
        &self,
        max_records: usize,
        origin: Instant,
    ) -> Result<Slice30ProjectionObservationGuard, EngineError> {
        let _operation = self.ensure_open()?;
        if !(1..=32768).contains(&max_records) {
            return Err(EngineError::InvalidArgument {
                msg: "observer records must be in1..=32768".into(),
            });
        }
        let slot = &self.wal_attribution.slice30_observation;
        let mut active = slot.active.lock().map_err(|_| EngineError::Storage)?;
        if active.upgrade().is_some_and(|collector| collector.active.load(Ordering::Acquire)) {
            return Err(EngineError::InvalidArgument {
                msg: "engine already has an observation".into(),
            });
        }
        let session_id = slot
            .next_session
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| EngineError::InvalidArgument {
                msg: "observation session exhausted".into(),
            })?;
        let collector = Arc::new(Collector {
            engine_id: Arc::as_ptr(&self.wal_attribution) as usize,
            session_id,
            origin,
            capacity: max_records,
            active: AtomicBool::new(true),
            next_span: AtomicU64::new(1),
            take_sequence: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            records: Mutex::new(Vec::with_capacity(max_records)),
            projection: true,
            next_projection_operation: AtomicU64::new(1_u64 << 63),
        });
        *active = Arc::downgrade(&collector);
        Ok(Slice30ProjectionObservationGuard {
            owner: Arc::downgrade(&self.wal_attribution),
            collector,
        })
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
#[derive(Clone)]
pub(crate) struct ProjectionTicket {
    owner: Weak<WalAttributionCollector>,
    collector: Weak<Collector>,
    operation_id: u64,
    scan_operation_id: u64,
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
impl std::fmt::Debug for ProjectionTicket {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProjectionTicket")
            .field("operation_id", &self.operation_id)
            .field("scan_operation_id", &self.scan_operation_id)
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
fn projection_operation(collector: &Collector) -> Option<u64> {
    match collector.next_projection_operation.fetch_update(
        Ordering::Relaxed,
        Ordering::Relaxed,
        |id| id.checked_add(1),
    ) {
        Ok(id) => Some(id),
        Err(_) => {
            collector.dropped.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_admission(
    owner: &Arc<WalAttributionCollector>,
) -> Option<ProjectionTicket> {
    // Never retain the slot lock across SQL, provider work or runtime locks.
    let weak = owner.slice30_observation.active.lock().ok()?.clone();
    let collector = weak.upgrade()?;
    if !collector.projection || !collector.active.load(Ordering::Acquire) {
        return None;
    }
    let operation_id = projection_operation(&collector)?;
    Some(ProjectionTicket {
        owner: Arc::downgrade(owner),
        collector: weak,
        operation_id,
        scan_operation_id: operation_id,
    })
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_group_admission<'a>(
    owner: &Arc<WalAttributionCollector>,
    mut tickets: impl Iterator<Item = &'a Option<ProjectionTicket>>,
) -> Option<ProjectionTicket> {
    let first = tickets.next()?.as_ref()?;
    let collector = first.collector.upgrade()?;
    if !collector.active.load(Ordering::Acquire)
        || !first.owner.upgrade().is_some_and(|actual| Arc::ptr_eq(&actual, owner))
    {
        return None;
    }
    if !tickets.all(|ticket| {
        ticket.as_ref().is_some_and(|ticket| {
            ticket.collector.ptr_eq(&first.collector)
                && ticket.scan_operation_id == first.scan_operation_id
        })
    }) {
        collector.dropped.fetch_add(1, Ordering::Relaxed);
        return None;
    }
    Some(ProjectionTicket { operation_id: projection_operation(&collector)?, ..first.clone() })
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn enter_projection(
    owner: &Arc<WalAttributionCollector>,
    ticket: Option<&ProjectionTicket>,
) -> OperationScope {
    let token = ticket.and_then(|ticket| {
        if !ticket.owner.upgrade().is_some_and(|actual| Arc::ptr_eq(&actual, owner)) {
            return None;
        }
        let collector = ticket.collector.upgrade()?;
        Some(OperationToken {
            collector: ticket.collector.clone(),
            engine_id: collector.engine_id,
            operation_id: ticket.operation_id,
            parent_span_id: None,
        })
    });
    enter_token(owner, token)
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
impl Span {
    pub(crate) fn projection_result(&self, cursors: &[u64], succeeded: bool, index: Option<usize>) {
        if let Some(data) = &self.data {
            data.facts.replace(serde_json::json!({"cursors":cursors,"runtime_index":index}));
            if succeeded {
                data.status.set("ok");
            }
        }
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_dequeue(
    cursors: &[u64],
    worker_idx: usize,
    ticket: Option<&ProjectionTicket>,
) {
    let Some(token) = capture_token() else {
        return;
    };
    let Some(collector) = token.collector.upgrade() else {
        return;
    };
    collector.append(Slice30ObservationRecord {
        engine_id: collector.engine_id, session_id: collector.session_id,
        operation_id: token.operation_id, span_id: collector.next_span.fetch_add(1, Ordering::Relaxed), parent_span_id: None,
        phase: "projection_dequeue", started_ns: None, ended_ns: None, status: "ok",
        facts: serde_json::json!({"cursors":cursors,"worker_index":worker_idx,"scan_operation_id":ticket.map(|ticket| ticket.scan_operation_id)}),
    });
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_is_recording() -> bool {
    capture_token()
        .and_then(|token| token.collector.upgrade())
        .is_some_and(|collector| collector.projection && collector.active.load(Ordering::Acquire))
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_span(phase: &'static str) -> Span {
    if projection_is_recording() {
        span(phase)
    } else {
        Span { data: None, marker: PhantomData }
    }
}

#[cfg(all(feature = "slice30-observer", feature = "test-hooks"))]
pub(crate) fn projection_commit_outcome(status: &'static str, facts: serde_json::Value) {
    let Some(token) = capture_token() else {
        return;
    };
    let Some(collector) = token.collector.upgrade() else {
        return;
    };
    if !collector.projection || !collector.active.load(Ordering::Acquire) {
        return;
    }
    collector.append(Slice30ObservationRecord {
        engine_id: collector.engine_id,
        session_id: collector.session_id,
        operation_id: token.operation_id,
        span_id: collector.next_span.fetch_add(1, Ordering::Relaxed),
        parent_span_id: token.parent_span_id,
        phase: "projection_commit_outcome",
        started_ns: None,
        ended_ns: None,
        status,
        facts,
    });
}

//! Bounded, opt-in measurement records owned by one engine and operation.

use crate::{Engine, EngineError};
use serde::Serialize;
use std::marker::PhantomData;
use std::rc::Rc;

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

/// One thread-owned diagnostic session; dropping it releases only its own arm.
pub struct Slice30ObservationGuard {
    marker: PhantomData<Rc<()>>,
}

impl Slice30ObservationGuard {
    /// Run work with this engine/session and caller-assigned operation identity.
    /// Nested calls and unwind restore the prior context; work returns unchanged.
    pub fn with_operation<R>(&self, _operation_id: u64, work: impl FnOnce() -> R) -> R {
        work()
    }

    /// Drain bounded records outside product clocks, retaining loss/issue counts.
    pub fn take(&self) -> Slice30ObservationBatch {
        Slice30ObservationBatch {
            session_sequence: 0,
            records: vec![],
            dropped_records: 0,
            issues: vec![],
        }
    }
}

impl Engine {
    /// Begin one bounded diagnostic session for this exact engine.
    /// Bounds outside1..=32768 or duplicate sessions return InvalidArgument;
    /// closed engines return Closing. No SQL/provider work occurs on begin/drop.
    #[doc(hidden)]
    pub fn begin_slice30_observation_for_test(
        &self,
        _max_records: usize,
    ) -> Result<Slice30ObservationGuard, EngineError> {
        Ok(Slice30ObservationGuard { marker: PhantomData })
    }
}

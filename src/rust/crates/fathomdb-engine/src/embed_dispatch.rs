use std::any::Any;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
pub(crate) struct DispatchAccounting;

impl DispatchAccounting {
    pub(crate) fn snapshot(&self) -> DispatchSnapshot {
        unimplemented!()
    }
}

pub(crate) struct EmbedReply;

impl EmbedReply {
    pub(crate) fn deadline(&self) -> Instant {
        unimplemented!()
    }

    pub(crate) fn wait(self) -> Result<EmbedOutput, DispatchError> {
        unimplemented!()
    }

    pub(crate) fn cancel(&self) {
        unimplemented!()
    }
}

pub(crate) struct EmbedDispatcher;

impl EmbedDispatcher {
    pub(crate) fn new(
        _provider: Option<Arc<dyn Embedder>>,
        _pool_size: usize,
        _timeout: Duration,
    ) -> io::Result<Self> {
        unimplemented!()
    }

    pub(crate) fn submit_text(&self, _text: String) -> Result<EmbedReply, DispatchError> {
        unimplemented!()
    }

    pub(crate) fn submit_batch(&self, _texts: Vec<String>) -> Result<EmbedReply, DispatchError> {
        unimplemented!()
    }

    pub(crate) fn close(&self) {
        unimplemented!()
    }

    pub(crate) fn join_until(&self, _deadline: Instant) -> bool {
        unimplemented!()
    }

    pub(crate) fn snapshot(&self) -> DispatchSnapshot {
        unimplemented!()
    }

    pub(crate) fn accounting(&self) -> Option<DispatchAccounting> {
        unimplemented!()
    }
}

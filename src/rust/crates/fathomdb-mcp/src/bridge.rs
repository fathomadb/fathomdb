//! Engine bridge (design §6): runs one SDK closure on a blocking thread,
//! raced against the run limit measured from the call's admission permit,
//! and maps its failures to tool errors.
//!
//! SDK calls have no interrupt, so a call past its run limit is abandoned,
//! not cancelled: its closure keeps the store guard until the blocking thread
//! returns, which is how admission (MR-033.2) and the idle clock (MR-005.4)
//! keep counting it.

use std::sync::Arc;
use std::time::Duration;

use fathomdb_sdk::Engine;

use crate::error::{BusyReason, ToolError};
use crate::hooks::Hooks;
use crate::registry::{Cancel, OpenInfo, StoreGuard};

/// What a call learned about the store it ran on.
#[derive(Clone)]
pub(crate) struct Answered {
    pub(crate) store: String,
    pub(crate) epoch: u64,
    pub(crate) info: Arc<OpenInfo>,
}

pub(crate) async fn call<T, F>(
    guard: StoreGuard,
    run_limit: Duration,
    cancel: &Cancel,
    hooks: &Hooks,
    f: F,
) -> Result<(Answered, T), ToolError>
where
    T: Send + 'static,
    F: FnOnce(&Engine) -> fathomdb_sdk::Result<T> + Send + 'static,
{
    let answered =
        Answered { store: guard.name.clone(), epoch: guard.epoch, info: guard.info.clone() };
    let abandoner = guard.abandoner();
    // A started call runs to completion; the run limit, from its permit,
    // only reports stuck work.
    let run_until = guard.admitted_at() + run_limit;
    let hooks = hooks.clone();
    let task = tokio::task::spawn_blocking(move || {
        let guard = guard;
        hooks.before_call(&guard.name);
        f(&guard.engine)
    });
    let mut cancel = cancel.clone();
    let joined = tokio::select! {
        biased;
        joined = tokio::time::timeout_at(run_until.into(), task) => joined,
        () = cancel.cancelled() => {
            // The read cannot be stopped: it keeps its permit until it
            // returns, and the client is answered at once (rmcp then
            // discards the answer of a cancelled request).
            if abandoner.abandon() {
                let load = abandoner.busy(BusyReason::Deadline);
                tracing::warn!(
                    store = %answered.store,
                    in_flight = load.in_flight,
                    queued = load.queued,
                    abandoned = load.abandoned,
                    "running call cancelled by the client; its engine work keeps its permit"
                );
            }
            return Err(ToolError::cancelled());
        }
    };
    match joined {
        Err(_) => {
            if abandoner.abandon() {
                let load = abandoner.busy(BusyReason::Deadline);
                tracing::warn!(
                    store = %answered.store,
                    in_flight = load.in_flight,
                    queued = load.queued,
                    abandoned = load.abandoned,
                    "call abandoned at its run limit; its engine work keeps its permit"
                );
            }
            Err(ToolError::deadline(Some(&answered.store)))
        }
        Ok(Err(join)) if join.is_panic() => {
            tracing::error!(store = %answered.store, "engine call panicked");
            Err(ToolError::panic())
        }
        Ok(Err(_)) => Err(ToolError::internal()),
        Ok(Ok(Err(error))) => {
            tracing::debug!(store = %answered.store, kind = error.kind().name(), "engine call failed");
            let mut mapped = ToolError::from_sdk(&answered.store, &error);
            if let Some(busy) = mapped.busy.as_mut() {
                **busy = abandoner.busy(busy.reason);
            }
            Err(mapped)
        }
        Ok(Ok(Ok(value))) => Ok((answered, value)),
    }
}

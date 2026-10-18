//! Engine bridge (design §6): runs one SDK closure on a blocking thread,
//! raced against the call deadline, and maps its failures to tool errors.
//!
//! SDK calls have no interrupt, so a timed-out call is abandoned, not
//! cancelled: its closure keeps the store guard until the blocking thread
//! returns, which is how admission (MR-033.2) and the idle clock (MR-005.4)
//! keep counting it.

use std::sync::Arc;
use std::time::Instant;

use fathomdb_sdk::Engine;

use crate::error::ToolError;
use crate::hooks::Hooks;
use crate::registry::{OpenInfo, StoreGuard};

/// What a call learned about the store it ran on.
#[derive(Clone)]
pub(crate) struct Answered {
    pub(crate) store: String,
    pub(crate) epoch: u64,
    pub(crate) info: Arc<OpenInfo>,
}

pub(crate) async fn call<T, F>(
    guard: StoreGuard,
    deadline: Instant,
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
    let hooks = hooks.clone();
    let task = tokio::task::spawn_blocking(move || {
        let guard = guard;
        hooks.before_call(&guard.name);
        f(&guard.engine)
    });
    match tokio::time::timeout_at(deadline.into(), task).await {
        Err(_) => {
            if abandoner.abandon() {
                let (in_flight, queued, abandoned) = abandoner.load();
                tracing::warn!(
                    store = %answered.store,
                    in_flight,
                    queued,
                    abandoned,
                    "call abandoned at its deadline; its engine work keeps its permit"
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
            Err(ToolError::from_sdk(&answered.store, &error))
        }
        Ok(Ok(Ok(value))) => Ok((answered, value)),
    }
}

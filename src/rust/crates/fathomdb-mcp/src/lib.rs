//! # fathomdb-mcp
//!
//! A read-only Model Context Protocol server over `fathomdb-sdk`. The tool
//! contract is `dev/interfaces/mcp.md`; the design is
//! `dev/design/mcp-adapter/design.md`.
//!
//! Standalone use is the `fathomdb-mcp` binary over stdio. Embedded use
//! (MR-004) is this library: build an [`Adapter`], register the host's open
//! engines under names, and serve [`Adapter::connection`] over any rmcp
//! transport.

mod bridge;
pub mod config;
pub mod edge;
mod error;
mod hooks;
mod names;
mod output;
mod registry;
#[cfg(feature = "test-seams")]
pub mod seams;
mod session;
mod shape;
mod tools;

use std::sync::Arc;
use std::time::Duration;

use config::Config;
use hooks::Hooks;
use registry::{RegisterError, Registry};
use tools::Shared;

/// Why an adapter could not be built or an engine registered.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    /// The configuration breaks a rule of `dev/interfaces/mcp.md` §2.
    #[error("invalid configuration: {0}")]
    InvalidConfig(#[from] config::ConfigError),
    /// [`Adapter::new`] was called outside a Tokio runtime.
    #[error("the adapter must be created inside a Tokio runtime")]
    NoRuntime,
    /// The name breaks the store-name grammar.
    #[error("{0:?} is not a valid store name")]
    BadStoreName(String),
    /// The name is already registered or configured.
    #[error("store {0:?} is already registered")]
    DuplicateStore(String),
}

/// The shared adapter core: configuration, store registry and idle clock.
/// Cloning is cheap and shares the same registry.
#[derive(Clone)]
pub struct Adapter {
    shared: Arc<Shared>,
}

impl Adapter {
    /// Build an adapter, start its idle clock and open `eager` stores.
    ///
    /// # Errors
    /// [`AdapterError::InvalidConfig`] when `config` breaks a rule of the
    /// interface (it is re-validated here, since its fields are public), and
    /// [`AdapterError::NoRuntime`] outside a Tokio runtime.
    pub fn new(config: Config) -> Result<Adapter, AdapterError> {
        config.validate()?;
        let runtime = tokio::runtime::Handle::try_current().map_err(|_| AdapterError::NoRuntime)?;
        let config = Arc::new(config);
        let hooks = Hooks::default();
        let registry = Registry::new(config.clone(), hooks.clone());
        if config.idle_timeout_secs > 0 {
            let weak = Arc::downgrade(&registry);
            runtime.spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_secs(1));
                tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tick.tick().await;
                    let Some(registry) = weak.upgrade() else { break };
                    registry.expire_idle();
                }
            });
        }
        let _enter = runtime.enter();
        registry.open_eager();
        Ok(Adapter { shared: Arc::new(Shared { config, registry, hooks }) })
    }

    /// Register a host-owned open engine under `name` (MR-004). The adapter
    /// serves it but never opens, drains or closes it, and it does not count
    /// toward the open-store limit. Its epoch is fixed at 1. When the config
    /// sets `telemetry_dir`, the apply-on-open list enables a sink
    /// `<dir>/<name>.<pid>.1.jsonl` on the engine, replacing any sink the host
    /// had enabled.
    ///
    /// # Errors
    /// [`AdapterError::BadStoreName`] for a name outside the grammar and
    /// [`AdapterError::DuplicateStore`] for a name already in use.
    pub fn register_engine(
        &self,
        name: &str,
        engine: Arc<fathomdb_sdk::Engine>,
    ) -> Result<(), AdapterError> {
        self.shared.registry.register_hosted(name, engine).map_err(|error| match error {
            RegisterError::BadName => AdapterError::BadStoreName(name.to_string()),
            RegisterError::Duplicate => AdapterError::DuplicateStore(name.to_string()),
        })
    }

    /// A server handler for one MCP connection, with its own session.
    #[must_use]
    pub fn connection(&self) -> edge::FathomServer {
        edge::FathomServer::new(self.shared.clone())
    }

    /// Stop admitting calls, then drain and close every store the adapter
    /// opened. Host-registered engines are left open. Waits at most
    /// `call_deadline_ms` + 5 s: a store whose started or abandoned engine
    /// call is still running after that is not waited for.
    pub async fn shutdown(&self) {
        self.shared.registry.shutdown().await;
    }

    /// The test seams of this adapter (feature `test-seams`).
    #[cfg(feature = "test-seams")]
    #[must_use]
    pub fn seams(&self) -> &seams::Seams {
        &self.shared.hooks.seams
    }
}

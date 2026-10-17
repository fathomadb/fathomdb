//! # fathomdb-mcp
//!
//! A read-only Model Context Protocol server over `fathomdb-sdk`. The tool
//! contract is `dev/interfaces/mcp.md`; the design is
//! `dev/design/mcp-adapter/design.md`.

pub mod config;
pub mod edge;
pub mod names;
#[cfg(feature = "test-seams")]
pub mod seams;

use config::Config;

/// The shared adapter core.
#[derive(Clone)]
pub struct Adapter;

/// Why an adapter could not be built.
#[derive(Debug)]
pub struct AdapterError;

impl Adapter {
    /// Build an adapter.
    pub fn new(_config: Config) -> Result<Adapter, AdapterError> {
        todo!()
    }

    /// A server handler for one MCP connection.
    pub fn connection(&self) -> edge::FathomServer {
        todo!()
    }

    /// Drain and close every open store.
    pub async fn shutdown(&self) {
        todo!()
    }

    /// Test seams.
    #[cfg(feature = "test-seams")]
    pub fn seams(&self) -> &seams::Seams {
        todo!()
    }
}

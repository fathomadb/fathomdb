//! Protocol edge: the only module (with `main.rs`) that names rmcp types.

/// One MCP connection's server handler.
#[derive(Clone)]
pub struct FathomServer;

impl rmcp::ServerHandler for FathomServer {}

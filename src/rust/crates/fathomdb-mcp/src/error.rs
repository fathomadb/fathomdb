//! Tool errors (`dev/interfaces/mcp.md` §6). Messages are the adapter's own
//! templates; SDK message text is never copied into them because it may
//! quote record content (MR-012.2).

use fathomdb_sdk::{Error as SdkError, ErrorKind, GraphExpansionErrorReasonV1};
use schemars::JsonSchema;
use serde::Serialize;

/// The closed error-class set of interface §6.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    InvalidRequest,
    NotFound,
    NotReady,
    Unavailable,
    Timeout,
    Busy,
    Internal,
}

/// A typed tool error: `{class, code, message, remedy, candidates?, detail?}`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, JsonSchema)]
pub struct ToolError {
    pub class: ErrorClass,
    /// Stable code within the class, e.g. `unknown_store`.
    pub code: &'static str,
    /// One-line adapter template; never contains record content.
    pub message: String,
    /// One-line corrective action.
    pub remedy: &'static str,
    /// Nearest addressable store names; present only for `unknown_store`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidates: Option<Vec<String>>,
    /// The SDK error class name when the error came from the SDK.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<&'static str>,
    /// Why the call was refused and the store's load; present only for
    /// `busy/admission`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub busy: Option<Box<Busy>>,
}

/// Why an admission was refused (`busy.reason`).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BusyReason {
    /// The waiting line was full on arrival (`queue_limit`).
    QueueFull,
    /// No permit came in time, or too little deadline was left to start.
    Deadline,
    /// The engine refused the work as overloaded (SDK `Overloaded`).
    EngineOverloaded,
}

impl BusyReason {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::QueueFull => "queue_full",
            Self::Deadline => "deadline",
            Self::EngineOverloaded => "engine_overloaded",
        }
    }
}

/// The `busy` sub-object of a `busy/admission` error. It carries no advised
/// wait: agents re-issue a call in a later turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Busy {
    pub reason: BusyReason,
    /// Permits held: running calls, including abandoned ones.
    pub in_flight: usize,
    /// Calls waiting for a permit.
    pub queued: usize,
    /// Calls past their deadline whose engine work still holds a permit.
    pub abandoned: usize,
    /// Every permit is held by abandoned work.
    pub stuck: bool,
}

impl ToolError {
    fn new(class: ErrorClass, code: &'static str, message: String, remedy: &'static str) -> Self {
        Self { class, code, message, remedy, candidates: None, detail: None, busy: None }
    }

    fn with_detail(mut self, kind: ErrorKind) -> Self {
        self.detail = Some(kind.name());
        self
    }

    pub(crate) fn bad_argument(message: impl Into<String>) -> Self {
        Self::new(
            ErrorClass::InvalidRequest,
            "bad_argument",
            message.into(),
            "fix the argument and retry; see the tool's input schema",
        )
    }

    pub(crate) fn no_store() -> Self {
        Self::bad_argument(
            "no store named: pass `store`, call use_store, or configure default_store",
        )
    }

    pub(crate) fn bad_store_name(store: &str) -> Self {
        Self::new(
            ErrorClass::InvalidRequest,
            "bad_store_name",
            format!(
                "{} is not a store name; names match ^[A-Za-z0-9][A-Za-z0-9._-]{{0,63}}$ and are never paths",
                quoted_name(store)
            ),
            "call list_stores and use one of the listed names",
        )
    }

    pub(crate) fn unknown_store(store: &str, candidates: Vec<String>) -> Self {
        let mut error = Self::new(
            ErrorClass::InvalidRequest,
            "unknown_store",
            format!("store {store:?} is not addressable"),
            "call list_stores and use one of the listed names",
        );
        error.candidates = Some(candidates);
        error
    }

    pub(crate) fn unsupported_ref() -> Self {
        Self::new(
            ErrorClass::InvalidRequest,
            "unsupported_ref",
            "only l: (logical id) references are accepted in this release".to_string(),
            "pass the l: reference of a search hit or fetch result",
        )
    }

    pub(crate) fn malformed_ref() -> Self {
        Self::bad_argument("reference is malformed; expected l:<logical id>")
    }

    pub(crate) fn stale_handle(what: &str) -> Self {
        Self::new(
            ErrorClass::InvalidRequest,
            "stale_handle",
            format!("this {what} is unknown, expired, from another store, or from an earlier open"),
            "re-run the search",
        )
    }

    pub(crate) fn wrong_store(store: &str) -> Self {
        Self::new(
            ErrorClass::InvalidRequest,
            "wrong_store",
            format!("the response was issued by another store, not {store:?}"),
            "send feedback to the store that answered the search",
        )
    }

    pub(crate) fn not_found() -> Self {
        Self::new(
            ErrorClass::NotFound,
            "not_found",
            "no record is visible for this reference under the requested view".to_string(),
            "check the reference, as_of, include_superseded and include_inactive",
        )
    }

    pub(crate) fn not_ready(store: &str, phase: &'static str) -> Self {
        Self::new(
            ErrorClass::NotReady,
            phase,
            format!("store {store:?} is not ready: {phase}"),
            "retry shortly; the call waited until its deadline",
        )
    }

    pub(crate) fn embedder_not_ready() -> Self {
        Self::new(
            ErrorClass::NotReady,
            "embedder_not_ready",
            "the requested search needs an embedder this store does not have".to_string(),
            "use mode \"text\", or have the operator enable the default embedder",
        )
    }

    pub(crate) fn vector_disabled() -> Self {
        Self::new(
            ErrorClass::NotReady,
            "vector_disabled",
            "the dense side of this store is disabled: its embedder failed the open-time \
             vector-equivalence check"
                .to_string(),
            "use mode \"text\", or ask the operator to check the store's embedder",
        )
    }

    pub(crate) fn missing_file(store: &str) -> Self {
        Self::new(
            ErrorClass::Unavailable,
            "missing_file",
            format!(
                "store {store:?}: its database file does not exist and creation is not allowed"
            ),
            "ask the operator to create the file or set create: true for this store",
        )
    }

    pub(crate) fn deadline(store: Option<&str>) -> Self {
        let message = match store {
            Some(store) => {
                format!("the call to store {store:?} did not finish within its deadline")
            }
            None => "the call did not finish within its deadline".to_string(),
        };
        Self::new(
            ErrorClass::Timeout,
            "deadline",
            message,
            "narrow the request (lower limit, max_depth or refs) or retry later",
        )
    }

    /// Another store is open with calls in flight, or mid-open, so `store`
    /// cannot open (one open store at a time, MR-005.7).
    pub(crate) fn store_switch(store: &str) -> Self {
        Self::new(
            ErrorClass::Busy,
            "store_switch",
            format!("store {store:?} cannot open while another store is in use"),
            "use the open store, or retry in a later turn",
        )
    }

    /// The client cancelled the call. rmcp discards the response of a
    /// cancelled request, so no client sees this result.
    pub(crate) fn cancelled() -> Self {
        Self::new(
            ErrorClass::Timeout,
            "cancelled",
            "the client cancelled this call".to_string(),
            "none; the call was cancelled by the client",
        )
    }

    pub(crate) fn admission(store: &str, busy: Busy) -> Self {
        let message = match busy.reason {
            BusyReason::QueueFull => {
                format!("store {store:?} has as many calls waiting as its queue_limit allows")
            }
            BusyReason::Deadline => {
                format!("store {store:?} could not start this call before its deadline")
            }
            BusyReason::EngineOverloaded => {
                format!("store {store:?}: the engine is overloaded")
            }
        };
        let mut error = Self::new(
            ErrorClass::Busy,
            "admission",
            message,
            "retry in a later turn, or issue fewer calls at once",
        );
        error.busy = Some(Box::new(busy));
        error
    }

    pub(crate) fn internal() -> Self {
        Self::new(
            ErrorClass::Internal,
            "internal",
            "internal error".to_string(),
            "retry; if it persists, report it with the server's stderr log",
        )
    }

    pub(crate) fn panic() -> Self {
        Self::new(
            ErrorClass::Internal,
            "panic",
            "the engine call panicked".to_string(),
            "retry; if it persists, report it with the server's stderr log",
        )
    }

    /// Map an SDK failure from an open (`Engine::open` and the apply-on-open
    /// list) to the `unavailable` class (MR-003.3).
    pub(crate) fn from_open(store: &str, error: &SdkError) -> Self {
        let kind = error.kind();
        let (code, remedy) = match kind {
            ErrorKind::DatabaseLocked => (
                "database_locked",
                "another process holds this database; close it there or retry later",
            ),
            ErrorKind::Corruption => {
                ("corrupt", "the database file is damaged; ask the operator to run fathomdb doctor")
            }
            ErrorKind::Migration | ErrorKind::IncompatibleSchemaVersion => (
                "migration_refused",
                "the database schema cannot be opened by this build; ask the operator",
            ),
            _ => ("open_failed", "ask the operator to check the server's stderr log"),
        };
        Self::new(
            ErrorClass::Unavailable,
            code,
            format!("store {store:?} could not be opened: {code}"),
            remedy,
        )
        .with_detail(kind)
    }

    /// Map an SDK failure from a read.
    pub(crate) fn from_sdk(store: &str, error: &SdkError) -> Self {
        let kind = error.kind();
        let mapped = match kind {
            ErrorKind::InvalidArgument
            | ErrorKind::WriteValidation
            | ErrorKind::SchemaValidation
            | ErrorKind::InvalidFilter
            | ErrorKind::Page
            | ErrorKind::FrozenRead => Self::bad_argument(
                "the engine rejected the request arguments (empty query, bad filter or bounds)",
            ),
            ErrorKind::EmbedderNotConfigured | ErrorKind::KindNotVectorIndexed => {
                Self::embedder_not_ready()
            }
            ErrorKind::VectorEquivalenceMismatch => Self::vector_disabled(),
            ErrorKind::GraphExpansion => graph_error(error),
            // The load fields are the bridge's to fill in.
            ErrorKind::Overloaded => Self::admission(
                store,
                Busy {
                    reason: BusyReason::EngineOverloaded,
                    in_flight: 0,
                    queued: 0,
                    abandoned: 0,
                    stuck: false,
                },
            ),
            ErrorKind::Closing => Self::not_ready(store, "draining"),
            _ => Self::internal(),
        };
        mapped.with_detail(kind)
    }

    pub(crate) fn render(&self) -> String {
        format!(
            "error {}/{}: {} Remedy: {}",
            self.class_str(),
            self.code,
            self.message,
            self.remedy
        )
    }

    pub(crate) fn class_str(&self) -> &'static str {
        match self.class {
            ErrorClass::InvalidRequest => "invalid_request",
            ErrorClass::NotFound => "not_found",
            ErrorClass::NotReady => "not_ready",
            ErrorClass::Unavailable => "unavailable",
            ErrorClass::Timeout => "timeout",
            ErrorClass::Busy => "busy",
            ErrorClass::Internal => "internal",
        }
    }
}

fn graph_error(error: &SdkError) -> ToolError {
    let reason = match error {
        SdkError::Engine(fathomdb_sdk::EngineError::GraphExpansion(inner)) => Some(inner.reason),
        _ => None,
    };
    match reason {
        Some(GraphExpansionErrorReasonV1::GraphSeedUnavailable) => ToolError::not_found(),
        Some(GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded) => ToolError::new(
            ErrorClass::InvalidRequest,
            "bound_exceeded",
            format!(
                "the expansion exceeded its work bound ({} edges examined)",
                crate::tools::EXPAND_WORK_UNITS
            ),
            "reduce max_depth or choose a less-connected seed",
        ),
        Some(GraphExpansionErrorReasonV1::GraphProjectionUnavailable) => ToolError::new(
            ErrorClass::NotReady,
            "projection_unavailable",
            "the graph projection is not available".to_string(),
            "retry later",
        ),
        Some(GraphExpansionErrorReasonV1::GraphCorrupt) | None => ToolError::internal(),
        Some(_) => ToolError::bad_argument("the engine rejected the expansion bounds or kinds"),
    }
}

// Names that fail the grammar can contain anything the agent typed, so only
// a bounded, escaped prefix is echoed.
fn quoted_name(name: &str) -> String {
    let prefix: String = name.chars().take(64).collect();
    format!("{prefix:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use fathomdb_sdk::EngineError;

    // No read path the adapter calls can produce these without an engine
    // change (the engine raises them from embed and operator verbs), so the
    // mapping is checked directly.
    #[test]
    fn embedder_errors_map_to_not_ready_embedder_not_ready() {
        for (error, detail) in [
            (EngineError::EmbedderNotConfigured, "EmbedderNotConfiguredError"),
            (EngineError::KindNotVectorIndexed, "KindNotVectorIndexedError"),
        ] {
            let mapped = ToolError::from_sdk("notes", &SdkError::Engine(error));
            assert_eq!(mapped.class, ErrorClass::NotReady);
            assert_eq!(mapped.code, "embedder_not_ready");
            assert_eq!(mapped.detail, Some(detail));
        }
    }

    // `Overloaded` comes only from the engine's embed queue, which no read
    // path the adapter calls reports, so the mapping is checked directly.
    #[test]
    fn sdk_overloaded_maps_to_admission_engine_overloaded() {
        let mapped = ToolError::from_sdk("notes", &SdkError::Engine(EngineError::Overloaded));
        assert_eq!(mapped.class, ErrorClass::Busy);
        assert_eq!(mapped.code, "admission");
        assert_eq!(mapped.detail, Some("OverloadedError"));
        let json = serde_json::to_value(&mapped).unwrap();
        assert_eq!(json["busy"]["reason"], "engine_overloaded", "{json}");
    }
}

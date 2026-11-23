use serde::Serialize;
use serde_json::Value;
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Preflight,
    Startup,
    Read,
    Transport,
}
#[derive(Clone, Debug, Serialize)]
pub(crate) struct Failure {
    pub(crate) error: &'static str,
    pub(crate) status: &'static str,
    pub(crate) phase: Phase,
    pub(crate) effects_possible: bool,
    pub(crate) supported_schema: Option<u32>,
    pub(crate) observed_schema: Option<u32>,
}
impl Failure {
    pub(crate) fn read(error: &'static str) -> Self {
        Self {
            error,
            status: match error {
                "refused"
                | "unknown_classification"
                | "invalid_range"
                | "invalid_utf8_boundary"
                | "invalid_destination"
                | "invalid_as_of" => "refused",
                "unsupported_operation"
                | "unsupported_constraint"
                | "unsupported_field"
                | "unsupported_value_encoding" => "unsupported",
                "unavailable"
                | "model_unavailable"
                | "native_evidence_unavailable"
                | "not_attached"
                | "missing"
                | "null" => "unavailable",
                "expired" | "reference_expired" | "attachment_expired" => "expired",
                "budget_exhausted" => "budget_exhausted",
                "queue_full" => "busy",
                _ => "error",
            },
            phase: Phase::Read,
            effects_possible: true,
            supported_schema: None,
            observed_schema: None,
        }
    }
    pub(crate) fn preflight(error: &'static str) -> Self {
        Self {
            phase: Phase::Preflight,
            status: "refused",
            effects_possible: false,
            ..Self::read(error)
        }
    }
    pub(crate) fn startup() -> Self {
        Self { phase: Phase::Startup, ..Self::read("startup_failed_effects_possible") }
    }
    pub(crate) fn schema(observed: u32) -> Self {
        Self {
            observed_schema: Some(observed),
            supported_schema: Some(fathomdb_schema::SCHEMA_VERSION),
            ..Self::preflight("unsupported_schema")
        }
    }
    pub(crate) fn json(&self) -> Value {
        serde_json::to_value(self).expect("closed failure fields serialize")
    }
    pub(crate) fn decode(value: &Value) -> Result<Self, &'static str> {
        const CODES: &[&str] = &[
            "invalid_sink",
            "model_unavailable",
            "native_query_refused",
            "native_evidence_unavailable",
            "missing",
            "empty",
            "not_regular",
            "missing_lock",
            "locked",
            "initial_sidecar",
            "malformed_header",
            "unsupported_schema",
            "unsupported_encoding",
            "malformed_layout",
            "malformed_vector_shape",
            "incompatible_profile",
            "missing_profile",
            "invalid_profile",
            "missing_marker",
            "invalid_generation",
            "invalid_visibility",
            "mean_recovery_required",
            "runtime_configuration",
            "invalid_path",
            "storage",
            "progress_handler",
            "startup_failed_effects_possible",
            "unavailable",
            "refused",
            "expired",
            "reference_expired",
            "attachment_expired",
            "unsupported_operation",
            "unsupported_value_encoding",
            "unsupported_constraint",
            "unsupported_field",
            "invalid_range",
            "invalid_as_of",
            "invalid_utf8",
            "invalid_utf8_boundary",
            "not_json",
            "budget_exhausted",
            "unknown_classification",
            "invalid_destination",
            "destination_unavailable",
            "null",
            "close_failed",
        ];
        let object = value.as_object().ok_or("invalid_frame")?;
        if object.keys().any(|k| {
            !["error", "status", "phase", "effects_possible", "supported_schema", "observed_schema"]
                .contains(&k.as_str())
        }) {
            return Err("invalid_frame");
        }
        let raw = value["error"].as_str().ok_or("invalid_frame")?;
        let code = *CODES.iter().find(|code| **code == raw).ok_or("invalid_frame")?;
        let mut failure = match value["phase"].as_str() {
            Some("preflight") => Self::preflight(code),
            Some("startup") if code == "startup_failed_effects_possible" => Self::startup(),
            Some("read") => Self::read(code),
            Some("transport") => Self { phase: Phase::Transport, ..Self::read(code) },
            _ => return Err("invalid_frame"),
        };
        if value["status"].as_str() != Some(failure.status)
            || value["effects_possible"].as_bool() != Some(failure.effects_possible)
        {
            return Err("invalid_frame");
        }
        for (key, destination) in [
            ("supported_schema", &mut failure.supported_schema),
            ("observed_schema", &mut failure.observed_schema),
        ] {
            if !value[key].is_null() {
                *destination = Some(
                    u32::try_from(value[key].as_u64().ok_or("invalid_frame")?)
                        .map_err(|_| "invalid_frame")?,
                );
            }
        }
        if code != "unsupported_schema"
            && (failure.supported_schema.is_some() || failure.observed_schema.is_some())
        {
            return Err("invalid_frame");
        }
        Ok(failure)
    }
}
impl From<&'static str> for Failure {
    fn from(error: &'static str) -> Self {
        Self { phase: Phase::Transport, ..Self::read(error) }
    }
}

//! Governed MCP write tools (`actuate` and `put_latest_state`, ADR-0.8.32 #99).
//!
//! Enforces:
//! - per-store `allow_writes: true` + non-empty bound writable `ScopePolicy` (`AC32-MCP-SAFETY-01`),
//! - per-store write serialization via `Registry::write_lock` (`AC32-MCP-SAFETY-01`, `AC32-MCP-SAFETY-03`),
//! - strict-only `ActuationBatchV1` operations with zero bulk/non-strict or operator escape hatches
//!   (`AC32-MCP-SAFETY-05`, `AC32-MCP-SAFETY-09`),
//! - caller anti-spoof (rejecting any `"scope"` field in operation payloads, `AC32-MCP-SAFETY-04`),
//! - per-store `allowed_kinds` / `allowed_node_kinds` / `allowed_edge_kinds` / `allowed_collections`,
//! - atomic idempotent `(store, scope, operation_id)` replay and conflict refusal (`AC32-MCP-SAFETY-02`),
//! - compare-and-set `expected_write_boundary` (`stale_write_boundary`, `AC32-MCP-SAFETY-03`).

use std::collections::BTreeMap;

use fathomdb_sdk::{
    read, ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ActuationReceiptV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, InitialState,
    LatestStateActuationV1, LifecycleActuationV1, LifecycleState, PropertyMap, PropertyValue,
    ProvenancedEdgeV1, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, VisibilityStateV1, WriteProvenanceV1,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::bridge;
use crate::error::ToolError;
use crate::output::{ActuateOutput, StoreStateOut};
use crate::registry::StorePolicy;
use crate::shape;
use crate::tools::{self, Ctx, Outcome};

/// Forbidden operator / bulk tool names refused deterministically on the MCP edge
/// (`AC32-MCP-SAFETY-09`).
pub(crate) const FORBIDDEN_TOOL_NAMES: &[&str] = &[
    "recover",
    "restore",
    "repair",
    "fix",
    "rebuild",
    "purge",
    "erase_source",
    "migrate",
    "safe_export",
    "sql",
    "execute_sql",
    "bulk_write",
    "write_with_options",
];

/// Forbidden bulk / non-strict flags refused inside actuation payloads (`AC32-MCP-SAFETY-05`).
const FORBIDDEN_OP_KEYS: &[&str] =
    &["bulk", "non_strict", "endpoint_validation", "allow_dangling_edges", "raw_sql", "sql"];

/// Schema-only view of one strict-only actuation operation for `inputSchema` generation.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ActuateOpSchema {
    PutCanonicalNode {
        kind: String,
        body: String,
        source_id: String,
        logical_id: Option<String>,
        state: Option<String>,
        reason: Option<String>,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
        properties: Option<BTreeMap<String, Value>>,
        artifact_revision_id: String,
        source_version_id: String,
    },
    PutDerivedNode {
        kind: String,
        body: String,
        source_id: String,
        logical_id: Option<String>,
        state: Option<String>,
        reason: Option<String>,
        valid_from: Option<i64>,
        valid_until: Option<i64>,
        properties: Option<BTreeMap<String, Value>>,
        artifact_revision_id: String,
        source_version_id: String,
        source_revision_id: String,
        canonical_source_hash: String,
        locator: Option<Value>,
    },
    PutDerivedEdge {
        kind: String,
        from: String,
        to: String,
        source_id: String,
        logical_id: Option<String>,
        body: Option<String>,
        t_valid: Option<i64>,
        t_invalid: Option<i64>,
        confidence: Option<f64>,
        extractor_model_id: Option<String>,
        temporal_fallback: Option<bool>,
        properties: Option<BTreeMap<String, Value>>,
        artifact_revision_id: String,
        source_version_id: String,
        source_revision_id: String,
        canonical_source_hash: String,
        locator: Option<Value>,
    },
    RegisterSourceDependency {
        dependency_id: String,
        source_revision_id: String,
        derived_revision_id: String,
    },
    TransitionLifecycle {
        logical_id: String,
        expected_current_revision_id: String,
        to_state: String,
        reason: Option<String>,
    },
    PutLatestState {
        collection: String,
        record_key: String,
        payload_json: String,
    },
}

/// `actuate` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ActuateArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// Caller-owned idempotency key (required, non-empty).
    pub operation_id: String,
    /// Optional caller policy/version identity.
    pub decision_policy_id: Option<String>,
    /// Optional compare-and-set canonical write boundary precondition.
    pub expected_write_boundary: Option<u64>,
    /// Ordered strict-only operations (1..=128).
    #[schemars(with = "Vec<ActuateOpSchema>", length(min = 1, max = 128))]
    pub operations: Vec<Value>,
    /// Include the `hints` sidecar (accepted for tool parity).
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// `put_latest_state` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PutLatestStateArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// Caller-owned idempotency key (required, non-empty).
    pub operation_id: String,
    /// Registered `latest_state` operational collection name.
    pub collection: String,
    /// Caller-owned record key within the collection.
    pub record_key: String,
    /// Exact JSON payload text validated against the collection's schema.
    pub payload_json: String,
    /// Optional compare-and-set canonical write boundary precondition.
    pub expected_write_boundary: Option<u64>,
    /// Optional caller policy/version identity.
    pub decision_policy_id: Option<String>,
    /// Include the `hints` sidecar (accepted for tool parity).
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// Returns true if `name` is a forbidden operator/bulk tool name.
pub(crate) fn is_forbidden_tool_name(name: &str) -> bool {
    FORBIDDEN_TOOL_NAMES.contains(&name)
}

fn require_writable_store(
    ctx: &Ctx<'_>,
    store: &str,
) -> Result<std::sync::Arc<StorePolicy>, ToolError> {
    let policy = ctx.shared.registry.store_policy(store)?;
    if !policy.allow_writes {
        return Err(ToolError::writes_disabled(Some(store)));
    }
    if policy.scope.is_unscoped() || policy.scope.write_scope().is_none() {
        return Err(ToolError::scope_violation());
    }
    Ok(policy)
}

pub(crate) async fn actuate(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: ActuateArgs = serde_json::from_value(Value::Object(arguments))
        .map_err(|_| ToolError::bad_argument("invalid actuate arguments"))?;
    let store = tools::resolve_store(ctx, args.store.as_deref())?;
    let policy = require_writable_store(ctx, &store)?;

    if args.operations.is_empty() || args.operations.len() > 128 {
        return Err(ToolError::bad_argument("operations must contain 1..=128 entries"));
    }

    let mut ops = Vec::with_capacity(args.operations.len());
    for raw in &args.operations {
        ops.push(parse_operation(raw, &policy)?);
    }

    let mut batch = ActuationBatchV1::new(args.operation_id, ops)
        .map_err(|_| ToolError::bad_argument("invalid operation_id or operations count"))?;
    if let Some(decision_policy_id) = args.decision_policy_id {
        batch = batch
            .with_decision_policy_id(decision_policy_id)
            .map_err(|_| ToolError::bad_argument("invalid decision_policy_id"))?;
    }
    if let Some(boundary) = args.expected_write_boundary {
        batch = batch.with_expected_write_boundary(boundary);
    }

    execute_actuation(ctx, &store, batch).await
}

pub(crate) async fn put_latest_state(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: PutLatestStateArgs = serde_json::from_value(Value::Object(arguments))
        .map_err(|_| ToolError::bad_argument("invalid put_latest_state arguments"))?;
    let store = tools::resolve_store(ctx, args.store.as_deref())?;
    let policy = require_writable_store(ctx, &store)?;
    policy.check_collection(&args.collection)?;

    let op = LatestStateActuationV1::new(args.collection, args.record_key, args.payload_json)
        .map_err(|_| ToolError::bad_argument("invalid collection, record_key, or payload_json"))?;
    let mut batch =
        ActuationBatchV1::new(args.operation_id, vec![ActuationOperationV1::PutLatestState(op)])
            .map_err(|_| ToolError::bad_argument("invalid operation_id"))?;
    if let Some(decision_policy_id) = args.decision_policy_id {
        batch = batch
            .with_decision_policy_id(decision_policy_id)
            .map_err(|_| ToolError::bad_argument("invalid decision_policy_id"))?;
    }
    if let Some(boundary) = args.expected_write_boundary {
        batch = batch.with_expected_write_boundary(boundary);
    }

    execute_actuation(ctx, &store, batch).await
}

async fn execute_actuation(ctx: &Ctx<'_>, store: &str, batch: ActuationBatchV1) -> Outcome {
    let deadline = ctx.deadline();
    let write_lock = ctx.shared.registry.write_lock(store)?;

    // Serialize all MCP writes targeting this canonical store while honoring
    // the call deadline and client cancellation (`AC32-MCP-SAFETY-01`).
    let mut cancel = ctx.cancel.clone();
    let _write_guard = tokio::select! {
        biased;
        guard = write_lock.lock_owned() => guard,
        () = tokio::time::sleep_until(deadline.into()) => {
            return Err(ToolError::deadline(Some(store)));
        }
        () = cancel.cancelled() => {
            return Err(ToolError::cancelled());
        }
    };

    let guard = ctx.acquire(store, deadline).await?;
    let validity_instant = tools::now_secs();
    let (answered, (receipt, visibility)) = bridge::call(
        guard,
        ctx.run_limit(),
        &ctx.cancel,
        ctx.progress.as_ref(),
        &ctx.shared.hooks,
        move |engine| {
            let receipt = engine.actuate(batch)?;
            let visibility = read::visibility_state(engine)?;
            Ok((receipt, visibility))
        },
    )
    .await?;

    if receipt.outcome == ActuationOutcomeV1::Refused {
        return Err(map_refusal(&receipt, &visibility));
    }

    let drift = ctx.session.observe_state(&answered.store, &answered.policy.scope_key, &visibility);
    let state = Some(StoreStateOut::from_visibility(&visibility, drift));
    let max_tokens = ctx.shared.config.default_max_tokens;
    let outcome = match receipt.outcome {
        ActuationOutcomeV1::Committed => "committed",
        ActuationOutcomeV1::CommittedClosurePending => "committed_closure_pending",
        ActuationOutcomeV1::Refused => unreachable!("handled above"),
    };
    let output = ActuateOutput {
        envelope: tools::envelope(
            &answered.store,
            answered.epoch,
            validity_instant,
            state,
            vec![],
            max_tokens,
        ),
        operation_id: receipt.operation_id,
        request_sha256: receipt.request_sha256,
        outcome: outcome.to_string(),
        affected_revision_ids: receipt.affected_revision_ids,
        resulting_write_boundary: receipt.resulting_write_boundary,
        resulting_dependency_generation: receipt.resulting_dependency_generation,
        pending_projection_write_cursors: receipt.pending_projection_write_cursors,
    };
    let text = render_actuate(&output);
    Ok(tools::finish(&output, text, max_tokens, false).0)
}

fn map_refusal(receipt: &ActuationReceiptV1, visibility: &VisibilityStateV1) -> ToolError {
    if receipt.reason_codes.contains(&ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch) {
        let observed =
            receipt.resulting_write_boundary.or(Some(visibility.canonical_write_boundary));
        return ToolError::stale_write_boundary(observed);
    }
    if receipt.reason_codes.contains(&ActuationRefusalReasonV1::ReferenceUnavailable) {
        return ToolError::not_found();
    }
    if receipt.reason_codes.contains(&ActuationRefusalReasonV1::LifecycleRefused) {
        return ToolError::lifecycle_conflict();
    }
    ToolError::bad_argument(
        "the engine refused the actuation batch atomically (strict endpoint, schema, provenance, or bounds validation failed)",
    )
}

fn render_actuate(output: &ActuateOutput) -> String {
    let mut text = format!(
        "actuate store={} epoch={} validity_instant={} operation_id={} outcome={}",
        shape::quote(&output.envelope.store),
        output.envelope.epoch,
        output.envelope.validity_instant,
        shape::quote(&output.operation_id),
        output.outcome,
    );
    if let Some(boundary) = output.resulting_write_boundary {
        text.push_str(&format!(" resulting_write_boundary={boundary}"));
    }
    if let Some(state) = &output.envelope.state {
        text.push_str(&format!(
            " state={}",
            serde_json::to_string(state).expect("state serializes")
        ));
    }
    text.push_str(&format!(
        " tokens={}/{}{}",
        output.envelope.budget.used_tokens,
        output.envelope.budget.max_tokens,
        if output.envelope.budget.cut { " cut" } else { "" }
    ));
    text
}

// --------------------------------------------------------- strict op parsing

fn parse_operation(raw: &Value, policy: &StorePolicy) -> Result<ActuationOperationV1, ToolError> {
    let Some(obj) = raw.as_object() else {
        return Err(ToolError::bad_argument("each actuation operation must be an object"));
    };
    // Anti-spoof (`AC32-MCP-SAFETY-04`): caller payloads may NEVER pass `scope`.
    if obj.contains_key("scope") {
        return Err(ToolError::scope_violation());
    }
    // Strict-only (`AC32-MCP-SAFETY-05`): reject bulk / non-strict / raw SQL flags.
    for key in FORBIDDEN_OP_KEYS {
        if obj.contains_key(*key) {
            return Err(ToolError::forbidden_operation());
        }
    }
    let Some(op) = obj.get("op").and_then(Value::as_str) else {
        return Err(ToolError::bad_argument("operation must specify string field \"op\""));
    };
    if is_forbidden_tool_name(op) {
        return Err(ToolError::forbidden_operation());
    }
    match op {
        "put_canonical_node" => parse_put_canonical_node(obj, policy),
        "put_derived_node" => parse_put_derived_node(obj, policy),
        "put_derived_edge" => parse_put_derived_edge(obj, policy),
        "register_source_dependency" => parse_register_dependency(obj),
        "transition_lifecycle" => parse_transition_lifecycle(obj),
        "put_latest_state" => parse_put_latest_state(obj, policy),
        _ => Err(ToolError::bad_argument("unsupported actuation operation variant")),
    }
}

fn reject_unknown_op_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), ToolError> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(ToolError::bad_argument("unknown field in actuation operation"));
        }
    }
    Ok(())
}

fn req_str(obj: &Map<String, Value>, key: &str) -> Result<String, ToolError> {
    obj.get(key).and_then(Value::as_str).map(str::to_string).ok_or_else(|| {
        ToolError::bad_argument("missing or invalid required string field in operation")
    })
}

fn opt_str(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, ToolError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(ToolError::bad_argument("invalid optional string field in operation")),
    }
}

fn opt_i64(obj: &Map<String, Value>, key: &str) -> Result<Option<i64>, ToolError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .map(Some)
            .ok_or_else(|| ToolError::bad_argument("invalid integer field in operation")),
    }
}

fn opt_f64(obj: &Map<String, Value>, key: &str) -> Result<Option<f64>, ToolError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .filter(|n| n.is_finite())
            .map(Some)
            .ok_or_else(|| ToolError::bad_argument("invalid numeric field in operation")),
    }
}

fn opt_bool(obj: &Map<String, Value>, key: &str) -> Result<Option<bool>, ToolError> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(ToolError::bad_argument("invalid boolean field in operation")),
    }
}

fn parse_initial_state(obj: &Map<String, Value>) -> Result<InitialState, ToolError> {
    match opt_str(obj, "state")?.as_deref() {
        None | Some("active") => Ok(InitialState::Active),
        Some("pending") => Ok(InitialState::Pending),
        Some(_) => Err(ToolError::bad_argument("state must be \"active\" or \"pending\"")),
    }
}

fn parse_properties(obj: &Map<String, Value>) -> Result<Option<PropertyMap>, ToolError> {
    match obj.get("properties") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(map)) => {
            let mut out = PropertyMap::new();
            for (k, v) in map {
                let pv = match v {
                    Value::String(s) => PropertyValue::String(s.clone()),
                    Value::Bool(b) => PropertyValue::Bool(*b),
                    Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            PropertyValue::Int(i)
                        } else if let Some(f) = n.as_f64().filter(|f| f.is_finite()) {
                            PropertyValue::Float(f)
                        } else {
                            return Err(ToolError::bad_argument(
                                "property values must be string, integer, finite float, or boolean",
                            ));
                        }
                    }
                    _ => {
                        return Err(ToolError::bad_argument(
                            "property values must be string, integer, finite float, or boolean",
                        ))
                    }
                };
                out.insert(k.clone(), pv);
            }
            Ok(Some(out))
        }
        Some(_) => Err(ToolError::bad_argument("properties must be an object of scalar values")),
    }
}

fn parse_locator(obj: &Map<String, Value>) -> Result<SourceLocator, ToolError> {
    match obj.get("locator") {
        None | Some(Value::Null) => Ok(SourceLocator::whole_body()),
        Some(Value::Object(loc)) => match loc.get("type").and_then(Value::as_str) {
            Some("whole_body") => {
                reject_unknown_op_keys(loc, &["type"])?;
                Ok(SourceLocator::whole_body())
            }
            Some("utf8_bytes") => {
                reject_unknown_op_keys(loc, &["type", "start_inclusive", "end_exclusive"])?;
                let start = loc
                    .get("start_inclusive")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| ToolError::bad_argument("invalid locator start_inclusive"))?;
                let end = loc
                    .get("end_exclusive")
                    .and_then(Value::as_u64)
                    .ok_or_else(|| ToolError::bad_argument("invalid locator end_exclusive"))?;
                Ok(SourceLocator::utf8_bytes(start, end))
            }
            _ => Err(ToolError::bad_argument("locator.type must be whole_body or utf8_bytes")),
        },
        Some(_) => Err(ToolError::bad_argument("locator must be an object")),
    }
}

fn strip_logical_prefix(value: String) -> Result<String, ToolError> {
    let bare = value.strip_prefix("l:").unwrap_or(&value).to_string();
    if bare.is_empty() || bare.contains('\0') {
        return Err(ToolError::bad_argument("logical id must be non-empty without NUL"));
    }
    Ok(bare)
}

fn parse_put_canonical_node(
    obj: &Map<String, Value>,
    policy: &StorePolicy,
) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(
        obj,
        &[
            "op",
            "kind",
            "body",
            "source_id",
            "logical_id",
            "state",
            "reason",
            "valid_from",
            "valid_until",
            "properties",
            "artifact_revision_id",
            "source_version_id",
        ],
    )?;
    let kind = req_str(obj, "kind")?;
    policy.check_node_kind(&kind)?;
    let body = req_str(obj, "body")?;
    let source_id = SourceId::new(req_str(obj, "source_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_id"))?;
    let logical_id = opt_str(obj, "logical_id")?.map(strip_logical_prefix).transpose()?;
    let state = parse_initial_state(obj)?;
    let reason = opt_str(obj, "reason")?;
    let valid_from = opt_i64(obj, "valid_from")?;
    let valid_until = opt_i64(obj, "valid_until")?;
    let properties = parse_properties(obj)?;
    let rev = ArtifactRevisionId::new(req_str(obj, "artifact_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid artifact_revision_id"))?;
    let src_ver = SourceVersionId::new(req_str(obj, "source_version_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_version_id"))?;
    Ok(ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
        kind,
        body,
        source_id,
        logical_id,
        state,
        reason,
        valid_from,
        valid_until,
        properties,
        provenance: WriteProvenanceV1::canonical(rev, src_ver),
    }))
}

fn parse_put_derived_node(
    obj: &Map<String, Value>,
    policy: &StorePolicy,
) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(
        obj,
        &[
            "op",
            "kind",
            "body",
            "source_id",
            "logical_id",
            "state",
            "reason",
            "valid_from",
            "valid_until",
            "properties",
            "artifact_revision_id",
            "source_version_id",
            "source_revision_id",
            "canonical_source_hash",
            "locator",
        ],
    )?;
    let kind = req_str(obj, "kind")?;
    policy.check_node_kind(&kind)?;
    let body = req_str(obj, "body")?;
    let source_id = SourceId::new(req_str(obj, "source_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_id"))?;
    let logical_id = opt_str(obj, "logical_id")?.map(strip_logical_prefix).transpose()?;
    let state = parse_initial_state(obj)?;
    let reason = opt_str(obj, "reason")?;
    let valid_from = opt_i64(obj, "valid_from")?;
    let valid_until = opt_i64(obj, "valid_until")?;
    let properties = parse_properties(obj)?;
    let rev = ArtifactRevisionId::new(req_str(obj, "artifact_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid artifact_revision_id"))?;
    let src_ver = SourceVersionId::new(req_str(obj, "source_version_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_version_id"))?;
    let src_rev = SourceRevisionId::new(req_str(obj, "source_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_revision_id"))?;
    let hash = CanonicalHash::sha256(req_str(obj, "canonical_source_hash")?)
        .map_err(|_| ToolError::bad_argument("invalid canonical_source_hash"))?;
    let locator = parse_locator(obj)?;
    Ok(ActuationOperationV1::PutDerivedNode(ProvenancedNodeV1 {
        kind,
        body,
        source_id,
        logical_id,
        state,
        reason,
        valid_from,
        valid_until,
        properties,
        provenance: WriteProvenanceV1::derived(rev, src_ver, src_rev, locator, hash),
    }))
}

fn parse_put_derived_edge(
    obj: &Map<String, Value>,
    policy: &StorePolicy,
) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(
        obj,
        &[
            "op",
            "kind",
            "from",
            "to",
            "source_id",
            "logical_id",
            "body",
            "t_valid",
            "t_invalid",
            "confidence",
            "extractor_model_id",
            "temporal_fallback",
            "properties",
            "artifact_revision_id",
            "source_version_id",
            "source_revision_id",
            "canonical_source_hash",
            "locator",
        ],
    )?;
    let kind = req_str(obj, "kind")?;
    policy.check_edge_kind(&kind)?;
    let from = strip_logical_prefix(req_str(obj, "from")?)?;
    let to = strip_logical_prefix(req_str(obj, "to")?)?;
    let source_id = SourceId::new(req_str(obj, "source_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_id"))?;
    let logical_id = opt_str(obj, "logical_id")?.map(strip_logical_prefix).transpose()?;
    let body = opt_str(obj, "body")?;
    let t_valid = opt_i64(obj, "t_valid")?;
    let t_invalid = opt_i64(obj, "t_invalid")?;
    let confidence = opt_f64(obj, "confidence")?;
    let extractor_model_id = opt_str(obj, "extractor_model_id")?;
    let temporal_fallback = opt_bool(obj, "temporal_fallback")?;
    let properties = parse_properties(obj)?;
    let rev = ArtifactRevisionId::new(req_str(obj, "artifact_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid artifact_revision_id"))?;
    let src_ver = SourceVersionId::new(req_str(obj, "source_version_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_version_id"))?;
    let src_rev = SourceRevisionId::new(req_str(obj, "source_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid source_revision_id"))?;
    let hash = CanonicalHash::sha256(req_str(obj, "canonical_source_hash")?)
        .map_err(|_| ToolError::bad_argument("invalid canonical_source_hash"))?;
    let locator = parse_locator(obj)?;
    Ok(ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
        kind,
        from,
        to,
        source_id,
        logical_id,
        body,
        t_valid,
        t_invalid,
        confidence,
        extractor_model_id,
        temporal_fallback,
        properties,
        provenance: WriteProvenanceV1::derived(rev, src_ver, src_rev, locator, hash),
    }))
}

fn parse_register_dependency(obj: &Map<String, Value>) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(
        obj,
        &["op", "dependency_id", "source_revision_id", "derived_revision_id"],
    )?;
    let reg = SourceDependencyRegistrationV1::new(
        req_str(obj, "dependency_id")?,
        req_str(obj, "source_revision_id")?,
        req_str(obj, "derived_revision_id")?,
    )
    .map_err(|_| ToolError::bad_argument("invalid dependency registration identifiers"))?;
    Ok(ActuationOperationV1::RegisterSourceDependency(reg))
}

fn parse_transition_lifecycle(obj: &Map<String, Value>) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(
        obj,
        &["op", "logical_id", "expected_current_revision_id", "to_state", "reason"],
    )?;
    let logical_id = req_str(obj, "logical_id")?;
    let rev = ArtifactRevisionId::new(req_str(obj, "expected_current_revision_id")?)
        .map_err(|_| ToolError::bad_argument("invalid expected_current_revision_id"))?;
    let to_state = match req_str(obj, "to_state")?.as_str() {
        "active" => LifecycleState::Active,
        "deleted" => LifecycleState::Deleted,
        _ => return Err(ToolError::bad_argument("to_state must be \"active\" or \"deleted\"")),
    };
    let reason = opt_str(obj, "reason")?;
    let op = LifecycleActuationV1::new(logical_id, rev, to_state, reason)
        .map_err(|_| ToolError::bad_argument("invalid lifecycle transition operation"))?;
    Ok(ActuationOperationV1::TransitionLifecycle(op))
}

fn parse_put_latest_state(
    obj: &Map<String, Value>,
    policy: &StorePolicy,
) -> Result<ActuationOperationV1, ToolError> {
    reject_unknown_op_keys(obj, &["op", "collection", "record_key", "payload_json"])?;
    let collection = req_str(obj, "collection")?;
    policy.check_collection(&collection)?;
    let record_key = req_str(obj, "record_key")?;
    let payload_json = req_str(obj, "payload_json")?;
    let op = LatestStateActuationV1::new(collection, record_key, payload_json)
        .map_err(|_| ToolError::bad_argument("invalid collection, record_key, or payload_json"))?;
    Ok(ActuationOperationV1::PutLatestState(op))
}

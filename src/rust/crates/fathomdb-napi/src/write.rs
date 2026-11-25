use super::*;

// ===== Batch translation ==============================================
//
// The TS surface accepts a JS array of typed-write objects. Each item
// is one of:
//   { node: { kind, body?, sourceId? } }
//   { edge: { kind, from, to, sourceId? } }
//   { opStore: { collection, recordKey, schemaId?, body } }
//   { adminSchema: { name, kind, schemaJson, retentionJson? } }
// Plus a bare `{ kind, body?, sourceId? }` shape treated as Node (parity
// with the Python stub). serde_json::Value is the cheapest cross-thread
// representation; napi-rs converts JS objects via the `serde-json`
// feature.

pub(crate) fn translate_batch(batch: Vec<JsonValue>) -> Result<Vec<PreparedWrite>> {
    batch.into_iter().map(translate_write_item).collect()
}

pub(crate) fn json_get<'a>(v: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    v.as_object().and_then(|m| m.get(key))
}

pub(crate) fn json_str(v: &JsonValue, key: &str) -> Result<Option<String>> {
    match json_get(v, key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::String(s)) => {
            validate_ffi_string_napi(s)?;
            Ok(Some(s.clone()))
        }
        Some(_other) => Err(typed_error(
            CODE_WRITE_VALIDATION,
            format!("field {key:?} must be a string"),
            JsonValue::Null,
        )),
    }
}

pub(crate) fn json_str_required(v: &JsonValue, key: &str) -> Result<String> {
    json_str(v, key)?.ok_or_else(|| {
        typed_error(
            CODE_WRITE_VALIDATION,
            format!("write item missing required field {key:?}"),
            JsonValue::Null,
        )
    })
}

pub(crate) fn json_serialised(v: &JsonValue, key: &str) -> Result<Option<String>> {
    match json_get(v, key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::String(s)) => {
            validate_ffi_string_napi(s)?;
            Ok(Some(s.clone()))
        }
        Some(other) => {
            let serialised = serde_json::to_string(other).map_err(|e| {
                typed_error(
                    CODE_WRITE_VALIDATION,
                    format!("field {key:?} not serialisable: {e}"),
                    JsonValue::Null,
                )
            })?;
            validate_ffi_string_napi(&serialised)?;
            Ok(Some(serialised))
        }
    }
}

pub(crate) fn json_serialised_required(v: &JsonValue, key: &str) -> Result<String> {
    json_serialised(v, key)?.ok_or_else(|| {
        typed_error(
            CODE_WRITE_VALIDATION,
            format!("write item missing required field {key:?}"),
            JsonValue::Null,
        )
    })
}

pub(crate) fn translate_write_item(item: JsonValue) -> Result<PreparedWrite> {
    if !item.is_object() {
        return Err(typed_error(
            CODE_WRITE_VALIDATION,
            "write item must be an object",
            JsonValue::Null,
        ));
    }
    if let Some(inner) = json_get(&item, "edge") {
        return translate_edge(inner);
    }
    if let Some(inner) = json_get(&item, "opStore").or_else(|| json_get(&item, "op_store")) {
        return translate_op_store(inner);
    }
    if let Some(inner) = json_get(&item, "adminSchema").or_else(|| json_get(&item, "admin_schema"))
    {
        return translate_admin_schema(inner);
    }
    if let Some(inner) = json_get(&item, "node") {
        return translate_node(inner);
    }
    translate_node(&item)
}

/// Look up `camelCase` first, then `snake_case`, returning the first
/// present string. Both forms are checked so callers porting from the
/// Python stub keep working without surface-level rewrites.
pub(crate) fn json_str_alt(item: &JsonValue, camel: &str, snake: &str) -> Result<Option<String>> {
    if let Some(v) = json_str(item, camel)? {
        return Ok(Some(v));
    }
    json_str(item, snake)
}

pub(crate) fn json_str_alt_required(item: &JsonValue, camel: &str, snake: &str) -> Result<String> {
    json_str_alt(item, camel, snake)?.ok_or_else(|| {
        typed_error(
            CODE_WRITE_VALIDATION,
            format!("write item missing required field {camel:?}"),
            JsonValue::Null,
        )
    })
}

/// 0.8.20 Slice 5c (R-20-E3) — `sourceId` is now MANDATORY on every canonical
/// write. Rust makes its absence inexpressible via the `SourceId` newtype;
/// TypeScript has no such guarantee at the N-API boundary, so the binding throws
/// a typed write-validation error for a missing, empty or reserved
/// (`_`-prefixed) id. This is the TS arm of "an un-provenanced write does not
/// compile / raises", and it mirrors the Python binding exactly.
///
/// The rationale is not tidiness: `excise_source` addresses rows BY `source_id`,
/// so a row written without one is reachable by no erasure call — un-erasable.
pub(crate) fn source_id_write_error(message: String, field_path: Option<&str>) -> Error {
    let payload = field_path.map_or(JsonValue::Null, |path| json!({ "fieldPath": path }));
    typed_error(CODE_WRITE_VALIDATION, message, payload)
}

pub(crate) fn json_source_id_required_at(
    item: &JsonValue,
    kind: &str,
    field_path: Option<&str>,
) -> Result<SourceId> {
    let raw = json_get(item, "sourceId")
        .or_else(|| json_get(item, "source_id"))
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            source_id_write_error(
                format!(
                    "{kind} write item missing required field \"sourceId\": provenance is mandatory \
                     since 0.8.20 — a row written without it can never be erased by excise_source"
                ),
                field_path,
            )
        })?;
    SourceId::new(raw).map_err(|_| {
        source_id_write_error(
            "\"sourceId\" must be a non-empty identifier outside the engine's reserved \
             \"_\"-prefixed namespace"
                .to_string(),
            field_path,
        )
    })
}

pub(crate) fn json_source_id_required(item: &JsonValue, kind: &str) -> Result<SourceId> {
    json_source_id_required_at(item, kind, None)
}

pub(crate) fn provenance_napi_error(reason: &str, field_path: impl Into<String>) -> Error {
    let field_path = field_path.into();
    typed_error(
        CODE_PROVENANCE,
        format!("provenance {reason} at {field_path}"),
        json!({ "reason": reason, "fieldPath": field_path }),
    )
}

pub(crate) fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

pub(crate) fn dependency_napi_error(reason: &str, field_path: impl Into<String>) -> Error {
    let field_path = field_path.into();
    typed_error(
        CODE_DEPENDENCY,
        format!("dependency {reason} at {field_path}"),
        json!({ "reason": reason, "fieldPath": field_path }),
    )
}

pub(crate) fn actuation_napi_error(reason: &str, field_path: impl Into<String>) -> Error {
    let field_path = field_path.into();
    typed_error(
        CODE_ACTUATION,
        format!("actuation {reason} at {field_path}"),
        json!({ "reason": reason, "fieldPath": field_path }),
    )
}

pub(crate) fn nested_actuation_napi_error(error: Error, root: &str) -> Error {
    let nested_path = serde_json::from_str::<JsonValue>(&error.reason).ok().and_then(|envelope| {
        envelope.get("payload")?.get("fieldPath")?.as_str().map(str::to_string)
    });
    let field_path = nested_path
        .filter(|path| path.starts_with('/'))
        .map_or_else(|| root.to_string(), |path| format!("{root}{path}"));
    actuation_napi_error("nested_request_invalid", field_path)
}

pub(crate) fn strict_actuation_object<'a>(
    value: &'a JsonValue,
    allowed: &[&str],
    base_path: &str,
) -> Result<&'a serde_json::Map<String, JsonValue>> {
    let object =
        value.as_object().ok_or_else(|| actuation_napi_error("field_type_invalid", base_path))?;
    if let Some(key) = object.keys().filter(|key| !allowed.contains(&key.as_str())).min() {
        return Err(actuation_napi_error(
            "unknown_field",
            format!("{base_path}/{}", escape_json_pointer_token(key)),
        ));
    }
    Ok(object)
}

pub(crate) fn actuation_required_string(
    object: &serde_json::Map<String, JsonValue>,
    key: &str,
    path: &str,
) -> Result<String> {
    match object.get(key) {
        Some(JsonValue::String(value)) if !value.contains('\0') => Ok(value.clone()),
        None | Some(JsonValue::Null) => Err(actuation_napi_error("field_missing", path)),
        _ => Err(actuation_napi_error("field_type_invalid", path)),
    }
}

pub(crate) fn valid_actuation_caller_identity(value: &str) -> bool {
    ArtifactRevisionId::new(value).is_ok()
}

pub(crate) fn translate_actuation_request(value: &JsonValue) -> Result<ActuationBatchV1> {
    let object = value
        .as_object()
        .ok_or_else(|| actuation_napi_error("unsupported_schema_version", "/schemaVersion"))?;
    if object.get("schemaVersion").and_then(JsonValue::as_u64) != Some(1) {
        return Err(actuation_napi_error("unsupported_schema_version", "/schemaVersion"));
    }
    let object = strict_actuation_object(
        value,
        &[
            "schemaVersion",
            "operationId",
            "decisionPolicyId",
            "expectedWriteBoundary",
            "operations",
        ],
        "",
    )?;
    let operation_id = actuation_required_string(object, "operationId", "/operationId")?;
    let decision_policy_id = match object.get("decisionPolicyId") {
        None | Some(JsonValue::Null) => None,
        Some(JsonValue::String(value)) if !value.contains('\0') => Some(value.clone()),
        _ => return Err(actuation_napi_error("field_type_invalid", "/decisionPolicyId")),
    };
    let expected_write_boundary = match object.get("expectedWriteBoundary") {
        None | Some(JsonValue::Null) => None,
        Some(JsonValue::String(text)) => {
            Some(text.parse::<u64>().ok().filter(|parsed| parsed.to_string() == *text).ok_or_else(
                || actuation_napi_error("field_type_invalid", "/expectedWriteBoundary"),
            )?)
        }
        _ => return Err(actuation_napi_error("field_type_invalid", "/expectedWriteBoundary")),
    };
    let operation_values = object
        .get("operations")
        .ok_or_else(|| actuation_napi_error("field_missing", "/operations"))?
        .as_array()
        .ok_or_else(|| actuation_napi_error("field_type_invalid", "/operations"))?;
    if !valid_actuation_caller_identity(&operation_id) {
        return Err(actuation_napi_error("operation_id_invalid", "/operationId"));
    }
    if decision_policy_id.as_deref().is_some_and(|value| !valid_actuation_caller_identity(value)) {
        return Err(actuation_napi_error("decision_policy_id_invalid", "/decisionPolicyId"));
    }
    if !(1..=128).contains(&operation_values.len()) {
        return Err(actuation_napi_error("operation_count_invalid", "/operations"));
    }
    let operations = operation_values
        .iter()
        .enumerate()
        .map(|(index, operation)| translate_actuation_operation(operation, index))
        .collect::<Result<Vec<_>>>()?;
    let mut request = ActuationBatchV1::new(operation_id, operations)
        .map_err(|error| actuation_napi_error(error.reason.as_str(), error.field_path))?;
    if let Some(policy) = decision_policy_id {
        request = request
            .with_decision_policy_id(policy)
            .map_err(|error| actuation_napi_error(error.reason.as_str(), error.field_path))?;
    }
    if let Some(boundary) = expected_write_boundary {
        request = request.with_expected_write_boundary(boundary);
    }
    Ok(request)
}

pub(crate) fn translate_actuation_operation(
    value: &JsonValue,
    index: usize,
) -> Result<ActuationOperationV1> {
    let root = format!("/operations/{index}");
    let object =
        value.as_object().ok_or_else(|| actuation_napi_error("field_type_invalid", &root))?;
    let kind = actuation_required_string(object, "type", &format!("{root}/type"))?;
    match kind.as_str() {
        "put_canonical_node" | "put_derived_node" => {
            strict_actuation_object(value, &["type", "record"], &root)?;
            let record = object
                .get("record")
                .ok_or_else(|| actuation_napi_error("field_missing", format!("{root}/record")))?;
            strict_actuation_object(
                record,
                &[
                    "kind",
                    "body",
                    "sourceId",
                    "logicalId",
                    "state",
                    "reason",
                    "validFrom",
                    "validUntil",
                    "properties",
                    "provenance",
                ],
                &format!("{root}/record"),
            )?;
            let record_root = format!("{root}/record");
            let prepared = translate_node_at(record, Some("/sourceId"))
                .map_err(|error| nested_actuation_napi_error(error, &record_root))?;
            let PreparedWrite::ProvenancedNode(node) = prepared else {
                return Err(actuation_napi_error(
                    "nested_request_invalid",
                    format!("{root}/record/provenance"),
                ));
            };
            if kind == "put_canonical_node" {
                Ok(ActuationOperationV1::PutCanonicalNode(node))
            } else {
                Ok(ActuationOperationV1::PutDerivedNode(node))
            }
        }
        "put_derived_edge" => {
            strict_actuation_object(value, &["type", "record"], &root)?;
            let record = object
                .get("record")
                .ok_or_else(|| actuation_napi_error("field_missing", format!("{root}/record")))?;
            strict_actuation_object(
                record,
                &[
                    "kind",
                    "from",
                    "to",
                    "sourceId",
                    "logicalId",
                    "body",
                    "tValid",
                    "tInvalid",
                    "properties",
                    "provenance",
                ],
                &format!("{root}/record"),
            )?;
            let record_root = format!("{root}/record");
            let prepared = translate_edge(record)
                .map_err(|error| nested_actuation_napi_error(error, &record_root))?;
            let PreparedWrite::ProvenancedEdge(edge) = prepared else {
                return Err(actuation_napi_error(
                    "nested_request_invalid",
                    format!("{root}/record/provenance"),
                ));
            };
            Ok(ActuationOperationV1::PutDerivedEdge(edge))
        }
        "register_source_dependency" => {
            strict_actuation_object(value, &["type", "dependency"], &root)?;
            let dependency = object.get("dependency").ok_or_else(|| {
                actuation_napi_error("field_missing", format!("{root}/dependency"))
            })?;
            let dependency_root = format!("{root}/dependency");
            translate_dependency_registration(dependency)
                .map(ActuationOperationV1::RegisterSourceDependency)
                .map_err(|error| nested_actuation_napi_error(error, &dependency_root))
        }
        "transition_lifecycle" => {
            strict_actuation_object(
                value,
                &["type", "logicalId", "expectedCurrentRevisionId", "toState", "reason"],
                &root,
            )?;
            let logical_id =
                actuation_required_string(object, "logicalId", &format!("{root}/logicalId"))?;
            let revision = actuation_required_string(
                object,
                "expectedCurrentRevisionId",
                &format!("{root}/expectedCurrentRevisionId"),
            )?;
            let revision = ArtifactRevisionId::new(revision).map_err(|_| {
                actuation_napi_error(
                    "revision_id_invalid",
                    format!("{root}/expectedCurrentRevisionId"),
                )
            })?;
            let target = actuation_required_string(object, "toState", &format!("{root}/toState"))?;
            let target = RustLifecycleState::from_str_opt(&target).ok_or_else(|| {
                actuation_napi_error("lifecycle_target_invalid", format!("{root}/toState"))
            })?;
            let reason = match object.get("reason") {
                None | Some(JsonValue::Null) => None,
                Some(JsonValue::String(value)) if !value.contains('\0') => Some(value.clone()),
                _ => {
                    return Err(actuation_napi_error(
                        "field_type_invalid",
                        format!("{root}/reason"),
                    ))
                }
            };
            LifecycleActuationV1::new(logical_id, revision, target, reason)
                .map(ActuationOperationV1::TransitionLifecycle)
                .map_err(|error| {
                    actuation_napi_error(
                        error.reason.as_str(),
                        format!("{root}{}", error.field_path),
                    )
                })
        }
        "put_latest_state" => {
            strict_actuation_object(
                value,
                &["type", "collection", "recordKey", "payloadJson"],
                &root,
            )?;
            let collection =
                actuation_required_string(object, "collection", &format!("{root}/collection"))?;
            let record_key =
                actuation_required_string(object, "recordKey", &format!("{root}/recordKey"))?;
            let payload_json =
                actuation_required_string(object, "payloadJson", &format!("{root}/payloadJson"))?;
            LatestStateActuationV1::new(collection, record_key, payload_json)
                .map(ActuationOperationV1::PutLatestState)
                .map_err(|error| {
                    actuation_napi_error(
                        error.reason.as_str(),
                        format!("{root}{}", error.field_path),
                    )
                })
        }
        _ => Err(actuation_napi_error("unknown_operation_variant", format!("{root}/type"))),
    }
}

pub(crate) fn dependency_request_object<'a>(
    value: &'a JsonValue,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, JsonValue>> {
    let object = value
        .as_object()
        .ok_or_else(|| dependency_napi_error("unsupported_schema_version", "/schemaVersion"))?;
    if object.get("schemaVersion").and_then(JsonValue::as_u64) != Some(1) {
        return Err(dependency_napi_error("unsupported_schema_version", "/schemaVersion"));
    }
    if let Some(key) = object.keys().filter(|key| !allowed.contains(&key.as_str())).min() {
        return Err(dependency_napi_error(
            "unknown_field",
            format!("/{}", escape_json_pointer_token(key)),
        ));
    }
    Ok(object)
}

pub(crate) fn dependency_string(
    object: &serde_json::Map<String, JsonValue>,
    key: &str,
    reason: &str,
) -> Result<String> {
    match object.get(key) {
        Some(JsonValue::String(value)) if !value.contains('\0') => Ok(value.clone()),
        _ => Err(dependency_napi_error(reason, format!("/{key}"))),
    }
}

pub(crate) fn translate_dependency_registration(
    value: &JsonValue,
) -> Result<SourceDependencyRegistrationV1> {
    let object = dependency_request_object(
        value,
        &["schemaVersion", "dependencyId", "sourceRevisionId", "derivedRevisionId"],
    )?;
    SourceDependencyRegistrationV1::new(
        dependency_string(object, "dependencyId", "dependency_id_invalid")?,
        dependency_string(object, "sourceRevisionId", "dependency_reference_invalid")?,
        dependency_string(object, "derivedRevisionId", "dependency_reference_invalid")?,
    )
    .map_err(|error| dependency_napi_error(error.reason.as_str(), error.field_path))
}

pub(crate) fn translate_dependency_source_lookup(
    value: &JsonValue,
) -> Result<DependencySourceLookupV1> {
    let object = dependency_request_object(value, &["schemaVersion", "sourceRevisionId"])?;
    DependencySourceLookupV1::new(dependency_string(
        object,
        "sourceRevisionId",
        "dependency_reference_invalid",
    )?)
    .map_err(|error| dependency_napi_error(error.reason.as_str(), error.field_path))
}

pub(crate) fn translate_dependency_derived_lookup(
    value: &JsonValue,
) -> Result<DependencyDerivedLookupV1> {
    let object = dependency_request_object(value, &["schemaVersion", "derivedRevisionId"])?;
    DependencyDerivedLookupV1::new(dependency_string(
        object,
        "derivedRevisionId",
        "dependency_reference_invalid",
    )?)
    .map_err(|error| dependency_napi_error(error.reason.as_str(), error.field_path))
}

pub(crate) fn dependency_closure_napi_error(reason: &str, field_path: impl Into<String>) -> Error {
    let field_path = field_path.into();
    typed_error(
        CODE_DEPENDENCY_CLOSURE,
        format!("dependency closure {reason} at {field_path}"),
        json!({ "reason": reason, "fieldPath": field_path }),
    )
}

pub(crate) fn translate_closure_lookup(value: &JsonValue) -> Result<ClosureLookupV1> {
    let object = value.as_object().ok_or_else(|| {
        dependency_closure_napi_error("unsupported_schema_version", "/schemaVersion")
    })?;
    if object.get("schemaVersion").and_then(JsonValue::as_u64) != Some(1) {
        return Err(dependency_closure_napi_error("unsupported_schema_version", "/schemaVersion"));
    }
    if let Some(key) = object
        .keys()
        .filter(|key| !["schemaVersion", "closureOperationId"].contains(&key.as_str()))
        .min()
    {
        return Err(dependency_closure_napi_error(
            "unknown_field",
            format!("/{}", escape_json_pointer_token(key)),
        ));
    }
    let id = match object.get("closureOperationId") {
        Some(JsonValue::String(value)) if !value.contains('\0') => value.clone(),
        _ => {
            return Err(dependency_closure_napi_error(
                "closure_operation_id_invalid",
                "/closureOperationId",
            ))
        }
    };
    ClosureLookupV1::new(id)
        .map_err(|error| dependency_closure_napi_error(error.reason.as_str(), error.field_path))
}

pub(crate) fn closure_status_json(value: RustClosureStatusV1) -> JsonValue {
    let root = match value.root {
        ClosureRootV1::SourceRevision { source_revision_id } => json!({
            "type": "source_revision",
            "sourceRevisionId": source_revision_id.as_str(),
        }),
        ClosureRootV1::SourceBucket { source_id } => json!({
            "type": "source_bucket",
            "sourceId": source_id.as_str(),
        }),
    };
    let proof = value.proof.map(|proof| {
        json!({
            "schemaVersion": proof.schema_version,
            "proofWriteBoundary": proof.proof_write_boundary.to_string(),
            "currentActiveDependentNodes": proof.current_active_dependent_nodes.to_string(),
            "currentDerivedEdges": proof.current_derived_edges.to_string(),
            "viewEligibleDependents": proof.view_eligible_dependents.to_string(),
            "ownerlessProjectionRows": proof.ownerless_projection_rows.to_string(),
            "postAdmissionRegistrations": proof.post_admission_registrations.to_string(),
            "remainingDependencyRows": proof.remaining_dependency_rows.map(|item| item.to_string()),
            "remainingCanonicalRows": proof.remaining_canonical_rows.map(|item| item.to_string()),
            "remainingProjectionRows": proof.remaining_projection_rows.map(|item| item.to_string()),
            "remainingReceiptReferenceRows": proof.remaining_receipt_reference_rows.map(|item| item.to_string()),
        })
    });
    json!({
        "schemaVersion": value.schema_version,
        "closureOperationId": value.closure_operation_id.as_str(),
        "root": root,
        "cause": value.cause.as_str(),
        "phase": value.phase.as_str(),
        "effectiveAtEpochS": value.effective_at_epoch_s.to_string(),
        "admittedWriteBoundary": value.admitted_write_boundary.to_string(),
        "admittedDependencyGeneration": value.admitted_dependency_generation.to_string(),
        "affectedCount": value.affected_count.to_string(),
        "blockerCode": value.blocker_code,
        "proof": proof,
    })
}

pub(crate) fn strict_json_keys(value: &JsonValue, allowed: &[&str], base_path: &str) -> Result<()> {
    let object =
        value.as_object().ok_or_else(|| provenance_napi_error("unknown_field", base_path))?;
    if let Some(key) = object.keys().filter(|key| !allowed.contains(&key.as_str())).min() {
        return Err(provenance_napi_error(
            "unknown_field",
            format!("{base_path}/{}", escape_json_pointer_token(key)),
        ));
    }
    Ok(())
}

pub(crate) fn provenance_string(value: &JsonValue, key: &str, reason: &str) -> Result<String> {
    match json_get(value, key) {
        Some(JsonValue::String(value)) if !value.contains('\0') => Ok(value.clone()),
        _ => Err(provenance_napi_error(reason, format!("/provenance/{key}"))),
    }
}

pub(crate) fn decimal_json_offset(value: &JsonValue, key: &str) -> Result<u64> {
    let encoded = match json_get(value, key) {
        Some(JsonValue::String(value)) => value,
        _ => {
            return Err(provenance_napi_error(
                "locator_invalid",
                format!("/provenance/sourceLocator/{key}"),
            ))
        }
    };
    let valid = encoded == "0"
        || (!encoded.starts_with('0') && encoded.bytes().all(|byte| byte.is_ascii_digit()));
    let parsed = encoded.parse::<u64>().ok().filter(|offset| *offset <= i64::MAX as u64);
    if !valid || parsed.is_none() {
        return Err(provenance_napi_error(
            "locator_invalid",
            format!("/provenance/sourceLocator/{key}"),
        ));
    }
    Ok(parsed.unwrap())
}

pub(crate) fn translate_json_locator(value: &JsonValue) -> Result<SourceLocator> {
    let kind = match json_get(value, "kind") {
        Some(JsonValue::String(kind)) => kind.as_str(),
        _ => {
            return Err(provenance_napi_error("locator_invalid", "/provenance/sourceLocator/kind"))
        }
    };
    match kind {
        "whole_body" => {
            strict_json_keys(value, &["kind"], "/provenance/sourceLocator")?;
            Ok(SourceLocator::whole_body())
        }
        "utf8_bytes" => {
            strict_json_keys(
                value,
                &["kind", "startInclusive", "endExclusive"],
                "/provenance/sourceLocator",
            )?;
            Ok(SourceLocator::utf8_bytes(
                decimal_json_offset(value, "startInclusive")?,
                decimal_json_offset(value, "endExclusive")?,
            ))
        }
        _ => Err(provenance_napi_error("locator_invalid", "/provenance/sourceLocator/kind")),
    }
}

pub(crate) fn translate_json_hash(value: &JsonValue) -> Result<CanonicalHash> {
    strict_json_keys(value, &["algorithm", "digestHex"], "/provenance/canonicalSourceHash")?;
    if json_get(value, "algorithm").and_then(JsonValue::as_str) != Some("sha256") {
        return Err(provenance_napi_error(
            "hash_invalid",
            "/provenance/canonicalSourceHash/algorithm",
        ));
    }
    let digest = json_get(value, "digestHex").and_then(JsonValue::as_str).ok_or_else(|| {
        provenance_napi_error("hash_invalid", "/provenance/canonicalSourceHash/digestHex")
    })?;
    CanonicalHash::sha256(digest).map_err(engine_error_to_napi)
}

pub(crate) fn translate_json_provenance(value: &JsonValue) -> Result<WriteProvenanceV1> {
    if !value.is_object() {
        return Err(provenance_napi_error("role_invalid", "/provenance"));
    }
    if json_get(value, "schemaVersion").and_then(JsonValue::as_u64) != Some(1) {
        return Err(provenance_napi_error(
            "unsupported_schema_version",
            "/provenance/schemaVersion",
        ));
    }
    let role = json_get(value, "role")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| provenance_napi_error("role_invalid", "/provenance/role"))?;
    if !matches!(role, "canonical" | "derived") {
        return Err(provenance_napi_error("role_invalid", "/provenance/role"));
    }
    strict_json_keys(
        value,
        &[
            "schemaVersion",
            "role",
            "artifactRevisionId",
            "sourceVersionId",
            "sourceRevisionId",
            "sourceLocator",
            "canonicalSourceHash",
        ],
        "/provenance",
    )?;
    let artifact_revision_id = ArtifactRevisionId::new(provenance_string(
        value,
        "artifactRevisionId",
        "revision_id_invalid",
    )?)
    .map_err(engine_error_to_napi)?;
    let source_version_id = SourceVersionId::new(provenance_string(
        value,
        "sourceVersionId",
        "source_version_invalid",
    )?)
    .map_err(engine_error_to_napi)?;
    match role {
        "canonical" => {
            strict_json_keys(
                value,
                &["schemaVersion", "role", "artifactRevisionId", "sourceVersionId"],
                "/provenance",
            )?;
            Ok(WriteProvenanceV1::canonical(artifact_revision_id, source_version_id))
        }
        "derived" => {
            strict_json_keys(
                value,
                &[
                    "schemaVersion",
                    "role",
                    "artifactRevisionId",
                    "sourceVersionId",
                    "sourceRevisionId",
                    "sourceLocator",
                    "canonicalSourceHash",
                ],
                "/provenance",
            )?;
            let source_revision_id = SourceRevisionId::new(provenance_string(
                value,
                "sourceRevisionId",
                "revision_id_invalid",
            )?)
            .map_err(engine_error_to_napi)?;
            let locator = json_get(value, "sourceLocator")
                .ok_or_else(|| {
                    provenance_napi_error("locator_invalid", "/provenance/sourceLocator")
                })
                .and_then(translate_json_locator)?;
            let hash = json_get(value, "canonicalSourceHash")
                .ok_or_else(|| {
                    provenance_napi_error("hash_invalid", "/provenance/canonicalSourceHash")
                })
                .and_then(translate_json_hash)?;
            Ok(WriteProvenanceV1::derived(
                artifact_revision_id,
                source_version_id,
                source_revision_id,
                locator,
                hash,
            ))
        }
        _ => Err(provenance_napi_error("role_invalid", "/provenance/role")),
    }
}

pub(crate) fn json_serialised_alt(
    item: &JsonValue,
    camel: &str,
    snake: &str,
) -> Result<Option<String>> {
    if let Some(v) = json_serialised(item, camel)? {
        return Ok(Some(v));
    }
    json_serialised(item, snake)
}

pub(crate) fn json_serialised_alt_required(
    item: &JsonValue,
    camel: &str,
    snake: &str,
) -> Result<String> {
    json_serialised_alt(item, camel, snake)?.ok_or_else(|| {
        typed_error(
            CODE_WRITE_VALIDATION,
            format!("write item missing required field {camel:?}"),
            JsonValue::Null,
        )
    })
}

pub(crate) fn json_property_map(item: &JsonValue, key: &str) -> Result<Option<RustPropertyMap>> {
    let Some(raw) = json_get(item, key) else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let obj = raw.as_object().ok_or_else(|| {
        typed_error(CODE_WRITE_VALIDATION, "properties must be an object or null", JsonValue::Null)
    })?;
    let mut map = RustPropertyMap::new();
    if obj.len() == 1 {
        if let Some(entries_val) = obj.get("__fdb_props_v1__") {
            let entries = entries_val.as_array().ok_or_else(|| {
                typed_error(
                    CODE_WRITE_VALIDATION,
                    "invalid encoded properties payload",
                    JsonValue::Null,
                )
            })?;
            for entry in entries {
                let eobj = entry.as_object().ok_or_else(|| {
                    typed_error(
                        CODE_WRITE_VALIDATION,
                        "invalid encoded property entry",
                        JsonValue::Null,
                    )
                })?;
                let k = eobj.get("key").and_then(JsonValue::as_str).ok_or_else(|| {
                    typed_error(
                        CODE_WRITE_VALIDATION,
                        "property key must be string",
                        JsonValue::Null,
                    )
                })?;
                validate_ffi_string_napi(k)?;
                if k.is_empty() || k.contains('\x1e') {
                    return Err(typed_error(
                        CODE_WRITE_VALIDATION,
                        "property key must be non-empty and contain no record separator bytes",
                        JsonValue::Null,
                    ));
                }
                let value_type =
                    eobj.get("valueType").and_then(JsonValue::as_str).ok_or_else(|| {
                        typed_error(
                            CODE_WRITE_VALIDATION,
                            "property valueType missing",
                            JsonValue::Null,
                        )
                    })?;
                let prop_val = match value_type {
                    "string" => {
                        let s =
                            eobj.get("stringVal").and_then(JsonValue::as_str).ok_or_else(|| {
                                typed_error(
                                    CODE_WRITE_VALIDATION,
                                    "string property requires stringVal",
                                    JsonValue::Null,
                                )
                            })?;
                        validate_ffi_string_napi(s)?;
                        if s.contains('\x1e') {
                            return Err(typed_error(
                                CODE_WRITE_VALIDATION,
                                "property string value must not contain record separator bytes",
                                JsonValue::Null,
                            ));
                        }
                        RustPropertyValue::String(s.to_string())
                    }
                    "int" => {
                        let text =
                            eobj.get("intVal").and_then(JsonValue::as_str).ok_or_else(|| {
                                typed_error(
                                    CODE_WRITE_VALIDATION,
                                    "int property requires intVal",
                                    JsonValue::Null,
                                )
                            })?;
                        let i = text.parse::<i64>().map_err(|_| {
                            typed_error(
                                CODE_WRITE_VALIDATION,
                                "integer property value out of signed 64-bit range",
                                JsonValue::Null,
                            )
                        })?;
                        RustPropertyValue::Int(i)
                    }
                    "float" => {
                        let f =
                            eobj.get("floatVal").and_then(JsonValue::as_f64).ok_or_else(|| {
                                typed_error(
                                    CODE_WRITE_VALIDATION,
                                    "float property requires finite floatVal",
                                    JsonValue::Null,
                                )
                            })?;
                        if !f.is_finite() {
                            return Err(typed_error(
                                CODE_WRITE_VALIDATION,
                                "float property value must be finite",
                                JsonValue::Null,
                            ));
                        }
                        RustPropertyValue::Float(if f == 0.0 { 0.0 } else { f })
                    }
                    "bool" => {
                        let b =
                            eobj.get("boolVal").and_then(JsonValue::as_bool).ok_or_else(|| {
                                typed_error(
                                    CODE_WRITE_VALIDATION,
                                    "bool property requires boolVal",
                                    JsonValue::Null,
                                )
                            })?;
                        RustPropertyValue::Bool(b)
                    }
                    _ => {
                        return Err(typed_error(
                            CODE_WRITE_VALIDATION,
                            "unknown property valueType",
                            JsonValue::Null,
                        ));
                    }
                };
                map.insert(k.to_string(), prop_val);
            }
            return Ok(if map.is_empty() { None } else { Some(map) });
        }
    }
    for (k, v) in obj {
        validate_ffi_string_napi(k)?;
        if k.is_empty() || k.contains('\x1e') {
            return Err(typed_error(
                CODE_WRITE_VALIDATION,
                "property key must be non-empty and contain no record separator bytes",
                JsonValue::Null,
            ));
        }
        let prop_val = match v {
            JsonValue::String(s) => {
                validate_ffi_string_napi(s)?;
                if s.contains('\x1e') {
                    return Err(typed_error(
                        CODE_WRITE_VALIDATION,
                        "property string value must not contain record separator bytes",
                        JsonValue::Null,
                    ));
                }
                RustPropertyValue::String(s.clone())
            }
            JsonValue::Bool(b) => RustPropertyValue::Bool(*b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    RustPropertyValue::Int(i)
                } else if let Some(f) = n.as_f64() {
                    if !f.is_finite() {
                        return Err(typed_error(
                            CODE_WRITE_VALIDATION,
                            "float property value must be finite",
                            JsonValue::Null,
                        ));
                    }
                    RustPropertyValue::Float(if f == 0.0 { 0.0 } else { f })
                } else {
                    return Err(typed_error(
                        CODE_WRITE_VALIDATION,
                        "numeric property value out of supported range",
                        JsonValue::Null,
                    ));
                }
            }
            JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => {
                return Err(typed_error(
                    CODE_WRITE_VALIDATION,
                    "property value must be string, number, bigint, or boolean",
                    JsonValue::Null,
                ));
            }
        };
        map.insert(k.clone(), prop_val);
    }
    Ok(if map.is_empty() { None } else { Some(map) })
}

pub(crate) fn translate_node_at(
    item: &JsonValue,
    source_id_field_path: Option<&str>,
) -> Result<PreparedWrite> {
    let kind = json_str_required(item, "kind")?;
    let provenance = json_get(item, "provenance").map(translate_json_provenance).transpose()?;
    let body = if provenance.is_some() {
        json_str(item, "body")?.unwrap_or_else(|| "{}".to_string())
    } else {
        json_serialised(item, "body")?.unwrap_or_else(|| "{}".to_string())
    };
    let source_id = json_source_id_required_at(item, "node", source_id_field_path)?;
    let logical_id = json_str_alt(item, "logicalId", "logical_id")?;
    // OPP-12 Phase-1 (0.8.19 Slice 5) — create-time existence state + advisory
    // reason (X1 parity with the pyo3 binding). `state` defaults to `active`; an
    // out-of-subset value (`deleted`/`purged`/unknown) is a TYPED write-validation
    // rejection — you cannot CREATE a deleted/purged node. Thin pass-through.
    let state = match json_str_alt(item, "state", "state")? {
        Some(s) => InitialState::from_create_str(&s).ok_or_else(|| {
            typed_error(
                CODE_WRITE_VALIDATION,
                format!(
                    "cannot create a node with state {s:?}: only \"pending\" or \"active\" are creatable (deleted/purged require transition/purge)"
                ),
                JsonValue::Null,
            )
        })?,
        None => InitialState::Active,
    };
    let reason = json_str_alt(item, "reason", "reason")?;
    // 0.8.20 Slice 15b (TC-34) — world-time validity window (X1 parity with the
    // pyo3 binding). INTEGER epoch seconds; absent or `null` means unbounded on
    // that side, which lands NULL and reproduces pre-slice behaviour exactly.
    // Both spellings are accepted, exactly as `tValid`/`t_valid` are on edges.
    // The half-open pair is validated in the ENGINE (`validate_write`), so Rust,
    // Python and TypeScript share one rule and cannot drift.
    let valid_from = json_i64_alt(item, "validFrom", "valid_from")?;
    let valid_until = json_i64_alt(item, "validUntil", "valid_until")?;
    let properties = json_property_map(item, "properties")?;
    match provenance {
        Some(provenance) => Ok(PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            kind,
            body,
            source_id,
            logical_id,
            state,
            reason,
            valid_from,
            valid_until,
            properties,
            provenance,
        })),
        None => Ok(PreparedWrite::Node {
            kind,
            body,
            source_id,
            logical_id,
            state,
            reason,
            valid_from,
            valid_until,
            properties,
        }),
    }
}

pub(crate) fn translate_node(item: &JsonValue) -> Result<PreparedWrite> {
    translate_node_at(item, None)
}

/// 0.8.20 Slice 15b (TC-34) — read an optional INTEGER epoch-second field.
///
/// JavaScript has ONE number type, so `10.5` and `true` both arrive where an
/// integer was meant. Both are refused with a typed write-validation error
/// rather than truncated or coerced: a silently truncated instant is a wrong
/// answer that only surfaces at the window boundary. `serde_json`'s `as_i64`
/// returns `None` for any non-integral number, which is exactly the test wanted.
pub(crate) fn json_i64(v: &JsonValue, key: &str) -> Result<Option<i64>> {
    match json_get(v, key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::Number(n)) => n.as_i64().map(Some).ok_or_else(|| {
            typed_error(
                CODE_WRITE_VALIDATION,
                format!("field {key:?} must be an integer (epoch seconds), not {n}"),
                JsonValue::Null,
            )
        }),
        Some(_other) => Err(typed_error(
            CODE_WRITE_VALIDATION,
            format!("field {key:?} must be an integer (epoch seconds) or null"),
            JsonValue::Null,
        )),
    }
}

/// The `json_str_alt` analogue for integers: accept the camelCase spelling
/// first, then the snake_case one, so a caller porting from the Python stub
/// keeps working. See [`json_str_alt`].
pub(crate) fn json_i64_alt(item: &JsonValue, camel: &str, snake: &str) -> Result<Option<i64>> {
    if let Some(v) = json_i64(item, camel)? {
        return Ok(Some(v));
    }
    json_i64(item, snake)
}

pub(crate) fn translate_edge(item: &JsonValue) -> Result<PreparedWrite> {
    let kind = json_str_required(item, "kind")?;
    let from = json_str_required(item, "from")?;
    let to = json_str_required(item, "to")?;
    let source_id = json_source_id_required(item, "edge")?;
    let logical_id = json_str_alt(item, "logicalId", "logical_id")?;
    // Edge body (the relation text) — optional. Projected into `search_index_edges`
    // so the C1 graph arm can seed from edge-fact FTS (`source A`). NULL = not indexed.
    let provenance = json_get(item, "provenance").map(translate_json_provenance).transpose()?;
    let body =
        if provenance.is_some() { json_str(item, "body")? } else { json_serialised(item, "body")? };
    // R3 (Slice 30) — temporal validity fields accepted from user-facing write API.
    //
    // TC-33 (HITL-RATIFIED 2026-07-21) — `tValid`/`t_valid` and
    // `tInvalid`/`t_invalid` are **INTEGER epoch seconds (UTC)**, not ISO-8601
    // strings. This is the GOVERNED SDK WRITE SURFACE, which carries the same
    // representation as storage; ISO-8601 survives ONLY on the BYO-LLM extractor
    // wire, where the engine normalises it with hard rejection. Reuses the same
    // `json_i64_alt` helper as the node `validFrom`/`validUntil` window, so both
    // temporal axes validate identically — and unlike the old `json_str_alt`
    // (which did NO format validation at all) a wrong-typed value is rejected.
    //
    // `None` = "still valid"; that semantic is load-bearing and unchanged.
    let t_valid = json_i64_alt(item, "tValid", "t_valid")?;
    let t_invalid = json_i64_alt(item, "tInvalid", "t_invalid")?;
    let properties = json_property_map(item, "properties")?;
    match provenance {
        Some(provenance) => Ok(PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            kind,
            from,
            to,
            source_id,
            logical_id,
            body,
            t_valid,
            t_invalid,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties,
            provenance,
        })),
        None => Ok(PreparedWrite::Edge {
            kind,
            from,
            to,
            source_id,
            logical_id,
            body,
            t_valid,
            t_invalid,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties,
        }),
    }
}

pub(crate) fn translate_op_store(item: &JsonValue) -> Result<PreparedWrite> {
    let collection = json_str_required(item, "collection")?;
    let record_key = json_str_alt_required(item, "recordKey", "record_key")?;
    let schema_id = json_str_alt(item, "schemaId", "schema_id")?;
    let body = json_serialised_required(item, "body")?;
    Ok(PreparedWrite::OpStore { collection, record_key, schema_id, body })
}

pub(crate) fn translate_admin_schema(item: &JsonValue) -> Result<PreparedWrite> {
    let name = json_str_required(item, "name")?;
    let kind = json_str_required(item, "kind")?;
    let schema_json = json_serialised_alt_required(item, "schemaJson", "schema_json")?;
    let retention_json = json_serialised_alt(item, "retentionJson", "retention_json")?
        .unwrap_or_else(|| "{}".to_string());
    Ok(PreparedWrite::AdminSchema { name, kind, schema_json, retention_json })
}

// ===== Data classes ===================================================

#[napi(object)]
pub struct WriteOptions {
    pub endpoint_validation: Option<String>,
}

impl WriteOptions {
    pub(crate) fn into_rust(self) -> Result<fathomdb_engine::WriteOptions> {
        match self.endpoint_validation.as_deref() {
            None | Some("strict") => Ok(fathomdb_engine::WriteOptions::strict()),
            Some("bulk_non_strict") => Ok(fathomdb_engine::WriteOptions::bulk_non_strict()),
            Some(other) => Err(typed_error(
                CODE_INVALID_ARGUMENT,
                format!(
                    "writeOptions.endpointValidation={other:?}: expected strict or bulk_non_strict"
                ),
                JsonValue::Null,
            )),
        }
    }
}

#[napi(object)]
pub struct WriteReceipt {
    pub cursor: i64,
    /// G0 (Slice 15) — per-row `write_cursor`s, 1:1 with the input batch order
    /// (surfaced as `rowCursors`). Each `u64` is narrowed to `i64` at the FFI
    /// boundary, matching the existing `cursor` cast.
    pub row_cursors: Vec<i64>,
    /// Count of unresolved edge endpoints across surviving active edges written
    /// by this batch (surfaced as `danglingEdgeEndpoints`). Always `0` under
    /// strict mode (`write` and default `writeWithOptions`); exact `0`/`1`/`2`
    /// per surviving active edge under opt-in `bulk_non_strict` mode. Narrowed
    /// `u64 → i64` at the FFI boundary, matching the `cursor`/`rowCursors`
    /// precedent.
    pub dangling_edge_endpoints: i64,
}

impl WriteReceipt {
    pub(crate) fn from_rust(r: RustWriteReceipt) -> Self {
        Self {
            cursor: r.cursor as i64,
            row_cursors: r.row_cursors.into_iter().map(|c| c as i64).collect(),
            dangling_edge_endpoints: r.dangling_edge_endpoints as i64,
        }
    }
}

#[napi(object)]
pub struct SourceDependencyV1 {
    pub schema_version: u32,
    pub dependency_id: String,
    pub source_revision_id: String,
    pub derived_revision_id: String,
    pub registered_dependency_generation: String,
}

impl From<RustSourceDependencyV1> for SourceDependencyV1 {
    fn from(value: RustSourceDependencyV1) -> Self {
        Self {
            schema_version: value.schema_version,
            dependency_id: value.dependency_id.as_str().to_string(),
            source_revision_id: value.source_revision_id.as_str().to_string(),
            derived_revision_id: value.derived_revision_id.as_str().to_string(),
            registered_dependency_generation: value.registered_dependency_generation.to_string(),
        }
    }
}

#[napi(object)]
pub struct ActuationReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub request_sha256: String,
    pub outcome: String,
    pub refused_operation_index: Option<u32>,
    pub refused_field_path: Option<String>,
    pub reason_codes: Vec<String>,
    pub affected_revision_ids: Vec<String>,
    pub resulting_write_boundary: Option<String>,
    pub resulting_dependency_generation: Option<String>,
    pub pending_projection_write_cursors: Vec<String>,
    pub projection_generation_id: Option<String>,
    pub closure_operation_ids: Vec<String>,
}

impl TryFrom<RustActuationReceiptV1> for ActuationReceiptV1 {
    type Error = Error;

    fn try_from(value: RustActuationReceiptV1) -> Result<Self> {
        let outcome = match value.outcome {
            ActuationOutcomeV1::Committed => "committed",
            ActuationOutcomeV1::CommittedClosurePending => "committed_closure_pending",
            ActuationOutcomeV1::Refused => "refused",
        }
        .to_string();
        Ok(Self {
            schema_version: value.schema_version,
            operation_id: value.operation_id,
            request_sha256: value.request_sha256,
            outcome,
            refused_operation_index: value
                .refused_operation_index
                .map(|item| {
                    u32::try_from(item).map_err(|_| {
                        typed_error(CODE_STORAGE, "actuation index overflow", JsonValue::Null)
                    })
                })
                .transpose()?,
            refused_field_path: value.refused_field_path,
            reason_codes: value
                .reason_codes
                .into_iter()
                .map(|reason| reason.as_str().to_string())
                .collect(),
            affected_revision_ids: value.affected_revision_ids,
            resulting_write_boundary: value.resulting_write_boundary.map(|item| item.to_string()),
            resulting_dependency_generation: value
                .resulting_dependency_generation
                .map(|item| item.to_string()),
            pending_projection_write_cursors: value
                .pending_projection_write_cursors
                .into_iter()
                .map(|item| item.to_string())
                .collect(),
            projection_generation_id: value
                .projection_generation_id
                .map(|item| item.as_str().to_string()),
            closure_operation_ids: value.closure_operation_ids,
        })
    }
}

/// G11 (Slice 15) — BYO-LLM ingest receipt.
#[napi(object)]
pub struct IngestWithExtractorReceipt {
    pub nodes_written: i64,
    pub edges_written: i64,
    pub docs_processed: i64,
}

impl IngestWithExtractorReceipt {
    pub(crate) fn from_rust(r: RustIngestWithExtractorReceipt) -> Self {
        Self {
            nodes_written: r.nodes_written as i64,
            edges_written: r.edges_written as i64,
            docs_processed: r.docs_processed as i64,
        }
    }
}

/// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation receipt.
#[napi(object)]
pub struct ConsolidateReceipt {
    pub clusters_processed: i64,
    pub edges_examined: i64,
    pub edges_kept: i64,
    pub edges_invalidated: i64,
    pub edges_superseded: i64,
}

impl ConsolidateReceipt {
    pub(crate) fn from_rust(r: RustConsolidateReceipt) -> Self {
        Self {
            clusters_processed: r.clusters_processed as i64,
            edges_examined: r.edges_examined as i64,
            edges_kept: r.edges_kept as i64,
            edges_invalidated: r.edges_invalidated as i64,
            edges_superseded: r.edges_superseded as i64,
        }
    }
}

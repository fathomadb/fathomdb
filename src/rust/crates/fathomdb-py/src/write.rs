use super::*;

// ===== Batch translation ==============================================

pub(super) fn translate_batch(batch: &Bound<'_, PyList>) -> PyResult<Vec<PreparedWrite>> {
    let mut out = Vec::with_capacity(batch.len());
    for item in batch.iter() {
        out.push(translate_write_item(&item)?);
    }
    Ok(out)
}

pub(super) fn dict_get<'py>(
    d: &Bound<'py, PyDict>,
    key: &str,
) -> PyResult<Option<Bound<'py, PyAny>>> {
    d.get_item(key)
}

pub(super) fn dict_str(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<String>> {
    match dict_get(d, key)? {
        Some(v) if !v.is_none() => Ok(Some(extract_validated_str(&v)?)),
        _ => Ok(None),
    }
}

pub(super) fn dict_str_required(d: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    dict_str(d, key)?.ok_or_else(|| {
        WriteValidationError::new_err(format!("write item missing required field {key:?}"))
    })
}

/// 0.8.20 Slice 5c (R-20-E3) — `source_id` is now MANDATORY on every canonical
/// write. Rust makes its absence inexpressible via the `SourceId` newtype;
/// Python has no such type system at the boundary, so the binding raises
/// `WriteValidationError` for a missing, empty or reserved (`_`-prefixed) id.
/// This is the Python arm of "an un-provenanced write does not compile / raises".
///
/// The rationale is not tidiness: `excise_source` addresses rows BY `source_id`,
/// so a row written without one is reachable by no erasure call — un-erasable.
pub(super) fn dict_source_id_required(d: &Bound<'_, PyDict>, kind: &str) -> PyResult<SourceId> {
    let raw = match dict_get(d, "source_id")? {
        Some(value) if !value.is_none() => value.extract::<String>().map_err(|_| {
            WriteValidationError::new_err(
                "source_id contains characters not representable as UTF-8 (lone surrogate)",
            )
        })?,
        _ => {
            return Err(WriteValidationError::new_err(format!(
                "{kind} write item missing required field \"source_id\": provenance is mandatory \
                 since 0.8.20 — a row written without it can never be erased by excise_source"
            )))
        }
    };
    SourceId::new(raw).map_err(|_| {
        WriteValidationError::new_err(
            "\"source_id\" must be a non-empty identifier outside the engine's reserved \
             \"_\"-prefixed namespace",
        )
    })
}

pub(super) fn snake_to_camel(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut uppercase = false;
    for character in value.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

pub(super) fn provenance_input_error(reason: &str, field_path: impl Into<String>) -> PyErr {
    let field_path = field_path.into();
    let exc = ProvenanceError::new_err(format!("provenance {reason} at {field_path}"));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exc
}

pub(super) fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

pub(super) fn dependency_input_error(reason: &str, field_path: impl Into<String>) -> PyErr {
    let field_path = field_path.into();
    let exc = DependencyError::new_err(format!("dependency {reason} at {field_path}"));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exc
}

pub(super) fn dependency_closure_input_error(reason: &str, field_path: impl Into<String>) -> PyErr {
    let field_path = field_path.into();
    let exc =
        DependencyClosureError::new_err(format!("dependency closure {reason} at {field_path}"));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exc
}

pub(super) fn actuation_input_error(reason: &str, field_path: impl Into<String>) -> PyErr {
    let field_path = field_path.into();
    let exc = ActuationError::new_err(format!("actuation {reason} at {field_path}"));
    Python::attach(|py| {
        let value = exc.value(py);
        let _ = value.setattr("reason", reason);
        let _ = value.setattr("field_path", field_path);
    });
    exc
}

pub(super) fn nested_actuation_input_error(error: PyErr, root: &str) -> PyErr {
    let nested_path = Python::attach(|py| {
        error.value(py).getattr("field_path").and_then(|value| value.extract::<String>()).ok()
    });
    let field_path = nested_path
        .filter(|path| path.starts_with('/'))
        .map_or_else(|| root.to_string(), |path| format!("{root}{path}"));
    actuation_input_error("nested_request_invalid", field_path)
}

pub(super) fn strict_actuation_dict<'py>(
    value: &'py Bound<'py, PyAny>,
    allowed: &[&str],
    base_path: &str,
) -> PyResult<&'py Bound<'py, PyDict>> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| actuation_input_error("field_type_invalid", base_path))?;
    let mut unknown = None;
    for key in dict.keys().iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| actuation_input_error("unknown_field", base_path))?;
        if !allowed.contains(&key.as_str()) {
            let canonical = snake_to_camel(&key);
            if unknown.as_ref().is_none_or(|current| canonical < *current) {
                unknown = Some(canonical);
            }
        }
    }
    if let Some(key) = unknown {
        return Err(actuation_input_error(
            "unknown_field",
            format!("{base_path}/{}", escape_json_pointer_token(&key)),
        ));
    }
    Ok(dict)
}

pub(super) fn actuation_required_string(
    dict: &Bound<'_, PyDict>,
    key: &str,
    path: &str,
) -> PyResult<String> {
    match dict_get(dict, key)? {
        Some(value) if !value.is_none() => extract_validated_str(&value)
            .map_err(|_| actuation_input_error("field_type_invalid", path)),
        _ => Err(actuation_input_error("field_missing", path)),
    }
}

pub(super) fn valid_actuation_caller_identity(value: &str) -> bool {
    ArtifactRevisionId::new(value).is_ok()
}

pub(super) fn translate_actuation_request(value: &Bound<'_, PyAny>) -> PyResult<ActuationBatchV1> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| actuation_input_error("unsupported_schema_version", "/schemaVersion"))?;
    let schema = match dict_get(dict, "schema_version")? {
        Some(value) if !value.is_instance_of::<pyo3::types::PyBool>() => {
            value.extract::<u32>().ok()
        }
        _ => None,
    };
    if schema != Some(1) {
        return Err(actuation_input_error("unsupported_schema_version", "/schemaVersion"));
    }
    let dict = strict_actuation_dict(
        value,
        &[
            "schema_version",
            "operation_id",
            "decision_policy_id",
            "expected_write_boundary",
            "operations",
        ],
        "",
    )?;
    let operation_id = actuation_required_string(dict, "operation_id", "/operationId")?;
    let decision_policy_id = match dict_get(dict, "decision_policy_id")? {
        None => None,
        Some(value) if value.is_none() => None,
        Some(value) => Some(
            extract_validated_str(&value)
                .map_err(|_| actuation_input_error("field_type_invalid", "/decisionPolicyId"))?,
        ),
    };
    let expected_write_boundary = match dict_get(dict, "expected_write_boundary")? {
        None => None,
        Some(value) if value.is_none() => None,
        Some(value) => {
            if value.is_instance_of::<pyo3::types::PyBool>() {
                return Err(actuation_input_error("field_type_invalid", "/expectedWriteBoundary"));
            }
            let text = extract_validated_str(&value).map_err(|_| {
                actuation_input_error("field_type_invalid", "/expectedWriteBoundary")
            })?;
            Some(text.parse::<u64>().ok().filter(|parsed| parsed.to_string() == text).ok_or_else(
                || actuation_input_error("field_type_invalid", "/expectedWriteBoundary"),
            )?)
        }
    };
    let operations_value = dict_get(dict, "operations")?
        .ok_or_else(|| actuation_input_error("field_missing", "/operations"))?;
    let operations_list = operations_value
        .cast::<PyList>()
        .map_err(|_| actuation_input_error("field_type_invalid", "/operations"))?;
    if !valid_actuation_caller_identity(&operation_id) {
        return Err(actuation_input_error("operation_id_invalid", "/operationId"));
    }
    if decision_policy_id.as_deref().is_some_and(|value| !valid_actuation_caller_identity(value)) {
        return Err(actuation_input_error("decision_policy_id_invalid", "/decisionPolicyId"));
    }
    if !(1..=128).contains(&operations_list.len()) {
        return Err(actuation_input_error("operation_count_invalid", "/operations"));
    }
    let mut operations = Vec::with_capacity(operations_list.len());
    for (index, operation) in operations_list.iter().enumerate() {
        operations.push(translate_actuation_operation(&operation, index)?);
    }
    let mut request = ActuationBatchV1::new(operation_id, operations)
        .map_err(|error| actuation_error_to_py(&error))?;
    if let Some(policy) = decision_policy_id {
        request = request
            .with_decision_policy_id(policy)
            .map_err(|error| actuation_error_to_py(&error))?;
    }
    if let Some(parsed) = expected_write_boundary {
        request = request.with_expected_write_boundary(parsed);
    }
    Ok(request)
}

pub(super) fn translate_actuation_operation(
    value: &Bound<'_, PyAny>,
    index: usize,
) -> PyResult<ActuationOperationV1> {
    let root = format!("/operations/{index}");
    let dict =
        value.cast::<PyDict>().map_err(|_| actuation_input_error("field_type_invalid", &root))?;
    let kind = actuation_required_string(dict, "type", &format!("{root}/type"))?;
    match kind.as_str() {
        "put_canonical_node" | "put_derived_node" => {
            strict_actuation_dict(value, &["type", "record"], &root)?;
            let record = dict_get(dict, "record")?
                .ok_or_else(|| actuation_input_error("field_missing", format!("{root}/record")))?;
            strict_actuation_dict(
                &record,
                &[
                    "kind",
                    "body",
                    "source_id",
                    "logical_id",
                    "state",
                    "reason",
                    "valid_from",
                    "valid_until",
                    "provenance",
                ],
                &format!("{root}/record"),
            )?;
            let record_root = format!("{root}/record");
            let prepared = translate_node(&record)
                .map_err(|error| nested_actuation_input_error(error, &record_root))?;
            let PreparedWrite::ProvenancedNode(node) = prepared else {
                return Err(actuation_input_error(
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
            strict_actuation_dict(value, &["type", "record"], &root)?;
            let record = dict_get(dict, "record")?
                .ok_or_else(|| actuation_input_error("field_missing", format!("{root}/record")))?;
            strict_actuation_dict(
                &record,
                &[
                    "kind",
                    "from",
                    "to",
                    "source_id",
                    "logical_id",
                    "body",
                    "t_valid",
                    "t_invalid",
                    "provenance",
                ],
                &format!("{root}/record"),
            )?;
            let record_root = format!("{root}/record");
            let prepared = translate_edge(&record)
                .map_err(|error| nested_actuation_input_error(error, &record_root))?;
            let PreparedWrite::ProvenancedEdge(edge) = prepared else {
                return Err(actuation_input_error(
                    "nested_request_invalid",
                    format!("{root}/record/provenance"),
                ));
            };
            Ok(ActuationOperationV1::PutDerivedEdge(edge))
        }
        "register_source_dependency" => {
            strict_actuation_dict(value, &["type", "dependency"], &root)?;
            let dependency = dict_get(dict, "dependency")?.ok_or_else(|| {
                actuation_input_error("field_missing", format!("{root}/dependency"))
            })?;
            let dependency_root = format!("{root}/dependency");
            translate_dependency_registration(&dependency)
                .map(ActuationOperationV1::RegisterSourceDependency)
                .map_err(|error| nested_actuation_input_error(error, &dependency_root))
        }
        "transition_lifecycle" => {
            strict_actuation_dict(
                value,
                &["type", "logical_id", "expected_current_revision_id", "to_state", "reason"],
                &root,
            )?;
            let logical_id =
                actuation_required_string(dict, "logical_id", &format!("{root}/logicalId"))?;
            let revision = actuation_required_string(
                dict,
                "expected_current_revision_id",
                &format!("{root}/expectedCurrentRevisionId"),
            )?;
            let revision = ArtifactRevisionId::new(revision).map_err(|_| {
                actuation_input_error(
                    "revision_id_invalid",
                    format!("{root}/expectedCurrentRevisionId"),
                )
            })?;
            let target = actuation_required_string(dict, "to_state", &format!("{root}/toState"))?;
            let target = RustLifecycleState::from_str_opt(&target).ok_or_else(|| {
                actuation_input_error("lifecycle_target_invalid", format!("{root}/toState"))
            })?;
            let reason = match dict_get(dict, "reason")? {
                Some(value) if !value.is_none() => {
                    Some(extract_validated_str(&value).map_err(|_| {
                        actuation_input_error("field_type_invalid", format!("{root}/reason"))
                    })?)
                }
                _ => None,
            };
            LifecycleActuationV1::new(logical_id, revision, target, reason)
                .map(ActuationOperationV1::TransitionLifecycle)
                .map_err(|error| {
                    actuation_input_error(
                        error.reason.as_str(),
                        format!("{root}{}", error.field_path),
                    )
                })
        }
        _ => Err(actuation_input_error("unknown_operation_variant", format!("{root}/type"))),
    }
}

pub(super) fn dependency_request_dict<'py>(
    value: &'py Bound<'py, PyAny>,
    allowed: &[&str],
) -> PyResult<&'py Bound<'py, PyDict>> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| dependency_input_error("unsupported_schema_version", "/schemaVersion"))?;
    let schema = match dict_get(dict, "schema_version")? {
        Some(value) if !value.is_instance_of::<pyo3::types::PyBool>() => {
            value.extract::<u32>().ok()
        }
        _ => None,
    };
    if schema != Some(1) {
        return Err(dependency_input_error("unsupported_schema_version", "/schemaVersion"));
    }
    let mut unknown = None;
    for key in dict.keys().iter() {
        let key =
            key.extract::<String>().map_err(|_| dependency_input_error("unknown_field", ""))?;
        if !allowed.contains(&key.as_str()) {
            let canonical = snake_to_camel(&key);
            if unknown.as_ref().is_none_or(|current| canonical < *current) {
                unknown = Some(canonical);
            }
        }
    }
    if let Some(key) = unknown {
        return Err(dependency_input_error(
            "unknown_field",
            format!("/{}", escape_json_pointer_token(&key)),
        ));
    }
    Ok(dict)
}

pub(super) fn translate_closure_lookup(value: &Bound<'_, PyAny>) -> PyResult<ClosureLookupV1> {
    let dict = value.cast::<PyDict>().map_err(|_| {
        dependency_closure_input_error("unsupported_schema_version", "/schemaVersion")
    })?;
    let schema = match dict_get(dict, "schema_version")? {
        Some(value) if !value.is_instance_of::<pyo3::types::PyBool>() => {
            value.extract::<u32>().ok()
        }
        _ => None,
    };
    if schema != Some(1) {
        return Err(dependency_closure_input_error("unsupported_schema_version", "/schemaVersion"));
    }
    let mut unknown = None;
    for key in dict.keys().iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| dependency_closure_input_error("unknown_field", ""))?;
        if !["schema_version", "closure_operation_id"].contains(&key.as_str()) {
            let canonical = snake_to_camel(&key);
            if unknown.as_ref().is_none_or(|current| canonical < *current) {
                unknown = Some(canonical);
            }
        }
    }
    if let Some(key) = unknown {
        return Err(dependency_closure_input_error(
            "unknown_field",
            format!("/{}", escape_json_pointer_token(&key)),
        ));
    }
    let id = match dict_get(dict, "closure_operation_id")? {
        Some(value) if !value.is_none() => extract_validated_str(&value).map_err(|_| {
            dependency_closure_input_error("closure_operation_id_invalid", "/closureOperationId")
        })?,
        _ => {
            return Err(dependency_closure_input_error(
                "closure_operation_id_invalid",
                "/closureOperationId",
            ))
        }
    };
    ClosureLookupV1::new(id).map_err(|error| dependency_closure_error_to_py(&error))
}

pub(super) fn dependency_required_string(
    dict: &Bound<'_, PyDict>,
    key: &str,
    reason: &str,
) -> PyResult<String> {
    let path = format!("/{}", snake_to_camel(key));
    match dict_get(dict, key)? {
        Some(value) if !value.is_none() => {
            extract_validated_str(&value).map_err(|_| dependency_input_error(reason, path))
        }
        _ => Err(dependency_input_error(reason, path)),
    }
}

pub(super) fn translate_dependency_registration(
    value: &Bound<'_, PyAny>,
) -> PyResult<SourceDependencyRegistrationV1> {
    let dict = dependency_request_dict(
        value,
        &["schema_version", "dependency_id", "source_revision_id", "derived_revision_id"],
    )?;
    let dependency_id = dependency_required_string(dict, "dependency_id", "dependency_id_invalid")?;
    let source_revision_id =
        dependency_required_string(dict, "source_revision_id", "dependency_reference_invalid")?;
    let derived_revision_id =
        dependency_required_string(dict, "derived_revision_id", "dependency_reference_invalid")?;
    SourceDependencyRegistrationV1::new(dependency_id, source_revision_id, derived_revision_id)
        .map_err(|error| dependency_error_to_py(&error))
}

pub(super) fn translate_dependency_source_lookup(
    value: &Bound<'_, PyAny>,
) -> PyResult<DependencySourceLookupV1> {
    let dict = dependency_request_dict(value, &["schema_version", "source_revision_id"])?;
    DependencySourceLookupV1::new(dependency_required_string(
        dict,
        "source_revision_id",
        "dependency_reference_invalid",
    )?)
    .map_err(|error| dependency_error_to_py(&error))
}

pub(super) fn translate_dependency_derived_lookup(
    value: &Bound<'_, PyAny>,
) -> PyResult<DependencyDerivedLookupV1> {
    let dict = dependency_request_dict(value, &["schema_version", "derived_revision_id"])?;
    DependencyDerivedLookupV1::new(dependency_required_string(
        dict,
        "derived_revision_id",
        "dependency_reference_invalid",
    )?)
    .map_err(|error| dependency_error_to_py(&error))
}

pub(super) fn provenance_required_str(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<String> {
    let reason =
        if key == "source_version_id" { "source_version_invalid" } else { "revision_id_invalid" };
    match dict_get(dict, key)? {
        Some(value) if !value.is_none() => extract_validated_str(&value).map_err(|_| {
            provenance_input_error(reason, format!("/provenance/{}", snake_to_camel(key)))
        }),
        _ => Err(provenance_input_error(reason, format!("/provenance/{}", snake_to_camel(key)))),
    }
}

pub(super) fn strict_provenance_keys(
    dict: &Bound<'_, PyDict>,
    allowed: &[&str],
    field_path: &str,
) -> PyResult<()> {
    let mut first_unknown: Option<String> = None;
    for key in dict.keys().iter() {
        let key = key
            .extract::<String>()
            .map_err(|_| provenance_input_error("unknown_field", "/provenance"))?;
        if !allowed.contains(&key.as_str()) {
            let canonical_key = snake_to_camel(&key);
            if first_unknown.as_ref().is_none_or(|current| canonical_key < *current) {
                first_unknown = Some(canonical_key);
            }
        }
    }
    if let Some(key) = first_unknown {
        return Err(provenance_input_error(
            "unknown_field",
            format!("{field_path}/{}", escape_json_pointer_token(&key)),
        ));
    }
    Ok(())
}

pub(super) fn decimal_offset(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<u64> {
    let value = provenance_required_str(dict, key).map_err(|_| {
        provenance_input_error(
            "locator_invalid",
            format!("/provenance/sourceLocator/{}", snake_to_camel(key)),
        )
    })?;
    let valid = value == "0"
        || (!value.starts_with('0') && value.bytes().all(|byte| byte.is_ascii_digit()));
    let parsed = value.parse::<u64>().ok().filter(|offset| *offset <= i64::MAX as u64);
    if !valid || parsed.is_none() {
        return Err(provenance_input_error(
            "locator_invalid",
            format!("/provenance/sourceLocator/{}", snake_to_camel(key)),
        ));
    }
    Ok(parsed.unwrap())
}

pub(super) fn translate_source_locator(value: &Bound<'_, PyAny>) -> PyResult<SourceLocator> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| provenance_input_error("locator_invalid", "/provenance/sourceLocator"))?;
    let kind = match dict_get(dict, "kind")? {
        Some(value) => extract_validated_str(&value).map_err(|_| {
            provenance_input_error("locator_invalid", "/provenance/sourceLocator/kind")
        })?,
        None => {
            return Err(provenance_input_error("locator_invalid", "/provenance/sourceLocator/kind"))
        }
    };
    match kind.as_str() {
        "whole_body" => {
            strict_provenance_keys(dict, &["kind"], "/provenance/sourceLocator")?;
            Ok(SourceLocator::whole_body())
        }
        "utf8_bytes" => {
            strict_provenance_keys(
                dict,
                &["kind", "start_inclusive", "end_exclusive"],
                "/provenance/sourceLocator",
            )?;
            Ok(SourceLocator::utf8_bytes(
                decimal_offset(dict, "start_inclusive")?,
                decimal_offset(dict, "end_exclusive")?,
            ))
        }
        _ => Err(provenance_input_error("locator_invalid", "/provenance/sourceLocator/kind")),
    }
}

pub(super) fn translate_canonical_hash(value: &Bound<'_, PyAny>) -> PyResult<CanonicalHash> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| provenance_input_error("hash_invalid", "/provenance/canonicalSourceHash"))?;
    strict_provenance_keys(dict, &["algorithm", "digest_hex"], "/provenance/canonicalSourceHash")?;
    let algorithm = provenance_required_str(dict, "algorithm").map_err(|_| {
        provenance_input_error("hash_invalid", "/provenance/canonicalSourceHash/algorithm")
    })?;
    if algorithm != "sha256" {
        return Err(provenance_input_error(
            "hash_invalid",
            "/provenance/canonicalSourceHash/algorithm",
        ));
    }
    let digest = provenance_required_str(dict, "digest_hex").map_err(|_| {
        provenance_input_error("hash_invalid", "/provenance/canonicalSourceHash/digestHex")
    })?;
    CanonicalHash::sha256(digest).map_err(engine_error_to_py)
}

pub(super) fn translate_provenance(value: &Bound<'_, PyAny>) -> PyResult<WriteProvenanceV1> {
    let dict = value
        .cast::<PyDict>()
        .map_err(|_| provenance_input_error("role_invalid", "/provenance"))?;
    let schema_version = match dict_get(dict, "schema_version")? {
        Some(value) if !value.is_instance_of::<pyo3::types::PyBool>() => {
            value.extract::<u32>().ok()
        }
        _ => None,
    };
    if schema_version != Some(1) {
        return Err(provenance_input_error(
            "unsupported_schema_version",
            "/provenance/schemaVersion",
        ));
    }
    let role = match dict_get(dict, "role")? {
        Some(value) => extract_validated_str(&value)
            .map_err(|_| provenance_input_error("role_invalid", "/provenance/role"))?,
        None => return Err(provenance_input_error("role_invalid", "/provenance/role")),
    };
    if !matches!(role.as_str(), "canonical" | "derived") {
        return Err(provenance_input_error("role_invalid", "/provenance/role"));
    }
    strict_provenance_keys(
        dict,
        &[
            "schema_version",
            "role",
            "artifact_revision_id",
            "source_version_id",
            "source_revision_id",
            "source_locator",
            "canonical_source_hash",
        ],
        "/provenance",
    )?;
    let artifact_revision_id =
        ArtifactRevisionId::new(provenance_required_str(dict, "artifact_revision_id")?)
            .map_err(engine_error_to_py)?;
    let source_version_id =
        SourceVersionId::new(provenance_required_str(dict, "source_version_id")?)
            .map_err(engine_error_to_py)?;
    match role.as_str() {
        "canonical" => {
            strict_provenance_keys(
                dict,
                &["schema_version", "role", "artifact_revision_id", "source_version_id"],
                "/provenance",
            )?;
            Ok(WriteProvenanceV1::canonical(artifact_revision_id, source_version_id))
        }
        "derived" => {
            strict_provenance_keys(
                dict,
                &[
                    "schema_version",
                    "role",
                    "artifact_revision_id",
                    "source_version_id",
                    "source_revision_id",
                    "source_locator",
                    "canonical_source_hash",
                ],
                "/provenance",
            )?;
            let source_revision_id =
                SourceRevisionId::new(provenance_required_str(dict, "source_revision_id")?)
                    .map_err(engine_error_to_py)?;
            let locator = dict_get(dict, "source_locator")?
                .ok_or_else(|| {
                    provenance_input_error("locator_invalid", "/provenance/sourceLocator")
                })
                .and_then(|value| translate_source_locator(&value))?;
            let hash = dict_get(dict, "canonical_source_hash")?
                .ok_or_else(|| {
                    provenance_input_error("hash_invalid", "/provenance/canonicalSourceHash")
                })
                .and_then(|value| translate_canonical_hash(&value))?;
            Ok(WriteProvenanceV1::derived(
                artifact_revision_id,
                source_version_id,
                source_revision_id,
                locator,
                hash,
            ))
        }
        _ => Err(provenance_input_error("role_invalid", "/provenance/role")),
    }
}

pub(super) fn translate_write_item(item: &Bound<'_, PyAny>) -> PyResult<PreparedWrite> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| WriteValidationError::new_err("write item must be a dict"))?;

    if let Some(inner) = dict_get(dict, "edge")? {
        return translate_edge(&inner);
    }
    if let Some(inner) = dict_get(dict, "op_store")? {
        return translate_op_store(&inner);
    }
    if let Some(inner) = dict_get(dict, "admin_schema")? {
        return translate_admin_schema(&inner);
    }
    if let Some(inner) = dict_get(dict, "node")? {
        return translate_node(&inner);
    }

    // Bare `{"kind": ..., ...}` shape is treated as a Node — keeps the
    // five-verb test surface terse and matches the 0.6.0 Python stub.
    translate_node(item)
}

pub(super) fn translate_node(item: &Bound<'_, PyAny>) -> PyResult<PreparedWrite> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| WriteValidationError::new_err("node write item must be a dict"))?;
    let kind = dict_str_required(dict, "kind")?;
    let body = dict_str(dict, "body")?.unwrap_or_else(|| "{}".to_string());
    let source_id = dict_source_id_required(dict, "node")?;
    let logical_id = dict_str(dict, "logical_id")?;
    // OPP-12 Phase-1 (0.8.19 Slice 5) — create-time existence state + advisory
    // reason (X1 parity with the N-API binding). `state` defaults to `active`; an
    // out-of-subset value (`deleted`/`purged`/unknown) is a TYPED write-validation
    // rejection — you cannot CREATE a deleted/purged node. Thin pass-through.
    let state = match dict_str(dict, "state")? {
        Some(s) => InitialState::from_create_str(&s).ok_or_else(|| {
            WriteValidationError::new_err(format!(
                "cannot create a node with state {s:?}: only \"pending\" or \"active\" are creatable (deleted/purged require transition/purge)"
            ))
        })?,
        None => InitialState::Active,
    };
    let reason = dict_str(dict, "reason")?;
    // 0.8.20 Slice 15b (TC-34) — world-time validity window (X1 parity with the
    // N-API binding). INTEGER epoch seconds; absent or `None` means unbounded on
    // that side, which lands NULL and reproduces pre-slice behaviour exactly. The
    // half-open pair is validated in the ENGINE (`validate_write`), so Rust,
    // Python and TypeScript share one rule and cannot drift.
    let valid_from = dict_epoch_seconds(dict, "valid_from")?;
    let valid_until = dict_epoch_seconds(dict, "valid_until")?;
    let provenance = match dict_get(dict, "provenance")? {
        Some(value) => Some(translate_provenance(&value)?),
        _ => None,
    };
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
        }),
    }
}

/// 0.8.20 Slice 15b (TC-34) — read an optional INTEGER epoch-second field from a
/// write item. Absent or `None` yields `None` (the `dict_str` convention).
///
/// `bool` is rejected EXPLICITLY. Python's `bool` is a subclass of `int`, so a
/// bare `extract::<i64>()` would silently accept `True` as the instant `1` —
/// a silent coercion of exactly the kind this field must never perform. Floats
/// are rejected by `extract::<i64>()` itself.
pub(super) fn dict_epoch_seconds(d: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<i64>> {
    let Some(v) = dict_get(d, key)?.filter(|v| !v.is_none()) else {
        return Ok(None);
    };
    if v.is_instance_of::<pyo3::types::PyBool>() {
        return Err(WriteValidationError::new_err(format!(
            "field {key:?} must be an integer (epoch seconds) or None, not a bool"
        )));
    }
    v.extract::<i64>().map(Some).map_err(|_| {
        WriteValidationError::new_err(format!(
            "field {key:?} must be an integer (epoch seconds) or None"
        ))
    })
}

pub(super) fn translate_edge(item: &Bound<'_, PyAny>) -> PyResult<PreparedWrite> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| WriteValidationError::new_err("edge write item must be a dict"))?;
    let kind = dict_str_required(dict, "kind")?;
    let from = dict_str_required(dict, "from")?;
    let to = dict_str_required(dict, "to")?;
    let source_id = dict_source_id_required(dict, "edge")?;
    let logical_id = dict_str(dict, "logical_id")?;
    // Edge body (the relation text) — optional. Projected into `search_index_edges`
    // so the C1 graph arm can seed from edge-fact FTS (`source A`). NULL = not indexed.
    let body = dict_str(dict, "body")?;
    // R3 (Slice 30) — temporal validity fields accepted from user-facing write API.
    //
    // TC-33 (HITL-RATIFIED 2026-07-21) — these are **INTEGER epoch seconds
    // (UTC)**, not ISO-8601 strings. This is the GOVERNED SDK WRITE SURFACE,
    // which carries the same representation as storage; ISO-8601 survives ONLY
    // on the BYO-LLM extractor wire, where the engine normalises it with hard
    // rejection. Reuses the same `dict_epoch_seconds` helper as the node
    // `valid_from`/`valid_until` window, so both temporal axes now validate
    // identically (and a bool is rejected rather than coerced to 0/1).
    //
    // `None` = "still valid"; that semantic is load-bearing and unchanged.
    let t_valid = dict_epoch_seconds(dict, "t_valid")?;
    let t_invalid = dict_epoch_seconds(dict, "t_invalid")?;
    let provenance = match dict_get(dict, "provenance")? {
        Some(value) => Some(translate_provenance(&value)?),
        _ => None,
    };
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
        }),
    }
}

pub(super) fn translate_op_store(item: &Bound<'_, PyAny>) -> PyResult<PreparedWrite> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| WriteValidationError::new_err("op_store write item must be a dict"))?;
    let collection = dict_str_required(dict, "collection")?;
    let record_key = dict_str_required(dict, "record_key")?;
    let schema_id = dict_str(dict, "schema_id")?;
    let body = dict_str_required(dict, "body")?;
    Ok(PreparedWrite::OpStore { collection, record_key, schema_id, body })
}

pub(super) fn translate_admin_schema(item: &Bound<'_, PyAny>) -> PyResult<PreparedWrite> {
    let dict = item
        .cast::<PyDict>()
        .map_err(|_| PyTypeError::new_err("admin_schema write item must be a dict"))?;
    let name = dict_str_required(dict, "name")?;
    let kind = dict_str_required(dict, "kind")?;
    let schema_json = dict_str_required(dict, "schema_json")?;
    let retention_json = dict_str(dict, "retention_json")?.unwrap_or_else(|| "{}".to_string());
    Ok(PreparedWrite::AdminSchema { name, kind, schema_json, retention_json })
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "WriteReceipt",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyWriteReceipt {
    cursor: u64,
    /// G0 (Slice 15) — per-row `write_cursor`s, 1:1 with the input batch order.
    row_cursors: Vec<u64>,
    /// G8 (Slice 20) — count of edge endpoints in this batch pointing at a
    /// non-existent or superseded canonical node (informational; flag-and-count).
    dangling_edge_endpoints: u64,
}

impl PyWriteReceipt {
    pub(super) fn from_rust(r: RustWriteReceipt) -> Self {
        Self {
            cursor: r.cursor,
            row_cursors: r.row_cursors,
            dangling_edge_endpoints: r.dangling_edge_endpoints,
        }
    }
}

#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ActuationReceiptV1",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyActuationReceiptV1 {
    schema_version: u32,
    operation_id: String,
    request_sha256: String,
    outcome: String,
    refused_operation_index: Option<usize>,
    refused_field_path: Option<String>,
    reason_codes: Vec<String>,
    affected_revision_ids: Vec<String>,
    resulting_write_boundary: Option<String>,
    resulting_dependency_generation: Option<String>,
    pending_projection_write_cursors: Vec<String>,
    projection_generation_id: Option<String>,
    closure_operation_ids: Vec<String>,
}

impl From<RustActuationReceiptV1> for PyActuationReceiptV1 {
    fn from(value: RustActuationReceiptV1) -> Self {
        let outcome = match value.outcome {
            ActuationOutcomeV1::Committed => "committed",
            ActuationOutcomeV1::CommittedClosurePending => "committed_closure_pending",
            ActuationOutcomeV1::Refused => "refused",
        }
        .to_string();
        Self {
            schema_version: value.schema_version,
            operation_id: value.operation_id,
            request_sha256: value.request_sha256,
            outcome,
            refused_operation_index: value.refused_operation_index,
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
        }
    }
}

/// G11 (Slice 15) — BYO-LLM ingest receipt.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "IngestWithExtractorReceipt",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyIngestWithExtractorReceipt {
    nodes_written: u64,
    edges_written: u64,
    docs_processed: u64,
}

impl PyIngestWithExtractorReceipt {
    pub(super) fn from_rust(r: RustIngestWithExtractorReceipt) -> Self {
        Self {
            nodes_written: r.nodes_written,
            edges_written: r.edges_written,
            docs_processed: r.docs_processed,
        }
    }
}

/// 0.8.12 Slice 15 (OPP-2) — BYO-LLM consolidation receipt.
#[pyclass(
    module = "fathomdb._fathomdb",
    name = "ConsolidateReceipt",
    frozen,
    get_all,
    skip_from_py_object
)]
#[derive(Clone)]
pub(super) struct PyConsolidateReceipt {
    clusters_processed: u64,
    edges_examined: u64,
    edges_kept: u64,
    edges_invalidated: u64,
    edges_superseded: u64,
}

impl PyConsolidateReceipt {
    pub(super) fn from_rust(r: RustConsolidateReceipt) -> Self {
        Self {
            clusters_processed: r.clusters_processed,
            edges_examined: r.edges_examined,
            edges_kept: r.edges_kept,
            edges_invalidated: r.edges_invalidated,
            edges_superseded: r.edges_superseded,
        }
    }
}

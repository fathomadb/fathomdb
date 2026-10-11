use super::*;

#[napi(object)]
pub struct ProjectionGenerationStatusV1 {
    pub schema_version: u32,
    pub generation_id: String,
    pub declaration_sha256: String,
    pub origin: String,
    pub transition_boundary: String,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: String,
    pub ready_through: String,
    pub readiness: String,
    pub runtime_state: String,
    pub pending_count: String,
    pub failed_count: String,
}

impl From<RustProjectionGenerationStatusV1> for ProjectionGenerationStatusV1 {
    fn from(value: RustProjectionGenerationStatusV1) -> Self {
        Self {
            schema_version: value.schema_version,
            generation_id: value.generation_id.as_str().to_string(),
            declaration_sha256: value.declaration_sha256,
            origin: value.origin.as_str().to_string(),
            transition_boundary: value.transition_boundary.to_string(),
            effective_at_epoch_s: value.effective_at_epoch_s,
            observed_boundary: value.observed_boundary.to_string(),
            ready_through: value.ready_through.to_string(),
            readiness: value.readiness.as_str().to_string(),
            runtime_state: value.runtime_state.as_str().to_string(),
            pending_count: value.pending_count.to_string(),
            failed_count: value.failed_count.to_string(),
        }
    }
}

#[napi(object)]
pub struct MutationProjectionStatusV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub write_cursor: String,
    pub generation_id: String,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: String,
    pub ready_through: String,
    pub readiness: String,
    pub runtime_state: String,
    pub pending_count: String,
    pub failed_count: String,
}

impl From<RustMutationProjectionStatusV1> for MutationProjectionStatusV1 {
    fn from(value: RustMutationProjectionStatusV1) -> Self {
        Self {
            schema_version: value.schema_version,
            operation_id: value.operation_id,
            write_cursor: value.write_cursor.to_string(),
            generation_id: value.generation_id.as_str().to_string(),
            effective_at_epoch_s: value.effective_at_epoch_s,
            observed_boundary: value.observed_boundary.to_string(),
            ready_through: value.ready_through.to_string(),
            readiness: value.readiness.as_str().to_string(),
            runtime_state: value.runtime_state.as_str().to_string(),
            pending_count: value.pending_count.to_string(),
            failed_count: value.failed_count.to_string(),
        }
    }
}

pub(crate) fn translate_mutation_projection_status_request(
    value: &JsonValue,
) -> Result<RustMutationProjectionStatusRequestV1> {
    let object = value.as_object().ok_or_else(|| {
        typed_error(
            CODE_PROJECTION_GENERATION,
            "projection generation unsupported_schema_version at /schemaVersion",
            json!({"reason":"unsupported_schema_version","fieldPath":"/schemaVersion"}),
        )
    })?;
    let schema_version = object.get("schemaVersion").and_then(JsonValue::as_u64).unwrap_or(0);
    if schema_version != 1 {
        return Err(typed_error(
            CODE_PROJECTION_GENERATION,
            "projection generation unsupported_schema_version at /schemaVersion",
            json!({"reason":"unsupported_schema_version","fieldPath":"/schemaVersion"}),
        ));
    }
    let allowed = ["expectedGenerationId", "operationId", "schemaVersion", "writeCursor"];
    if let Some(key) = object.keys().filter(|key| !allowed.contains(&key.as_str())).min() {
        let escaped = key.replace('~', "~0").replace('/', "~1");
        return Err(typed_error(
            CODE_PROJECTION_GENERATION,
            format!("projection generation unknown_field at /{escaped}"),
            json!({"reason":"unknown_field","fieldPath":format!("/{escaped}")}),
        ));
    }
    let operation_id =
        object.get("operationId").and_then(JsonValue::as_str).unwrap_or_default().to_string();
    let operation_bytes = operation_id.as_bytes();
    let operation_valid = !operation_id.starts_with("_fdb:")
        && (1..=128).contains(&operation_bytes.len())
        && operation_bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && operation_bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'));
    if !operation_valid {
        return Err(typed_error(
            CODE_PROJECTION_GENERATION,
            "projection generation invalid_operation_id at /operationId",
            json!({"reason":"invalid_operation_id","fieldPath":"/operationId"}),
        ));
    }
    let cursor_text = object.get("writeCursor").and_then(JsonValue::as_str).unwrap_or_default();
    let write_cursor = if !(cursor_text.is_empty()
        || cursor_text.len() > 1 && cursor_text.starts_with('0'))
        && cursor_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        cursor_text.parse::<u64>().ok().filter(|value| *value != 0)
    } else {
        None
    }
    .ok_or_else(|| {
        typed_error(
            CODE_PROJECTION_GENERATION,
            "projection generation invalid_write_cursor at /writeCursor",
            json!({"reason":"invalid_write_cursor","fieldPath":"/writeCursor"}),
        )
    })?;
    let generation =
        object.get("expectedGenerationId").and_then(JsonValue::as_str).unwrap_or_default();
    let expected_generation_id =
        RustProjectionGenerationId::new(generation.to_string()).map_err(|error| {
            typed_error(
                CODE_PROJECTION_GENERATION,
                format!("projection generation {} at {}", error.reason.as_str(), error.field_path),
                json!({"reason":error.reason.as_str(),"fieldPath":error.field_path}),
            )
        })?;
    Ok(RustMutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id,
        write_cursor,
        expected_generation_id,
    })
}

#[napi(object)]
pub struct DependencyListV1 {
    pub schema_version: u32,
    pub items: Vec<SourceDependencyV1>,
}

impl From<RustDependencyListV1> for DependencyListV1 {
    fn from(value: RustDependencyListV1) -> Self {
        Self {
            schema_version: value.schema_version,
            items: value.items.into_iter().map(Into::into).collect(),
        }
    }
}

/// 0.8.20 Slice 5d (R-20-E4) — outcome of the `eraseSource` lifecycle verb.
/// Mirrors the Rust `ExciseReport`. Counts are narrowed `u64 -> i64` at the FFI
/// boundary, matching the `WriteReceipt.cursor` precedent.
#[napi(object)]
pub struct EraseReport {
    pub source_ref: String,
    pub nodes_excised: i64,
    pub edges_excised: i64,
    /// Row-owned projection rows (FTS5 + vec0 + `search_index_v2`) dropped
    /// alongside the canonical rows.
    pub projections_invalidated: i64,
}

impl EraseReport {
    pub(crate) fn from_rust(r: RustExciseReport) -> Self {
        Self {
            source_ref: r.source_ref,
            nodes_excised: r.nodes_excised as i64,
            edges_excised: r.edges_excised as i64,
            projections_invalidated: r.projections_invalidated as i64,
        }
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — a declarative projection declaration. Flat at
/// the FFI boundary; `fts`/`vector` booleans carry the sub-object PRESENCE and
/// the optional tokenizer/embedder carry the value. JS field names are
/// camelCase (`ftsTokenizer` / `vectorEmbedder`).
#[napi(object)]
pub struct ProjectionSpec {
    pub name: String,
    pub roles: Vec<String>,
    pub fts: bool,
    pub fts_tokenizer: Option<String>,
    pub vector: bool,
    pub vector_embedder: Option<String>,
    /// READ METADATA, engine-set (`vectorDenseReadiness` in JS):
    /// `"unavailable"` / `"embedding"` / `"ready"` on the way OUT of
    /// `read.projections`, omitted on every caller-authored spec.
    /// `"unavailable"` means no usable dense runtime; the other values derive
    /// from outstanding work under one. Inert on the way IN (the engine reports
    /// the derived truth), so read output still re-applies as a no-op.
    pub vector_dense_readiness: Option<String>,
    /// Ordered literal object-member path; absent preserves top-level lookup.
    pub source: Option<Vec<String>>,
}

impl ProjectionSpec {
    pub(crate) fn from_rust(s: &RustProjectionSpec) -> Self {
        Self {
            name: s.name.clone(),
            roles: s.roles.iter().map(|r| r.as_str().to_string()).collect(),
            fts: s.fts.is_some(),
            fts_tokenizer: s.fts.as_ref().and_then(|f| f.tokenizer.clone()),
            vector: s.vector.is_some(),
            vector_embedder: s.vector.as_ref().and_then(|v| v.embedder.clone()),
            vector_dense_readiness: s
                .vector
                .as_ref()
                .and_then(|v| v.dense_readiness)
                .map(|r| r.as_str().to_string()),
            source: s.source.clone(),
        }
    }

    pub(crate) fn to_rust(&self) -> Result<RustProjectionSpec> {
        // AC-068a/b — reject every string crossing the FFI into the spec BEFORE
        // the engine (writer transaction) is reached. Mirrors the per-string
        // gate applied at every other napi call site (e.g. `:1141`).
        validate_ffi_string_napi(&self.name)?;
        if let Some(tokenizer) = &self.fts_tokenizer {
            validate_ffi_string_napi(tokenizer)?;
        }
        if let Some(embedder) = &self.vector_embedder {
            validate_ffi_string_napi(embedder)?;
        }
        if let Some(source) = &self.source {
            for segment in source {
                validate_ffi_string_napi(segment)?;
            }
        }
        // 0.8.20 keystone closeout fix-4 — ROUND-TRIP CONSISTENCY GATE. A spec
        // the binding ACCEPTS must round-trip through `read.projections`
        // IDENTICALLY; otherwise reject it HERE with the typed validation error
        // rather than let the engine silently drop or normalize a sub-field.
        // Kept byte-for-byte in step with the pyo3 binding (Py ≡ TS): the two
        // must refuse the same shapes the same way. `fts`/`vector` carry the
        // sub-object PRESENCE, so an `ftsTokenizer` supplied while `fts` is
        // false (or an empty `""` that the engine collapses to the default)
        // could never survive the round-trip and is refused.
        match (self.fts, self.fts_tokenizer.as_deref()) {
            (false, Some(_)) => {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: fts_tokenizer is set but fts is false — the tokenizer would be silently dropped and cannot round-trip; set fts=true or omit fts_tokenizer",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
            (true, Some("")) => {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: fts_tokenizer is an empty string, which the engine normalizes to the default and cannot round-trip; omit fts_tokenizer for the engine default",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
            _ => {}
        }
        match (self.vector, self.vector_embedder.as_deref()) {
            (false, Some(_)) => {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: vector_embedder is set but vector is false — the embedder would be silently dropped and cannot round-trip; set vector=true or omit vector_embedder",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
            (true, Some("")) => {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: vector_embedder is an empty string, which the engine normalizes to the default and cannot round-trip; omit vector_embedder for the engine default",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
            _ => {}
        }
        // 0.8.20 Slice 20 (R-20-DR) — the SAME round-trip gate applied to the
        // engine-set readiness field. It is READ METADATA, so its VALUE is inert
        // on the way in (the engine always reports the derived truth, which is
        // what keeps `read.projections` output re-appliable as a no-op — the
        // fix-4 read→configure round-trip, pinned by a test in both bindings).
        // But the two shapes that could NEVER round-trip are refused, exactly as
        // for `vectorEmbedder`. Kept byte-for-byte in step with the pyo3 binding
        // (Py ≡ TS): the two must refuse the same shapes the same way.
        if let Some(readiness) = self.vector_dense_readiness.as_deref() {
            validate_ffi_string_napi(readiness)?;
            if !self.vector {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: vectorDenseReadiness is set but vector is false — readiness belongs to the vector sub-object and cannot round-trip without it; set vector=true or omit vectorDenseReadiness",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
            if RustDenseReadiness::from_str_opt(readiness).is_none() {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: unknown vectorDenseReadiness {readiness:?}: expected \"unavailable\", \"embedding\", or \"ready\" (\"pending\" is reserved for the admission axis and is never a readiness value). It is engine-set read metadata; omit it",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
        }
        let mut roles = std::collections::BTreeSet::new();
        for r in &self.roles {
            validate_ffi_string_napi(r)?;
            let role = RustProjectionRole::from_str_opt(r).ok_or_else(|| {
                typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "unknown projection role {r:?}: expected filterable/rankable/searchable"
                    ),
                    JsonValue::Null,
                )
            })?;
            // fix-4 — `roles` is a SET; a duplicate spelling in the flat list
            // cannot round-trip (the registry stores a de-duplicated
            // `BTreeSet`), so refuse it rather than silently coalesce.
            if !roles.insert(role) {
                return Err(typed_error(
                    CODE_INVALID_ARGUMENT,
                    format!(
                        "projection {:?}: role {r:?} is repeated; roles is a set and duplicates cannot round-trip",
                        self.name
                    ),
                    JsonValue::Null,
                ));
            }
        }
        Ok(RustProjectionSpec {
            name: self.name.clone(),
            roles,
            fts: self.fts.then(|| RustProjectionFts { tokenizer: self.fts_tokenizer.clone() }),
            vector: self.vector.then(|| RustProjectionVector {
                embedder: self.vector_embedder.clone(),
                // 0.8.20 Slice 20 (R-20-DR) — readiness is engine-set READ
                // METADATA. Carried across so the engine can see what the caller
                // sent, but the registry never stores it and never honours it.
                dense_readiness: self
                    .vector_dense_readiness
                    .as_deref()
                    .and_then(RustDenseReadiness::from_str_opt),
            }),
            source: self.source.clone(),
        })
    }
}

/// 0.8.20 Slice 15d (R-20-PR) — the diff `configureProjections` applied.
#[napi(object)]
pub struct ProjectionDelta {
    pub built: Vec<String>,
    pub dropped: Vec<String>,
    pub deferred: Vec<String>,
    pub unchanged: bool,
    /// 0.8.20 Slice 22 (R-20-VC / TC-67) — node KINDS the vector writer can never
    /// commit. A different axis from the three attribute-name lists above; the
    /// name says so. Output-only: `configureProjections` takes specs, never a
    /// delta, so there is no inbound direction to round-trip.
    pub vector_unsupported_kinds: Vec<String>,
}

impl ProjectionDelta {
    pub(crate) fn from_rust(d: &RustProjectionDelta) -> Self {
        Self {
            built: d.built.clone(),
            dropped: d.dropped.clone(),
            deferred: d.deferred.clone(),
            unchanged: d.unchanged,
            vector_unsupported_kinds: d.vector_unsupported_kinds.clone(),
        }
    }
}

/// One declared projection's current dense status in the pure status facade.
#[napi(object)]
pub struct ProjectionRuntimeStatusEntry {
    pub name: String,
    /// `not_declared` / `unavailable` / `embedding` / `ready`.
    pub dense_readiness: String,
}

impl ProjectionRuntimeStatusEntry {
    pub(crate) fn from_rust(entry: &RustProjectionRuntimeStatusEntry) -> Self {
        Self {
            name: entry.name.clone(),
            dense_readiness: entry.dense_readiness.as_str().to_string(),
        }
    }
}

/// A pure current view of projection runtime facts for one open engine session.
#[napi(object)]
pub struct ProjectionRuntimeStatus {
    pub runtime_embedder_available: bool,
    /// `none` / `no_runtime` / `vector_equivalence_disabled`.
    pub runtime_unavailability_reason: String,
    pub projections: Vec<ProjectionRuntimeStatusEntry>,
    pub vector_unsupported_kinds: Vec<String>,
}

impl ProjectionRuntimeStatus {
    pub(crate) fn from_rust(status: &RustProjectionRuntimeStatus) -> Self {
        Self {
            runtime_embedder_available: status.runtime_embedder_available,
            runtime_unavailability_reason: status
                .runtime_unavailability_reason
                .as_str()
                .to_string(),
            projections: status
                .projections
                .iter()
                .map(ProjectionRuntimeStatusEntry::from_rust)
                .collect(),
            vector_unsupported_kinds: status.vector_unsupported_kinds.clone(),
        }
    }
}

#[napi(object)]
pub struct EmbeddingReadiness {
    pub state: String,
    pub usable_embedder: bool,
    pub pending_count: i64,
    pub affected_kinds: Vec<String>,
    pub code: Option<String>,
    pub operation: Option<String>,
    pub remediations: Vec<String>,
    pub documentation_url: Option<String>,
}

impl EmbeddingReadiness {
    pub(crate) fn from_rust(readiness: &RustEmbeddingReadiness) -> Self {
        let blocked = readiness.blocked.as_ref();
        Self {
            state: readiness.state.as_str().to_string(),
            usable_embedder: readiness.usable_embedder,
            pending_count: readiness.pending_count as i64,
            affected_kinds: readiness.affected_kinds.clone(),
            code: blocked.map(|b| b.code.to_string()),
            operation: blocked.map(|b| b.operation.as_str().to_string()),
            remediations: blocked
                .map(|b| b.remediations.iter().map(|s| (*s).to_string()).collect())
                .unwrap_or_default(),
            documentation_url: blocked.map(|b| b.documentation_url.to_string()),
        }
    }
}

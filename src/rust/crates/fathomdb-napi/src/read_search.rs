use super::*;

// ===== read.* (G2/G3) =================================================
//
// Slice 30 — the governed `read.*` namespace native fns. `read.get` /
// `read.getMany` are active-only point lookups by `logicalId` (not-found is a
// normal `null`, never a thrown error — a typed NotFound class is reserved-gap
// Slice 31). `read.collection` / `read.mutations` are the paginated op-store
// read-back with a MANDATORY limit + after-id cursor. All four ride the engine's
// ReaderWorkerPool DEFERRED-tx path; the binding only marshals.

/// Slice 30 (G3) — options for `read.collection` / `read.mutations`. `limit` is
/// MANDATORY (no default — the engine clamps it to the ~1M cap); `afterId` is
/// the exclusive cursor.
/// 0.8.20 Slice 10b (R-20-RV / R-20-NV) — the TypeScript face of `ReadView`.
///
/// Idiomatic `camelCase`; every field is optional and every one defaults to the
/// STRICT view, so omitting `view` entirely reproduces the shipped read
/// behaviour exactly.
///
/// World-time only — there is deliberately no `historyAsOf`.
#[napi(object)]
pub struct ReadViewInput {
    /// Relax `superseded_at IS NULL` — include historical versions.
    pub include_superseded: Option<bool>,
    /// Relax `state = 'active'` — include non-active lifecycle states.
    pub include_inactive: Option<bool>,
    /// Relax the validity window entirely (ignores `validAsOf`).
    pub include_out_of_window: Option<bool>,
    /// Validity instant, INTEGER epoch SECONDS. Omitted = now.
    pub valid_as_of: Option<i64>,
}

/// Versioned validity and eligibility request for a frozen read context.
#[napi(object)]
pub struct ReadContextV1 {
    pub schema_version: u32,
    pub view: ReadViewInput,
    pub eligibility: SearchFilterInput,
}

/// Engine-minted database-local authenticated read context.
#[napi(object)]
pub struct FrozenReadContextV1 {
    pub schema_version: u32,
    pub effective_valid_at: i64,
    pub context: ReadContextV1,
    pub token: String,
}

pub(crate) fn search_filter_from_required_input(
    input: SearchFilterInput,
) -> Result<RustSearchFilter> {
    Ok(search_filter_input_to_rust(Some(input))?.unwrap_or_default())
}

pub(crate) fn read_context_to_rust(input: ReadContextV1) -> Result<RustReadContextV1> {
    if input.schema_version != 1 {
        return Err(typed_error(
            CODE_FROZEN_READ,
            "unsupported_schema_version at /schemaVersion",
            json!({ "reason": "unsupported_schema_version", "fieldPath": "/schemaVersion" }),
        ));
    }
    let view = read_view_or_default(Some(input.view));
    let eligibility = search_filter_from_required_input(input.eligibility)?;
    RustReadContextV1::new(view, eligibility).map_err(engine_error_to_napi)
}

pub(crate) fn read_context_from_rust(context: &RustReadContextV1) -> ReadContextV1 {
    ReadContextV1 {
        schema_version: context.schema_version,
        view: ReadViewInput {
            include_superseded: Some(context.view.include_superseded),
            include_inactive: Some(context.view.include_inactive),
            include_out_of_window: Some(context.view.include_out_of_window),
            valid_as_of: context.view.valid_as_of,
        },
        eligibility: SearchFilterInput {
            source_type: context.eligibility.source_type.clone(),
            kind: context.eligibility.kind.clone(),
            created_after: context.eligibility.created_after,
            status: context.eligibility.status.clone(),
            attributes: Some(
                context
                    .eligibility
                    .attributes
                    .iter()
                    .map(|(name, value)| vec![name.clone(), value.clone()])
                    .collect(),
            ),
        },
    }
}

pub(crate) fn frozen_context_to_rust(
    input: FrozenReadContextV1,
) -> Result<RustFrozenReadContextV1> {
    let context = input.context;
    let eligibility = context.eligibility;
    let attributes = eligibility
        .attributes
        .unwrap_or_default()
        .into_iter()
        .map(|pair| match pair.as_slice() {
            [name, value] => (name.clone(), value.clone()),
            _ => ("\0invalid-frozen-attribute-pair".to_string(), format!("{pair:?}")),
        })
        .collect();
    let mut rust_eligibility = RustSearchFilter::default();
    rust_eligibility.source_type = eligibility.source_type;
    rust_eligibility.kind = eligibility.kind;
    rust_eligibility.created_after = eligibility.created_after;
    rust_eligibility.status = eligibility.status;
    rust_eligibility.attributes = attributes;
    Ok(RustFrozenReadContextV1 {
        schema_version: input.schema_version,
        effective_valid_at: input.effective_valid_at,
        context: RustReadContextV1 {
            schema_version: context.schema_version,
            view: read_view_or_default(Some(context.view)),
            eligibility: rust_eligibility,
        },
        token: input.token,
    })
}

pub(crate) fn frozen_context_from_rust(context: RustFrozenReadContextV1) -> FrozenReadContextV1 {
    FrozenReadContextV1 {
        schema_version: context.schema_version,
        effective_valid_at: context.effective_valid_at,
        context: read_context_from_rust(&context.context),
        token: context.token,
    }
}

/// An omitted `view` means the strict default view.
pub(crate) fn read_view_or_default(view: Option<ReadViewInput>) -> RustReadView {
    match view {
        None => RustReadView::default(),
        Some(v) => RustReadView {
            include_superseded: v.include_superseded.unwrap_or(false),
            include_inactive: v.include_inactive.unwrap_or(false),
            include_out_of_window: v.include_out_of_window.unwrap_or(false),
            valid_as_of: v.valid_as_of,
        },
    }
}

/// 0.8.20 Slice 10b (R-20-NV) — the TypeScript face of `BoundaryCrossing`.
#[napi(object)]
pub struct BoundaryCrossing {
    /// The node that crossed a validity boundary.
    pub node: NodeRecord,
    /// Set when the node BECAME VALID inside the interrogated interval.
    pub became_valid_at: Option<i64>,
    /// Set when the node BECAME INVALID inside the interrogated interval.
    pub became_invalid_at: Option<i64>,
}

impl BoundaryCrossing {
    pub(crate) fn from_rust(c: &RustBoundaryCrossing) -> Self {
        Self {
            node: NodeRecord::from_rust(&c.node),
            became_valid_at: c.became_valid_at,
            became_invalid_at: c.became_invalid_at,
        }
    }
}

#[napi(object)]
pub struct ReadCollectionOptions {
    pub after_id: Option<i64>,
    pub limit: i64,
}

#[napi(js_name = "readGet")]
pub async fn read_get(
    engine: &Engine,
    logical_id: String,
    view: Option<ReadViewInput>,
) -> Result<Option<NodeRecord>> {
    validate_ffi_string_napi(&logical_id)?;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let record = call_engine(move || inner.read_get(&logical_id, &view)).await?;
    Ok(record.as_ref().map(NodeRecord::from_rust))
}

#[napi(js_name = "readGetMany")]
pub async fn read_get_many(
    engine: &Engine,
    logical_ids: Vec<String>,
    view: Option<ReadViewInput>,
) -> Result<Vec<Option<NodeRecord>>> {
    for id in &logical_ids {
        validate_ffi_string_napi(id)?;
    }
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(move || inner.read_get_many(&logical_ids, &view)).await?;
    Ok(rows.iter().map(|r| r.as_ref().map(NodeRecord::from_rust)).collect())
}

#[napi(js_name = "readCollection")]
pub async fn read_collection(
    engine: &Engine,
    collection: String,
    options: ReadCollectionOptions,
) -> Result<Vec<OpStoreRow>> {
    read_collection_impl(engine, collection, options).await
}

#[napi(js_name = "readMutations")]
pub async fn read_mutations(
    engine: &Engine,
    collection: String,
    options: ReadCollectionOptions,
) -> Result<Vec<OpStoreRow>> {
    read_collection_impl(engine, collection, options).await
}

pub(crate) async fn read_collection_impl(
    engine: &Engine,
    collection: String,
    options: ReadCollectionOptions,
) -> Result<Vec<OpStoreRow>> {
    validate_ffi_string_napi(&collection)?;
    let after_id = options.after_id;
    // A negative limit is meaningless; clamp the floor to 0 (empty read). The
    // engine clamps the ceiling to the ~1M cap.
    let limit = options.limit.max(0) as usize;
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(move || inner.read_collection(&collection, after_id, limit)).await?;
    Ok(rows.iter().map(OpStoreRow::from_rust).collect())
}

// ===== read.list (G4 / Slice 35) ======================================

/// G4 (Slice 35) — predicate input for `readList`. Shape mirrors the TS
/// `Predicate` interface: `type` ∈ `{"eq","gt","gte","lt","lte"}`, `path`,
/// `value` (JS `string | number | boolean` — carried as `f64` for numbers).
#[napi(object)]
pub struct PredicateInput {
    /// Comparison type: "eq" | "gt" | "gte" | "lt" | "lte".
    pub r#type: String,
    /// JSON path from the allowlist (e.g. "$.status", "$.priority").
    pub path: String,
    /// String value for eq/gt/gte/lt/lte string comparisons.
    pub value_str: Option<String>,
    /// Integer value for numeric comparisons.
    pub value_int: Option<i64>,
    /// Boolean value for bool comparisons.
    pub value_bool: Option<bool>,
}

pub(crate) fn napi_predicate_to_rust(pred: PredicateInput) -> Result<RustPredicate> {
    // Determine the scalar value: bool > int > str (bool is also "truthy" int in JS).
    let scalar = if let Some(b) = pred.value_bool {
        RustScalarValue::Bool(b)
    } else if let Some(i) = pred.value_int {
        RustScalarValue::Integer(i)
    } else if let Some(s) = pred.value_str {
        RustScalarValue::Text(s)
    } else {
        return Err(typed_error(
            CODE_INVALID_FILTER,
            "predicate must have one of value_str, value_int, or value_bool",
            JsonValue::Null,
        ));
    };
    match pred.r#type.as_str() {
        "eq" => RustPredicate::json_path_eq(pred.path, scalar).map_err(engine_error_to_napi),
        "gt" => RustPredicate::json_path_compare(pred.path, RustComparisonOp::Gt, scalar)
            .map_err(engine_error_to_napi),
        "gte" => RustPredicate::json_path_compare(pred.path, RustComparisonOp::Gte, scalar)
            .map_err(engine_error_to_napi),
        "lt" => RustPredicate::json_path_compare(pred.path, RustComparisonOp::Lt, scalar)
            .map_err(engine_error_to_napi),
        "lte" => RustPredicate::json_path_compare(pred.path, RustComparisonOp::Lte, scalar)
            .map_err(engine_error_to_napi),
        other => Err(typed_error(
            CODE_INVALID_FILTER,
            format!("unknown predicate type '{other}'; expected eq/gt/gte/lt/lte"),
            JsonValue::Null,
        )),
    }
}

#[napi(js_name = "readList")]
pub async fn read_list(
    engine: &Engine,
    kind: String,
    predicates: Option<Vec<PredicateInput>>,
    limit: Option<i64>,
    view: Option<ReadViewInput>,
) -> Result<Vec<NodeRecord>> {
    validate_ffi_string_napi(&kind)?;
    let mut rust_predicates: Vec<RustPredicate> = Vec::new();
    if let Some(plist) = predicates {
        for pred in plist {
            rust_predicates.push(napi_predicate_to_rust(pred)?);
        }
    }
    let limit = limit.unwrap_or(100).max(0) as usize;
    let view = read_view_or_default(view);
    let inner = Arc::clone(&engine.inner);
    let rows = call_engine(move || inner.read_list(&kind, &rust_predicates, limit, &view)).await?;
    Ok(rows.iter().map(NodeRecord::from_rust).collect())
}

/// 0.8.11 Slice 40 (#17) — one term of the unified `Filter` grammar. `term` ∈
/// `{"source_type","kind","created_after","status","json"}`. For the four
/// shorthand terms set `valueStr`/`valueInt`; for `json` set `predicate`.
#[napi(object)]
pub struct FilterTermInput {
    /// Discriminator: source_type | kind | created_after | status | json.
    pub term: String,
    /// String value for source_type/kind/status terms.
    pub value_str: Option<String>,
    /// Integer value for the created_after term (unix seconds).
    pub value_int: Option<i64>,
    /// The G4 predicate for a `json` term.
    pub predicate: Option<PredicateInput>,
}

pub(crate) fn napi_filter_term_to_rust(term: FilterTermInput) -> Result<RustFilterTerm> {
    match term.term.as_str() {
        "source_type" => term.value_str.map(RustFilterTerm::SourceType).ok_or_else(|| {
            typed_error(CODE_INVALID_FILTER, "source_type term requires valueStr", JsonValue::Null)
        }),
        "kind" => term.value_str.map(RustFilterTerm::Kind).ok_or_else(|| {
            typed_error(CODE_INVALID_FILTER, "kind term requires valueStr", JsonValue::Null)
        }),
        "created_after" => term.value_int.map(RustFilterTerm::CreatedAfter).ok_or_else(|| {
            typed_error(CODE_INVALID_FILTER, "created_after term requires valueInt", JsonValue::Null)
        }),
        "status" => term.value_str.map(RustFilterTerm::Status).ok_or_else(|| {
            typed_error(CODE_INVALID_FILTER, "status term requires valueStr", JsonValue::Null)
        }),
        "json" => {
            let pred = term.predicate.ok_or_else(|| {
                typed_error(CODE_INVALID_FILTER, "json term requires predicate", JsonValue::Null)
            })?;
            Ok(RustFilterTerm::Json(napi_predicate_to_rust(pred)?))
        }
        other => Err(typed_error(
            CODE_INVALID_FILTER,
            format!("unknown filter term '{other}'; expected source_type/kind/created_after/status/json"),
            JsonValue::Null,
        )),
    }
}

/// 0.8.11 Slice 40 (#17) — unified `Filter` → `read.list` backend. The engine
/// performs the authoritative total dispatch (Json `json_extract`;
/// SourceType/Kind constant-fold vs the partition kind).
#[napi(js_name = "readListFilter")]
pub async fn read_list_filter(
    engine: &Engine,
    kind: String,
    terms: Option<Vec<FilterTermInput>>,
    limit: Option<i64>,
    view: Option<ReadViewInput>,
) -> Result<Vec<NodeRecord>> {
    validate_ffi_string_napi(&kind)?;
    let mut rust_terms: Vec<RustFilterTerm> = Vec::new();
    if let Some(tlist) = terms {
        for t in tlist {
            rust_terms.push(napi_filter_term_to_rust(t)?);
        }
    }
    let filter = RustFilter { terms: rust_terms };
    let limit = limit.unwrap_or(100).max(0) as usize;
    let inner = Arc::clone(&engine.inner);
    let view = read_view_or_default(view);
    let rows = call_engine(move || inner.read_list_filter(&kind, &filter, limit, &view)).await?;
    Ok(rows.iter().map(NodeRecord::from_rust).collect())
}

#[napi(js_name = "readCanonicalPage")]
pub async fn read_canonical_page(
    engine: &Engine,
    kind: String,
    context: FrozenReadContextV1,
    page: PageRequestV1,
) -> Result<NodePageV1> {
    validate_ffi_string_napi(&kind)?;
    let frozen = frozen_context_to_rust(context)?;
    let page = page.into_rust()?;
    let inner = Arc::clone(&engine.inner);
    call_engine(move || inner.read_canonical_page(&kind, &frozen, &page))
        .await
        .map(NodePageV1::from_rust)
}

#[napi(js_name = "readOperationalState")]
pub async fn read_operational_state(
    engine: &Engine,
    collection: String,
    record_key: String,
    context: Option<FrozenReadContextV1>,
) -> Result<Option<OperationalStateRecordV1>> {
    validate_ffi_string_napi(&collection)?;
    validate_ffi_string_napi(&record_key)?;
    let frozen = context.map(frozen_context_to_rust).transpose()?;
    let inner = Arc::clone(&engine.inner);
    let record = call_engine(move || {
        inner.read_operational_state(&collection, &record_key, frozen.as_ref())
    })
    .await?;
    Ok(record.as_ref().map(OperationalStateRecordV1::from_rust))
}

#[napi(js_name = "readOperationalStatePage")]
pub async fn read_operational_state_page(
    engine: &Engine,
    collection: String,
    context: FrozenReadContextV1,
    page: PageRequestV1,
) -> Result<OperationalStatePageV1> {
    validate_ffi_string_napi(&collection)?;
    let frozen = frozen_context_to_rust(context)?;
    let page = page.into_rust()?;
    let inner = Arc::clone(&engine.inner);
    call_engine(move || inner.read_operational_state_page(&collection, &frozen, &page))
        .await
        .map(OperationalStatePageV1::from_rust)
}

#[napi(object)]
pub struct SoftFallback {
    /// "vector" | "text" | "text_edge"
    pub branch: String,
}

/// C-2 (0.8.19 / OPP-12 Phase-1, TC-8) — the typed id-space carrier for
/// [`SearchHit::id`], surfaced to JS as `{ space, value }`. `space` is the
/// lowercase discriminant (`"logical"` | `"content"` | `"passage"`), mirroring
/// the engine's `IdSpaceKind` enum (the C-2 binding — a typed carrier, not a
/// magic-prefixed string). `value` is the bare id (id-space prefix stripped).
#[napi(object)]
pub struct IdSpace {
    /// "logical" | "content" | "passage"
    pub space: String,
    /// The bare id value (id-space prefix stripped).
    pub value: String,
}

impl IdSpace {
    pub(crate) fn from_rust(id: &RustIdSpace) -> Self {
        Self { space: id.space.as_str().to_string(), value: id.value.clone() }
    }
}

#[napi(object)]
pub struct SearchHit {
    /// C-2 (0.8.19 / TC-8) — the typed, non-null, id-space-total hit id
    /// (`{ space, value }`). Governed hits are `logical` (`"l:"`), doc-seeded hits
    /// `content` (`"h:"`), synthetic passages `passage` (`"p:"`). Its `value`
    /// equals the pre-0.8.19 `stableId` (which this subsumes) so cross-session
    /// real-gold keying continues on `id`. The pre-C-2 positional `write_cursor`
    /// id is engine-internal and no longer surfaced.
    pub id: IdSpace,
    pub kind: String,
    pub body: String,
    /// Raw per-branch relevance: `vec_distance_l2` (vector) or `bm25()`
    /// (text). Not comparable across branches raw.
    pub score: f64,
    /// "vector" | "text"
    pub branch: String,
    /// Source-document provenance (`sourceId` in JS) — the identifier
    /// `eraseSource` consumes. TC-31 (0.8.20): populated on EVERY hit path, not
    /// just the graph arm. Node hits (text/vector) carry the node's own
    /// `source_id`; edge hits (edge-FTS, vector edge-fact) carry the edge's own;
    /// graph-arm hits carry the traversed edge's (unchanged). `null` only when
    /// the stored row really has NULL provenance: written before 0.8.20, or a
    /// governed row spared by the step-21 backfill under the TC-11 pin.
    pub source_id: Option<String>,
    /// 0.8.5 (EXP-0) — per-candidate CE score `ce_norm = sigmoid(ce_logit)`
    /// (`ceScore` in JS). Set only for hits inside the reranked pool; `null`
    /// otherwise (out-of-pool, identity path, or no CE model loaded).
    pub ce_score: Option<f64>,
}

impl SearchHit {
    pub(crate) fn from_rust(h: &RustSearchHit) -> Self {
        Self {
            id: IdSpace::from_rust(&h.id),
            kind: h.kind.clone(),
            body: h.body.clone(),
            score: h.score,
            branch: match h.branch {
                SoftFallbackBranch::Vector => "vector".to_string(),
                SoftFallbackBranch::Text => "text".to_string(),
                SoftFallbackBranch::TextEdge => "text_edge".to_string(),
                SoftFallbackBranch::GraphArm => "graph_arm".to_string(),
            },
            source_id: h.source_id.clone(),
            ce_score: h.ce_score,
        }
    }
}

/// Slice 30 (G2) — an active canonical node row from `read.get` /
/// `read.getMany`. napi maps snake_case → camelCase JS (`logicalId`,
/// `writeCursor`).
#[napi(object)]
pub struct NodeRecord {
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: i64,
}

impl NodeRecord {
    pub(crate) fn from_rust(r: &RustNodeRecord) -> Self {
        Self {
            logical_id: r.logical_id.clone(),
            kind: r.kind.clone(),
            body: r.body.clone(),
            write_cursor: r.write_cursor as i64,
        }
    }
}

/// Slice 30 (G3) — one `operational_mutations` row from `read.collection` /
/// `read.mutations`. `id` is the after-id cursor key. napi maps snake_case →
/// camelCase JS (`recordKey`, `opKind`, `schemaId`, `writeCursor`).
#[napi(object)]
pub struct OpStoreRow {
    pub id: i64,
    pub collection: String,
    pub record_key: String,
    pub op_kind: String,
    pub payload: String,
    pub schema_id: Option<String>,
    pub write_cursor: i64,
}

impl OpStoreRow {
    pub(crate) fn from_rust(r: &RustOpStoreRow) -> Self {
        Self {
            id: r.id,
            collection: r.collection.clone(),
            record_key: r.record_key.clone(),
            op_kind: r.op_kind.clone(),
            payload: r.payload.clone(),
            schema_id: r.schema_id.clone(),
            write_cursor: r.write_cursor as i64,
        }
    }
}

#[napi(object)]
pub struct OperationalStateRecordV1 {
    pub schema_version: u32,
    pub collection: String,
    pub record_key: String,
    pub payload: String,
    pub schema_id: Option<String>,
    pub write_cursor: i64,
}

impl OperationalStateRecordV1 {
    pub(crate) fn from_rust(record: &RustOperationalStateRecordV1) -> Self {
        Self {
            schema_version: record.schema_version,
            collection: record.collection.clone(),
            record_key: record.record_key.clone(),
            payload: record.payload.clone(),
            schema_id: record.schema_id.clone(),
            write_cursor: record.write_cursor as i64,
        }
    }
}

#[napi(object)]
pub struct PageRequestV1 {
    pub schema_version: u32,
    pub limit: i64,
    pub cursor: Option<String>,
}

impl PageRequestV1 {
    pub(crate) fn into_rust(self) -> Result<RustPageRequestV1> {
        if let Some(value) = &self.cursor {
            validate_ffi_string_napi(value)?;
        }
        Ok(RustPageRequestV1 {
            schema_version: self.schema_version,
            limit: usize::try_from(self.limit).unwrap_or(0),
            cursor: self.cursor.map(RustPageCursor),
        })
    }
}

#[napi(object)]
pub struct NodePageV1 {
    pub schema_version: u32,
    pub items: Vec<NodeRecord>,
    pub next_cursor: Option<String>,
}

impl NodePageV1 {
    pub(crate) fn from_rust(page: RustPageV1<RustNodeRecord>) -> Self {
        Self {
            schema_version: page.schema_version,
            items: page.items.iter().map(NodeRecord::from_rust).collect(),
            next_cursor: page.next_cursor.map(|cursor| cursor.0),
        }
    }
}

#[napi(object)]
pub struct OperationalStatePageV1 {
    pub schema_version: u32,
    pub items: Vec<OperationalStateRecordV1>,
    pub next_cursor: Option<String>,
}

impl OperationalStatePageV1 {
    pub(crate) fn from_rust(page: RustPageV1<RustOperationalStateRecordV1>) -> Self {
        Self {
            schema_version: page.schema_version,
            items: page.items.iter().map(OperationalStateRecordV1::from_rust).collect(),
            next_cursor: page.next_cursor.map(|cursor| cursor.0),
        }
    }
}

/// G10 — closed metadata filter input for `search(query, filter?)`. All fields
/// optional; an all-`None` filter (or omitted) is the unfiltered path. Mirrors
/// the Python `SearchFilter` (cross-binding parity). napi maps the snake_case
/// fields to camelCase JS (`sourceType`, `createdAfter`).
#[napi(object)]
pub struct SearchFilterInput {
    pub source_type: Option<String>,
    pub kind: Option<String>,
    pub created_after: Option<i64>,
    pub status: Option<String>,
    /// Ordered `[projectionName, canonicalText]` pairs.
    pub attributes: Option<Vec<Vec<String>>>,
}

pub(crate) fn search_filter_input_to_rust(
    input: Option<SearchFilterInput>,
) -> Result<Option<RustSearchFilter>> {
    let Some(input) = input else { return Ok(None) };
    for text in [input.source_type.as_deref(), input.kind.as_deref(), input.status.as_deref()]
        .into_iter()
        .flatten()
    {
        validate_ffi_string_napi(text)?;
    }
    let mut attributes = Vec::new();
    for pair in input.attributes.unwrap_or_default() {
        if pair.len() != 2 {
            return Err(typed_error(
                CODE_INVALID_ARGUMENT,
                "each attributes entry must be a [projectionName, canonicalText] pair",
                JsonValue::Null,
            ));
        }
        validate_ffi_string_napi(&pair[0])?;
        validate_ffi_string_napi(&pair[1])?;
        attributes.push((pair[0].clone(), pair[1].clone()));
    }
    let mut rust = RustSearchFilter::default();
    rust.source_type = input.source_type;
    rust.kind = input.kind;
    rust.created_after = input.created_after;
    rust.status = input.status;
    rust.attributes = attributes;
    if rust.source_type.is_none()
        && rust.kind.is_none()
        && rust.created_after.is_none()
        && rust.status.is_none()
        && rust.attributes.is_empty()
    {
        Ok(None)
    } else {
        Ok(Some(rust))
    }
}

#[napi(object)]
pub struct SearchResult {
    pub projection_cursor: i64,
    pub soft_fallback: Option<SoftFallback>,
    pub results: Vec<SearchHit>,
    /// 0.8.8 EXP-OBS (Slice 10) — opt-in retrieval explanation sidecar
    /// (`explanation` in JS). Present only when `search(..., explain=true)`; `null`
    /// (default) keeps the payload byte-identical to the pre-0.8.8 shape.
    pub explanation: Option<Explanation>,
}

impl SearchResult {
    pub(crate) fn from_rust(r: RustSearchResult) -> Self {
        Self {
            projection_cursor: r.projection_cursor as i64,
            soft_fallback: r.soft_fallback.as_ref().map(|s| SoftFallback {
                branch: match s.branch {
                    SoftFallbackBranch::Vector => "vector".to_string(),
                    SoftFallbackBranch::Text => "text".to_string(),
                    SoftFallbackBranch::TextEdge => "text_edge".to_string(),
                    SoftFallbackBranch::GraphArm => "graph_arm".to_string(),
                },
            }),
            results: r.results.iter().map(SearchHit::from_rust).collect(),
            explanation: r.explanation.as_ref().map(Explanation::from_rust),
        }
    }
}

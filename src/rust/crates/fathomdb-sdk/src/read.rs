//! The `read` namespace: point, list, page, boundary and projection-status
//! reads. Each function takes the [`Engine`] first, as in Python and TypeScript.

use fathomdb_engine::{
    BoundaryCrossing, EmbeddingReadiness, FrozenReadContextV1, MutationProjectionStatusRequestV1,
    MutationProjectionStatusV1, NodeRecord, OpStoreRow, OperationalStateRecordV1, PageRequestV1,
    PageV1, ProjectionGenerationStatusV1, ProjectionRuntimeStatus, ProjectionSpec, ReadView,
};

use crate::error::{Error, Result};
use crate::guard;
use crate::options::ListOptions;
use crate::Engine;

fn view_or_default(view: Option<&ReadView>) -> ReadView {
    view.copied().unwrap_or_default()
}

/// The current (or `view`-selected) record for `logical_id`, if any.
pub fn get(
    engine: &Engine,
    logical_id: &str,
    view: Option<&ReadView>,
) -> Result<Option<NodeRecord>> {
    guard::text(logical_id)?;
    Ok(engine.core().read_get(logical_id, &view_or_default(view))?)
}

/// One slot per requested id, `None` where absent, in request order.
pub fn get_many(
    engine: &Engine,
    logical_ids: &[String],
    view: Option<&ReadView>,
) -> Result<Vec<Option<NodeRecord>>> {
    guard::texts(logical_ids)?;
    Ok(engine.core().read_get_many(logical_ids, &view_or_default(view))?)
}

/// Op-store rows of `collection` after `after_id`, at most `limit`.
pub fn collection(
    engine: &Engine,
    collection: &str,
    after_id: Option<i64>,
    limit: usize,
) -> Result<Vec<OpStoreRow>> {
    guard::text(collection)?;
    Ok(engine.core().read_collection(collection, after_id, limit)?)
}

/// Mutation rows of `collection`; the same read as [`collection`], as in both
/// bindings.
pub fn mutations(
    engine: &Engine,
    collection: &str,
    after_id: Option<i64>,
    limit: usize,
) -> Result<Vec<OpStoreRow>> {
    self::collection(engine, collection, after_id, limit)
}

/// Records of `kind`, filtered by `predicates` or by `filter` (not both).
pub fn list(engine: &Engine, kind: &str, options: ListOptions) -> Result<Vec<NodeRecord>> {
    let ListOptions { predicates, filter, limit, view } = options;
    guard::text(kind)?;
    let view = view.unwrap_or_default();
    match filter {
        Some(_) if !predicates.is_empty() => Err(Error::invalid_argument(
            "read.list: pass either `predicates` or `filter`, not both",
        )),
        Some(filter) => {
            guard::filter(&filter)?;
            Ok(engine.core().read_list_filter(kind, &filter, limit, &view)?)
        }
        None => Ok(engine.core().read_list(kind, &predicates, limit, &view)?),
    }
}

/// One page of canonical records of `kind` under a frozen context.
pub fn canonical_page(
    engine: &Engine,
    kind: &str,
    context: &FrozenReadContextV1,
    page: &PageRequestV1,
) -> Result<PageV1<NodeRecord>> {
    guard::text(kind)?;
    Ok(engine.core().read_canonical_page(kind, context, page)?)
}

/// The operational-state record for `record_key`, optionally under a frozen context.
pub fn operational_state(
    engine: &Engine,
    collection: &str,
    record_key: &str,
    context: Option<&FrozenReadContextV1>,
) -> Result<Option<OperationalStateRecordV1>> {
    guard::text(collection)?;
    guard::text(record_key)?;
    Ok(engine.core().read_operational_state(collection, record_key, context)?)
}

/// One page of operational-state records under a frozen context.
pub fn operational_state_page(
    engine: &Engine,
    collection: &str,
    context: &FrozenReadContextV1,
    page: &PageRequestV1,
) -> Result<PageV1<OperationalStateRecordV1>> {
    guard::text(collection)?;
    Ok(engine.core().read_operational_state_page(collection, context, page)?)
}

/// Validity-window boundaries crossed since `since` (epoch ms).
pub fn crossed_boundary_since(
    engine: &Engine,
    since: i64,
    view: Option<&ReadView>,
) -> Result<Vec<BoundaryCrossing>> {
    Ok(engine.core().crossed_boundary_since(since, &view_or_default(view))?)
}

/// The declared projection registry.
pub fn projections(engine: &Engine) -> Result<Vec<ProjectionSpec>> {
    Ok(engine.core().read_projections()?)
}

/// The current projection runtime status.
pub fn projection_status(engine: &Engine) -> Result<ProjectionRuntimeStatus> {
    Ok(engine.core().read_projection_status()?)
}

/// Projection generation status.
pub fn projection_generation_status(engine: &Engine) -> Result<ProjectionGenerationStatusV1> {
    Ok(engine.core().read_projection_generation_status()?)
}

/// Projection status of one mutation.
pub fn mutation_projection_status(
    engine: &Engine,
    request: MutationProjectionStatusRequestV1,
) -> Result<MutationProjectionStatusV1> {
    Ok(engine.core().read_mutation_projection_status(request)?)
}

/// Embedding readiness of the dense projection.
pub fn embedding_readiness(engine: &Engine) -> Result<EmbeddingReadiness> {
    Ok(engine.core().read_embedding_readiness()?)
}

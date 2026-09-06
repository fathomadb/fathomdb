use std::fmt::{Display, Formatter};

use rusqlite::{Connection, OptionalExtension};
use serde::ser::{SerializeMap, Serializer as _};

use crate::{
    edge_fts_hit_passes_filter, frozen_read, load_dependency_generation, load_next_cursor,
    projection_generation, text_hit_passes_filter, valid_caller_identity, EngineError,
    FrozenReadContextV1, LifecycleState, ReadContextV1,
};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_RELATIONS: u32 = 100;
const DEFAULT_MAX_WORK_UNITS: u32 = 101;

type StoredNodeLifecycle = (String, String, Option<i64>, Option<i64>, Option<i64>);
type StoredEdgeLifecycle = (String, Option<i64>, Option<i64>, Option<i64>);
type StoredSourceLink =
    (i64, String, String, String, String, Option<i64>, Option<i64>, String, String);

/// Direction of a one-hop dependency trace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyTraceDirectionV1 {
    ToSource,
    ToDependents,
}

impl DependencyTraceDirectionV1 {
    /// Stable lower-snake-case wire spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ToSource => "to_source",
            Self::ToDependents => "to_dependents",
        }
    }
}

/// Artifact role exposed by dependency tracing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceArtifactRoleV1 {
    CanonicalSource,
    Derived,
}

impl TraceArtifactRoleV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalSource => "canonical_source",
            Self::Derived => "derived",
        }
    }
}

/// Class of a traced artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceArtifactClassV1 {
    Node,
    Edge,
}

impl TraceArtifactClassV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Edge => "edge",
        }
    }
}

/// Class-correct lifecycle facts for a visible traced artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceNodeLifecycleV1 {
    Node { schema_version: u32, state: LifecycleState, superseded: bool, valid_at_effective: bool },
    Edge { schema_version: u32, superseded: bool, valid_at_effective: bool },
}

/// One visible endpoint in a trace result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTraceNodeV1 {
    pub schema_version: u32,
    pub artifact_revision_id: String,
    pub artifact_class: TraceArtifactClassV1,
    pub role: TraceArtifactRoleV1,
    pub depth: u32,
    pub lifecycle: TraceNodeLifecycleV1,
}

/// One reciprocal registered dependency relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTraceEdgeV1 {
    pub schema_version: u32,
    pub dependency_id: String,
    pub source_revision_id: String,
    pub derived_revision_id: String,
    pub registered_dependency_generation: u64,
}

/// Database-local boundary observed by a trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReadBoundaryV1 {
    pub schema_version: u32,
    pub effective_at_epoch_s: i64,
    pub observed_write_boundary: u64,
    pub dependency_generation: u64,
    pub projection_generation_id: String,
}

/// Complete one-page dependency trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTraceResultV1 {
    pub schema_version: u32,
    pub root_revision_id: String,
    pub direction: DependencyTraceDirectionV1,
    pub nodes: Vec<DependencyTraceNodeV1>,
    pub dependency_edges: Vec<DependencyTraceEdgeV1>,
    pub checked_work_units: u32,
    pub complete: bool,
    pub read_boundary: TraceReadBoundaryV1,
}

/// Versioned, bounded dependency trace request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTraceRequestV1 {
    pub schema_version: u32,
    pub root_revision_id: String,
    pub direction: DependencyTraceDirectionV1,
    pub context: FrozenReadContextV1,
    pub max_relations: u32,
    pub max_work_units: u32,
}

impl DependencyTraceRequestV1 {
    /// Construct a version-1 request with the fixed default bounds.
    pub fn new(
        root_revision_id: impl Into<String>,
        direction: DependencyTraceDirectionV1,
        context: FrozenReadContextV1,
    ) -> Result<Self, DependencyTraceErrorV1> {
        let root_revision_id = root_revision_id.into();
        if !valid_caller_identity(&root_revision_id) {
            return Err(DependencyTraceErrorV1::new(
                DependencyTraceErrorReasonV1::TraceRootInvalid,
                "/rootRevisionId",
            ));
        }
        Ok(Self {
            schema_version: SCHEMA_VERSION,
            root_revision_id,
            direction,
            context,
            max_relations: DEFAULT_MAX_RELATIONS,
            max_work_units: DEFAULT_MAX_WORK_UNITS,
        })
    }

    /// Apply caller bounds within the fixed version-1 ceilings.
    pub fn with_bounds(
        mut self,
        max_relations: u32,
        max_work_units: u32,
    ) -> Result<Self, DependencyTraceErrorV1> {
        if !(1..=DEFAULT_MAX_RELATIONS).contains(&max_relations) {
            return Err(DependencyTraceErrorV1::new(
                DependencyTraceErrorReasonV1::TraceLimitInvalid,
                "/maxRelations",
            ));
        }
        if !(1..=DEFAULT_MAX_WORK_UNITS).contains(&max_work_units) {
            return Err(DependencyTraceErrorV1::new(
                DependencyTraceErrorReasonV1::TraceLimitInvalid,
                "/maxWorkUnits",
            ));
        }
        self.max_relations = max_relations;
        self.max_work_units = max_work_units;
        Ok(self)
    }
}

/// Closed reason for a dependency-trace refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DependencyTraceErrorReasonV1 {
    UnsupportedSchemaVersion,
    UnknownField,
    TraceRootInvalid,
    TraceDirectionInvalid,
    TraceLimitInvalid,
    TraceUnavailable,
    TraceRootRoleInvalid,
    TraceBoundExceeded,
    TraceCorrupt,
}

impl DependencyTraceErrorReasonV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::TraceRootInvalid => "trace_root_invalid",
            Self::TraceDirectionInvalid => "trace_direction_invalid",
            Self::TraceLimitInvalid => "trace_limit_invalid",
            Self::TraceUnavailable => "trace_unavailable",
            Self::TraceRootRoleInvalid => "trace_root_role_invalid",
            Self::TraceBoundExceeded => "trace_bound_exceeded",
            Self::TraceCorrupt => "trace_corrupt",
        }
    }
}

/// Privacy-safe typed trace error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyTraceErrorV1 {
    pub reason: DependencyTraceErrorReasonV1,
    pub field_path: String,
}

impl DependencyTraceErrorV1 {
    pub(crate) fn new(reason: DependencyTraceErrorReasonV1, path: impl Into<String>) -> Self {
        Self { reason, field_path: path.into() }
    }
}

impl Display for DependencyTraceErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for DependencyTraceErrorV1 {}

fn unavailable() -> EngineError {
    DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceUnavailable, "/rootRevisionId")
        .into()
}

fn visible_artifact(
    connection: &Connection,
    revision_id: &str,
    effective: i64,
    context: &ReadContextV1,
) -> Result<Option<DependencyTraceNodeV1>, EngineError> {
    let row: Option<(String, String, String, i64)> = connection
        .query_row(
            "SELECT artifact_class,artifact_role,completeness,write_cursor \
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1 AND schema_version=1",
            [revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((class, role, completeness, cursor)) = row else { return Ok(None) };
    if completeness != "complete" {
        return Ok(None);
    }
    let role = match role.as_str() {
        "canonical_source" => TraceArtifactRoleV1::CanonicalSource,
        "derived_semantic" => TraceArtifactRoleV1::Derived,
        _ => return Ok(None),
    };
    let (artifact_class, lifecycle, visible) = if class == "node" {
        let state: Option<StoredNodeLifecycle> = connection
            .query_row(
                "SELECT kind,state,superseded_at,valid_from,valid_until FROM canonical_nodes \
                 WHERE write_cursor=?1",
                [cursor],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((kind, state, superseded, valid_from, valid_until)) = state else {
            return Ok(None);
        };
        let state = LifecycleState::from_str_opt(&state).ok_or(EngineError::Storage)?;
        let valid = valid_from.is_none_or(|start| start <= effective)
            && valid_until.is_none_or(|end| end > effective);
        let visible = (context.view.include_inactive || state == LifecycleState::Active)
            && (context.view.include_superseded || superseded.is_none())
            && (context.view.include_out_of_window || valid)
            && text_hit_passes_filter(
                connection,
                u64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                &kind,
                Some(&context.eligibility),
            )
            .map_err(|_| EngineError::Storage)?;
        (
            TraceArtifactClassV1::Node,
            TraceNodeLifecycleV1::Node {
                schema_version: 1,
                state,
                superseded: superseded.is_some(),
                valid_at_effective: valid,
            },
            visible,
        )
    } else if class == "edge" {
        let state: Option<StoredEdgeLifecycle> = connection
            .query_row(
                "SELECT kind,superseded_at,t_valid,t_invalid FROM canonical_edges \
                 WHERE write_cursor=?1",
                [cursor],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((kind, superseded, valid_from, valid_until)) = state else {
            return Ok(None);
        };
        let valid = valid_from.is_none_or(|start| start <= effective)
            && valid_until.is_none_or(|end| end > effective);
        let visible = (context.view.include_superseded || superseded.is_none())
            && (context.view.include_out_of_window || valid)
            && edge_fts_hit_passes_filter(
                connection,
                u64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                &kind,
                Some(&context.eligibility),
            )
            .map_err(|_| EngineError::Storage)?;
        (
            TraceArtifactClassV1::Edge,
            TraceNodeLifecycleV1::Edge {
                schema_version: 1,
                superseded: superseded.is_some(),
                valid_at_effective: valid,
            },
            visible,
        )
    } else {
        return Ok(None);
    };
    if !visible {
        return Ok(None);
    }
    Ok(Some(DependencyTraceNodeV1 {
        schema_version: 1,
        artifact_revision_id: revision_id.to_string(),
        artifact_class,
        role,
        depth: 0,
        lifecycle,
    }))
}

type StoredDependencyCandidate = (String, String, String, i64, i64);

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn source_link_valid(
    connection: &Connection,
    artifact_revision_id: &str,
    expected_source_revision_id: Option<&str>,
) -> Result<bool, EngineError> {
    let row: Option<StoredSourceLink> = connection
        .query_row(
            "SELECT schema_version,source_id,source_version_id,source_revision_id,locator_kind,\
                    start_byte,end_byte,hash_algorithm,hash_digest \
             FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
            [artifact_revision_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((schema, source_id, version_id, source_revision, locator, start, end, algo, digest)) =
        row
    else {
        return Ok(false);
    };
    Ok(schema == 1
        && valid_caller_identity(&source_id)
        && valid_caller_identity(&version_id)
        && valid_caller_identity(&source_revision)
        && expected_source_revision_id.is_none_or(|expected| source_revision == expected)
        && ((locator == "whole_body" && start.is_none() && end.is_none())
            || (locator == "utf8_bytes"
                && start.is_some_and(|value| value >= 0)
                && end.is_some_and(|value| value >= 0)
                && start < end))
        && algo == "sha256"
        && valid_hash(&digest))
}

fn canonical_source_chain_valid(
    connection: &Connection,
    source_revision_id: &str,
) -> Result<bool, EngineError> {
    if !source_link_valid(connection, source_revision_id, Some(source_revision_id))? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_versions v \
             JOIN _fathomdb_source_links l \
               ON l.artifact_revision_id=v.source_revision_id \
              AND l.source_id=v.source_id AND l.source_version_id=v.source_version_id \
             WHERE v.source_revision_id=?1 AND v.schema_version=1 \
               AND l.schema_version=1 AND l.source_revision_id=?1 \
               AND l.locator_kind='whole_body' AND l.start_byte IS NULL \
               AND l.end_byte IS NULL AND l.hash_algorithm='sha256')",
            [source_revision_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)
}

fn include_candidate(
    connection: &Connection,
    request: &DependencyTraceRequestV1,
    effective: i64,
    dependency_generation: u64,
    candidate: StoredDependencyCandidate,
    nodes: &mut Vec<DependencyTraceNodeV1>,
    edges: &mut Vec<DependencyTraceEdgeV1>,
) -> Result<(), EngineError> {
    let (dependency_id, source_revision_id, derived_revision_id, generation, schema) = candidate;
    let Some(source) =
        visible_artifact(connection, &source_revision_id, effective, &request.context.context)?
    else {
        return Ok(());
    };
    let Some(derived) =
        visible_artifact(connection, &derived_revision_id, effective, &request.context.context)?
    else {
        return Ok(());
    };
    if source.role != TraceArtifactRoleV1::CanonicalSource
        || derived.role != TraceArtifactRoleV1::Derived
        || !canonical_source_chain_valid(connection, &source_revision_id)?
        || !source_link_valid(connection, &derived_revision_id, Some(&source_revision_id))?
    {
        return Ok(());
    }
    if schema != 1
        || !valid_caller_identity(&dependency_id)
        || generation <= 0
        || u64::try_from(generation).ok().is_none_or(|value| value > dependency_generation)
    {
        return Err(
            DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, "").into()
        );
    }
    if edges.len() >= request.max_relations as usize
        || 1usize.saturating_add(edges.len()) >= request.max_work_units as usize
    {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::TraceBoundExceeded,
            "",
        )
        .into());
    }
    let mut counterpart = match request.direction {
        DependencyTraceDirectionV1::ToSource => source,
        DependencyTraceDirectionV1::ToDependents => derived,
    };
    counterpart.depth = 1;
    edges.push(DependencyTraceEdgeV1 {
        schema_version: 1,
        dependency_id,
        source_revision_id,
        derived_revision_id,
        registered_dependency_generation: u64::try_from(generation)
            .map_err(|_| EngineError::Storage)?,
    });
    nodes.push(counterpart);
    Ok(())
}

pub(crate) fn execute(
    connection: &mut Connection,
    request: DependencyTraceRequestV1,
) -> Result<DependencyTraceResultV1, EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        )
        .into());
    }
    let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
    let binding = frozen_read::authenticate(&tx, &request.context)?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    let effective = request.context.effective_valid_at;
    let Some(mut root) =
        visible_artifact(&tx, &request.root_revision_id, effective, &request.context.context)?
    else {
        return Err(unavailable());
    };
    if crate::dependency_closure::guard_no_pending_physical(&tx).is_err()
        || (root.role == TraceArtifactRoleV1::CanonicalSource
            && !canonical_source_chain_valid(&tx, &request.root_revision_id)?)
        || (root.role == TraceArtifactRoleV1::Derived
            && !source_link_valid(&tx, &request.root_revision_id, None)?)
    {
        return Err(unavailable());
    }
    let role_ok = matches!(
        (request.direction, root.role),
        (DependencyTraceDirectionV1::ToSource, TraceArtifactRoleV1::Derived)
            | (DependencyTraceDirectionV1::ToDependents, TraceArtifactRoleV1::CanonicalSource)
    );
    if !role_ok {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::TraceRootRoleInvalid,
            "/rootRevisionId",
        )
        .into());
    }
    root.depth = 0;
    let dependency_generation = load_dependency_generation(&tx)?;
    let mut nodes = vec![root];
    let mut edges = Vec::new();
    match request.direction {
        DependencyTraceDirectionV1::ToSource => {
            let candidate = tx
                .query_row(
                    "SELECT d.dependency_id,l.source_revision_id,d.derived_revision_id,\
                            d.registered_dependency_generation,d.schema_version \
                     FROM _fathomdb_source_dependencies d \
                     JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id \
                     WHERE d.derived_revision_id=?1 LIMIT 2",
                    [&request.root_revision_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, i64>(3)?,
                            row.get::<_, i64>(4)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            if let Some(candidate) = candidate {
                include_candidate(
                    &tx,
                    &request,
                    effective,
                    dependency_generation,
                    candidate,
                    &mut nodes,
                    &mut edges,
                )?;
            }
        }
        DependencyTraceDirectionV1::ToDependents => {
            let mut statement = tx
                .prepare(
                    "SELECT d.dependency_id,l.source_revision_id,d.derived_revision_id,\
                            d.registered_dependency_generation,d.schema_version \
                     FROM _fathomdb_source_links l INDEXED BY _fathomdb_source_links_source_derived_idx \
                     JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=l.artifact_revision_id \
                     WHERE l.source_revision_id=?1 \
                     ORDER BY l.source_revision_id,l.artifact_revision_id",
                )
                .map_err(|_| EngineError::Storage)?;
            let mut rows =
                statement.query([&request.root_revision_id]).map_err(|_| EngineError::Storage)?;
            while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
                let candidate = (
                    row.get::<_, String>(0).map_err(|_| EngineError::Storage)?,
                    row.get::<_, String>(1).map_err(|_| EngineError::Storage)?,
                    row.get::<_, String>(2).map_err(|_| EngineError::Storage)?,
                    row.get::<_, i64>(3).map_err(|_| EngineError::Storage)?,
                    row.get::<_, i64>(4).map_err(|_| EngineError::Storage)?,
                );
                include_candidate(
                    &tx,
                    &request,
                    effective,
                    dependency_generation,
                    candidate,
                    &mut nodes,
                    &mut edges,
                )?;
            }
        }
    }
    nodes[1..].sort_by(|left, right| left.artifact_revision_id.cmp(&right.artifact_revision_id));
    edges.sort_by(|left, right| {
        (&left.derived_revision_id, &left.dependency_id)
            .cmp(&(&right.derived_revision_id, &right.dependency_id))
    });
    let boundary = TraceReadBoundaryV1 {
        schema_version: 1,
        effective_at_epoch_s: effective,
        observed_write_boundary: load_next_cursor(&tx),
        dependency_generation,
        projection_generation_id: projection_generation::current_generation_id(&tx)?
            .as_str()
            .to_string(),
    };
    tx.commit().map_err(|_| EngineError::Storage)?;
    Ok(DependencyTraceResultV1 {
        schema_version: 1,
        root_revision_id: request.root_revision_id,
        direction: request.direction,
        checked_work_units: 1 + u32::try_from(edges.len()).unwrap_or(u32::MAX),
        nodes,
        dependency_edges: edges,
        complete: true,
        read_boundary: boundary,
    })
}

/// Encode a trace response into canonical declaration-order JSON bytes.
pub fn encode_dependency_trace_result_v1(
    value: &DependencyTraceResultV1,
) -> Result<Vec<u8>, DependencyTraceErrorV1> {
    fn lifecycle(value: &TraceNodeLifecycleV1) -> serde_json::Value {
        match value {
            TraceNodeLifecycleV1::Node {
                schema_version,
                state,
                superseded,
                valid_at_effective,
            } => serde_json::json!({
                "schemaVersion": schema_version,
                "artifactClass": "node",
                "state": state.as_str(),
                "superseded": superseded,
                "validAtEffective": valid_at_effective,
            }),
            TraceNodeLifecycleV1::Edge { schema_version, superseded, valid_at_effective } => {
                serde_json::json!({
                    "schemaVersion": schema_version,
                    "artifactClass": "edge",
                    "superseded": superseded,
                    "validAtEffective": valid_at_effective,
                })
            }
        }
    }
    let nodes = value
        .nodes
        .iter()
        .map(|node| {
            serde_json::json!({
                "schemaVersion": node.schema_version,
                "artifactRevisionId": node.artifact_revision_id,
                "artifactClass": node.artifact_class.as_str(),
                "role": node.role.as_str(),
                "depth": node.depth,
                "lifecycle": lifecycle(&node.lifecycle),
            })
        })
        .collect::<Vec<_>>();
    let edges = value
        .dependency_edges
        .iter()
        .map(|edge| {
            serde_json::json!({
                "schemaVersion": edge.schema_version,
                "dependencyId": edge.dependency_id,
                "sourceRevisionId": edge.source_revision_id,
                "derivedRevisionId": edge.derived_revision_id,
                "registeredDependencyGeneration": edge.registered_dependency_generation.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    let mut serializer = serde_json::Serializer::new(&mut output);
    let mut object = serializer
        .serialize_map(Some(8))
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("schemaVersion", &value.schema_version)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("rootRevisionId", &value.root_revision_id)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("direction", value.direction.as_str())
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("nodes", &nodes)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("dependencyEdges", &edges)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("checkedWorkUnits", &value.checked_work_units)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry("complete", &value.complete)
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .serialize_entry(
            "readBoundary",
            &serde_json::json!({
                "schemaVersion": value.read_boundary.schema_version,
                "effectiveAtEpochS": value.read_boundary.effective_at_epoch_s,
                "observedWriteBoundary": value.read_boundary.observed_write_boundary.to_string(),
                "dependencyGeneration": value.read_boundary.dependency_generation.to_string(),
                "projectionGenerationId": value.read_boundary.projection_generation_id,
            }),
        )
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    object
        .end()
        .map_err(|_| DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, ""))?;
    Ok(output)
}

/// Decode a canonical trace response. Unknown additive response members are ignored.
pub fn decode_dependency_trace_result_v1(
    bytes: &[u8],
) -> Result<DependencyTraceResultV1, DependencyTraceErrorV1> {
    fn corrupt(path: impl Into<String>) -> DependencyTraceErrorV1 {
        DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, path)
    }

    fn required<'a>(
        object: &'a serde_json::Map<String, serde_json::Value>,
        field: &str,
        path: &str,
    ) -> Result<&'a serde_json::Value, DependencyTraceErrorV1> {
        object.get(field).ok_or_else(|| corrupt(path))
    }

    fn object<'a>(
        value: &'a serde_json::Value,
        path: &str,
    ) -> Result<&'a serde_json::Map<String, serde_json::Value>, DependencyTraceErrorV1> {
        value.as_object().ok_or_else(|| corrupt(path))
    }

    fn schema(
        object: &serde_json::Map<String, serde_json::Value>,
        path: &str,
    ) -> Result<(), DependencyTraceErrorV1> {
        if required(object, "schemaVersion", path)?.as_u64() != Some(1) {
            return Err(DependencyTraceErrorV1::new(
                DependencyTraceErrorReasonV1::UnsupportedSchemaVersion,
                path,
            ));
        }
        Ok(())
    }

    fn u32_at(value: &serde_json::Value, path: &str) -> Result<u32, DependencyTraceErrorV1> {
        value.as_u64().and_then(|number| u32::try_from(number).ok()).ok_or_else(|| corrupt(path))
    }

    fn canonical_u64(value: &serde_json::Value, path: &str) -> Result<u64, DependencyTraceErrorV1> {
        let text = value.as_str().ok_or_else(|| corrupt(path))?;
        if text.is_empty()
            || (text.len() > 1 && text.starts_with('0'))
            || !text.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(corrupt(path));
        }
        text.parse().map_err(|_| corrupt(path))
    }

    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| corrupt(""))?;
    let root = object(&value, "")?;
    if required(root, "schemaVersion", "/schemaVersion")?.as_u64() != Some(1) {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    let root_revision_id = required(root, "rootRevisionId", "/rootRevisionId")?
        .as_str()
        .filter(|value| valid_caller_identity(value))
        .ok_or_else(|| corrupt("/rootRevisionId"))?
        .to_string();
    let direction = match required(root, "direction", "/direction")?.as_str() {
        Some("to_source") => DependencyTraceDirectionV1::ToSource,
        Some("to_dependents") => DependencyTraceDirectionV1::ToDependents,
        _ => return Err(corrupt("/direction")),
    };
    let node_values =
        required(root, "nodes", "/nodes")?.as_array().ok_or_else(|| corrupt("/nodes"))?;
    if node_values.is_empty() {
        return Err(corrupt("/nodes"));
    }
    let mut nodes = Vec::new();
    for (index, node_value) in node_values.iter().enumerate() {
        let base = format!("/nodes/{index}");
        let node = object(node_value, &base)?;
        schema(node, &format!("{base}/schemaVersion"))?;
        let artifact_revision_id =
            required(node, "artifactRevisionId", &format!("{base}/artifactRevisionId"))?
                .as_str()
                .filter(|value| valid_caller_identity(value))
                .ok_or_else(|| corrupt(format!("{base}/artifactRevisionId")))?
                .to_string();
        let class =
            match required(node, "artifactClass", &format!("{base}/artifactClass"))?.as_str() {
                Some("node") => TraceArtifactClassV1::Node,
                Some("edge") => TraceArtifactClassV1::Edge,
                _ => return Err(corrupt(format!("{base}/artifactClass"))),
            };
        let role = match required(node, "role", &format!("{base}/role"))?.as_str() {
            Some("canonical_source") => TraceArtifactRoleV1::CanonicalSource,
            Some("derived") => TraceArtifactRoleV1::Derived,
            _ => return Err(corrupt(format!("{base}/role"))),
        };
        let depth =
            u32_at(required(node, "depth", &format!("{base}/depth"))?, &format!("{base}/depth"))?;
        let lifecycle_path = format!("{base}/lifecycle");
        let lifecycle_value =
            object(required(node, "lifecycle", &lifecycle_path)?, &lifecycle_path)?;
        schema(lifecycle_value, &format!("{lifecycle_path}/schemaVersion"))?;
        let lifecycle_class =
            required(lifecycle_value, "artifactClass", &format!("{lifecycle_path}/artifactClass"))?
                .as_str();
        if lifecycle_class != Some(class.as_str()) {
            return Err(corrupt(format!("{lifecycle_path}/artifactClass")));
        }
        let superseded =
            required(lifecycle_value, "superseded", &format!("{lifecycle_path}/superseded"))?
                .as_bool()
                .ok_or_else(|| corrupt(format!("{lifecycle_path}/superseded")))?;
        let valid_at_effective = required(
            lifecycle_value,
            "validAtEffective",
            &format!("{lifecycle_path}/validAtEffective"),
        )?
        .as_bool()
        .ok_or_else(|| corrupt(format!("{lifecycle_path}/validAtEffective")))?;
        let lifecycle = if class == TraceArtifactClassV1::Node {
            let state = LifecycleState::from_str_opt(
                required(lifecycle_value, "state", &format!("{lifecycle_path}/state"))?
                    .as_str()
                    .unwrap_or(""),
            )
            .ok_or_else(|| corrupt(format!("{lifecycle_path}/state")))?;
            TraceNodeLifecycleV1::Node { schema_version: 1, state, superseded, valid_at_effective }
        } else {
            if lifecycle_value.contains_key("state") {
                return Err(corrupt(format!("{lifecycle_path}/state")));
            }
            TraceNodeLifecycleV1::Edge { schema_version: 1, superseded, valid_at_effective }
        };
        nodes.push(DependencyTraceNodeV1 {
            schema_version: 1,
            artifact_revision_id,
            artifact_class: class,
            role,
            depth,
            lifecycle,
        });
    }

    let edge_values = required(root, "dependencyEdges", "/dependencyEdges")?
        .as_array()
        .ok_or_else(|| corrupt("/dependencyEdges"))?;
    let mut dependency_edges = Vec::with_capacity(edge_values.len());
    for (index, edge_value) in edge_values.iter().enumerate() {
        let base = format!("/dependencyEdges/{index}");
        let edge = object(edge_value, &base)?;
        schema(edge, &format!("{base}/schemaVersion"))?;
        let text = |name: &str| -> Result<String, DependencyTraceErrorV1> {
            let path = format!("{base}/{name}");
            required(edge, name, &path)?
                .as_str()
                .filter(|value| valid_caller_identity(value))
                .map(str::to_string)
                .ok_or_else(|| corrupt(path))
        };
        dependency_edges.push(DependencyTraceEdgeV1 {
            schema_version: 1,
            dependency_id: text("dependencyId")?,
            source_revision_id: text("sourceRevisionId")?,
            derived_revision_id: text("derivedRevisionId")?,
            registered_dependency_generation: canonical_u64(
                required(
                    edge,
                    "registeredDependencyGeneration",
                    &format!("{base}/registeredDependencyGeneration"),
                )?,
                &format!("{base}/registeredDependencyGeneration"),
            )?,
        });
    }
    let checked_work_units =
        u32_at(required(root, "checkedWorkUnits", "/checkedWorkUnits")?, "/checkedWorkUnits")?;
    if required(root, "complete", "/complete")?.as_bool() != Some(true) {
        return Err(corrupt("/complete"));
    }
    if checked_work_units as usize != dependency_edges.len() + 1
        || nodes.len() != dependency_edges.len() + 1
    {
        return Err(corrupt("/checkedWorkUnits"));
    }
    if nodes[0].artifact_revision_id != root_revision_id || nodes[0].depth != 0 {
        return Err(corrupt("/nodes/0"));
    }
    if nodes.iter().skip(1).any(|node| node.depth != 1) {
        return Err(corrupt("/nodes"));
    }

    let boundary = object(required(root, "readBoundary", "/readBoundary")?, "/readBoundary")?;
    schema(boundary, "/readBoundary/schemaVersion")?;
    let projection_generation_id =
        required(boundary, "projectionGenerationId", "/readBoundary/projectionGenerationId")?
            .as_str()
            .filter(|value| {
                value.len() == 38
                    && value.starts_with("pgen1:")
                    && value[6..]
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| corrupt("/readBoundary/projectionGenerationId"))?
            .to_string();
    Ok(DependencyTraceResultV1 {
        schema_version: 1,
        root_revision_id,
        direction,
        nodes,
        dependency_edges,
        checked_work_units,
        complete: true,
        read_boundary: TraceReadBoundaryV1 {
            schema_version: 1,
            effective_at_epoch_s: required(
                boundary,
                "effectiveAtEpochS",
                "/readBoundary/effectiveAtEpochS",
            )?
            .as_i64()
            .ok_or_else(|| corrupt("/readBoundary/effectiveAtEpochS"))?,
            observed_write_boundary: canonical_u64(
                required(boundary, "observedWriteBoundary", "/readBoundary/observedWriteBoundary")?,
                "/readBoundary/observedWriteBoundary",
            )?,
            dependency_generation: canonical_u64(
                required(boundary, "dependencyGeneration", "/readBoundary/dependencyGeneration")?,
                "/readBoundary/dependencyGeneration",
            )?,
            projection_generation_id,
        },
    })
}

#[cfg(feature = "test-hooks")]
pub fn encode_dependency_trace_root_for_test(root: &str) -> Result<Vec<u8>, EngineError> {
    if !valid_caller_identity(root) {
        return Err(EngineError::WriteValidation);
    }
    Ok(root.as_bytes().to_vec())
}

#[cfg(feature = "test-hooks")]
pub fn decode_dependency_trace_root_for_test(bytes: &[u8]) -> Result<String, EngineError> {
    String::from_utf8(bytes.to_vec()).map_err(|_| EngineError::WriteValidation)
}

use std::fmt::{Display, Formatter};

use rusqlite::{Connection, OptionalExtension};
use serde::{
    ser::{SerializeMap, Serializer as _},
    Serialize,
};
use sha2::{Digest, Sha256};

use crate::{
    edge_fts_hit_passes_filter, frozen_read, load_dependency_generation, load_next_cursor,
    projection_generation, text_hit_passes_filter, valid_caller_identity, EngineError,
    FrozenReadContextV1, LifecycleState, ReadContextV1,
};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_RELATIONS: u32 = 100;
const DEFAULT_MAX_WORK_UNITS: u32 = 101;

const TO_SOURCE_CANDIDATE_QUERY: &str = "SELECT d.rowid,d.derived_revision_id \
     FROM _fathomdb_source_dependencies d \
     JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id \
     JOIN _fathomdb_artifact_revisions sr ON sr.revision_id=l.source_revision_id \
     JOIN canonical_nodes sn ON sn.write_cursor=sr.write_cursor \
     WHERE d.derived_revision_id=?1 AND typeof(d.derived_revision_id)='text' \
       AND typeof(l.source_revision_id)='text' AND sr.schema_version=1 \
       AND sr.artifact_class='node' AND sr.artifact_role='canonical_source' \
       AND sr.completeness='complete' \
       AND sn.state IN ('active','pending','deleted','purged') \
       AND (?3 OR sn.state='active') AND (?4 OR sn.superseded_at IS NULL) \
       AND (?5 OR ((sn.valid_from IS NULL OR sn.valid_from<=?6) \
                   AND (sn.valid_until IS NULL OR sn.valid_until>?6))) \
       AND (?7 IS NULL OR sn.kind=?7) \
       AND (?8 IS NULL OR CASE sn.kind WHEN 'email' THEN 'email' \
            WHEN 'article' THEN 'article' WHEN 'paper' THEN 'paper' \
            WHEN 'meeting' THEN 'meeting' WHEN 'note' THEN 'note' \
            WHEN 'todo' THEN 'todo' WHEN 'doc' THEN 'article' END=?8) \
       AND (?9 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
            WHERE vm.rowid=sn.write_cursor AND vm.created_at>=?9)) \
       AND (?10 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
            WHERE vm.rowid=sn.write_cursor AND vm.status=?10)) \
       AND NOT EXISTS(SELECT 1 FROM json_each(?11) f WHERE NOT EXISTS(\
            SELECT 1 FROM canonical_attributes ca WHERE ca.write_cursor=sn.write_cursor \
              AND ca.attr_name=json_extract(f.value,'$[0]') \
              AND ca.attr_value=json_extract(f.value,'$[1]'))) \
     ORDER BY d.derived_revision_id,d.dependency_id LIMIT ?2";

const TO_DEPENDENTS_CANDIDATE_QUERY: &str = "SELECT d.rowid,l.artifact_revision_id \
     FROM _fathomdb_source_links l INDEXED BY _fathomdb_source_links_source_derived_idx \
     JOIN _fathomdb_source_dependencies d ON d.derived_revision_id=l.artifact_revision_id \
     JOIN _fathomdb_artifact_revisions dr ON dr.revision_id=d.derived_revision_id \
     LEFT JOIN canonical_nodes dn ON dr.artifact_class='node' AND dn.write_cursor=dr.write_cursor \
     LEFT JOIN canonical_edges de ON dr.artifact_class='edge' AND de.write_cursor=dr.write_cursor \
     WHERE l.source_revision_id=?1 AND l.artifact_revision_id>?2 \
       AND typeof(l.source_revision_id)='text' \
       AND typeof(l.artifact_revision_id)='text' \
       AND typeof(d.derived_revision_id)='text' \
       AND dr.schema_version=1 AND dr.artifact_role='derived_semantic' \
       AND dr.completeness='complete' AND (\
         (dr.artifact_class='node' AND dn.state IN ('active','pending','deleted','purged') \
          AND (?4 OR dn.state='active') AND (?5 OR dn.superseded_at IS NULL) \
          AND (?6 OR ((dn.valid_from IS NULL OR dn.valid_from<=?7) \
                      AND (dn.valid_until IS NULL OR dn.valid_until>?7))) \
          AND (?8 IS NULL OR dn.kind=?8) \
          AND (?9 IS NULL OR CASE dn.kind WHEN 'email' THEN 'email' \
               WHEN 'article' THEN 'article' WHEN 'paper' THEN 'paper' \
               WHEN 'meeting' THEN 'meeting' WHEN 'note' THEN 'note' \
               WHEN 'todo' THEN 'todo' WHEN 'doc' THEN 'article' END=?9) \
          AND (?10 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
               WHERE vm.rowid=dn.write_cursor AND vm.created_at>=?10)) \
          AND (?11 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
               WHERE vm.rowid=dn.write_cursor AND vm.status=?11)) \
          AND NOT EXISTS(SELECT 1 FROM json_each(?12) f WHERE NOT EXISTS(\
               SELECT 1 FROM canonical_attributes ca WHERE ca.write_cursor=dn.write_cursor \
                 AND ca.attr_name=json_extract(f.value,'$[0]') \
                 AND ca.attr_value=json_extract(f.value,'$[1]')))) OR \
         (dr.artifact_class='edge' AND (?5 OR de.superseded_at IS NULL) \
          AND (?6 OR ((de.t_valid IS NULL OR de.t_valid<=?7) \
                      AND (de.t_invalid IS NULL OR de.t_invalid>?7))) \
          AND (?8 IS NULL OR de.kind=?8) AND (?9 IS NULL OR ?9='edge_fact') \
          AND (?10 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
               WHERE vm.rowid=de.write_cursor AND vm.created_at>=?10)) \
          AND (?11 IS NULL OR EXISTS(SELECT 1 FROM vector_default vm \
               WHERE vm.rowid=de.write_cursor AND vm.status=?11)) \
          AND NOT EXISTS(SELECT 1 FROM json_each(?12) f WHERE NOT EXISTS(\
               SELECT 1 FROM canonical_attributes ca WHERE ca.write_cursor=de.write_cursor \
                 AND ca.attr_name=json_extract(f.value,'$[0]') \
                 AND ca.attr_value=json_extract(f.value,'$[1]'))))) \
     ORDER BY l.source_revision_id,l.artifact_revision_id LIMIT ?3";

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
    pub schema_version: u32,
    pub reason: DependencyTraceErrorReasonV1,
    pub field_path: String,
}

impl DependencyTraceErrorV1 {
    pub(crate) fn new(reason: DependencyTraceErrorReasonV1, path: impl Into<String>) -> Self {
        Self { schema_version: SCHEMA_VERSION, reason, field_path: path.into() }
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
             FROM _fathomdb_artifact_revisions WHERE revision_id=?1 AND schema_version=1 \
               AND typeof(artifact_class)='text' AND typeof(artifact_role)='text' \
               AND typeof(completeness)='text' AND typeof(write_cursor)='integer'",
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
                 WHERE write_cursor=?1 AND typeof(kind)='text' AND typeof(state)='text' \
                   AND (superseded_at IS NULL OR typeof(superseded_at)='integer') \
                   AND (valid_from IS NULL OR typeof(valid_from)='integer') \
                   AND (valid_until IS NULL OR typeof(valid_until)='integer')",
                [cursor],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let Some((kind, state, superseded, valid_from, valid_until)) = state else {
            return Ok(None);
        };
        let Some(state) = LifecycleState::from_str_opt(&state) else {
            return Ok(None);
        };
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
                 WHERE write_cursor=?1 AND typeof(kind)='text' \
                   AND (superseded_at IS NULL OR typeof(superseded_at)='integer') \
                   AND (t_valid IS NULL OR typeof(t_valid)='integer') \
                   AND (t_invalid IS NULL OR typeof(t_invalid)='integer')",
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
type StoredDependencyCandidateKey = (i64, String);
type GuardedDependencyCandidate =
    (Option<String>, Option<String>, Option<String>, Option<i64>, Option<i64>);

#[cfg(feature = "test-hooks")]
pub(crate) fn candidate_queries_for_test() -> [&'static str; 2] {
    [TO_SOURCE_CANDIDATE_QUERY, TO_DEPENDENTS_CANDIDATE_QUERY]
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn source_link_valid(
    connection: &Connection,
    artifact_revision_id: &str,
    expected_source_revision_id: Option<&str>,
) -> Result<bool, EngineError> {
    let shape_decodable: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_links \
             WHERE artifact_revision_id=?1 \
               AND typeof(schema_version)='integer' \
               AND typeof(source_id)='text' \
               AND typeof(source_version_id)='text' \
               AND typeof(source_revision_id)='text' \
               AND typeof(locator_kind)='text' \
               AND (start_byte IS NULL OR typeof(start_byte)='integer') \
               AND (end_byte IS NULL OR typeof(end_byte)='integer') \
               AND typeof(hash_algorithm)='text' \
               AND typeof(hash_digest)='text')",
            [artifact_revision_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !shape_decodable {
        return Ok(false);
    }
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
    let shape_valid = schema == 1
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
        && valid_hash(&digest);
    if !shape_valid {
        return Ok(false);
    }
    let canonical: Option<(String, String)> = connection
        .query_row(
            "SELECT n.source_id,n.body FROM _fathomdb_artifact_revisions r \
             JOIN canonical_nodes n ON n.write_cursor=r.write_cursor \
             WHERE r.revision_id=?1 AND r.schema_version=1 \
               AND r.artifact_class='node' AND r.artifact_role='canonical_source' \
               AND r.completeness='complete' \
               AND typeof(n.source_id)='text' AND typeof(n.body)='text'",
            [&source_revision],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((canonical_source_id, canonical_body)) = canonical else {
        return Ok(false);
    };
    if canonical_source_id != source_id {
        return Ok(false);
    }
    let version_valid: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_versions \
             WHERE schema_version=1 AND source_revision_id=?1 \
               AND source_id=?2 AND source_version_id=?3)",
            rusqlite::params![source_revision, source_id, version_id],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !version_valid {
        return Ok(false);
    }
    let selected = match (locator.as_str(), start, end) {
        ("whole_body", None, None) => Some(canonical_body.as_bytes()),
        ("utf8_bytes", Some(start), Some(end)) => {
            usize::try_from(start).ok().zip(usize::try_from(end).ok()).and_then(|(start, end)| {
                (canonical_body.is_char_boundary(start) && canonical_body.is_char_boundary(end))
                    .then(|| canonical_body.as_bytes().get(start..end))
                    .flatten()
            })
        }
        _ => None,
    };
    Ok(selected.is_some_and(|bytes| {
        Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>() == digest
    }))
}

pub(crate) fn canonical_source_chain_valid(
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

pub(crate) fn registered_dependency_for_cursor(
    connection: &Connection,
    write_cursor: u64,
    effective_at: i64,
) -> Result<bool, EngineError> {
    let cursor = i64::try_from(write_cursor).map_err(|_| EngineError::Storage)?;
    let derived_revision_id: Option<String> = connection
        .query_row(
            "SELECT revision_id FROM _fathomdb_artifact_revisions \
             WHERE write_cursor=?1 AND schema_version=1 \
               AND artifact_role='derived_semantic' AND completeness='complete'",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some(derived_revision_id) = derived_revision_id else { return Ok(false) };
    let dependency: Option<(i64, String, String, i64)> = connection
        .query_row(
            "SELECT d.schema_version,d.dependency_id,l.source_revision_id,\
                    d.registered_dependency_generation \
             FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id \
             WHERE d.derived_revision_id=?1",
            [&derived_revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((schema, dependency_id, source_revision_id, generation)) = dependency else {
        return Ok(false);
    };
    let current_generation = load_dependency_generation(connection)?;
    if schema != 1
        || !valid_caller_identity(&dependency_id)
        || generation <= 0
        || u64::try_from(generation).ok().is_none_or(|value| value > current_generation)
        || !source_link_valid(connection, &derived_revision_id, Some(&source_revision_id))?
        || !canonical_source_chain_valid(connection, &source_revision_id)?
        || crate::dependency_closure::active_barrier_for_source(connection, &source_revision_id)?
        || !crate::dependency_closure::source_revision_is_strictly_eligible(
            connection,
            &source_revision_id,
            effective_at,
        )?
    {
        return Ok(false);
    }
    Ok(true)
}

fn root_chain_role(
    connection: &Connection,
    revision_id: &str,
) -> Result<Option<TraceArtifactRoleV1>, EngineError> {
    let role: Option<String> = connection
        .query_row(
            "SELECT artifact_role FROM _fathomdb_artifact_revisions \
             WHERE revision_id=?1 AND schema_version=1 \
               AND typeof(artifact_role)='text'",
            [revision_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some(role) = role else { return Ok(None) };
    let (role, source_revision_id) = match role.as_str() {
        "canonical_source" => (TraceArtifactRoleV1::CanonicalSource, revision_id.to_string()),
        "derived_semantic" => {
            let source_revision_id: Option<String> = connection
                .query_row(
                    "SELECT source_revision_id FROM _fathomdb_source_links \
                     WHERE artifact_revision_id=?1 AND typeof(source_revision_id)='text'",
                    [revision_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let Some(source_revision_id) = source_revision_id else { return Ok(None) };
            if !source_link_valid(connection, revision_id, Some(&source_revision_id))? {
                return Ok(None);
            }
            (TraceArtifactRoleV1::Derived, source_revision_id)
        }
        _ => return Ok(None),
    };
    if !canonical_source_chain_valid(connection, &source_revision_id)?
        || crate::dependency_closure::active_barrier_for_source(connection, &source_revision_id)?
    {
        return Ok(None);
    }
    Ok(Some(role))
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
        || crate::dependency_closure::active_barrier_for_source(connection, &source_revision_id)?
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

fn authorize_candidate_chain(
    connection: &Connection,
    request: &DependencyTraceRequestV1,
    effective: i64,
    rowid: i64,
) -> Result<Option<(DependencyTraceNodeV1, DependencyTraceNodeV1)>, EngineError> {
    let endpoints: Option<(String, String)> = connection
        .query_row(
            "SELECT l.source_revision_id,d.derived_revision_id \
             FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id \
             WHERE d.rowid=?1 AND typeof(l.source_revision_id)='text' \
               AND typeof(d.derived_revision_id)='text'",
            [rowid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((source_revision_id, derived_revision_id)) = endpoints else {
        return Ok(None);
    };
    if !valid_caller_identity(&source_revision_id)
        || !valid_caller_identity(&derived_revision_id)
        || (request.direction == DependencyTraceDirectionV1::ToSource
            && derived_revision_id != request.root_revision_id)
        || (request.direction == DependencyTraceDirectionV1::ToDependents
            && source_revision_id != request.root_revision_id)
    {
        return Ok(None);
    }
    if !canonical_source_chain_valid(connection, &source_revision_id)?
        || !source_link_valid(connection, &derived_revision_id, Some(&source_revision_id))?
        || crate::dependency_closure::active_barrier_for_source(connection, &source_revision_id)?
    {
        return Ok(None);
    }
    let Some(source) =
        visible_artifact(connection, &source_revision_id, effective, &request.context.context)?
    else {
        return Ok(None);
    };
    let Some(derived) =
        visible_artifact(connection, &derived_revision_id, effective, &request.context.context)?
    else {
        return Ok(None);
    };
    if source.role != TraceArtifactRoleV1::CanonicalSource
        || derived.role != TraceArtifactRoleV1::Derived
    {
        return Ok(None);
    }
    Ok(Some((source, derived)))
}

fn load_authorized_candidate(
    connection: &Connection,
    rowid: i64,
) -> Result<StoredDependencyCandidate, EngineError> {
    let candidate: Option<GuardedDependencyCandidate> = connection
        .query_row(
            "SELECT CASE WHEN typeof(d.dependency_id)='text' THEN d.dependency_id END,\
                    CASE WHEN typeof(l.source_revision_id)='text' THEN l.source_revision_id END,\
                    CASE WHEN typeof(d.derived_revision_id)='text' THEN d.derived_revision_id END,\
                    CASE WHEN typeof(d.registered_dependency_generation)='integer' \
                         THEN d.registered_dependency_generation END,\
                    CASE WHEN typeof(d.schema_version)='integer' THEN d.schema_version END \
             FROM _fathomdb_source_dependencies d \
             JOIN _fathomdb_source_links l ON l.artifact_revision_id=d.derived_revision_id \
             WHERE d.rowid=?1",
            [rowid],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((Some(dependency), Some(source), Some(derived), Some(generation), Some(schema))) =
        candidate
    else {
        return Err(
            DependencyTraceErrorV1::new(DependencyTraceErrorReasonV1::TraceCorrupt, "").into()
        );
    };
    Ok((dependency, source, derived, generation, schema))
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
    if !valid_caller_identity(&request.root_revision_id) {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::TraceRootInvalid,
            "/rootRevisionId",
        )
        .into());
    }
    if !(1..=DEFAULT_MAX_RELATIONS).contains(&request.max_relations) {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::TraceLimitInvalid,
            "/maxRelations",
        )
        .into());
    }
    if !(1..=DEFAULT_MAX_WORK_UNITS).contains(&request.max_work_units) {
        return Err(DependencyTraceErrorV1::new(
            DependencyTraceErrorReasonV1::TraceLimitInvalid,
            "/maxWorkUnits",
        )
        .into());
    }
    let tx = connection.transaction().map_err(|_| EngineError::Storage)?;
    let binding = frozen_read::authenticate(&tx, &request.context)?;
    frozen_read::validate_snapshot(&tx, &binding)?;
    let effective = request.context.effective_valid_at;
    let Some(preflight_role) = root_chain_role(&tx, &request.root_revision_id)? else {
        return Err(unavailable());
    };
    let Some(mut root) =
        visible_artifact(&tx, &request.root_revision_id, effective, &request.context.context)?
    else {
        return Err(unavailable());
    };
    if root.role != preflight_role {
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
    let context = &request.context.context;
    let view = &context.view;
    let filter = &context.eligibility;
    let attribute_filter =
        serde_json::to_string(&filter.attributes).map_err(|_| EngineError::Storage)?;
    match request.direction {
        DependencyTraceDirectionV1::ToSource => {
            let candidate_key = tx
                .query_row(
                    TO_SOURCE_CANDIDATE_QUERY,
                    rusqlite::params![
                        &request.root_revision_id,
                        2,
                        view.include_inactive,
                        view.include_superseded,
                        view.include_out_of_window,
                        effective,
                        filter.kind.as_deref(),
                        filter.source_type.as_deref(),
                        filter.created_after,
                        filter.status.as_deref(),
                        &attribute_filter,
                    ],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            if let Some((rowid, _)) = candidate_key {
                if authorize_candidate_chain(&tx, &request, effective, rowid)?.is_some() {
                    let candidate = load_authorized_candidate(&tx, rowid)?;
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
        DependencyTraceDirectionV1::ToDependents => {
            let mut after_key = String::new();
            loop {
                let remaining = request
                    .max_relations
                    .saturating_sub(u32::try_from(edges.len()).unwrap_or(request.max_relations));
                let page_limit = i64::from(remaining.saturating_add(1));
                let page = {
                    let mut statement = tx
                        .prepare_cached(TO_DEPENDENTS_CANDIDATE_QUERY)
                        .map_err(|_| EngineError::Storage)?;
                    let rows = statement
                        .query_map(
                            rusqlite::params![
                                &request.root_revision_id,
                                &after_key,
                                page_limit,
                                view.include_inactive,
                                view.include_superseded,
                                view.include_out_of_window,
                                effective,
                                filter.kind.as_deref(),
                                filter.source_type.as_deref(),
                                filter.created_after,
                                filter.status.as_deref(),
                                &attribute_filter,
                            ],
                            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
                        )
                        .map_err(|_| EngineError::Storage)?;
                    rows.collect::<rusqlite::Result<Vec<StoredDependencyCandidateKey>>>()
                        .map_err(|_| EngineError::Storage)?
                };
                if page.is_empty() {
                    break;
                }
                for (rowid, derived_revision_id) in page {
                    after_key = derived_revision_id;
                    let authorized = authorize_candidate_chain(&tx, &request, effective, rowid)?;
                    if authorized.is_some() {
                        let candidate = load_authorized_candidate(&tx, rowid)?;
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

fn validate_edge_generations(
    edges: &[DependencyTraceEdgeV1],
    dependency_generation: u64,
) -> Result<(), DependencyTraceErrorV1> {
    for (index, edge) in edges.iter().enumerate() {
        if edge.registered_dependency_generation == 0
            || edge.registered_dependency_generation > dependency_generation
        {
            return Err(DependencyTraceErrorV1::new(
                DependencyTraceErrorReasonV1::TraceCorrupt,
                format!("/dependencyEdges/{index}/registeredDependencyGeneration"),
            ));
        }
    }
    Ok(())
}

/// Encode a trace response into canonical declaration-order JSON bytes.
pub fn encode_dependency_trace_result_v1(
    value: &DependencyTraceResultV1,
) -> Result<Vec<u8>, DependencyTraceErrorV1> {
    validate_edge_generations(&value.dependency_edges, value.read_boundary.dependency_generation)?;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NodeLifecycleWire<'a> {
        schema_version: u32,
        artifact_class: &'static str,
        state: &'a str,
        superseded: bool,
        valid_at_effective: bool,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct EdgeLifecycleWire {
        schema_version: u32,
        artifact_class: &'static str,
        superseded: bool,
        valid_at_effective: bool,
    }
    #[derive(Serialize)]
    #[serde(untagged)]
    enum LifecycleWire<'a> {
        Node(NodeLifecycleWire<'a>),
        Edge(EdgeLifecycleWire),
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct NodeWire<'a> {
        schema_version: u32,
        artifact_revision_id: &'a str,
        artifact_class: &'a str,
        role: &'a str,
        depth: u32,
        lifecycle: LifecycleWire<'a>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct EdgeWire<'a> {
        schema_version: u32,
        dependency_id: &'a str,
        source_revision_id: &'a str,
        derived_revision_id: &'a str,
        registered_dependency_generation: String,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct BoundaryWire<'a> {
        schema_version: u32,
        effective_at_epoch_s: i64,
        observed_write_boundary: String,
        dependency_generation: String,
        projection_generation_id: &'a str,
    }

    fn lifecycle(value: &TraceNodeLifecycleV1) -> LifecycleWire<'_> {
        match value {
            TraceNodeLifecycleV1::Node {
                schema_version,
                state,
                superseded,
                valid_at_effective,
            } => LifecycleWire::Node(NodeLifecycleWire {
                schema_version: *schema_version,
                artifact_class: "node",
                state: state.as_str(),
                superseded: *superseded,
                valid_at_effective: *valid_at_effective,
            }),
            TraceNodeLifecycleV1::Edge { schema_version, superseded, valid_at_effective } => {
                LifecycleWire::Edge(EdgeLifecycleWire {
                    schema_version: *schema_version,
                    artifact_class: "edge",
                    superseded: *superseded,
                    valid_at_effective: *valid_at_effective,
                })
            }
        }
    }
    let nodes = value
        .nodes
        .iter()
        .map(|node| NodeWire {
            schema_version: node.schema_version,
            artifact_revision_id: &node.artifact_revision_id,
            artifact_class: node.artifact_class.as_str(),
            role: node.role.as_str(),
            depth: node.depth,
            lifecycle: lifecycle(&node.lifecycle),
        })
        .collect::<Vec<_>>();
    let edges = value
        .dependency_edges
        .iter()
        .map(|edge| EdgeWire {
            schema_version: edge.schema_version,
            dependency_id: &edge.dependency_id,
            source_revision_id: &edge.source_revision_id,
            derived_revision_id: &edge.derived_revision_id,
            registered_dependency_generation: edge.registered_dependency_generation.to_string(),
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
            &BoundaryWire {
                schema_version: value.read_boundary.schema_version,
                effective_at_epoch_s: value.read_boundary.effective_at_epoch_s,
                observed_write_boundary: value.read_boundary.observed_write_boundary.to_string(),
                dependency_generation: value.read_boundary.dependency_generation.to_string(),
                projection_generation_id: &value.read_boundary.projection_generation_id,
            },
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
    let expected_root_role = match direction {
        DependencyTraceDirectionV1::ToSource => TraceArtifactRoleV1::Derived,
        DependencyTraceDirectionV1::ToDependents => TraceArtifactRoleV1::CanonicalSource,
    };
    if nodes[0].role != expected_root_role {
        return Err(corrupt("/nodes/0/role"));
    }
    if nodes.iter().skip(1).any(|node| node.depth != 1) {
        return Err(corrupt("/nodes"));
    }
    if dependency_edges.len() > DEFAULT_MAX_RELATIONS as usize
        || checked_work_units > DEFAULT_MAX_WORK_UNITS
    {
        return Err(corrupt("/dependencyEdges"));
    }
    let mut revision_ids = std::collections::BTreeSet::new();
    for (index, node) in nodes.iter().enumerate() {
        if !revision_ids.insert(&node.artifact_revision_id) {
            return Err(corrupt(format!("/nodes/{index}/artifactRevisionId")));
        }
    }
    let mut dependency_ids = std::collections::BTreeSet::new();
    for (index, edge) in dependency_edges.iter().enumerate() {
        if !dependency_ids.insert(&edge.dependency_id) {
            return Err(corrupt(format!("/dependencyEdges/{index}/dependencyId")));
        }
    }
    for index in 1..dependency_edges.len() {
        let previous = &dependency_edges[index - 1];
        let current = &dependency_edges[index];
        if (&previous.derived_revision_id, &previous.dependency_id)
            >= (&current.derived_revision_id, &current.dependency_id)
        {
            return Err(corrupt(format!("/dependencyEdges/{index}")));
        }
    }
    for index in 2..nodes.len() {
        if nodes[index - 1].artifact_revision_id >= nodes[index].artifact_revision_id {
            return Err(corrupt(format!("/nodes/{index}/artifactRevisionId")));
        }
    }
    for (index, (node, edge)) in nodes.iter().skip(1).zip(&dependency_edges).enumerate() {
        let edge_base = format!("/dependencyEdges/{index}");
        match direction {
            DependencyTraceDirectionV1::ToDependents => {
                if node.role != TraceArtifactRoleV1::Derived {
                    return Err(corrupt(format!("/nodes/{}/role", index + 1)));
                }
                if edge.source_revision_id != root_revision_id {
                    return Err(corrupt(format!("{edge_base}/sourceRevisionId")));
                }
                if edge.derived_revision_id != node.artifact_revision_id {
                    return Err(corrupt(format!("{edge_base}/derivedRevisionId")));
                }
            }
            DependencyTraceDirectionV1::ToSource => {
                if node.role != TraceArtifactRoleV1::CanonicalSource {
                    return Err(corrupt(format!("/nodes/{}/role", index + 1)));
                }
                if edge.derived_revision_id != root_revision_id {
                    return Err(corrupt(format!("{edge_base}/derivedRevisionId")));
                }
                if edge.source_revision_id != node.artifact_revision_id {
                    return Err(corrupt(format!("{edge_base}/sourceRevisionId")));
                }
            }
        }
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
    let effective_at_epoch_s =
        required(boundary, "effectiveAtEpochS", "/readBoundary/effectiveAtEpochS")?
            .as_i64()
            .ok_or_else(|| corrupt("/readBoundary/effectiveAtEpochS"))?;
    let observed_write_boundary = canonical_u64(
        required(boundary, "observedWriteBoundary", "/readBoundary/observedWriteBoundary")?,
        "/readBoundary/observedWriteBoundary",
    )?;
    let dependency_generation = canonical_u64(
        required(boundary, "dependencyGeneration", "/readBoundary/dependencyGeneration")?,
        "/readBoundary/dependencyGeneration",
    )?;
    validate_edge_generations(&dependency_edges, dependency_generation)?;
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
            effective_at_epoch_s,
            observed_write_boundary,
            dependency_generation,
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

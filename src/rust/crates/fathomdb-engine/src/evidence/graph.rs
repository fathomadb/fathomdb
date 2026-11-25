use rusqlite::Connection;

use super::codec::{keyed, random_nonce, slice};
use super::ranked::{
    authorize_graph_intrinsic_lifecycle, evidence_as_of_sql_i64, load_dependency, load_stored,
    validate_full_provenance, validate_source_bytes,
};
use super::{
    EvidenceArtifactClassV1, EvidenceErrorReasonV1, EvidenceErrorV1, GraphEvidenceRefV1,
    INTRINSIC_EDGE_PREFLIGHT_SQL, INTRINSIC_NODE_PREFLIGHT_SQL,
};
use crate::dependency::SourceDependencyV1;
use crate::errors::EngineError;
use crate::filter::{
    append_node_eligibility_sql, append_node_eligibility_sql_historical,
    append_node_eligibility_sql_view, text_hit_passes_filter, text_hit_passes_filter_historical,
    text_hit_passes_filter_view,
};
use crate::frozen_read::{self, FrozenReadContextV1};
use crate::identity::{ArtifactRevisionId, CanonicalHash, DependencyId, SourceRevisionId};
use crate::properties::{decode_properties_row_col, PropertyMap};
use crate::provenance::SourceLocator;
use crate::record_lifecycle::LifecycleState;
use crate::write_commit::canonical_body_hash;

const GRAPH_DATABASE_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.database.v1\0";
const GRAPH_CONTEXT_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.context.v1\0";
const GRAPH_REQUEST_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.request.v1\0";
const GRAPH_TARGET_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.target.v1\0";
const GRAPH_PREDECESSOR_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.predecessor.v1\0";
const GRAPH_EDGE_KIND_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.edge-kind.v1\0";
const GRAPH_TARGET_REVISION_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.target-revision.v1\0";
const GRAPH_EDGE_REVISION_DOMAIN: &[u8] = b"fathomdb.graph-evidence.commit.edge-revision.v1\0";
const GRAPH_STREAM_DOMAIN: &[u8] = b"fathomdb.graph-evidence.stream.v1\0";
pub(super) const GRAPH_MAC_DOMAIN: &[u8] = b"fathomdb.graph-evidence.mac.v1\0";
pub(super) const GRAPH_TOKEN_PREFIX: &str = "fdbgev1.";
pub(super) const GRAPH_SELECTOR_BYTES: usize = 300;
pub(super) const GRAPH_TOKEN_BYTES: usize = 704;

/// Evidence-local traversal direction so `evidence::graph` stays decoupled from `graph_expand::types`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GraphEvidenceDirection {
    Outgoing,
    Incoming,
    Both,
}

#[derive(Clone)]
pub(crate) struct GraphEvidenceAuthority {
    pub(super) database_commitment: [u8; 32],
    pub(super) context_commitment: [u8; 32],
    pub(super) key: Vec<u8>,
}

pub(crate) fn graph_evidence_authority(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
) -> Result<GraphEvidenceAuthority, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let context = frozen_read::validate_context(&frozen.context)?;
    Ok(GraphEvidenceAuthority {
        database_commitment: keyed(&key, GRAPH_DATABASE_DOMAIN, database_id.as_bytes()),
        context_commitment: keyed(&key, GRAPH_CONTEXT_DOMAIN, &context),
        key,
    })
}

pub(crate) fn graph_request_commitment(
    authority: &GraphEvidenceAuthority,
    canonical_request: &[u8],
) -> [u8; 32] {
    keyed(&authority.key, GRAPH_REQUEST_DOMAIN, canonical_request)
}

pub(crate) struct GraphEvidenceDisclosure<'a> {
    pub target_index: u32,
    pub target_cursor: u64,
    pub terminal_edge_cursor: u64,
    pub direction: GraphEvidenceDirection,
    pub target_logical_id: &'a str,
    pub predecessor_logical_id: &'a str,
    pub terminal_edge_kind: &'a str,
    pub target_revision_id: &'a str,
    pub terminal_edge_revision_id: &'a str,
    pub request_commitment: [u8; 32],
    pub historical: bool,
}

#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct GraphEvidenceMaterial {
    artifact_class: EvidenceArtifactClassV1,
    write_cursor: u64,
    artifact_revision_id: String,
    logical_id: Option<String>,
    artifact_kind: String,
    artifact_body: Option<String>,
    node_state: Option<String>,
    node_valid_from: Option<i64>,
    node_valid_until: Option<i64>,
    edge_t_valid: Option<i64>,
    edge_t_invalid: Option<i64>,
    edge_temporal_fallback: bool,
    artifact_superseded_at: Option<i64>,
    source_id: String,
    source_version_id: String,
    source_revision_id: String,
    locator: SourceLocator,
    hash_digest: String,
    source_lifecycle: String,
    source_superseded_at: Option<i64>,
    source_valid_from: Option<i64>,
    source_valid_until: Option<i64>,
    dependency_id: Option<String>,
    dependency_generation: Option<u64>,
    edge_from: Option<String>,
    edge_to: Option<String>,
    artifact_properties: Option<PropertyMap>,
}

impl GraphEvidenceMaterial {
    pub(crate) fn artifact_revision_id(&self) -> &str {
        &self.artifact_revision_id
    }
}

pub(crate) struct GraphEvidencePreflight {
    pub nodes: Vec<GraphEvidenceMaterial>,
    pub edges: Vec<GraphEvidenceMaterial>,
    #[allow(dead_code)]
    pub stats: GraphEvidencePreflightStats,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GraphEvidencePreflightStats {
    pub data_statement_count: usize,
    pub node_rows: usize,
    pub edge_rows: usize,
    pub source_hash_count: usize,
    pub source_bytes_hashed: usize,
}

fn parse_locator(
    kind: String,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<SourceLocator, EngineError> {
    match (kind.as_str(), start, end) {
        ("whole_body", None, None) => Ok(SourceLocator::WholeBody),
        ("utf8_bytes", Some(start), Some(end)) if start >= 0 && end >= start => {
            Ok(SourceLocator::utf8_bytes(start as u64, end as u64))
        }
        _ => Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/provenance/sourceLocator",
        )
        .into()),
    }
}

struct PendingGraphEvidence {
    artifact_class: EvidenceArtifactClassV1,
    write_cursor: u64,
    target_index: u32,
    values: Vec<rusqlite::types::Value>,
    properties: Option<PropertyMap>,
    n_head_inactive: bool,
    sn_head_inactive: bool,
}

fn pending_i64(row: &PendingGraphEvidence, index: usize) -> Option<i64> {
    match row.values.get(index) {
        Some(rusqlite::types::Value::Integer(value)) => Some(*value),
        _ => None,
    }
}

fn pending_string(row: &PendingGraphEvidence, index: usize) -> Option<String> {
    match row.values.get(index) {
        Some(rusqlite::types::Value::Text(value)) => Some(value.clone()),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn query_graph_evidence_batch(
    connection: &Connection,
    sql: &str,
    artifact_class: EvidenceArtifactClassV1,
    cursors: &[u64],
    target_indices: &[u32],
    frozen: &FrozenReadContextV1,
    history_as_of: Option<u64>,
    stats: &mut GraphEvidencePreflightStats,
) -> Result<Vec<PendingGraphEvidence>, EngineError> {
    if cursors.len() > 50 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    stats.data_statement_count += 1;
    let json = serde_json::to_string(cursors).map_err(|_| EngineError::Storage)?;
    let mut params = vec![rusqlite::types::Value::Text(json)];
    let target_filter = if artifact_class == EvidenceArtifactClassV1::Node {
        if history_as_of.is_some() {
            append_node_eligibility_sql_historical(
                connection,
                Some(&frozen.context.eligibility),
                "n",
                &mut params,
            )
        } else {
            append_node_eligibility_sql_view(
                connection,
                Some(&frozen.context.eligibility),
                "n",
                &mut params,
                &frozen.context.view,
            )
        }
        .map_err(|_| EngineError::Storage)?
    } else {
        String::new()
    };
    let mut source_filter = frozen.context.eligibility.clone();
    source_filter.kind = None;
    source_filter.source_type = None;
    let source_filter = if history_as_of.is_some() {
        append_node_eligibility_sql_historical(connection, Some(&source_filter), "sn", &mut params)
            .map_err(|_| EngineError::Storage)?
    } else {
        append_node_eligibility_sql(Some(&source_filter), "sn", &mut params)
    };
    let sql = sql
        .replace("/*TARGET_ELIGIBILITY*/", &target_filter)
        .replace("/*SOURCE_ELIGIBILITY*/", &source_filter);
    let mut statement = connection.prepare(&sql).map_err(|error| {
        eprintln!("Slice 15 preflight prepare: {error}");
        EngineError::Storage
    })?;
    let mut rows = statement.query(rusqlite::params_from_iter(params.iter())).map_err(|error| {
        eprintln!("Slice 15 preflight query: {error}");
        EngineError::Storage
    })?;
    let mut pending = Vec::with_capacity(cursors.len());
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        let ordinal: i64 = row.get(0).map_err(|_| EngineError::Storage)?;
        let ordinal = usize::try_from(ordinal).map_err(|_| EngineError::Storage)?;
        let write_cursor = cursors.get(ordinal).copied().ok_or(EngineError::Storage)?;
        let target_index = target_indices.get(ordinal).copied().ok_or(EngineError::Storage)?;
        let mut values = Vec::with_capacity(58);
        for index in 0..58 {
            values.push(row.get(index).map_err(|_| EngineError::Storage)?);
        }
        let properties = decode_properties_row_col(row, 58).map_err(|_| EngineError::Storage)?;
        let n_head_inactive = row.get::<_, i64>(59).map_err(|_| EngineError::Storage)? != 0;
        let sn_head_inactive = row.get::<_, i64>(60).map_err(|_| EngineError::Storage)? != 0;
        pending.push(PendingGraphEvidence {
            artifact_class,
            write_cursor,
            target_index,
            values,
            properties,
            n_head_inactive,
            sn_head_inactive,
        });
    }
    if pending.len() != cursors.len() {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(pending)
}

fn authorize_graph_evidence(
    batches: &[&[PendingGraphEvidence]],
    frozen: &FrozenReadContextV1,
    history_as_of: Option<u64>,
) -> Result<(), EngineError> {
    let as_of_i64 = history_as_of.map(evidence_as_of_sql_i64);
    for row in batches.iter().flat_map(|batch| batch.iter()) {
        let effective = frozen.effective_valid_at;
        let in_window = |start: Option<i64>, end: Option<i64>| {
            start.is_none_or(|value| value <= effective)
                && end.is_none_or(|value| value > effective)
        };
        let artifact_cursor_ok = as_of_i64
            .is_none_or(|as_of| i64::try_from(row.write_cursor).is_ok_and(|wc| wc <= as_of));
        let artifact_superseded_ok = match as_of_i64 {
            None => pending_i64(row, 10).is_none(),
            Some(as_of) => pending_i64(row, 10).is_none_or(|sup| sup > as_of),
        };
        let effective_lifecycle = match row.artifact_class {
            EvidenceArtifactClassV1::Node => {
                pending_string(row, 5).is_some()
                    && pending_string(row, 9).as_deref() == Some("active")
                    && artifact_cursor_ok
                    && artifact_superseded_ok
                    && (as_of_i64.is_none() || !row.n_head_inactive)
                    && in_window(pending_i64(row, 11), pending_i64(row, 12))
            }
            EvidenceArtifactClassV1::Edge => {
                artifact_cursor_ok
                    && artifact_superseded_ok
                    && pending_i64(row, 15).unwrap_or(0) == 0
                    && in_window(pending_i64(row, 11), pending_i64(row, 12))
            }
        };
        if pending_i64(row, 1).is_none() || pending_i64(row, 56) != Some(1) || !effective_lifecycle
        {
            return Err(EvidenceErrorV1::unavailable().into());
        }
        let source_cursor_ok = match as_of_i64 {
            None => pending_i64(row, 54).is_some(),
            Some(as_of) => pending_i64(row, 54).is_some_and(|sc| sc <= as_of),
        };
        let source_superseded_ok = match as_of_i64 {
            None => pending_i64(row, 32).is_none(),
            Some(as_of) => pending_i64(row, 32).is_none_or(|sup| sup > as_of),
        };
        if pending_i64(row, 16).is_some()
            && (pending_i64(row, 25) != Some(1)
                || pending_string(row, 26).as_deref() != Some("node")
                || pending_string(row, 27).as_deref() != Some("canonical_source")
                || pending_string(row, 28).as_deref() != Some("complete")
                || pending_string(row, 29).is_none()
                || pending_string(row, 30) != pending_string(row, 17)
                || pending_string(row, 31).as_deref() != Some("active")
                || !source_superseded_ok
                || (as_of_i64.is_some() && row.sn_head_inactive)
                || !in_window(pending_i64(row, 33), pending_i64(row, 34))
                || !source_cursor_ok
                || pending_i64(row, 55) != Some(0)
                || pending_i64(row, 57) != Some(1))
        {
            return Err(EvidenceErrorV1::unavailable().into());
        }
    }
    let missing = batches
        .iter()
        .flat_map(|batch| batch.iter())
        .filter(|row| pending_i64(row, 16).is_none())
        .min_by_key(|row| {
            (
                row.target_index,
                if row.artifact_class == EvidenceArtifactClassV1::Node { 0 } else { 1 },
            )
        });
    if let Some(row) = missing {
        let path = if row.artifact_class == EvidenceArtifactClassV1::Node {
            format!("/targets/{}/provenance", row.target_index)
        } else {
            format!("/targets/{}/terminalEdge/provenance", row.target_index)
        };
        return Err(EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, path).into());
    }
    Ok(())
}

fn materialize_graph_evidence_batch(
    pending: Vec<PendingGraphEvidence>,
    frozen: &FrozenReadContextV1,
    history_as_of: Option<u64>,
    sources: &mut std::collections::HashMap<String, (std::sync::Arc<String>, String)>,
    stats: &mut GraphEvidencePreflightStats,
) -> Result<Vec<GraphEvidenceMaterial>, EngineError> {
    let as_of_i64 = history_as_of.map(evidence_as_of_sql_i64);
    let mut material = Vec::with_capacity(pending.len());
    for raw in pending {
        let row = &raw;
        let artifact_class = raw.artifact_class;
        let write_cursor = raw.write_cursor;
        let registry_schema = pending_i64(row, 1).ok_or(EngineError::Storage)?;
        let artifact_revision_id = pending_string(row, 2).ok_or(EngineError::Storage)?;
        let role = pending_string(row, 3).ok_or(EngineError::Storage)?;
        let completeness = pending_string(row, 4).ok_or(EngineError::Storage)?;
        let logical_id = pending_string(row, 5);
        let artifact_kind = pending_string(row, 6).ok_or(EngineError::Storage)?;
        let artifact_body = pending_string(row, 7);
        let artifact_source_id = pending_string(row, 8);
        let artifact_state = pending_string(row, 9);
        let artifact_superseded = pending_i64(row, 10);
        let artifact_valid_from = pending_i64(row, 11);
        let artifact_valid_until = pending_i64(row, 12);
        let edge_from = pending_string(row, 13);
        let edge_to = pending_string(row, 14);
        let temporal_fallback = pending_i64(row, 15);
        let link_schema = pending_i64(row, 16).ok_or(EngineError::Storage)?;
        let source_id = pending_string(row, 17).ok_or(EngineError::Storage)?;
        let source_version_id = pending_string(row, 18).ok_or(EngineError::Storage)?;
        let source_revision_id = pending_string(row, 19).ok_or(EngineError::Storage)?;
        let locator = parse_locator(
            pending_string(row, 20).ok_or(EngineError::Storage)?,
            pending_i64(row, 21),
            pending_i64(row, 22),
        )?;
        let hash_algorithm = pending_string(row, 23).ok_or(EngineError::Storage)?;
        let hash_digest = pending_string(row, 24).ok_or(EngineError::Storage)?;
        let source_registry_schema = pending_i64(row, 25).ok_or(EngineError::Storage)?;
        let source_class = pending_string(row, 26).ok_or(EngineError::Storage)?;
        let source_role = pending_string(row, 27).ok_or(EngineError::Storage)?;
        let source_completeness = pending_string(row, 28).ok_or(EngineError::Storage)?;
        let source_body = pending_string(row, 29).ok_or(EngineError::Storage)?;
        let canonical_source_id = pending_string(row, 30);
        let source_state = pending_string(row, 31).ok_or(EngineError::Storage)?;
        let source_superseded = pending_i64(row, 32);
        let source_valid_from = pending_i64(row, 33);
        let source_valid_until = pending_i64(row, 34);
        let version_schema = pending_i64(row, 36).ok_or(EngineError::Storage)?;
        let version_source_id = pending_string(row, 37).ok_or(EngineError::Storage)?;
        let version_id = pending_string(row, 38).ok_or(EngineError::Storage)?;
        let version_revision = pending_string(row, 39).ok_or(EngineError::Storage)?;
        let self_schema = pending_i64(row, 40).ok_or(EngineError::Storage)?;
        let self_source_id = pending_string(row, 41).ok_or(EngineError::Storage)?;
        let self_version_id = pending_string(row, 42).ok_or(EngineError::Storage)?;
        let self_revision = pending_string(row, 43).ok_or(EngineError::Storage)?;
        let self_locator = pending_string(row, 44).ok_or(EngineError::Storage)?;
        let self_start = pending_i64(row, 45);
        let self_end = pending_i64(row, 46);
        let self_algorithm = pending_string(row, 47).ok_or(EngineError::Storage)?;
        let self_hash = pending_string(row, 48).ok_or(EngineError::Storage)?;
        let dependency_schema = pending_i64(row, 49);
        let dependency_id = pending_string(row, 50);
        let dependency_revision = pending_string(row, 51);
        let dependency_generation = pending_i64(row, 52);
        let current_dependency_generation = pending_i64(row, 53).ok_or(EngineError::Storage)?;
        let source_cursor_i64 = pending_i64(row, 54).ok_or(EngineError::Storage)?;
        let closure_active = pending_i64(row, 55) != Some(0);
        let source_cursor = u64::try_from(source_cursor_i64).map_err(|_| EngineError::Storage)?;
        let effective = frozen.effective_valid_at;
        let in_window = |start: Option<i64>, end: Option<i64>| {
            start.is_none_or(|value| value <= effective)
                && end.is_none_or(|value| value > effective)
        };
        if completeness != "complete" || source_completeness != "complete" {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                "/provenance",
            )
            .into());
        }
        let dependency_generation_u64 =
            dependency_generation.and_then(|value| u64::try_from(value).ok());
        let dependency_absent = dependency_schema.is_none()
            && dependency_id.is_none()
            && dependency_revision.is_none()
            && dependency_generation.is_none();
        let dependency_valid = dependency_absent
            || (dependency_schema == Some(1)
                && dependency_id.as_deref().is_some_and(|value| DependencyId::new(value).is_ok())
                && dependency_revision.as_deref() == Some(artifact_revision_id.as_str())
                && dependency_generation_u64.is_some_and(|value| {
                    value > 0
                        && i64::try_from(value)
                            .is_ok_and(|value| value <= current_dependency_generation)
                }));
        let artifact_cursor_ok =
            as_of_i64.is_none_or(|as_of| i64::try_from(write_cursor).is_ok_and(|wc| wc <= as_of));
        let source_cursor_ok =
            as_of_i64.is_none_or(|as_of| i64::try_from(source_cursor).is_ok_and(|sc| sc <= as_of));
        let artifact_superseded_ok = match as_of_i64 {
            None => frozen.context.view.include_superseded || artifact_superseded.is_none(),
            Some(as_of) => artifact_superseded.is_none_or(|sup| sup > as_of),
        };
        let source_superseded_ok = match as_of_i64 {
            None => source_superseded.is_none(),
            Some(as_of) => source_superseded.is_none_or(|sup| sup > as_of),
        };
        let artifact_lifecycle_valid = match artifact_class {
            EvidenceArtifactClassV1::Node => {
                artifact_state.as_deref().and_then(LifecycleState::from_str_opt).is_some_and(
                    |state| {
                        if as_of_i64.is_some() {
                            state == LifecycleState::Active
                        } else {
                            frozen.context.view.include_inactive || state == LifecycleState::Active
                        }
                    },
                ) && artifact_cursor_ok
                    && artifact_superseded_ok
                    && (as_of_i64.is_none() || !row.n_head_inactive)
                    && in_window(artifact_valid_from, artifact_valid_until)
            }
            EvidenceArtifactClassV1::Edge => {
                artifact_cursor_ok
                    && artifact_superseded_ok
                    && temporal_fallback.unwrap_or(0) == 0
                    && in_window(artifact_valid_from, artifact_valid_until)
            }
        };
        let canonical_self = artifact_class == EvidenceArtifactClassV1::Node
            && role == "canonical_source"
            && artifact_revision_id == source_revision_id;
        let metadata_valid = registry_schema == 1
            && (role == "derived_semantic" || canonical_self)
            && link_schema == 1
            && source_registry_schema == 1
            && source_class == "node"
            && source_role == "canonical_source"
            && version_schema == 1
            && self_schema == 1
            && artifact_source_id.as_deref() == Some(source_id.as_str())
            && canonical_source_id.as_deref() == Some(source_id.as_str())
            && version_source_id == source_id
            && self_source_id == source_id
            && version_id == source_version_id
            && self_version_id == source_version_id
            && version_revision == source_revision_id
            && self_revision == source_revision_id
            && self_locator == "whole_body"
            && self_start.is_none()
            && self_end.is_none()
            && hash_algorithm == "sha256"
            && self_algorithm == "sha256"
            && self_hash == hash_digest
            && source_state == "active"
            && source_cursor_ok
            && source_superseded_ok
            && (as_of_i64.is_none() || !row.sn_head_inactive)
            && in_window(source_valid_from, source_valid_until)
            && artifact_lifecycle_valid
            && dependency_valid
            && !closure_active;
        let source_hash_valid =
            if let Some((existing_body, existing_digest)) = sources.get(&source_revision_id) {
                // Each row must agree with the already validated canonical bytes and digest.
                existing_body.as_str() == source_body && existing_digest == &hash_digest
            } else {
                CanonicalHash::sha256(hash_digest.clone()).is_ok()
                    && canonical_body_hash(&source_body) == hash_digest
            };
        if !source_hash_valid {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance/canonicalSourceHash",
            )
            .into());
        }
        if slice(&source_body, &locator).is_err() {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance/sourceLocator",
            )
            .into());
        }
        if !metadata_valid
            || ArtifactRevisionId::new(artifact_revision_id.clone()).is_err()
            || SourceRevisionId::new(source_revision_id.clone()).is_err()
        {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance",
            )
            .into());
        }
        if !sources.contains_key(&source_revision_id) {
            stats.source_hash_count += 1;
            stats.source_bytes_hashed += source_body.len();
            let source = std::sync::Arc::new(source_body);
            sources.insert(source_revision_id.clone(), (source, hash_digest.clone()));
        }
        material.push(GraphEvidenceMaterial {
            artifact_class,
            write_cursor,
            artifact_revision_id,
            logical_id,
            artifact_kind,
            artifact_body,
            node_state: artifact_state,
            node_valid_from: if artifact_class == EvidenceArtifactClassV1::Node {
                artifact_valid_from
            } else {
                None
            },
            node_valid_until: if artifact_class == EvidenceArtifactClassV1::Node {
                artifact_valid_until
            } else {
                None
            },
            edge_t_valid: if artifact_class == EvidenceArtifactClassV1::Edge {
                artifact_valid_from
            } else {
                None
            },
            edge_t_invalid: if artifact_class == EvidenceArtifactClassV1::Edge {
                artifact_valid_until
            } else {
                None
            },
            edge_temporal_fallback: temporal_fallback.unwrap_or(0) != 0,
            artifact_superseded_at: artifact_superseded,
            source_id,
            source_version_id,
            source_revision_id,
            locator,
            hash_digest,
            source_lifecycle: source_state,
            source_superseded_at: source_superseded,
            source_valid_from,
            source_valid_until,
            dependency_id,
            dependency_generation: dependency_generation_u64,
            edge_from,
            edge_to,
            artifact_properties: raw.properties,
        });
    }
    Ok(material)
}

pub(crate) fn preflight_graph_evidence(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    history_as_of: Option<u64>,
    node_cursors: &[u64],
    edge_cursors: &[u64],
) -> Result<GraphEvidencePreflight, EngineError> {
    fn deduplicate(cursors: &[u64]) -> (Vec<u64>, Vec<u32>, Vec<usize>) {
        let mut unique = Vec::new();
        let mut first_positions = Vec::new();
        let mut positions = Vec::with_capacity(cursors.len());
        let mut by_cursor = std::collections::HashMap::new();
        for (target_index, cursor) in cursors.iter().copied().enumerate() {
            let unique_index = *by_cursor.entry(cursor).or_insert_with(|| {
                let index = unique.len();
                unique.push(cursor);
                first_positions.push(u32::try_from(target_index).unwrap_or(u32::MAX));
                index
            });
            positions.push(unique_index);
        }
        (unique, first_positions, positions)
    }

    let mut stats = GraphEvidencePreflightStats::default();
    let (unique_node_cursors, node_target_indices, node_positions) = deduplicate(node_cursors);
    let (unique_edge_cursors, edge_target_indices, edge_positions) = deduplicate(edge_cursors);
    let pending_nodes = query_graph_evidence_batch(
        connection,
        INTRINSIC_NODE_PREFLIGHT_SQL,
        EvidenceArtifactClassV1::Node,
        &unique_node_cursors,
        &node_target_indices,
        frozen,
        history_as_of,
        &mut stats,
    )?;
    let pending_edges = query_graph_evidence_batch(
        connection,
        INTRINSIC_EDGE_PREFLIGHT_SQL,
        EvidenceArtifactClassV1::Edge,
        &unique_edge_cursors,
        &edge_target_indices,
        frozen,
        history_as_of,
        &mut stats,
    )?;
    authorize_graph_evidence(&[&pending_nodes, &pending_edges], frozen, history_as_of)?;
    let mut ordered = pending_nodes.into_iter().chain(pending_edges).collect::<Vec<_>>();
    ordered.sort_by_key(|row| {
        (row.target_index, if row.artifact_class == EvidenceArtifactClassV1::Node { 0 } else { 1 })
    });
    let mut sources = std::collections::HashMap::new();
    let mut unique_nodes_by_cursor = std::collections::HashMap::new();
    let mut unique_edges_by_cursor = std::collections::HashMap::new();
    for row in ordered {
        let class = row.artifact_class;
        let target_index = row.target_index;
        let cursor = row.write_cursor;
        let item = materialize_graph_evidence_batch(
            vec![row],
            frozen,
            history_as_of,
            &mut sources,
            &mut stats,
        )
        .map_err(|error| contextualize_graph_detail_error(error, class, target_index))?
        .pop()
        .ok_or(EngineError::Storage)?;
        match class {
            EvidenceArtifactClassV1::Node => unique_nodes_by_cursor.insert(cursor, item),
            EvidenceArtifactClassV1::Edge => unique_edges_by_cursor.insert(cursor, item),
        };
    }
    let unique_nodes = unique_node_cursors
        .iter()
        .map(|cursor| unique_nodes_by_cursor.remove(cursor).ok_or(EngineError::Storage))
        .collect::<Result<Vec<_>, _>>()?;
    let unique_edges = unique_edge_cursors
        .iter()
        .map(|cursor| unique_edges_by_cursor.remove(cursor).ok_or(EngineError::Storage))
        .collect::<Result<Vec<_>, _>>()?;
    stats.node_rows = unique_nodes.len();
    stats.edge_rows = unique_edges.len();
    let nodes = node_positions.iter().map(|index| unique_nodes[*index].clone()).collect();
    let edges = edge_positions.iter().map(|index| unique_edges[*index].clone()).collect();
    Ok(GraphEvidencePreflight { nodes, edges, stats })
}

fn contextualize_graph_detail_error(
    error: EngineError,
    class: EvidenceArtifactClassV1,
    target_index: u32,
) -> EngineError {
    let base = if class == EvidenceArtifactClassV1::Node {
        format!("/targets/{target_index}/provenance")
    } else {
        format!("/targets/{target_index}/terminalEdge/provenance")
    };
    match error {
        EngineError::Evidence(mut evidence)
            if matches!(
                evidence.reason,
                EvidenceErrorReasonV1::EvidenceIncomplete | EvidenceErrorReasonV1::EvidenceCorrupt
            ) =>
        {
            let suffix = evidence.field_path.strip_prefix("/provenance").unwrap_or("");
            evidence.field_path = format!("{base}{suffix}");
            EngineError::Evidence(evidence)
        }
        EngineError::Storage => EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            base,
        )),
        other => other,
    }
}
pub(super) fn protect_graph_selector(key: &[u8], nonce: &[u8; 16], value: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(value.len());
    for (block_index, chunk) in value.chunks(32).enumerate() {
        let mut input = Vec::with_capacity(20);
        input.extend_from_slice(nonce);
        input.extend_from_slice(&(block_index as u32).to_be_bytes());
        let mask = frozen_read::hmac_sha256(key, GRAPH_STREAM_DOMAIN, &input);
        output.extend(chunk.iter().zip(mask).map(|(byte, mask)| byte ^ mask));
    }
    output
}

pub(super) fn frame_graph_selector(key: &[u8], nonce: &[u8; 16], payload: &[u8]) -> String {
    let ciphertext = protect_graph_selector(key, nonce, payload);
    let mut authenticated =
        Vec::with_capacity(GRAPH_TOKEN_PREFIX.len() + nonce.len() + ciphertext.len());
    authenticated.extend_from_slice(GRAPH_TOKEN_PREFIX.as_bytes());
    authenticated.extend_from_slice(nonce);
    authenticated.extend_from_slice(&ciphertext);
    let mac = frozen_read::hmac_sha256(key, GRAPH_MAC_DOMAIN, &authenticated);
    let mut framed = Vec::with_capacity(nonce.len() + ciphertext.len() + mac.len());
    framed.extend_from_slice(nonce);
    framed.extend_from_slice(&ciphertext);
    framed.extend_from_slice(&mac);
    format!("{GRAPH_TOKEN_PREFIX}{}", frozen_read::hex_encode(&framed))
}

pub(crate) fn mint_graph_evidence_reference(
    authority: &GraphEvidenceAuthority,
    frozen: &FrozenReadContextV1,
    material: &GraphEvidenceMaterial,
    role: u8,
    disclosure: &GraphEvidenceDisclosure<'_>,
) -> Result<(ArtifactRevisionId, GraphEvidenceRefV1), EngineError> {
    let artifact_class = material.artifact_class.tag();
    if !matches!((role, artifact_class), (0, 0) | (1, 1)) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let direction = match disclosure.direction {
        GraphEvidenceDirection::Outgoing => 0,
        GraphEvidenceDirection::Incoming => 1,
        GraphEvidenceDirection::Both => 2,
    };
    let mut payload = Vec::with_capacity(GRAPH_SELECTOR_BYTES);
    frozen_read::encode_u32(&mut payload, 1);
    payload.extend_from_slice(&[role, artifact_class, direction, u8::from(disclosure.historical)]);
    frozen_read::encode_u32(&mut payload, disclosure.target_index);
    frozen_read::encode_u64(&mut payload, material.write_cursor);
    frozen_read::encode_u64(&mut payload, disclosure.target_cursor);
    frozen_read::encode_u64(&mut payload, disclosure.terminal_edge_cursor);
    frozen_read::encode_i64(&mut payload, frozen.effective_valid_at);
    payload.extend_from_slice(&authority.database_commitment);
    payload.extend_from_slice(&authority.context_commitment);
    payload.extend_from_slice(&disclosure.request_commitment);
    payload.extend_from_slice(&keyed(
        &authority.key,
        GRAPH_TARGET_DOMAIN,
        disclosure.target_logical_id.as_bytes(),
    ));
    payload.extend_from_slice(&keyed(
        &authority.key,
        GRAPH_PREDECESSOR_DOMAIN,
        disclosure.predecessor_logical_id.as_bytes(),
    ));
    payload.extend_from_slice(&keyed(
        &authority.key,
        GRAPH_EDGE_KIND_DOMAIN,
        disclosure.terminal_edge_kind.as_bytes(),
    ));
    payload.extend_from_slice(&keyed(
        &authority.key,
        GRAPH_TARGET_REVISION_DOMAIN,
        disclosure.target_revision_id.as_bytes(),
    ));
    payload.extend_from_slice(&keyed(
        &authority.key,
        GRAPH_EDGE_REVISION_DOMAIN,
        disclosure.terminal_edge_revision_id.as_bytes(),
    ));
    debug_assert_eq!(payload.len(), GRAPH_SELECTOR_BYTES);
    let nonce = random_nonce()?;
    let reference = frame_graph_selector(&authority.key, &nonce, &payload);
    debug_assert_eq!(reference.len(), GRAPH_TOKEN_BYTES);
    Ok((
        ArtifactRevisionId::new(material.artifact_revision_id.clone())
            .map_err(|_| EvidenceErrorV1::unavailable())?,
        GraphEvidenceRefV1::new(reference)?,
    ))
}

pub(crate) struct GraphEvidenceResolvedMaterial {
    pub artifact_class: EvidenceArtifactClassV1,
    pub artifact_revision_id: String,
    pub logical_id: Option<String>,
    pub canonical_source_body: String,
    pub evidence_text: String,
    pub artifact_kind: String,
    pub artifact_body: Option<String>,
    pub source_id: String,
    pub source_version_id: String,
    pub source_revision_id: String,
    pub canonical_source_hash: String,
    pub source_locator: SourceLocator,
    pub node_state: Option<String>,
    pub edge_t_valid: Option<i64>,
    pub edge_t_invalid: Option<i64>,
    pub artifact_superseded_at: Option<i64>,
    pub source_state: String,
    pub dependency: Option<SourceDependencyV1>,
    pub edge_from: Option<String>,
    pub edge_to: Option<String>,
    pub artifact_properties: Option<PropertyMap>,
}

#[allow(dead_code)]
pub(super) struct GraphEvidenceSelector {
    role: u8,
    artifact_class: EvidenceArtifactClassV1,
    pub(super) historical: bool,
    pub(super) artifact_cursor: u64,
    pub(super) target_index: u32,
    target_cursor: u64,
    pub(super) terminal_edge_cursor: u64,
    effective_valid_at: i64,
    database_commitment: [u8; 32],
    context_commitment: [u8; 32],
    request_commitment: [u8; 32],
    target_commitment: [u8; 32],
    predecessor_commitment: [u8; 32],
    edge_kind_commitment: [u8; 32],
    target_revision_commitment: [u8; 32],
    edge_revision_commitment: [u8; 32],
    direction: GraphEvidenceDirection,
}

pub(super) fn decode_graph_evidence_reference(
    reference: &GraphEvidenceRefV1,
    key: &[u8],
) -> Result<GraphEvidenceSelector, EngineError> {
    let encoded = reference
        .as_str()
        .strip_prefix(GRAPH_TOKEN_PREFIX)
        .filter(|_| reference.as_str().len() == GRAPH_TOKEN_BYTES)
        .ok_or_else(EvidenceErrorV1::unavailable)?;
    if encoded.bytes().any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f')) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let framed = frozen_read::hex_decode(encoded).ok_or_else(EvidenceErrorV1::unavailable)?;
    let nonce: [u8; 16] = framed[0..16].try_into().map_err(|_| EvidenceErrorV1::unavailable())?;
    let ciphertext = &framed[16..316];
    let mac = &framed[316..348];
    let mut authenticated = Vec::with_capacity(GRAPH_TOKEN_PREFIX.len() + 316);
    authenticated.extend_from_slice(GRAPH_TOKEN_PREFIX.as_bytes());
    authenticated.extend_from_slice(&nonce);
    authenticated.extend_from_slice(ciphertext);
    let expected = frozen_read::hmac_sha256(key, GRAPH_MAC_DOMAIN, &authenticated);
    if !frozen_read::constant_time_eq(mac, &expected) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let payload = protect_graph_selector(key, &nonce, ciphertext);
    let read_u32 = |range: std::ops::Range<usize>| {
        u32::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    let read_u64 = |range: std::ops::Range<usize>| {
        u64::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    let read_i64 = |range: std::ops::Range<usize>| {
        i64::from_be_bytes(payload[range].try_into().expect("fixed intrinsic payload"))
    };
    if read_u32(0..4) != 1 {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let role = payload[4];
    let artifact_class =
        EvidenceArtifactClassV1::from_tag(payload[5]).ok_or_else(EvidenceErrorV1::unavailable)?;
    let direction = match payload[6] {
        0 => GraphEvidenceDirection::Outgoing,
        1 => GraphEvidenceDirection::Incoming,
        2 => GraphEvidenceDirection::Both,
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    let historical = match payload[7] {
        0 => false,
        1 => true,
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    if !matches!(
        (role, artifact_class),
        (0, EvidenceArtifactClassV1::Node) | (1, EvidenceArtifactClassV1::Edge)
    ) {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(GraphEvidenceSelector {
        role,
        artifact_class,
        historical,
        target_index: read_u32(8..12),
        artifact_cursor: read_u64(12..20),
        target_cursor: read_u64(20..28),
        terminal_edge_cursor: read_u64(28..36),
        effective_valid_at: read_i64(36..44),
        database_commitment: payload[44..76].try_into().expect("fixed intrinsic payload"),
        context_commitment: payload[76..108].try_into().expect("fixed intrinsic payload"),
        request_commitment: payload[108..140].try_into().expect("fixed intrinsic payload"),
        target_commitment: payload[140..172].try_into().expect("fixed intrinsic payload"),
        predecessor_commitment: payload[172..204].try_into().expect("fixed intrinsic payload"),
        edge_kind_commitment: payload[204..236].try_into().expect("fixed intrinsic payload"),
        target_revision_commitment: payload[236..268].try_into().expect("fixed intrinsic payload"),
        edge_revision_commitment: payload[268..300].try_into().expect("fixed intrinsic payload"),
        direction,
    })
}

pub(crate) fn resolve_graph_evidence(
    connection: &Connection,
    reference: &GraphEvidenceRefV1,
    frozen: &FrozenReadContextV1,
    scope: &crate::scope::ScopePredicate,
) -> Result<GraphEvidenceResolvedMaterial, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let payload = decode_graph_evidence_reference(reference, &key)?;
    let context_bytes = frozen_read::validate_context(&frozen.context)?;
    if payload.database_commitment != keyed(&key, GRAPH_DATABASE_DOMAIN, database_id.as_bytes())
        || payload.context_commitment != keyed(&key, GRAPH_CONTEXT_DOMAIN, &context_bytes)
        || payload.effective_valid_at != frozen.effective_valid_at
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let stored = load_stored(
        connection,
        scope,
        payload.artifact_class,
        payload.artifact_cursor,
        payload.effective_valid_at,
        frozen.context.view.include_out_of_window,
    )
    .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    authorize_graph_intrinsic_lifecycle(
        connection,
        &stored,
        payload.artifact_class,
        &frozen.context.view,
        payload.historical,
    )?;
    let eligible = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => {
            if payload.historical {
                text_hit_passes_filter_historical(
                    connection,
                    payload.artifact_cursor,
                    &stored.artifact_kind,
                    Some(&frozen.context.eligibility),
                )
            } else {
                text_hit_passes_filter_view(
                    connection,
                    payload.artifact_cursor,
                    &stored.artifact_kind,
                    Some(&frozen.context.eligibility),
                    &frozen.context.view,
                )
            }
        }
        EvidenceArtifactClassV1::Edge => Ok(true),
    }
    .map_err(|_| EngineError::Storage)?;
    let mut source_filter = frozen.context.eligibility.clone();
    source_filter.kind = None;
    source_filter.source_type = None;
    let source_eligible = if payload.historical {
        text_hit_passes_filter_historical(
            connection,
            stored.source_cursor,
            &stored.source_kind,
            Some(&source_filter),
        )
    } else {
        text_hit_passes_filter(
            connection,
            stored.source_cursor,
            &stored.source_kind,
            Some(&source_filter),
        )
    }
    .map_err(|_| EngineError::Storage)?;
    if !eligible || !source_eligible || stored.completeness != "complete" {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    validate_source_bytes(&stored)?;
    validate_full_provenance(connection, &stored)?;
    let cursor = i64::try_from(payload.artifact_cursor).map_err(|_| EngineError::Storage)?;
    let (
        artifact_body,
        edge_from,
        edge_to,
        node_state,
        node_valid_from,
        node_valid_until,
        edge_t_valid,
        edge_t_invalid,
        edge_temporal_fallback,
        artifact_properties,
    ) = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => connection
            .query_row(
                "SELECT body,state,valid_from,valid_until,properties FROM canonical_nodes WHERE scope=?1 AND write_cursor=?2",
                rusqlite::params![&stored.scope, cursor],
                |row| {
                    Ok((
                        Some(row.get(0)?),
                        None,
                        None,
                        Some(row.get(1)?),
                        row.get(2)?,
                        row.get(3)?,
                        None,
                        None,
                        false,
                        decode_properties_row_col(row, 4)?,
                    ))
                },
            )
            .map_err(|_| EngineError::Storage)?,
        EvidenceArtifactClassV1::Edge => connection
            .query_row(
                "SELECT body,from_id,to_id,t_valid,t_invalid,temporal_fallback,properties \
                 FROM canonical_edges WHERE scope=?1 AND write_cursor=?2",
                rusqlite::params![&stored.scope, cursor],
                |row| {
                    Ok((
                        row.get(0)?,
                        Some(row.get(1)?),
                        Some(row.get(2)?),
                        None,
                        None,
                        None,
                        row.get(3)?,
                        row.get(4)?,
                        row.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
                        decode_properties_row_col(row, 6)?,
                    ))
                },
            )
            .map_err(|_| EngineError::Storage)?,
    };
    let dependency = load_dependency(connection, &stored.scope, &stored.artifact_revision_id)?;
    let _material = GraphEvidenceMaterial {
        artifact_class: payload.artifact_class,
        write_cursor: payload.artifact_cursor,
        artifact_revision_id: stored.artifact_revision_id.clone(),
        logical_id: stored.logical_id.clone(),
        artifact_kind: stored.artifact_kind.clone(),
        artifact_body: artifact_body.clone(),
        node_state: node_state.clone(),
        node_valid_from,
        node_valid_until,
        edge_t_valid,
        edge_t_invalid,
        edge_temporal_fallback,
        artifact_superseded_at: stored.artifact_superseded_at,
        source_id: stored.source_id.clone(),
        source_version_id: stored.source_version_id.clone(),
        source_revision_id: stored.source_revision_id.clone(),
        locator: stored.locator.clone(),
        hash_digest: stored.hash_digest.clone(),
        source_lifecycle: stored.source_state.clone(),
        source_superseded_at: stored.source_superseded_at,
        source_valid_from: stored.source_valid_from,
        source_valid_until: stored.source_valid_until,
        dependency_id: dependency.as_ref().map(|value| value.dependency_id.as_str().to_string()),
        dependency_generation: dependency
            .as_ref()
            .map(|value| value.registered_dependency_generation),
        edge_from: edge_from.clone(),
        edge_to: edge_to.clone(),
        artifact_properties: artifact_properties.clone(),
    };
    let target_cursor = i64::try_from(payload.target_cursor).map_err(|_| EngineError::Storage)?;
    let edge_cursor =
        i64::try_from(payload.terminal_edge_cursor).map_err(|_| EngineError::Storage)?;
    let target: (String, String) = connection
        .query_row(
            "SELECT r.revision_id,n.logical_id FROM _fathomdb_artifact_revisions r \
             JOIN canonical_nodes n ON n.scope=r.scope AND n.write_cursor=r.write_cursor \
             WHERE r.scope=?1 AND r.artifact_class='node' AND r.write_cursor=?2",
            rusqlite::params![&stored.scope, target_cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let edge: (String, String, String, String) = connection
        .query_row(
            "SELECT r.revision_id,e.kind,e.from_id,e.to_id FROM _fathomdb_artifact_revisions r \
             JOIN canonical_edges e ON e.scope=r.scope AND e.write_cursor=r.write_cursor \
             WHERE r.scope=?1 AND r.artifact_class='edge' AND r.write_cursor=?2",
            rusqlite::params![&stored.scope, edge_cursor],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let predecessor = match payload.direction {
        GraphEvidenceDirection::Outgoing if edge.3 == target.1 => edge.2.as_str(),
        GraphEvidenceDirection::Incoming if edge.2 == target.1 => edge.3.as_str(),
        _ => return Err(EvidenceErrorV1::unavailable().into()),
    };
    let expected_artifact_cursor =
        if payload.role == 0 { payload.target_cursor } else { payload.terminal_edge_cursor };
    if payload.artifact_cursor != expected_artifact_cursor
        || payload.target_commitment != keyed(&key, GRAPH_TARGET_DOMAIN, target.1.as_bytes())
        || payload.predecessor_commitment
            != keyed(&key, GRAPH_PREDECESSOR_DOMAIN, predecessor.as_bytes())
        || payload.edge_kind_commitment != keyed(&key, GRAPH_EDGE_KIND_DOMAIN, edge.1.as_bytes())
        || payload.target_revision_commitment
            != keyed(&key, GRAPH_TARGET_REVISION_DOMAIN, target.0.as_bytes())
        || payload.edge_revision_commitment
            != keyed(&key, GRAPH_EDGE_REVISION_DOMAIN, edge.0.as_bytes())
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let evidence_text = slice(&stored.source_body, &stored.locator)?;
    Ok(GraphEvidenceResolvedMaterial {
        artifact_class: payload.artifact_class,
        artifact_revision_id: stored.artifact_revision_id,
        logical_id: stored.logical_id,
        canonical_source_body: stored.source_body,
        evidence_text,
        artifact_kind: stored.artifact_kind,
        artifact_body,
        source_id: stored.source_id,
        source_version_id: stored.source_version_id,
        source_revision_id: stored.source_revision_id,
        canonical_source_hash: stored.hash_digest,
        source_locator: stored.locator,
        node_state,
        edge_t_valid,
        edge_t_invalid,
        artifact_superseded_at: stored.artifact_superseded_at,
        source_state: stored.source_state,
        dependency,
        edge_from,
        edge_to,
        artifact_properties,
    })
}

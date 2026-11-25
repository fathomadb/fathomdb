use rusqlite::{Connection, OptionalExtension};

use super::codec::{
    decode_token, encode_token, keyed, locator_bytes, protect_generation, random_nonce, slice,
    Payload,
};
use super::{
    EvidenceArmV1, EvidenceArtifactClassV1, EvidenceArtifactLifecycleV1, EvidenceContributionV1,
    EvidenceErrorReasonV1, EvidenceErrorV1, EvidenceGraphOriginV1, EvidenceProjectionOriginV1,
    EvidenceRefV1, EvidenceResolveManyItemV1, EvidenceResolveRequestV1, EvidenceSearchResultV1,
    EvidenceSidecarEntryV1, ResolvedEvidenceV1, SCHEMA_VERSION, TOKEN_MAX_BYTES,
};
use crate::dependency::{
    load_persisted_canonical_source, validate_dependency_chain, validate_persisted_dependency_row,
    DependencyValidationMode, SourceDependencyV1,
};
use crate::dependency_closure::active_barrier_for_source;
use crate::errors::EngineError;
use crate::filter::{
    edge_fts_hit_passes_filter, text_hit_passes_filter, text_hit_passes_filter_view, SearchFilter,
};
use crate::frozen_read::{self, FrozenReadContextV1};
use crate::identity::{ArtifactRevisionId, CanonicalHash, DependencyId, SourceRevisionId};
use crate::projection_generation::{current_generation_id, ProjectionGenerationId};
use crate::properties::{decode_properties_json, PropertyMap};
use crate::provenance::SourceLocator;
use crate::record_lifecycle::LifecycleState;
use crate::search_types::{
    CapturedGraphOrigin, GraphArmSeedSource, PerHitExplain, SearchResult, SoftFallbackBranch,
};
use crate::temporal::ReadView;
use crate::write_commit::canonical_body_hash;

const DATABASE_DOMAIN: &[u8] = b"fathomdb.evidence.database.v1\0";
const CONTEXT_DOMAIN: &[u8] = b"fathomdb.evidence.context.v1\0";
const ARTIFACT_DOMAIN: &[u8] = b"fathomdb.evidence.artifact.v1\0";
const SOURCE_DOMAIN: &[u8] = b"fathomdb.evidence.source.v1\0";
const LOCATOR_DOMAIN: &[u8] = b"fathomdb.evidence.locator.v1\0";
const HASH_DOMAIN: &[u8] = b"fathomdb.evidence.hash.v1\0";
const GRAPH_EDGE_DOMAIN: &[u8] = b"fathomdb.evidence.graph-edge.v1\0";

#[derive(Clone, Debug)]
pub(super) struct StoredEvidence {
    pub(super) artifact_revision_id: String,
    pub(super) completeness: String,
    pub(super) logical_id: Option<String>,
    pub(super) artifact_kind: String,
    artifact_state: Option<String>,
    artifact_superseded: bool,
    pub(super) artifact_superseded_at: Option<i64>,
    artifact_head_inactive: bool,
    edge_valid: bool,
    edge_temporal_fallback: bool,
    pub(super) source_id: String,
    pub(super) source_version_id: String,
    pub(super) source_revision_id: String,
    pub(super) locator: SourceLocator,
    pub(super) hash_digest: String,
    pub(super) source_body: String,
    pub(super) source_cursor: u64,
    pub(super) source_kind: String,
    pub(super) source_state: String,
    source_superseded: bool,
    pub(super) source_superseded_at: Option<i64>,
    source_head_inactive: bool,
    pub(super) source_valid_from: Option<i64>,
    pub(super) source_valid_until: Option<i64>,
    artifact_properties: Option<PropertyMap>,
}

type SourceLinkRow = (String, String, String, String, Option<i64>, Option<i64>, String);
type SourceArtifactRow = (String, String, Option<i64>, i64, String, Option<i64>, Option<i64>, i64);
type NodeArtifactRow =
    (Option<String>, String, String, Option<i64>, Option<i64>, Option<i64>, Option<String>, i64);
type EdgeArtifactRow =
    (Option<String>, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>, Option<String>);

pub(super) fn evidence_as_of_sql_i64(cursor: u64) -> i64 {
    i64::try_from(cursor).unwrap_or(i64::MAX)
}

pub(crate) fn build_search_result(
    connection: &Connection,
    frozen: &FrozenReadContextV1,
    mut search_result: SearchResult,
    include_explanation: bool,
    graph_origins: &std::collections::HashMap<u64, CapturedGraphOrigin>,
    ownership: &[crate::fusion::FusionOwnership],
) -> Result<EvidenceSearchResultV1, EngineError> {
    let explanation = search_result.explanation.as_ref().ok_or_else(|| {
        EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/explanation")
    })?;
    if explanation.per_hit.len() != search_result.results.len()
        || ownership.len() != search_result.results.len()
    {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/explanation/perHit",
        )
        .into());
    }
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let generation = current_generation_id(connection)?;
    let context_bytes = frozen_read::validate_context(&frozen.context)?;
    let database_commitment = keyed(&key, DATABASE_DOMAIN, database_id.as_bytes());
    let context_commitment = keyed(&key, CONTEXT_DOMAIN, &context_bytes);
    let mut evidence = Vec::with_capacity(search_result.results.len());
    for (index, (hit, per_hit)) in
        search_result.results.iter().zip(explanation.per_hit.iter()).enumerate()
    {
        if per_hit.id != hit.write_cursor {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/explanation/perHit",
            )
            .into());
        }
        let artifact_class = match ownership[index] {
            crate::fusion::FusionOwnership::CanonicalNode => EvidenceArtifactClassV1::Node,
            crate::fusion::FusionOwnership::CanonicalEdge => EvidenceArtifactClassV1::Edge,
            crate::fusion::FusionOwnership::OwnerlessNodeFts { .. } => {
                return Err(EvidenceErrorV1::unavailable().into())
            }
        };
        let graph_origin = if hit.branch == SoftFallbackBranch::GraphArm {
            Some(graph_origins.get(&hit.write_cursor).cloned().ok_or_else(|| {
                EvidenceErrorV1::new(
                    EvidenceErrorReasonV1::EvidenceIncomplete,
                    format!("/results/{index}/graphOrigin"),
                )
            })?)
        } else {
            None
        };
        let graph_edge_commitment = match graph_origin.as_ref() {
            Some(CapturedGraphOrigin::EdgeSeed { edge_cursor })
            | Some(CapturedGraphOrigin::Traversal { edge_cursor, .. }) => {
                let (revision, _, _, _, source_revision) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    frozen.effective_valid_at,
                    frozen.context.view.include_out_of_window,
                    frozen.context.view.include_superseded,
                )?;
                if !source_revision_is_eligible(
                    connection,
                    &source_revision,
                    frozen.effective_valid_at,
                    frozen.context.view.include_out_of_window,
                )? {
                    return Err(EvidenceErrorV1::unavailable().into());
                }
                validate_dependency_chain(
                    connection,
                    &source_revision,
                    &revision,
                    DependencyValidationMode::Persisted,
                )
                .map_err(|_| {
                    EngineError::Evidence(EvidenceErrorV1::new(
                        EvidenceErrorReasonV1::EvidenceCorrupt,
                        format!("/results/{index}/graphOrigin"),
                    ))
                })?;
                Some(keyed(&key, GRAPH_EDGE_DOMAIN, revision.as_bytes()))
            }
            _ => None,
        };
        let stored = load_stored(
            connection,
            artifact_class,
            hit.write_cursor,
            frozen.effective_valid_at,
            frozen.context.view.include_out_of_window,
        )?;
        authorize_intrinsic_lifecycle(connection, &stored, artifact_class, &frozen.context.view)?;
        authorize_source_access(connection, &stored, &frozen.context.eligibility)?;
        if stored.completeness != "complete" {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceIncomplete,
                format!("/results/{index}/provenance"),
            )
            .into());
        }
        validate_source_bytes(&stored).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        validate_full_provenance(connection, &stored).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        load_dependency(connection, &stored.artifact_revision_id).map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                format!("/results/{index}/provenance"),
            ))
        })?;
        let locator_bytes = locator_bytes(&stored.locator);
        let contribution = contribution(per_hit)?;
        let generation_nonce = random_nonce()?;
        let payload = Payload {
            artifact_class,
            write_cursor: hit.write_cursor,
            effective_valid_at: frozen.effective_valid_at,
            database_commitment,
            context_commitment,
            artifact_commitment: keyed(
                &key,
                ARTIFACT_DOMAIN,
                stored.artifact_revision_id.as_bytes(),
            ),
            source_commitment: keyed(&key, SOURCE_DOMAIN, stored.source_revision_id.as_bytes()),
            locator_commitment: keyed(&key, LOCATOR_DOMAIN, &locator_bytes),
            hash_commitment: keyed(&key, HASH_DOMAIN, stored.hash_digest.as_bytes()),
            generation_nonce,
            generation_ciphertext: protect_generation(
                &key,
                &generation_nonce,
                generation.as_str().as_bytes(),
            ),
            arm: EvidenceArmV1::from_branch(hit.branch),
            contribution,
            graph_origin,
            graph_edge_commitment,
        };
        let token = encode_token(&key, &payload)?;
        evidence.push(EvidenceSidecarEntryV1 {
            schema_version: SCHEMA_VERSION,
            result_index: u32::try_from(index).map_err(|_| EngineError::Storage)?,
            artifact_revision_id: stored.artifact_revision_id,
            evidence_ref: EvidenceRefV1(token),
        });
    }
    if !include_explanation {
        search_result.explanation = None;
    }
    Ok(EvidenceSearchResultV1 { schema_version: SCHEMA_VERSION, search_result, evidence })
}

pub(crate) fn resolve(
    connection: &Connection,
    request: &EvidenceResolveRequestV1,
) -> Result<ResolvedEvidenceV1, EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        )
        .into());
    }
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let context_bytes = frozen_read::validate_context(&request.context.context)?;
    let database_commitment = keyed(&key, DATABASE_DOMAIN, database_id.as_bytes());
    let context_commitment = keyed(&key, CONTEXT_DOMAIN, &context_bytes);
    resolve_single_ref(
        connection,
        &key,
        &database_commitment,
        &context_commitment,
        request.evidence_ref.as_str(),
        &request.context,
    )
}

pub(crate) fn resolve_many_raw(
    connection: &Connection,
    raw_refs: &[&str],
    context: &FrozenReadContextV1,
) -> Result<Vec<EvidenceResolveManyItemV1>, EngineError> {
    let (database_id, key) = frozen_read::page_cursor_material(connection)?;
    let context_bytes = frozen_read::validate_context(&context.context)
        .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let database_commitment = keyed(&key, DATABASE_DOMAIN, database_id.as_bytes());
    let context_commitment = keyed(&key, CONTEXT_DOMAIN, &context_bytes);

    let mut cache: std::collections::HashMap<
        &str,
        Result<ResolvedEvidenceV1, EvidenceErrorReasonV1>,
    > = std::collections::HashMap::with_capacity(raw_refs.len());
    let mut out = Vec::with_capacity(raw_refs.len());

    for (index, &raw_ref) in raw_refs.iter().enumerate() {
        if !cache.contains_key(raw_ref) {
            let outcome = if raw_ref.is_empty() || raw_ref.len() > TOKEN_MAX_BYTES {
                Err(EvidenceErrorReasonV1::EvidenceUnavailable)
            } else {
                match resolve_single_ref(
                    connection,
                    &key,
                    &database_commitment,
                    &context_commitment,
                    raw_ref,
                    context,
                ) {
                    Ok(resolved) => Ok(resolved),
                    Err(EngineError::Evidence(error)) => {
                        let reason = match error.reason {
                            EvidenceErrorReasonV1::EvidenceCorrupt => {
                                EvidenceErrorReasonV1::EvidenceCorrupt
                            }
                            EvidenceErrorReasonV1::EvidenceIncomplete => {
                                EvidenceErrorReasonV1::EvidenceIncomplete
                            }
                            _ => EvidenceErrorReasonV1::EvidenceUnavailable,
                        };
                        Err(reason)
                    }
                    Err(other) => return Err(other),
                }
            };
            cache.insert(raw_ref, outcome);
        }
        match cache.get(raw_ref).expect("just inserted") {
            Ok(resolved) => {
                out.push(EvidenceResolveManyItemV1::Resolved(Box::new(resolved.clone())))
            }
            Err(reason) => out.push(EvidenceResolveManyItemV1::Refused(EvidenceErrorV1::new(
                *reason,
                format!("/evidenceRefs/{index}"),
            ))),
        }
    }
    Ok(out)
}

fn resolve_single_ref(
    connection: &Connection,
    key: &[u8],
    database_commitment: &[u8; 32],
    context_commitment: &[u8; 32],
    raw_ref: &str,
    context: &FrozenReadContextV1,
) -> Result<ResolvedEvidenceV1, EngineError> {
    let payload = decode_token(key, raw_ref)?;
    if payload.database_commitment != *database_commitment
        || payload.context_commitment != *context_commitment
        || payload.effective_valid_at != context.effective_valid_at
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let stored = load_stored(
        connection,
        payload.artifact_class,
        payload.write_cursor,
        payload.effective_valid_at,
        context.context.view.include_out_of_window,
    )
    .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    if payload.artifact_commitment
        != keyed(key, ARTIFACT_DOMAIN, stored.artifact_revision_id.as_bytes())
        || payload.source_commitment
            != keyed(key, SOURCE_DOMAIN, stored.source_revision_id.as_bytes())
        || payload.locator_commitment != keyed(key, LOCATOR_DOMAIN, &locator_bytes(&stored.locator))
        || payload.hash_commitment != keyed(key, HASH_DOMAIN, stored.hash_digest.as_bytes())
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let (graph_origin, graph_provenance) =
        match (&payload.graph_origin, payload.graph_edge_commitment) {
            (None, None) if payload.arm != EvidenceArmV1::GraphArm => (None, None),
            (Some(CapturedGraphOrigin::EdgeSeed { edge_cursor }), Some(commitment))
                if payload.arm == EvidenceArmV1::GraphArm =>
            {
                let (revision, from_id, to_id, _, source_revision_id) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    payload.effective_valid_at,
                    context.context.view.include_out_of_window,
                    context.context.view.include_superseded,
                )
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
                if commitment != keyed(key, GRAPH_EDGE_DOMAIN, revision.as_bytes())
                    || stored
                        .logical_id
                        .as_ref()
                        .is_none_or(|logical| logical != &from_id && logical != &to_id)
                    || !source_revision_is_eligible(
                        connection,
                        &source_revision_id,
                        payload.effective_valid_at,
                        context.context.view.include_out_of_window,
                    )?
                {
                    return Err(EvidenceErrorV1::unavailable().into());
                }
                (
                    Some(EvidenceGraphOriginV1::EdgeSeed {
                        edge_artifact_revision_id: revision.clone(),
                        seed_source: GraphArmSeedSource::EdgeFts,
                    }),
                    Some((revision, source_revision_id)),
                )
            }
            (
                Some(CapturedGraphOrigin::Traversal { edge_cursor, hop_count, seed_source }),
                Some(commitment),
            ) if payload.arm == EvidenceArmV1::GraphArm => {
                let (revision, from_id, to_id, _, source_revision_id) = load_graph_edge_revision(
                    connection,
                    *edge_cursor,
                    payload.effective_valid_at,
                    context.context.view.include_out_of_window,
                    context.context.view.include_superseded,
                )
                .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
                if commitment != keyed(key, GRAPH_EDGE_DOMAIN, revision.as_bytes())
                    || stored
                        .logical_id
                        .as_ref()
                        .is_none_or(|logical| logical != &from_id && logical != &to_id)
                    || !source_revision_is_eligible(
                        connection,
                        &source_revision_id,
                        payload.effective_valid_at,
                        context.context.view.include_out_of_window,
                    )?
                {
                    return Err(EvidenceErrorV1::unavailable().into());
                }
                (
                    Some(EvidenceGraphOriginV1::Traversal {
                        edge_artifact_revision_id: revision.clone(),
                        hop_count: *hop_count,
                        seed_source: *seed_source,
                    }),
                    Some((revision, source_revision_id)),
                )
            }
            _ => return Err(EvidenceErrorV1::unavailable().into()),
        };
    let (source_state, artifact_lifecycle) = authorize_intrinsic_lifecycle(
        connection,
        &stored,
        payload.artifact_class,
        &context.context.view,
    )?;
    let eligibility = &context.context.eligibility;
    let artifact_eligible = match payload.artifact_class {
        EvidenceArtifactClassV1::Node => text_hit_passes_filter_view(
            connection,
            payload.write_cursor,
            &stored.artifact_kind,
            Some(eligibility),
            &context.context.view,
        ),
        EvidenceArtifactClassV1::Edge => edge_fts_hit_passes_filter(
            connection,
            payload.write_cursor,
            &stored.artifact_kind,
            Some(eligibility),
        ),
    }
    .map_err(|_| EngineError::Storage)?;
    if !artifact_eligible {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    authorize_source_access(connection, &stored, eligibility)?;
    if stored.completeness != "complete" {
        return Err(
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, "/provenance").into()
        );
    }
    let canonical_source_hash =
        CanonicalHash::sha256(stored.hash_digest.clone()).map_err(|_| {
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/canonicalSourceHash")
        })?;
    let actual_hash = canonical_body_hash(&stored.source_body);
    if actual_hash != stored.hash_digest {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/canonicalSourceHash",
        )
        .into());
    }
    let evidence_text = slice(&stored.source_body, &stored.locator)?;
    validate_full_provenance(connection, &stored)?;
    if let Some((edge_revision, source_revision)) = graph_provenance {
        validate_dependency_chain(
            connection,
            &source_revision,
            &edge_revision,
            DependencyValidationMode::Persisted,
        )
        .map_err(|_| {
            EngineError::Evidence(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/projectionOrigin/graphOrigin",
            ))
        })?;
    }
    let generation = resolve_generation(
        connection,
        key,
        &payload.generation_nonce,
        &payload.generation_ciphertext,
    )?;
    let dependency = load_dependency(connection, &stored.artifact_revision_id)?;
    Ok(ResolvedEvidenceV1 {
        schema_version: SCHEMA_VERSION,
        logical_id: stored.logical_id,
        artifact_revision_id: stored.artifact_revision_id,
        source_id: stored.source_id,
        source_version_id: stored.source_version_id,
        source_revision_id: stored.source_revision_id,
        locator: stored.locator,
        canonical_source_body: stored.source_body,
        evidence_text,
        canonical_source_hash,
        effective_valid_at: payload.effective_valid_at,
        artifact_lifecycle,
        source_lifecycle_state: source_state,
        projection_origin: EvidenceProjectionOriginV1 {
            schema_version: SCHEMA_VERSION,
            artifact_class: payload.artifact_class,
            representative_arm: payload.arm,
            projection_generation_id: generation,
            graph_origin,
        },
        retrieval_contribution: payload.contribution,
        dependency,
        properties: stored.artifact_properties,
    })
}

fn authorize_source_access(
    connection: &Connection,
    stored: &StoredEvidence,
    eligibility: &SearchFilter,
) -> Result<(), EngineError> {
    // A source is a separately authorized canonical artifact. Domain-oriented
    // kind/source-type constraints select the returned artifact, while access-
    // bearing metadata and declared attributes constrain both artifact and bytes.
    let mut source_filter = eligibility.clone();
    source_filter.kind = None;
    source_filter.source_type = None;
    if !text_hit_passes_filter(
        connection,
        stored.source_cursor,
        &stored.source_kind,
        Some(&source_filter),
    )
    .map_err(|_| EngineError::Storage)?
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(())
}

fn authorize_intrinsic_lifecycle(
    connection: &Connection,
    stored: &StoredEvidence,
    artifact_class: EvidenceArtifactClassV1,
    view: &ReadView,
) -> Result<(LifecycleState, EvidenceArtifactLifecycleV1), EngineError> {
    if (stored.artifact_superseded && !view.include_superseded)
        || stored.source_superseded
        || active_barrier_for_source(connection, &stored.source_revision_id)?
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let source_state = LifecycleState::from_str_opt(&stored.source_state)
        .filter(|state| *state == LifecycleState::Active)
        .ok_or_else(EvidenceErrorV1::unavailable)?;
    let artifact_lifecycle = match artifact_class {
        EvidenceArtifactClassV1::Node => {
            let state = stored
                .artifact_state
                .as_deref()
                .and_then(LifecycleState::from_str_opt)
                .filter(|state| view.include_inactive || *state == LifecycleState::Active)
                .ok_or_else(EvidenceErrorV1::unavailable)?;
            EvidenceArtifactLifecycleV1::Node { state, superseded: stored.artifact_superseded }
        }
        EvidenceArtifactClassV1::Edge => {
            if !stored.edge_valid {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            EvidenceArtifactLifecycleV1::Edge {
                superseded: stored.artifact_superseded,
                valid_at_effective: true,
            }
        }
    };
    Ok((source_state, artifact_lifecycle))
}

pub(super) fn authorize_graph_intrinsic_lifecycle(
    connection: &Connection,
    stored: &StoredEvidence,
    artifact_class: EvidenceArtifactClassV1,
    view: &ReadView,
    historical: bool,
) -> Result<(), EngineError> {
    if !historical {
        authorize_intrinsic_lifecycle(connection, stored, artifact_class, view)?;
        if artifact_class == EvidenceArtifactClassV1::Edge && stored.edge_temporal_fallback {
            return Err(EvidenceErrorV1::unavailable().into());
        }
        return Ok(());
    }
    if stored.artifact_head_inactive
        || stored.source_head_inactive
        || active_barrier_for_source(connection, &stored.source_revision_id)?
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    LifecycleState::from_str_opt(&stored.source_state)
        .filter(|state| *state == LifecycleState::Active)
        .ok_or_else(EvidenceErrorV1::unavailable)?;
    match artifact_class {
        EvidenceArtifactClassV1::Node => {
            stored
                .artifact_state
                .as_deref()
                .and_then(LifecycleState::from_str_opt)
                .filter(|state| *state == LifecycleState::Active)
                .ok_or_else(EvidenceErrorV1::unavailable)?;
        }
        EvidenceArtifactClassV1::Edge => {
            if !stored.edge_valid || stored.edge_temporal_fallback {
                return Err(EvidenceErrorV1::unavailable().into());
            }
        }
    }
    Ok(())
}

fn contribution(per_hit: &PerHitExplain) -> Result<EvidenceContributionV1, EngineError> {
    let values = [
        Some(per_hit.fused_score),
        per_hit.ce_score,
        Some(per_hit.blended),
        per_hit.importance,
        per_hit.confidence,
    ];
    if values.into_iter().flatten().any(|value| !value.is_finite()) {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/explanation/perHit",
        )
        .into());
    }
    Ok(EvidenceContributionV1 {
        schema_version: SCHEMA_VERSION,
        vector_rank: per_hit.vector_rank,
        text_rank: per_hit.text_rank,
        graph_rank: per_hit.graph_rank,
        fused_score: per_hit.fused_score,
        ce_score: per_hit.ce_score,
        blended_score: per_hit.blended,
        importance: per_hit.importance,
        confidence: per_hit.confidence,
    })
}

pub(super) fn load_stored(
    connection: &Connection,
    class: EvidenceArtifactClassV1,
    cursor: u64,
    effective: i64,
    include_out_of_window: bool,
) -> Result<StoredEvidence, EngineError> {
    let cursor = i64::try_from(cursor).map_err(|_| EvidenceErrorV1::unavailable())?;
    let artifact: Option<(String, String)> = connection
        .query_row(
            "SELECT revision_id,completeness FROM _fathomdb_artifact_revisions \
             WHERE artifact_class=?1 AND write_cursor=?2",
            rusqlite::params![class.as_str(), cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let (artifact_revision_id, completeness) = artifact.ok_or_else(EvidenceErrorV1::unavailable)?;
    let link: Option<SourceLinkRow> = connection
        .query_row(
            "SELECT source_id,source_version_id,source_revision_id,locator_kind,\
                    start_byte,end_byte,hash_digest FROM _fathomdb_source_links \
             WHERE artifact_revision_id=?1",
            [&artifact_revision_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let (source_id, source_version_id, source_revision_id, locator_kind, start, end, hash_digest) =
        link.ok_or_else(|| {
            EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceIncomplete, "/provenance")
        })?;
    let locator = match (locator_kind.as_str(), start, end) {
        ("whole_body", None, None) => SourceLocator::WholeBody,
        ("utf8_bytes", Some(start), Some(end)) if start >= 0 && end >= 0 => {
            SourceLocator::utf8_bytes(start as u64, end as u64)
        }
        _ => {
            return Err(EvidenceErrorV1::new(
                EvidenceErrorReasonV1::EvidenceCorrupt,
                "/provenance/sourceLocator",
            )
            .into())
        }
    };
    let source: Option<SourceArtifactRow> = connection
        .query_row(
            "SELECT n.body,n.state,n.superseded_at,n.write_cursor,n.kind,n.valid_from,n.valid_until,\
                    CASE WHEN n.superseded_at IS NOT NULL AND n.logical_id IS NOT NULL \
                         AND EXISTS(SELECT 1 FROM canonical_nodes cur \
                                    WHERE cur.logical_id=n.logical_id \
                                      AND cur.superseded_at IS NULL \
                                      AND cur.state!='active') \
                         THEN 1 ELSE 0 END \
             FROM _fathomdb_artifact_revisions ar \
             JOIN canonical_nodes n ON n.write_cursor=ar.write_cursor \
             WHERE ar.revision_id=?1 AND ar.artifact_class='node' \
               AND ar.artifact_role='canonical_source'",
            [&source_revision_id],
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
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let (
        source_body,
        source_state,
        source_superseded_at,
        source_cursor,
        source_kind,
        source_valid_from,
        source_valid_until,
        source_head_inactive,
    ) = source.ok_or_else(EvidenceErrorV1::unavailable)?;
    if !include_out_of_window
        && (source_valid_from.is_some_and(|start| start > effective)
            || source_valid_until.is_some_and(|end| end <= effective))
    {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    let (
        logical_id,
        artifact_kind,
        artifact_state,
        artifact_superseded_at,
        artifact_head_inactive,
        _artifact_valid_from,
        _artifact_valid_until,
        edge_valid,
        edge_temporal_fallback,
        artifact_properties_raw,
    ) = match class {
        EvidenceArtifactClassV1::Node => {
            let row: Option<NodeArtifactRow> = connection
                .query_row(
                    "SELECT logical_id,kind,state,superseded_at,valid_from,valid_until,properties,\
                            CASE WHEN superseded_at IS NOT NULL AND logical_id IS NOT NULL \
                                 AND EXISTS(SELECT 1 FROM canonical_nodes cur \
                                            WHERE cur.logical_id=canonical_nodes.logical_id \
                                              AND cur.superseded_at IS NULL \
                                              AND cur.state!='active') \
                                 THEN 1 ELSE 0 END \
                         FROM canonical_nodes WHERE write_cursor=?1",
                    [cursor],
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
                        ))
                    },
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let (
                logical,
                kind,
                state,
                superseded,
                valid_from,
                valid_until,
                props_raw,
                head_inactive,
            ) = row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = valid_from.is_none_or(|start| start <= effective)
                && valid_until.is_none_or(|end| end > effective);
            if !include_out_of_window && !valid {
                return Err(EvidenceErrorV1::unavailable().into());
            }
            (
                logical,
                kind,
                Some(state),
                superseded,
                head_inactive != 0,
                valid_from,
                valid_until,
                true,
                false,
                props_raw,
            )
        }
        EvidenceArtifactClassV1::Edge => {
            let row: Option<EdgeArtifactRow> = connection
                .query_row(
                    "SELECT logical_id,kind,superseded_at,t_valid,t_invalid,temporal_fallback,properties \
                     FROM canonical_edges WHERE write_cursor=?1",
                    [cursor],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| EngineError::Storage)?;
            let (logical, kind, superseded, valid_from, valid_until, temporal_fallback, props_raw) =
                row.ok_or_else(EvidenceErrorV1::unavailable)?;
            let valid = include_out_of_window
                || (valid_from.is_none_or(|start| start <= effective)
                    && valid_until.is_none_or(|end| end > effective));
            (
                logical,
                kind,
                None,
                superseded,
                false,
                valid_from,
                valid_until,
                valid,
                temporal_fallback.unwrap_or(0) != 0,
                props_raw,
            )
        }
    };
    let artifact_properties = decode_properties_json(artifact_properties_raw.as_deref())?;
    Ok(StoredEvidence {
        artifact_revision_id,
        completeness,
        logical_id,
        artifact_kind,
        artifact_state,
        artifact_superseded: artifact_superseded_at.is_some(),
        artifact_superseded_at,
        artifact_head_inactive,
        edge_valid,
        edge_temporal_fallback,
        source_id,
        source_version_id,
        source_revision_id,
        locator,
        hash_digest,
        source_body,
        source_cursor: u64::try_from(source_cursor).map_err(|_| EngineError::Storage)?,
        source_kind,
        source_state,
        source_superseded: source_superseded_at.is_some(),
        source_superseded_at,
        source_head_inactive: source_head_inactive != 0,
        source_valid_from,
        source_valid_until,
        artifact_properties,
    })
}

fn load_graph_edge_revision(
    connection: &Connection,
    cursor: u64,
    effective: i64,
    include_out_of_window: bool,
    include_superseded: bool,
) -> Result<(String, String, String, String, String), EngineError> {
    let cursor = i64::try_from(cursor).map_err(|_| EvidenceErrorV1::unavailable())?;
    connection
        .query_row(
            "SELECT ar.revision_id,e.from_id,e.to_id,e.kind,l.source_revision_id \
             FROM canonical_edges e \
             JOIN _fathomdb_artifact_revisions ar \
               ON ar.write_cursor=e.write_cursor AND ar.artifact_class='edge' \
             JOIN _fathomdb_source_links l ON l.artifact_revision_id=ar.revision_id \
             WHERE e.write_cursor=?1 AND (?4=1 OR e.superseded_at IS NULL) \
               AND (?3=1 OR ((e.t_valid IS NULL OR e.t_valid<=?2) \
                             AND (e.t_invalid IS NULL OR e.t_invalid>?2))) \
               AND (e.temporal_fallback IS NULL OR e.temporal_fallback=0)",
            rusqlite::params![
                cursor,
                effective,
                i64::from(include_out_of_window),
                i64::from(include_superseded)
            ],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .ok_or_else(|| EvidenceErrorV1::unavailable().into())
}

fn source_revision_is_eligible(
    connection: &Connection,
    source_revision_id: &str,
    effective: i64,
    include_out_of_window: bool,
) -> Result<bool, EngineError> {
    if active_barrier_for_source(connection, source_revision_id)? {
        return Ok(false);
    }
    connection
        .query_row(
            "SELECT EXISTS(\
               SELECT 1 FROM _fathomdb_artifact_revisions ar \
               JOIN canonical_nodes n ON n.write_cursor=ar.write_cursor \
               WHERE ar.revision_id=?1 AND ar.artifact_class='node' \
                 AND ar.artifact_role='canonical_source' AND ar.completeness='complete' \
                 AND n.superseded_at IS NULL AND n.state='active' \
                 AND (?3=1 OR ((n.valid_from IS NULL OR n.valid_from<=?2) \
                               AND (n.valid_until IS NULL OR n.valid_until>?2)))\
             )",
            rusqlite::params![source_revision_id, effective, i64::from(include_out_of_window)],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)
}

fn resolve_generation(
    connection: &Connection,
    key: &[u8],
    nonce: &[u8; 16],
    ciphertext: &[u8],
) -> Result<ProjectionGenerationId, EngineError> {
    let plaintext = protect_generation(key, nonce, ciphertext);
    let generation = String::from_utf8(plaintext).map_err(|_| EvidenceErrorV1::unavailable())?;
    let generation = ProjectionGenerationId::new(generation)
        .map_err(|_| EngineError::Evidence(EvidenceErrorV1::unavailable()))?;
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
             WHERE generation_id=?1)",
            [generation.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !exists {
        return Err(EvidenceErrorV1::unavailable().into());
    }
    Ok(generation)
}

pub(super) fn validate_source_bytes(stored: &StoredEvidence) -> Result<(), EngineError> {
    CanonicalHash::sha256(stored.hash_digest.clone()).map_err(|_| {
        EvidenceErrorV1::new(EvidenceErrorReasonV1::EvidenceCorrupt, "/canonicalSourceHash")
    })?;
    if canonical_body_hash(&stored.source_body) != stored.hash_digest {
        return Err(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/canonicalSourceHash",
        )
        .into());
    }
    slice(&stored.source_body, &stored.locator).map(|_| ())
}

pub(super) fn validate_full_provenance(
    connection: &Connection,
    stored: &StoredEvidence,
) -> Result<(), EngineError> {
    let validation = if stored.artifact_revision_id == stored.source_revision_id {
        load_persisted_canonical_source(connection, &stored.source_revision_id)
            .and_then(|source| source.ok_or(EngineError::Storage).map(|_| ()))
    } else {
        validate_dependency_chain(
            connection,
            &stored.source_revision_id,
            &stored.artifact_revision_id,
            DependencyValidationMode::Persisted,
        )
    };
    validation.map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/provenance",
        ))
    })
}

pub(super) fn load_dependency(
    connection: &Connection,
    artifact_revision_id: &str,
) -> Result<Option<SourceDependencyV1>, EngineError> {
    let row: Option<(i64, String, i64, String)> = connection
        .query_row(
            "SELECT d.schema_version,d.dependency_id,d.registered_dependency_generation,\
                    l.source_revision_id \
             FROM _fathomdb_source_dependencies d \
             LEFT JOIN _fathomdb_source_links l \
               ON l.artifact_revision_id=d.derived_revision_id \
             WHERE d.derived_revision_id=?1",
            [artifact_revision_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((schema, dependency_id, generation, source_revision_id)) = row else {
        return Ok(None);
    };
    validate_persisted_dependency_row(
        connection,
        schema,
        &dependency_id,
        artifact_revision_id,
        generation,
    )
    .map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/dependency",
        ))
    })?;
    validate_dependency_chain(
        connection,
        &source_revision_id,
        artifact_revision_id,
        DependencyValidationMode::Persisted,
    )
    .map_err(|_| {
        EngineError::Evidence(EvidenceErrorV1::new(
            EvidenceErrorReasonV1::EvidenceCorrupt,
            "/dependency",
        ))
    })?;
    Ok(Some(SourceDependencyV1 {
        schema_version: 1,
        dependency_id: DependencyId(dependency_id),
        source_revision_id: SourceRevisionId(source_revision_id),
        derived_revision_id: ArtifactRevisionId(artifact_revision_id.to_string()),
        registered_dependency_generation: u64::try_from(generation)
            .map_err(|_| EngineError::Storage)?,
    }))
}

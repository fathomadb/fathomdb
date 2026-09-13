//! 0.8.26 Slice 10 — frozen explanation completion and equivalence.

use std::fs;

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn seed(engine: &Engine) {
    let source_body = "slice10 canonical evidence bytes";
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: source_body.into(),
                source_id: SourceId::new("slice10-source").unwrap(),
                logical_id: Some("slice10-source-logical".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("slice10-source-r1").unwrap(),
                    SourceVersionId::new("slice10-source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "claim".into(),
                body: "slice10frozenneedle".into(),
                source_id: SourceId::new("slice10-source").unwrap(),
                logical_id: Some("slice10-claim-logical".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("slice10-claim-r1").unwrap(),
                    SourceVersionId::new("slice10-source-v1").unwrap(),
                    SourceRevisionId::new("slice10-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(source_body)).unwrap(),
                ),
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
}

fn evidence_request(
    context: fathomdb_engine::FrozenReadContextV1,
    include_explanation: bool,
) -> EvidenceSearchRequestV1 {
    EvidenceSearchRequestV1 {
        schema_version: 1,
        query: "slice10frozenneedle".into(),
        context,
        rerank_depth: 0,
        use_graph_arm: false,
        alpha: 0.3,
        pool_n: 0,
        include_explanation,
        limit: 10,
    }
}

#[test]
fn frozen_explanations_finalize_once_and_preserve_results() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("frozen-explanation{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    seed(&opened.engine);
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();

    let plain_frozen = opened
        .engine
        .search_frozen("slice10frozenneedle", &frozen, 0, false, 0.3, 0, false, 10)
        .unwrap();
    let explained_frozen = opened
        .engine
        .search_frozen("slice10frozenneedle", &frozen, 0, false, 0.3, 0, true, 10)
        .unwrap();
    let plain_evidence =
        opened.engine.search_with_evidence(&evidence_request(frozen.clone(), false)).unwrap();
    let explained_evidence =
        opened.engine.search_with_evidence(&evidence_request(frozen.clone(), true)).unwrap();

    assert!(plain_frozen.explanation.is_none());
    assert!(plain_evidence.search_result.explanation.is_none());
    assert_eq!(plain_frozen.results, explained_frozen.results);
    assert_eq!(plain_frozen.projection_cursor, explained_frozen.projection_cursor);
    assert_eq!(plain_frozen.soft_fallback, explained_frozen.soft_fallback);
    assert_eq!(plain_evidence.search_result.results, explained_evidence.search_result.results);
    assert_eq!(
        plain_evidence.search_result.projection_cursor,
        explained_evidence.search_result.projection_cursor
    );
    assert_eq!(
        plain_evidence.search_result.soft_fallback,
        explained_evidence.search_result.soft_fallback
    );
    assert_eq!(
        plain_evidence
            .evidence
            .iter()
            .map(|entry| (entry.result_index, entry.artifact_revision_id.clone()))
            .collect::<Vec<_>>(),
        explained_evidence
            .evidence
            .iter()
            .map(|entry| (entry.result_index, entry.artifact_revision_id.clone()))
            .collect::<Vec<_>>()
    );

    let frozen_id = &explained_frozen.explanation.as_ref().unwrap().correlation_id;
    let evidence_id =
        &explained_evidence.search_result.explanation.as_ref().unwrap().correlation_id;
    assert!(frozen_id.starts_with('x'), "unexpected fallback identity: {frozen_id:?}");
    assert!(evidence_id.starts_with('x'), "unexpected fallback identity: {evidence_id:?}");
    assert_ne!(frozen_id, evidence_id);
    assert!(opened.engine.last_telemetry_query_id().is_none());

    let resolve = |entry: &fathomdb_engine::EvidenceSidecarEntryV1| {
        opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.evidence_ref.clone(),
                context: frozen.clone(),
            })
            .unwrap()
    };
    assert_eq!(resolve(&plain_evidence.evidence[0]), resolve(&explained_evidence.evidence[0]));

    let sink = dir.path().join("telemetry.jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    opened
        .engine
        .search_frozen("slice10frozenneedle", &frozen, 0, false, 0.3, 0, false, 10)
        .unwrap();
    opened.engine.search_with_evidence(&evidence_request(frozen.clone(), false)).unwrap();
    assert_eq!(fs::read_to_string(&sink).unwrap(), "");
    assert!(opened.engine.last_telemetry_query_id().is_none());

    let telemetry_frozen = opened
        .engine
        .search_frozen("slice10frozenneedle", &frozen, 0, false, 0.3, 0, true, 10)
        .unwrap();
    let telemetry_evidence =
        opened.engine.search_with_evidence(&evidence_request(frozen, true)).unwrap();
    assert_eq!(telemetry_frozen.explanation.unwrap().correlation_id, "q0-0");
    assert_eq!(telemetry_evidence.search_result.explanation.unwrap().correlation_id, "q0-1");
    assert_eq!(opened.engine.last_telemetry_query_id().as_deref(), Some("q0-1"));
    assert_eq!(fs::read_to_string(sink).unwrap().lines().count(), 2);
}

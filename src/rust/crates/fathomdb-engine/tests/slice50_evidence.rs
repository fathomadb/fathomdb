//! 0.8.25 Slice 50 — compact, eligibility-bound source evidence.

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, EvidenceArtifactLifecycleV1,
    EvidenceErrorReasonV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1, InitialState,
    LifecycleState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("source-low-entropy").unwrap(),
        logical_id: Some("source-logical-low-entropy".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-revision-low-entropy").unwrap(),
            SourceVersionId::new("source-version-low-entropy").unwrap(),
        ),
    })
}

fn derived(source_body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "claim".into(),
        body: "needle derived claim".into(),
        source_id: SourceId::new("source-low-entropy").unwrap(),
        logical_id: Some("derived-logical-low-entropy".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("derived-revision-low-entropy").unwrap(),
            SourceVersionId::new("source-version-low-entropy").unwrap(),
            SourceRevisionId::new("source-revision-low-entropy").unwrap(),
            SourceLocator::utf8_bytes(1, 3),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
    })
}

fn request(query: &str, context: fathomdb_engine::FrozenReadContextV1) -> EvidenceSearchRequestV1 {
    EvidenceSearchRequestV1 {
        schema_version: 1,
        query: query.into(),
        context,
        rerank_depth: 0,
        use_graph_arm: false,
        alpha: 0.3,
        pool_n: 0,
        include_explanation: false,
        limit: 10,
    }
}

#[test]
fn resolves_exact_utf8_source_span_and_associates_sidecar_by_position() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("evidence{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();

    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();

    assert_eq!(result.schema_version, 1);
    assert_eq!(result.search_result.results.len(), 1);
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(result.evidence[0].result_index, 0);
    assert_eq!(result.evidence[0].artifact_revision_id, "derived-revision-low-entropy");

    let resolved = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, "derived-revision-low-entropy");
    assert_eq!(resolved.source_id, "source-low-entropy");
    assert_eq!(resolved.source_version_id, "source-version-low-entropy");
    assert_eq!(resolved.source_revision_id, "source-revision-low-entropy");
    assert_eq!(resolved.canonical_source_body, source_body);
    assert_eq!(resolved.evidence_text, "é");
    assert_eq!(resolved.locator, SourceLocator::utf8_bytes(1, 3));
    assert_eq!(
        resolved.artifact_lifecycle,
        EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: false }
    );
    assert_eq!(resolved.source_lifecycle_state, LifecycleState::Active);
}

#[test]
fn tamper_and_context_mismatch_share_the_exact_nondisclosure_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("nondisclosure{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();

    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();
    let reference = &result.evidence[0].evidence_ref;

    let mut tampered_text = reference.as_str().to_string();
    let last = tampered_text.pop().unwrap();
    tampered_text.push(if last == '0' { '1' } else { '0' });
    let tampered = fathomdb_engine::EvidenceRefV1::new(tampered_text).unwrap();

    let mut different_filter = SearchFilter::default();
    different_filter.kind = Some("claim".into());
    let different = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), different_filter).unwrap())
        .unwrap();

    let errors = [tampered, reference.clone()]
        .into_iter()
        .zip([frozen, different])
        .map(|(evidence_ref, context)| {
            opened
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref,
                    context,
                })
                .unwrap_err()
        })
        .collect::<Vec<_>>();

    for error in errors {
        match error {
            EngineError::Evidence(error) => {
                assert_eq!(error.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
                assert_eq!(error.field_path, "/evidenceRef");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[test]
fn visible_reference_payload_uses_keyed_not_dictionary_matchable_commitments() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("privacy{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();

    let result = opened.engine.search_with_evidence(&request("needle", frozen)).unwrap();
    let token = result.evidence[0].evidence_ref.as_str();
    let payload_hex = token.split('.').nth(1).unwrap();
    let payload = (0..payload_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&payload_hex[index..index + 2], 16).unwrap())
        .collect::<Vec<_>>();

    for secret in [
        "source-low-entropy",
        "source-revision-low-entropy",
        "derived-revision-low-entropy",
        source_body,
    ] {
        assert!(!String::from_utf8_lossy(&payload).contains(secret));
        assert!(!payload
            .windows(32)
            .any(|window| window == Sha256::digest(secret.as_bytes()).as_slice()));
    }
}

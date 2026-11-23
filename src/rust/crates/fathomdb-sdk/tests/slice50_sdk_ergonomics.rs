//! Slice 50 (0.8.31) — `fathomdb-sdk` tests for `resolve_evidence_many`,
//! `SearchOptions.hints`, `FrozenSearchOptions.hints`, `EvidenceSearchRequestV1::with_hints`,
//! and `OpenReport` capability booleans.

use fathomdb_sdk::{
    ArtifactRevisionId, Engine, ErrorKind, EvidenceErrorReasonV1, EvidenceRefV1,
    EvidenceResolveManyItemV1, EvidenceResolveManyRequestV1, EvidenceSearchRequestV1,
    FrozenSearchOptions, HintAction, HintArm, HintCause, HintReason, InitialState, OpenOptions,
    PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SearchOptions,
    SourceId, SourceVersionId, WriteProvenanceV1, EVIDENCE_RESOLVE_MANY_MAX,
};
use tempfile::TempDir;

fn canonical_doc(i: usize, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new(format!("src:{i}")).unwrap(),
        logical_id: Some(format!("doc:{i}")),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(format!("rev:{i}")).unwrap(),
            SourceVersionId::new(format!("ver:{i}")).unwrap(),
        ),
        properties: None,
    })
}

#[test]
fn sdk_resolve_evidence_many_bounds_duplicates_and_partial_refusal() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path().join("sdk_many.sqlite"), OpenOptions::default()).unwrap();
    engine
        .write(&[canonical_doc(1, "needle alpha sdk"), canonical_doc(2, "needle beta sdk")])
        .unwrap();

    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let searched = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), frozen.clone()))
        .unwrap();
    assert_eq!(searched.evidence.len(), 2);
    let r0 = searched.evidence[0].evidence_ref.clone();

    // 0 -> InvalidArgument
    let err = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: Vec::new(),
            context: frozen.clone(),
        })
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidArgument);

    // 51 -> InvalidArgument
    let err = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![r0.clone(); EVIDENCE_RESOLVE_MANY_MAX + 1],
            context: frozen.clone(),
        })
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidArgument);

    // [valid, bad, valid] -> partial results preserving order
    let bad = EvidenceRefV1::new("bad-ref").unwrap();
    let items = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![r0.clone(), bad, r0],
            context: frozen,
        })
        .unwrap();
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0], EvidenceResolveManyItemV1::Resolved(_)));
    assert_eq!(items[0], items[2]);
    match &items[1] {
        EvidenceResolveManyItemV1::Refused(err) => {
            assert_eq!(err.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
            assert_eq!(err.field_path, "/evidenceRefs/1");
        }
        other => panic!("expected Refused at slot 1, got {other:?}"),
    }

    engine.close().unwrap();
}

#[test]
fn sdk_search_hints_and_open_report_booleans() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path().join("sdk_hints.sqlite"), OpenOptions::default()).unwrap();

    // OpenReport booleans
    let report = engine.open_report();
    assert_eq!(report.reranker_compiled, cfg!(feature = "default-reranker"));
    assert_eq!(report.cuda_compiled, cfg!(any(feature = "embed-cuda", feature = "rerank-cuda")));
    if !cfg!(feature = "default-reranker") {
        assert!(!report.rerank_available);
    }

    engine
        .write(&[canonical_doc(1, "needle delta one"), canonical_doc(2, "needle delta two")])
        .unwrap();

    // Default search -> hints None
    let off = engine.search("needle", SearchOptions::default()).unwrap();
    assert!(off.hints.is_none());

    // Opt-in search hints
    let mut opts = SearchOptions::default();
    opts.limit = 1;
    opts.hints = true;
    let on = engine.search("needle", opts).unwrap();
    let hints = on.hints.as_ref().expect("hints=true must populate Some");
    assert_eq!(hints.len(), 2);
    assert_eq!(hints[0].reason, HintReason::Degraded);
    assert_eq!(hints[0].arm, Some(HintArm::Vector));
    assert_eq!(hints[0].cause, Some(HintCause::EmbedderAbsent));
    assert_eq!(hints[1].reason, HintReason::MoreResults);
    assert_eq!(hints[1].action, HintAction::Search);
    assert_eq!(hints[1].limit, Some(2));

    // Frozen search hints
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let mut fopts = FrozenSearchOptions::default();
    fopts.hints = true;
    let frozen_empty = engine.search_frozen("notfoundxyz", &frozen, fopts).unwrap();
    let fhints = frozen_empty.hints.as_ref().expect("frozen hints=true must populate Some");
    assert!(fhints.iter().any(|h| h.reason == HintReason::NoResults));

    // Evidence search hints
    let ev = engine
        .search_with_evidence(
            &EvidenceSearchRequestV1::new("needle".into(), frozen).with_limit(1).with_hints(true),
        )
        .unwrap();
    let ehints = ev.search_result.hints.as_ref().expect("evidence hints=true must populate");
    assert!(ehints.iter().any(|h| h.reason == HintReason::MoreResults && h.limit == Some(2)));

    engine.close().unwrap();
}

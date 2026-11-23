//! Slice 50 (0.8.31) — opt-in search hints (`search`, `search_frozen`, `EvidenceSearchRequestV1`).

use fathomdb_engine::{
    ArtifactRevisionId, Engine, EvidenceSearchRequestV1, GraphArmOptions, HintAction, HintArm,
    HintCause, HintReason, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceId, SourceVersionId, WriteProvenanceV1,
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
fn hints_default_and_false_are_none_and_byte_identical() {
    let dir = TempDir::new().unwrap();
    let engine = Engine::open(dir.path().join("hints_off.sqlite")).unwrap().engine;
    engine
        .write(&[canonical_doc(1, "alpha beta river"), canonical_doc(2, "alpha gamma mountain")])
        .unwrap();

    let plain = engine.search("alpha").unwrap();
    assert!(plain.hints.is_none(), "default search must leave hints as None");

    let explicit_false = engine
        .search_reranked_view_with_hints(
            "alpha",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            None,
            &GraphArmOptions::default(),
            false,
        )
        .unwrap();
    assert!(explicit_false.hints.is_none());
    assert_eq!(plain, explicit_false);

    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let frozen_plain = engine.search_frozen("alpha", &frozen, 0, false, 0.3, 0, false, 10).unwrap();
    assert!(frozen_plain.hints.is_none());

    let frozen_false = engine
        .search_frozen_with_hints(
            "alpha",
            &frozen,
            0,
            false,
            0.3,
            0,
            false,
            10,
            None,
            &GraphArmOptions::default(),
            false,
        )
        .unwrap();
    assert!(frozen_false.hints.is_none());
    assert_eq!(frozen_plain, frozen_false);

    let ev_default = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("alpha".into(), frozen.clone()))
        .unwrap();
    assert!(ev_default.search_result.hints.is_none());

    let ev_false = engine
        .search_with_evidence(
            &EvidenceSearchRequestV1::new("alpha".into(), frozen).with_hints(false),
        )
        .unwrap();
    assert!(ev_false.search_result.hints.is_none());
    assert_eq!(ev_default.search_result, ev_false.search_result);

    engine.close().unwrap();
}

#[test]
fn hints_true_emits_degraded_no_results_and_more_results_across_all_three_paths() {
    let dir = TempDir::new().unwrap();
    // Opened without an embedder -> vector arm reports EmbedderAbsent degradation.
    let engine = Engine::open(dir.path().join("hints_on.sqlite")).unwrap().engine;
    engine
        .write(&[
            canonical_doc(1, "needle alpha one"),
            canonical_doc(2, "needle alpha two"),
            canonical_doc(3, "needle alpha three"),
        ])
        .unwrap();

    // 1. Full page (limit=2, returned=2) -> Degraded(Vector, EmbedderAbsent) + MoreResults(limit=4)
    let full_page = engine
        .search_reranked_view_with_hints(
            "needle",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            2,
            None,
            &GraphArmOptions::default(),
            true,
        )
        .unwrap();
    let full_hints = full_page.hints.as_ref().expect("hints=true must populate Some");
    assert_eq!(full_hints.len(), 2);
    assert_eq!(full_hints[0].reason, HintReason::Degraded);
    assert_eq!(full_hints[0].action, HintAction::None);
    assert_eq!(full_hints[0].arm, Some(HintArm::Vector));
    assert_eq!(full_hints[0].cause, Some(HintCause::EmbedderAbsent));
    assert_eq!(full_hints[1].reason, HintReason::MoreResults);
    assert_eq!(full_hints[1].action, HintAction::Search);
    assert_eq!(full_hints[1].limit, Some(4));

    // 2. Frozen search with zero hits -> Degraded + NoResults(mode=None)
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let frozen_empty = engine
        .search_frozen_with_hints(
            "nonexistentxyz",
            &frozen,
            0,
            false,
            0.3,
            0,
            false,
            5,
            None,
            &GraphArmOptions::default(),
            true,
        )
        .unwrap();
    let frozen_hints = frozen_empty.hints.as_ref().expect("frozen hints=true must populate Some");
    assert_eq!(frozen_hints.len(), 2);
    assert_eq!(frozen_hints[0].reason, HintReason::Degraded);
    assert_eq!(frozen_hints[1].reason, HintReason::NoResults);
    assert_eq!(frozen_hints[1].mode, None);

    // 3. EvidenceSearchRequestV1::with_hints(true) -> full page limit=2 emits Degraded + MoreResults(4)
    let ev_res = engine
        .search_with_evidence(
            &EvidenceSearchRequestV1::new("needle".into(), frozen).with_limit(2).with_hints(true),
        )
        .unwrap();
    let ev_hints = ev_res.search_result.hints.as_ref().expect("evidence hints=true must populate");
    assert_eq!(ev_hints.len(), 2);
    assert_eq!(ev_hints[0].reason, HintReason::Degraded);
    assert_eq!(ev_hints[1].reason, HintReason::MoreResults);
    assert_eq!(ev_hints[1].limit, Some(4));

    engine.close().unwrap();
}

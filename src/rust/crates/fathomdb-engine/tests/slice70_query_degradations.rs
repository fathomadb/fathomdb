//! Per-call missing-provider feedback through each real search route.
use fathomdb_engine::{
    ArmDegradation, ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine,
    EvidenceSearchRequestV1, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1,
    ReadView, SearchFilter, SearchHit, SoftFallbackBranch, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};

const QUERY: &str = "sliceseventyneedle";

fn seed(engine: &Engine) {
    let body = "original source bytes";
    let hash =
        Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>();
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "document".into(),
                body: body.into(),
                source_id: SourceId::new("s70-source").unwrap(),
                logical_id: Some("s70-source".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("s70-source-r1").unwrap(),
                    SourceVersionId::new("s70-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "claim".into(),
                body: format!("{QUERY} claim"),
                source_id: SourceId::new("s70-source").unwrap(),
                logical_id: Some("s70-claim".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("s70-claim-r1").unwrap(),
                    SourceVersionId::new("s70-v1").unwrap(),
                    SourceRevisionId::new("s70-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(hash).unwrap(),
                ),
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
}

fn absent_route(route: &str) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open_with_choice(dir.path().join("absence.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    seed(&engine);
    let text = engine.search_text_only(QUERY).unwrap();
    assert!(text.degradations.is_empty(), "explicit text-only requests no vector arm");
    assert_eq!(text.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["s70-claim"]);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let (hits, reasons): (Vec<SearchHit>, Vec<ArmDegradation>) = match route {
        "ordinary" => {
            let r = engine.search(QUERY).unwrap();
            (r.results, r.degradations)
        }
        "frozen" => {
            let r = engine.search_frozen(QUERY, &frozen, 0, false, 0.3, 0, false, 10).unwrap();
            (r.results, r.degradations)
        }
        "evidence" => {
            let r = engine
                .search_with_evidence(&EvidenceSearchRequestV1 {
                    schema_version: 1,
                    query: QUERY.into(),
                    context: frozen.clone(),
                    rerank_depth: 0,
                    use_graph_arm: false,
                    alpha: 0.3,
                    pool_n: 0,
                    include_explanation: false,
                    limit: 10,
                })
                .unwrap();
            assert_eq!(r.evidence.len(), 1);
            (r.search_result.results, r.search_result.degradations)
        }
        "expand" => {
            let r = engine.search_expand(QUERY, None, 1).unwrap();
            (r.search_hits, r.degradations)
        }
        "expand_frozen" => {
            let r = engine.search_expand_frozen(QUERY, &frozen, 1, 10).unwrap();
            (r.search_hits, r.degradations)
        }
        _ => unreachable!(),
    };
    engine.close().unwrap();
    assert_eq!(
        reasons.iter().map(|e| (e.arm, e.reason.as_str())).collect::<Vec<_>>(),
        [(SoftFallbackBranch::Vector, "embedder_absent")],
        "{route}: per-call absence must survive this route"
    );
    assert_eq!(hits.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["s70-claim"]);
    assert!(hits.iter().all(|h| h.branch == SoftFallbackBranch::Text));
    // One lexical rank contributes the accepted text weight 3 / (30 + 1).
    assert_eq!(hits[0].score, 3.0 / 31.0);
}

macro_rules! absence_case {
    ($name:ident,$route:literal) => {
        #[test]
        fn $name() {
            absent_route($route);
        }
    };
}
absence_case!(ordinary_reports_per_call_absence, "ordinary");
absence_case!(frozen_reports_per_call_absence, "frozen");
absence_case!(evidence_reports_per_call_absence, "evidence");
absence_case!(expand_reports_per_call_absence, "expand");
absence_case!(frozen_expand_reports_per_call_absence, "expand_frozen");

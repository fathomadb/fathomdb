use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine, InitialState, PreparedWrite,
    ProvenancedNodeV1, ReadView, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn plain() -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: "source evidence".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("plain".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn canonical() -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "source source-r1".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("source".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
        ),
    })
}

fn derived() -> PreparedWrite {
    let hash =
        Sha256::digest(b"source source-r1").iter().map(|b| format!("{b:02x}")).collect::<String>();
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: "derived source-r1".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("derived".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("derived-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(hash).unwrap(),
        ),
    })
}

fn dependencies(engine: &Engine) {
    engine.write(&[canonical(), derived()]).unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
}

fn check_dependency_span(with_dependency: bool) {
    let root = TempDir::new().unwrap();
    let engine = Engine::open_with_choice(root.path().join("store.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    if with_dependency {
        dependencies(&engine);
    } else {
        engine.write(&[plain()]).unwrap();
    }
    engine.slice30_observe_queries_for_test(true).unwrap();
    for expected_call in 1..=2 {
        let result = engine
            .search_text_only_filtered_with_snippets("source", None, &ReadView::default(), 10, None)
            .unwrap();
        let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        ids.sort();
        assert_eq!(
            ids,
            if with_dependency { vec!["l:derived", "l:source"] } else { vec!["l:plain"] }
        );
        let records = engine.take_slice30_measurements_for_test().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["call_id"], expected_call);
        assert!(records[0]["engine_id"].as_u64().unwrap() > 0);
        let observed = &records[0]["predicates"]["has_source_dependencies"];
        assert_eq!(observed["invoked"], true, "missing dependency-presence invocation span");
        assert_eq!(observed["value"], with_dependency);
        assert!(observed["elapsed_ns"].as_u64().unwrap() > 0);
    }
    engine.close().unwrap();
}

#[test]
fn dependency_presence_is_called_false_on_ordinary_real_store() {
    check_dependency_span(false);
}

#[test]
fn dependency_presence_is_called_true_on_governed_dependency_store() {
    check_dependency_span(true);
}

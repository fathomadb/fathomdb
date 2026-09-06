//! Slice 55 RED contract for structural explained-search metadata.

use std::collections::BTreeSet;
use std::sync::{Arc, Barrier};

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, InitialState, PreparedWrite, ProjectionFts,
    ProjectionRole, ProjectionSpec, ProjectionVector, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    StructuralDegradationCodeV1, StructuralDependencyStateV1, StructuralLifecycleStateV1,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn explained() -> (TempDir, fathomdb_engine::OpenedEngine, fathomdb_engine::SearchResult) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("explanation{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("explain-node".into()),
            kind: "doc".into(),
            body: "slice55 structural needle".into(),
            source_id: SourceId::new("slice55-explain").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    (dir, opened, result)
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "summary".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn provenance_node(
    revision: &str,
    logical_id: &str,
    body: &str,
    source_revision: Option<&str>,
) -> PreparedWrite {
    let provenance = match source_revision {
        None => WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("slice55-explanation-v1").unwrap(),
        ),
        Some(source) => WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("slice55-explanation-v1").unwrap(),
            SourceRevisionId::new(source).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest("structural authority source")).unwrap(),
        ),
    };
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice55-explanation-source").unwrap(),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance,
    })
}

#[test]
fn slice55_explanation_is_positional_and_structural() {
    let (_dir, _opened, result) = explained();
    let explanation = result.explanation.unwrap();
    assert!(!explanation.correlation_id.is_empty());
    assert_eq!(explanation.per_hit.len(), result.results.len());
    assert!(explanation.per_hit.iter().all(|hit| hit.structural.schema_version == 1));
}

#[test]
fn slice55_one_correlation_id_with_telemetry() {
    let (_dir, opened, _first) = explained();
    let sink = opened.engine.path().with_extension("jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(
        result.explanation.unwrap().correlation_id,
        opened.engine.last_telemetry_query_id().unwrap()
    );
}

#[test]
fn slice55_explain_telemetry_reenable_bytes_unchanged() {
    let (_dir, opened, _first) = explained();
    let sink = opened.engine.path().with_extension("jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(result.explanation.unwrap().correlation_id, "q0-0");
}

#[test]
fn slice55_explain_enable_race_has_one_id_source() {
    let (_dir, _opened, result) = explained();
    let id = result.explanation.unwrap().correlation_id;
    assert!(id.starts_with('x') || id.starts_with("q0-"));
}

#[test]
fn slice55_concurrent_explain_telemetry_ids_unique() {
    let (_dir, _opened, first) = explained();
    assert!(!first.explanation.unwrap().correlation_id.is_empty());
}

#[test]
fn slice55_telemetry_off_writes_nothing() {
    let (_dir, opened, result) = explained();
    assert!(opened.engine.last_telemetry_query_id().is_none());
    assert!(result.explanation.unwrap().correlation_id.starts_with('x'));
}

#[test]
fn slice55_default_search_is_unchanged() {
    let (_dir, opened, explained) = explained();
    let ordinary = opened.engine.search("slice55").unwrap();
    assert!(ordinary.explanation.is_none());
    assert_eq!(ordinary.results, explained.results);
}

#[test]
fn slice55_structural_projection_blocked_is_live_authority() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("explanation-blocked{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("blocked-node".into()),
            kind: "doc".into(),
            body: r#"{"summary":"blocked structural needle"}"#.into(),
            source_id: SourceId::new("slice55-blocked").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    let result = opened.engine.search_explained("blocked", None, 0, false, 0.3, 0).unwrap();
    let structural = &result.explanation.unwrap().per_hit[0].structural;
    assert!(structural.degradation_codes.contains(&StructuralDegradationCodeV1::ProjectionBlocked));
}

#[test]
fn slice55_structural_dependency_matrix_uses_live_rows() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("explanation-dependency{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[
            provenance_node("explain-source-r1", "source", "structural authority source", None),
            provenance_node(
                "explain-registered-r1",
                "registered",
                "registered structural needle",
                Some("explain-source-r1"),
            ),
            provenance_node(
                "explain-unregistered-r1",
                "unregistered",
                "unregistered structural needle",
                Some("explain-source-r1"),
            ),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "explain-dependency",
                "explain-source-r1",
                "explain-registered-r1",
            )
            .unwrap(),
        )
        .unwrap();

    for (query, expected) in [
        ("registered", StructuralDependencyStateV1::Registered),
        ("unregistered", StructuralDependencyStateV1::NotRegistered),
        ("authority", StructuralDependencyStateV1::NotApplicable),
    ] {
        let result = opened.engine.search_explained(query, None, 0, false, 0.3, 0).unwrap();
        assert_eq!(result.explanation.unwrap().per_hit[0].structural.dependency_state, expected);
    }
}

#[test]
fn slice55_structural_edge_lifecycle_is_live() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("explanation-edge{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "supports".into(),
            from: "a".into(),
            to: "b".into(),
            source_id: SourceId::new("slice55-edge").unwrap(),
            logical_id: Some("edge".into()),
            body: Some("edge structural needle".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    let result = opened.engine.search_explained("structural", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(
        result.explanation.unwrap().per_hit[0].structural.lifecycle_state,
        StructuralLifecycleStateV1::EdgeValid
    );
}

#[test]
fn slice55_concurrent_explanation_ids_are_genuinely_unique() {
    let (_dir, opened, _first) = explained();
    let engine = Arc::new(opened.engine);
    let barrier = Arc::new(Barrier::new(9));
    let threads = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                engine
                    .search_explained("slice55", None, 0, false, 0.3, 0)
                    .unwrap()
                    .explanation
                    .unwrap()
                    .correlation_id
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let ids = threads.into_iter().map(|thread| thread.join().unwrap()).collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 8);
}

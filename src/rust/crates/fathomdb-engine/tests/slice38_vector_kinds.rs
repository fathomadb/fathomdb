//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 3 RED/GREEN suite for explicit
//! opt-in custom `ProjectionVector.kinds` enrolment, lifecycle reconciliation,
//! rejection of invalid/`edge_fact` kinds, and preservation of TC-67 reporting.

use std::collections::BTreeSet;
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    DenseReadiness, Engine, EngineError, Filter, FilterTerm, IdSpace, InitialState, PreparedWrite,
    ProjectionFts, ProjectionRole, ProjectionSpec, ProjectionVector, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct CustomKindEmbedder;

impl Embedder for CustomKindEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice38-vk-embedder", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vec = vec![0.0_f32; 8];
        for (i, b) in text.bytes().enumerate() {
            vec[i % 8] += f32::from(b) / 255.0;
        }
        if vec.iter().all(|v| *v == 0.0) {
            vec[0] = 1.0;
        }
        Ok(vec)
    }
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn vector_spec(name: &str, kinds: Option<Vec<String>>) -> ProjectionSpec {
    ProjectionSpec {
        name: name.to_string(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds }),
        source: None,
        entity_class: fathomdb_engine::PropertyEntityClass::Node,
    }
}

#[test]
fn slice38_explicit_vector_kind_enrolment_indexes_searches_and_reopens() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "custom-vk");

    {
        let opened =
            Engine::open_with_embedder_for_test(&path, Arc::new(CustomKindEmbedder)).expect("open");
        let engine = opened.engine;

        let delta = engine
            .configure_projections(
                &[
                    vector_spec(
                        "summary",
                        Some(vec![
                            "custom:alpha".into(),
                            "custom:memory".into(),
                            "custom:memory".into(),
                        ]),
                    ),
                    vector_spec("notes", Some(vec!["custom:memory".into()])),
                ],
                &[],
            )
            .expect("configure custom vector kinds");
        assert!(delta.vector_unsupported_kinds.is_empty());

        engine
            .write(&[
                PreparedWrite::Node {
                    kind: "custom:memory".into(),
                    body: "quarterly roadmap memory milestone".into(),
                    source_id: SourceId::new("src:vk").unwrap(),
                    logical_id: Some("mem-1".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                },
                PreparedWrite::Node {
                    kind: "note".into(),
                    body: "regular note milestone".into(),
                    source_id: SourceId::new("src:vk").unwrap(),
                    logical_id: Some("note-1".into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                },
            ])
            .expect("write nodes");

        engine.drain(15_000).expect("drain");

        // _fathomdb_vector_kinds contains custom:memory & custom:alpha.
        let enrolled_kinds = engine
            .query_text_col_for_test("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind ASC")
            .unwrap();
        assert!(enrolled_kinds.contains(&"custom:memory".to_string()), "{enrolled_kinds:?}");
        assert!(enrolled_kinds.contains(&"custom:alpha".to_string()), "{enrolled_kinds:?}");

        // Hybrid & vector-stage-only search with SearchFilter.source_type = "custom:memory"
        let mut sf = SearchFilter::default();
        sf.source_type = Some("custom:memory".into());
        let hybrid = engine.search_filtered("milestone", Some(sf.clone())).expect("hybrid search");
        assert_eq!(hybrid.results.len(), 1);
        assert_eq!(hybrid.results[0].id, IdSpace::logical("mem-1"));

        engine.set_vector_stage_only_for_test(true);
        let vec_only = engine.search_filtered("milestone", Some(sf)).expect("vector search");
        assert_eq!(vec_only.results.len(), 1);
        assert_eq!(vec_only.results[0].id, IdSpace::logical("mem-1"));
        engine.set_vector_stage_only_for_test(false);

        // read_list_filter with FilterTerm::SourceType
        let listed = engine
            .read_list_filter(
                "custom:memory",
                &Filter { terms: vec![FilterTerm::SourceType("custom:memory".into())] },
                10,
                &fathomdb_engine::ReadView::default(),
            )
            .expect("read_list_filter custom source_type");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].logical_id, "mem-1");

        // read_projections round-trips canonically sorted deduplicated vector_kinds.
        let persisted = engine.read_projections().expect("read_projections");
        let summary = persisted.iter().find(|s| s.name == "summary").expect("summary spec");
        let vec_cfg = summary.vector.as_ref().expect("vector cfg");
        assert_eq!(
            vec_cfg.kinds.as_deref(),
            Some(&["custom:alpha".to_string(), "custom:memory".to_string()][..])
        );
        assert_eq!(vec_cfg.dense_readiness, Some(DenseReadiness::Ready));

        // Shrink `summary` kinds: custom:alpha dropped, custom:memory stays because both `summary` & `notes` still declare it.
        engine
            .configure_projections(
                &[
                    vector_spec("summary", Some(vec!["custom:memory".into()])),
                    vector_spec("notes", Some(vec!["custom:memory".into()])),
                ],
                &[],
            )
            .expect("shrink vector kinds");
        let after_shrink = engine
            .query_text_col_for_test("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind ASC")
            .unwrap();
        assert!(
            !after_shrink.contains(&"custom:alpha".to_string()),
            "custom:alpha should reconcile away"
        );
        assert!(
            after_shrink.contains(&"custom:memory".to_string()),
            "custom:memory must stay enrolled"
        );

        engine.close().expect("close");
    }

    // Reopen and verify persistence, read_projections round-trip, readiness, and searchability.
    let reopened =
        Engine::open_with_embedder_for_test(&path, Arc::new(CustomKindEmbedder)).expect("reopen");
    let specs = reopened.engine.read_projections().expect("read_projections after reopen");
    let summary = specs.iter().find(|s| s.name == "summary").expect("summary after reopen");
    assert_eq!(
        summary.vector.as_ref().and_then(|v| v.kinds.as_deref()),
        Some(&["custom:memory".to_string()][..])
    );
    assert_eq!(
        summary.vector.as_ref().and_then(|v| v.dense_readiness),
        Some(DenseReadiness::Ready)
    );
    reopened.engine.configure_projections(&specs, &[]).expect("re-apply round-trip after reopen");

    let mut sf = SearchFilter::default();
    sf.source_type = Some("custom:memory".into());
    let hits = reopened.engine.search_filtered("milestone", Some(sf)).expect("search after reopen");
    assert_eq!(hits.results.len(), 1);
    assert_eq!(hits.results[0].id, IdSpace::logical("mem-1"));

    // Now drop custom kinds (`kinds: None`) on both specs while keeping vector_declared = true.
    reopened
        .engine
        .configure_projections(&[vector_spec("summary", None), vector_spec("notes", None)], &[])
        .expect("drop custom vector kinds while keeping vector declared");
    let after_drop = reopened
        .engine
        .query_text_col_for_test("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind ASC")
        .unwrap();
    assert!(
        !after_drop.contains(&"custom:memory".to_string()),
        "custom:memory must be un-enrolled when all specs clear vector.kinds"
    );
}

#[test]
fn slice38_vector_kind_rejects_edge_fact_and_preserves_tc67() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "vk-reject");
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(CustomKindEmbedder)).expect("open");
    let engine = opened.engine;

    for bad_kind in ["edge_fact", "", "   ", "bad\0kind", "bad\u{1e}kind"] {
        let err = engine
            .configure_projections(&[vector_spec("body", Some(vec![bad_kind.into()]))], &[])
            .expect_err(&format!("invalid vector kind {bad_kind:?} must reject"));
        assert!(
            matches!(err, EngineError::WriteValidation),
            "expected WriteValidation for {bad_kind:?}, got {err:?}"
        );
    }

    // Spec without Searchable or without vector carrying vector_kinds must reject.
    let no_vector_err = engine
        .configure_projections(
            &[ProjectionSpec {
                name: "bad_no_vec".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: Some(ProjectionVector {
                    embedder: None,
                    dense_readiness: None,
                    kinds: Some(vec!["custom:x".into()]),
                }),
                source: None,
                entity_class: fathomdb_engine::PropertyEntityClass::Node,
            }],
            &[],
        )
        .expect("Searchable+vector validates fine; now check non-Searchable with vector_kinds");
    let _ = no_vector_err;

    let filter_only_with_kinds = engine
        .configure_projections(
            &[ProjectionSpec {
                name: "filter_only_kinds".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: Some(ProjectionVector {
                    embedder: None,
                    dense_readiness: None,
                    kinds: Some(vec!["custom:x".into()]),
                }),
                source: None,
                entity_class: fathomdb_engine::PropertyEntityClass::Node,
            }],
            &[],
        )
        .expect_err("vector_kinds without Searchable must reject");
    assert!(matches!(filter_only_with_kinds, EngineError::WriteValidation));

    // Preserve TC-67: unenrolled "entity" / "invoice" kinds remain excluded and reported in vector_unsupported_kinds.
    engine.configure_projections(&[vector_spec("notes", None)], &[]).expect("reset vector spec");
    engine
        .write(&[
            PreparedWrite::Node {
                kind: "entity".into(),
                body: "entity corpus row".into(),
                source_id: SourceId::new("src:tc67").unwrap(),
                logical_id: Some("ent-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "invoice".into(),
                body: "invoice corpus row".into(),
                source_id: SourceId::new("src:tc67").unwrap(),
                logical_id: Some("inv-1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
        ])
        .unwrap();

    let delta = engine.configure_projections(&[vector_spec("notes", None)], &[]).unwrap();
    assert_eq!(delta.vector_unsupported_kinds, vec!["entity".to_string(), "invoice".to_string()]);

    let enrolled = engine
        .query_text_col_for_test("SELECT kind FROM _fathomdb_vector_kinds ORDER BY kind ASC")
        .unwrap();
    assert!(!enrolled.contains(&"entity".to_string()));
    assert!(!enrolled.contains(&"invoice".to_string()));
}

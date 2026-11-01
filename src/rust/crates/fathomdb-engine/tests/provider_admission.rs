#![cfg(feature = "test-hooks")]

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine};
use std::sync::Arc;
use tempfile::TempDir;

struct HistoricProvider;
impl Embedder for HistoricProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("historic-provider", "historic-revision", 384)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 384])
    }
}

#[test]
fn none_admits_historic_profile_as_metadata_without_substitution() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("historic.fathomdb");
    Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider))
        .unwrap()
        .engine
        .close()
        .unwrap();
    let opened = Engine::open_with_choice(&path, EmbedderChoice::None)
        .expect("explicit None must validate historic metadata, not invent a BGE provider");
    assert_eq!(opened.report.default_embedder, HistoricProvider.identity());
    assert_eq!(
        opened.engine.default_embedder_profile_for_test().unwrap(),
        HistoricProvider.identity()
    );
    opened.engine.close().unwrap();
}

#[test]
fn string_projection_provider_selectors_are_rejected_before_mutation() {
    use fathomdb_engine::{EngineError, ProjectionRole, ProjectionSpec, ProjectionVector};
    for selector in ["", "default", "fathomdb-bge-small-en-v1.5", "another-model"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("selector.fathomdb");
        let opened = Engine::open(&path).unwrap();
        let spec = ProjectionSpec {
            name: "body".into(),
            roles: [ProjectionRole::Searchable].into(),
            fts: None,
            vector: Some(ProjectionVector {
                embedder: Some(selector.into()),
                dense_readiness: None,
            }),
            source: None,
        };
        assert!(
            matches!(
                opened.engine.configure_projections(&[spec], &[]),
                Err(EngineError::WriteValidation)
            ),
            "unsupported selector {selector:?} must be rejected"
        );
        assert!(opened.engine.read_projections().unwrap().is_empty());
        opened.engine.close().unwrap();
    }
}

fn verified_historic_store(path: &std::path::Path) {
    let opened = Engine::open_with_embedder_for_test(path, Arc::new(HistoricProvider)).unwrap();
    opened.engine.configure_vector_kind_for_test("note").unwrap();
    opened.engine.close().unwrap();
    Engine::open_with_embedder_for_test(path, Arc::new(HistoricProvider))
        .unwrap()
        .engine
        .close()
        .unwrap();
    rusqlite::Connection::open(path)
        .unwrap()
        .execute(
            "DELETE FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'",
            [],
        )
        .unwrap();
}

struct DivergentProvider;
impl Embedder for DivergentProvider {
    fn identity(&self) -> EmbedderIdentity {
        HistoricProvider.identity()
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![-1.0; 384])
    }
}

#[test]
fn numeric_refusal_returns_no_engine_and_releases_admission() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("refused.fathomdb");
    verified_historic_store(&path);
    let attempt = Engine::open_with_embedder_for_test(&path, Arc::new(DivergentProvider));
    assert!(attempt.is_err(), "actual numeric refusal must return no Engine");
    let recovered = Engine::open_with_choice(&path, EmbedderChoice::None)
        .expect("failed open must release database ownership for deliberate None access");
    recovered.engine.close().unwrap();
}

struct FaultProvider(u8);
impl Embedder for FaultProvider {
    fn identity(&self) -> EmbedderIdentity {
        HistoricProvider.identity()
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        match self.0 {
            0 => Err(EmbedderError::Failed { message: "intentional provider failure".into() }),
            1 => panic!("intentional verification panic"),
            2 => {
                std::thread::sleep(std::time::Duration::from_millis(250));
                Ok(vec![1.0; 384])
            }
            _ => Ok(vec![1.0; 3]),
        }
    }
}

fn verification_failure(fault: u8) -> fathomdb_engine::VectorVerificationFailure {
    use fathomdb_engine::{EngineConfig, EngineOpenError};
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("failure.fathomdb");
    verified_historic_store(&path);
    let error = Engine::open_with_test_choice_and_config_for_test(
        &path,
        fathomdb_engine::TestEmbedderChoice::Caller(Arc::new(FaultProvider(fault))),
        EngineConfig { embedder_call_timeout_ms: Some(100), ..Default::default() },
    )
    .err()
    .expect("verification failure must return no Engine");
    let recovered = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    recovered.engine.close().unwrap();
    match error {
        EngineOpenError::VectorVerification { probe_ordinal: 0, cause } => cause,
        other => panic!("expected typed cannot-verify cause at probe zero, got {other:?}"),
    }
}

#[test]
fn provider_failure_preserves_typed_cause() {
    assert_eq!(
        verification_failure(0),
        fathomdb_engine::VectorVerificationFailure::Provider(EmbedderError::Failed {
            message: "intentional provider failure".into()
        })
    );
}
#[test]
fn panic_failure_preserves_typed_cause() {
    assert_eq!(
        verification_failure(1),
        fathomdb_engine::VectorVerificationFailure::Panic {
            message: "intentional verification panic".into()
        }
    );
}
#[test]
fn timeout_failure_preserves_typed_cause() {
    assert_eq!(verification_failure(2), fathomdb_engine::VectorVerificationFailure::StartedTimeout);
}
#[test]
fn malformed_output_preserves_typed_cause() {
    assert_eq!(verification_failure(3), fathomdb_engine::VectorVerificationFailure::InvalidOutput);
}

#[test]
fn invalid_baseline_is_distinct_from_numeric_mismatch() {
    use fathomdb_engine::EngineOpenError;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("invalid.fathomdb");
    verified_historic_store(&path);
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM _fathomdb_embed_probe WHERE probe_ordinal=7", [])
        .unwrap();
    let error =
        Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider)).err().unwrap();
    match error {
        EngineOpenError::VectorVerification { cause, .. } => {
            assert_eq!(cause.code(), "invalid_state")
        }
        other => panic!("invalid references must not be reported as numeric mismatch: {other:?}"),
    }
}

struct ChangesDuringPopulation(std::sync::atomic::AtomicUsize);
impl Embedder for ChangesDuringPopulation {
    fn identity(&self) -> EmbedderIdentity {
        HistoricProvider.identity()
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        let call = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(vec![if call < 45 { 1.0 } else { -1.0 }; 384])
    }
}

#[test]
fn rejected_registered_population_publishes_no_reference_authority() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("population.fathomdb");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider)).unwrap();
    opened.engine.configure_vector_kind_for_test("note").unwrap();
    opened.engine.close().unwrap();
    let error = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(ChangesDuringPopulation(std::sync::atomic::AtomicUsize::new(0))),
    )
    .err()
    .expect("population confirmation must reject changing output");
    assert!(matches!(error, fathomdb_engine::EngineOpenError::VectorEquivalenceMismatch { .. }));
    let db = rusqlite::Connection::open(&path).unwrap();
    let references: i64 =
        db.query_row("SELECT count(*) FROM _fathomdb_embed_probe", [], |row| row.get(0)).unwrap();
    assert_eq!(references, 0, "rejected registered references must never seed a later open");
    let markers: i64 = db.query_row("SELECT count(*) FROM _fathomdb_open_state WHERE key='vector_equivalence_verified_fingerprint'", [], |row| row.get(0)).unwrap();
    assert_eq!(markers, 0);
    drop(db);
    Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider))
        .expect("compatible retry must establish a clean baseline")
        .engine
        .close()
        .unwrap();
}

#[test]
fn historical_selector_remains_inspectable_but_cannot_activate_a_provider() {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec, ProjectionVector};
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("historic-selector.fathomdb");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider)).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "body".into(),
                roles: [ProjectionRole::Searchable].into(),
                fts: None,
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
    rusqlite::Connection::open(&path).unwrap().execute(
        "UPDATE _fathomdb_projection_registry SET vector_embedder='historical-alternative' WHERE name='body'", []
    ).unwrap();
    // Keep the persisted historical declaration and serving generation consistent.
    opened.engine.transition_projection_generation_for_test().unwrap();
    opened.engine.close().unwrap();
    let inspected = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap();
    assert_eq!(
        inspected.engine.read_projections().unwrap()[0]
            .vector
            .as_ref()
            .unwrap()
            .embedder
            .as_deref(),
        Some("historical-alternative")
    );
    inspected.engine.close().unwrap();
    assert!(
        Engine::open_with_embedder_for_test(&path, Arc::new(HistoricProvider)).is_err(),
        "historical selector cannot silently activate the attached default pipeline"
    );
}

#[test]
fn production_choices_are_exhaustively_default_or_none() {
    fn closed(choice: fathomdb_engine::EmbedderChoice) {
        match choice {
            fathomdb_engine::EmbedderChoice::Default => (),
            fathomdb_engine::EmbedderChoice::None => (),
        }
    }
    closed(fathomdb_engine::EmbedderChoice::None);
}

#[test]
fn returned_engine_report_and_runtime_have_no_refused_mode() {
    use fathomdb_engine::{
        OpenReport, ProjectionRuntimeStateV1, ProjectionRuntimeUnavailabilityReason,
    };
    fn runtime(state: ProjectionRuntimeStateV1) {
        match state {
            ProjectionRuntimeStateV1::Absent => (),
            ProjectionRuntimeStateV1::Usable => (),
        }
    }
    fn unavailable(reason: ProjectionRuntimeUnavailabilityReason) {
        match reason {
            ProjectionRuntimeUnavailabilityReason::None => (),
            ProjectionRuntimeUnavailabilityReason::NoRuntime => (),
        }
    }
    runtime(ProjectionRuntimeStateV1::Absent);
    unavailable(ProjectionRuntimeUnavailabilityReason::NoRuntime);
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join("normal.fathomdb")).unwrap();
    let OpenReport {
        schema_version_before: _,
        schema_version_after: _,
        migration_steps: _,
        embedder_warmup_ms: _,
        query_backend: _,
        default_embedder: _,
        embedder_download_ms: _,
        embedder_events: _,
        embedder_mean_centering_required: _,
        embedder_mean_vec_pinned: _,
        embedder_device_resolution: _,
        reranker_device_resolution: _,
        embedder_gpu_allocation_witness: _,
        stop_words_sha256: _,
    } = &opened.report;
    opened.engine.close().unwrap();
}

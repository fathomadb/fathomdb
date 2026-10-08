//! 0.8.28 Slice 30 (R30-04a) — the three CUDA pool kinds reach engine callers
//! as typed `EngineError` / `EngineOpenError` variants with their payloads,
//! never as the payload-free `EngineError::Embedder`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use fathomdb_embedder::{
    RerankerDevicePolicyError, RerankerDeviceResolutionError, RerankerDeviceResolutionReason,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::RerankPassagesError;
use fathomdb_engine::{rerank_passages, EmbedderChoice, Engine, EngineError, EngineOpenError};
use tempfile::TempDir;

const DIM: u32 = 8;

fn exhausted() -> EmbedderError {
    EmbedderError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 3 << 30,
        message: "embed forward: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
    }
}

fn context_lost() -> EmbedderError {
    EmbedderError::CudaContextLost {
        recorded_context_id: u64::MAX - 1,
        current_context_id: None,
        driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
        operation: "embed forward".to_owned(),
    }
}

fn refused() -> EmbedderError {
    EmbedderError::CudaPrivateBuildRefused { ordinal: 1, message: "build failed".to_owned() }
}

/// Opens cleanly, then fails every embed with `failure` once armed.
struct FailingEmbedder {
    armed: AtomicBool,
    failure: EmbedderError,
}

impl Embedder for FailingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-failing", "rev-a", DIM)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        if self.armed.load(Ordering::SeqCst) {
            Err(self.failure.clone())
        } else {
            Ok(vec![1.0; DIM as usize])
        }
    }
}

fn embed_text_error(failure: EmbedderError) -> EngineError {
    let dir = TempDir::new().expect("tempdir");
    let embedder = Arc::new(FailingEmbedder { armed: AtomicBool::new(false), failure });
    let opened = Engine::open_with_choice(
        dir.path().join("slice30.sqlite"),
        EmbedderChoice::Caller(embedder.clone()),
    )
    .expect("open");
    embedder.armed.store(true, Ordering::SeqCst);
    let error = opened.engine.embed_text("query").expect_err("armed embedder must fail");
    opened.engine.close().expect("close");
    error
}

#[test]
fn embed_text_surfaces_pool_exhaustion_with_its_payload() {
    assert_eq!(
        embed_text_error(exhausted()),
        EngineError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "embed forward: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
        }
    );
}

#[test]
fn embed_text_surfaces_context_loss_with_its_payload() {
    assert_eq!(
        embed_text_error(context_lost()),
        EngineError::CudaContextLost {
            recorded_context_id: u64::MAX - 1,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "embed forward".to_owned(),
        }
    );
}

#[test]
fn embed_text_surfaces_private_build_refusal_with_its_payload() {
    assert_eq!(
        embed_text_error(refused()),
        EngineError::CudaPrivateBuildRefused { ordinal: 1, message: "build failed".to_owned() }
    );
}

#[test]
fn an_ordinary_provider_failure_stays_the_embedder_class() {
    assert_eq!(
        embed_text_error(EmbedderError::Failed { message: "boom".to_owned() }),
        EngineError::Embedder
    );
}

#[test]
fn reranker_pool_kinds_convert_to_the_typed_engine_variants() {
    assert_eq!(
        EngineError::from(RerankerDevicePolicyError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 7,
            message: "m".to_owned(),
        }),
        EngineError::CudaPoolExhausted { ordinal: 0, max_size_bytes: 7, message: "m".to_owned() }
    );
    assert_eq!(
        EngineError::from(RerankerDevicePolicyError::CudaContextLost {
            recorded_context_id: 9,
            current_context_id: Some(10),
            driver_error: "d".to_owned(),
            operation: "rerank forward".to_owned(),
        }),
        EngineError::CudaContextLost {
            recorded_context_id: 9,
            current_context_id: Some(10),
            driver_error: "d".to_owned(),
            operation: "rerank forward".to_owned(),
        }
    );
    assert_eq!(
        EngineError::from(RerankerDevicePolicyError::CudaPrivateBuildRefused {
            ordinal: 2,
            message: "m".to_owned(),
        }),
        EngineError::CudaPrivateBuildRefused { ordinal: 2, message: "m".to_owned() }
    );
    let policy = RerankerDevicePolicyError::Resolution(
        RerankerDeviceResolutionError::ForcedCudaUnavailable {
            ordinal: 1,
            reason: RerankerDeviceResolutionReason::CudaProbeFailed,
        },
    );
    assert_eq!(EngineError::from(policy.clone()), EngineError::RerankerDevicePolicy(policy));
}

#[test]
fn the_engine_variants_display_their_payloads() {
    let rendered = EngineError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 3 << 30,
        message: "oom".to_owned(),
    }
    .to_string();
    assert!(rendered.contains(&(3_u64 << 30).to_string()), "{rendered}");
    assert!(rendered.contains("oom"), "{rendered}");
    let rendered = EngineError::CudaContextLost {
        recorded_context_id: 5,
        current_context_id: Some(6),
        driver_error: "destroyed".to_owned(),
        operation: "embed forward".to_owned(),
    }
    .to_string();
    assert!(rendered.contains("destroyed") && rendered.contains("embed forward"), "{rendered}");
    let rendered =
        EngineError::CudaPrivateBuildRefused { ordinal: 3, message: "nope".to_owned() }.to_string();
    assert!(rendered.contains("nope") && rendered.contains('3'), "{rendered}");
}

#[test]
fn the_open_variants_carry_and_display_their_payloads() {
    let error =
        EngineOpenError::CudaPoolExhausted { ordinal: 0, max_size_bytes: 1, message: "oom".into() };
    assert!(error.to_string().contains("oom"));
    let error = EngineOpenError::CudaContextLost {
        recorded_context_id: 1,
        current_context_id: None,
        driver_error: "destroyed".into(),
        operation: "embedder load".into(),
    };
    assert!(error.to_string().contains("destroyed"));
    let error = EngineOpenError::CudaPrivateBuildRefused { ordinal: 0, message: "nope".into() };
    assert!(error.to_string().contains("nope"));
}

#[test]
fn an_open_embedder_failure_of_a_pool_kind_converts_to_the_typed_open_variant() {
    assert_eq!(
        EngineOpenError::from(exhausted()),
        EngineOpenError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "embed forward: CUDA_ERROR_OUT_OF_MEMORY".to_owned(),
        }
    );
    assert!(matches!(
        EngineOpenError::from(context_lost()),
        EngineOpenError::CudaContextLost { recorded_context_id, .. } if recorded_context_id == u64::MAX - 1
    ));
    assert!(matches!(
        EngineOpenError::from(refused()),
        EngineOpenError::CudaPrivateBuildRefused { ordinal: 1, .. }
    ));
    let failed = EmbedderError::Failed { message: "x".to_owned() };
    assert_eq!(EngineOpenError::from(failed.clone()), EngineOpenError::Embedder(failed));
}

#[test]
fn rerank_passages_reports_a_non_finite_score_as_write_validation() {
    let error = rerank_passages("q", vec![(1, "b".to_owned(), f64::NAN)], 1, 0.3, 1)
        .expect_err("NaN must be refused");
    match error {
        RerankPassagesError::WriteValidation { message } => {
            assert!(message.contains("non-finite"), "{message}");
        }
        other => panic!("expected WriteValidation, got {other:?}"),
    }
}

#[test]
fn rerank_passages_error_converts_a_reranker_error_to_the_typed_engine_variant() {
    let error = RerankPassagesError::Reranker(RerankerDevicePolicyError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 1,
        message: "m".to_owned(),
    });
    assert!(error.to_string().contains("m"));
    assert_eq!(
        EngineError::from(error),
        EngineError::CudaPoolExhausted { ordinal: 0, max_size_bytes: 1, message: "m".to_owned() }
    );
    assert_eq!(
        EngineError::from(RerankPassagesError::WriteValidation { message: "w".to_owned() }),
        EngineError::WriteValidation
    );
}

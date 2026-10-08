//! 0.8.28 Slice 30 (R30-04c) — the three CUDA pool kinds in `fathomdb-sdk`:
//! their `ErrorKind`s, their parent, `Error::cuda_details`, and that no
//! catch-all arm swallows them.

use fathomdb_sdk::{
    CudaErrorDetails, EngineError, EngineOpenError, Error, ErrorKind, RerankerDevicePolicyError,
    RuntimeEmbedderError as EmbedderError,
};

fn pool_exhausted() -> CudaErrorDetails {
    CudaErrorDetails::PoolExhausted { ordinal: 0, max_size_bytes: 3 << 30, message: "oom".into() }
}

fn context_lost() -> CudaErrorDetails {
    CudaErrorDetails::ContextLost {
        recorded_context_id: u64::MAX,
        current_context_id: None,
        driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".into(),
        operation: "forward".into(),
    }
}

fn build_refused() -> CudaErrorDetails {
    CudaErrorDetails::PrivateBuildRefused { ordinal: 1, message: "refused".into() }
}

fn assert_kind(error: &Error, kind: ErrorKind, details: &CudaErrorDetails) {
    assert_eq!(error.kind(), kind, "{error}");
    assert_eq!(error.cuda_details(), Some(details), "{error}");
    assert!(error.to_string().starts_with(kind.name()), "{error}");
}

#[test]
fn the_kinds_are_named_and_derive_from_embedder() {
    for (kind, name) in [
        (ErrorKind::CudaPoolExhausted, "CudaPoolExhaustedError"),
        (ErrorKind::CudaContextLost, "CudaContextLostError"),
        (ErrorKind::CudaPrivateBuildRefused, "CudaPrivateBuildRefusedError"),
    ] {
        assert_eq!(kind.name(), name);
        assert_eq!(kind.parent(), Some(ErrorKind::Embedder));
        assert!(ErrorKind::ALL.contains(&kind));
    }
}

#[test]
fn engine_errors_of_each_kind_carry_their_details() {
    assert_kind(
        &Error::from(EngineError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "oom".into(),
        }),
        ErrorKind::CudaPoolExhausted,
        &pool_exhausted(),
    );
    assert_kind(
        &Error::from(EngineError::CudaContextLost {
            recorded_context_id: u64::MAX,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".into(),
            operation: "forward".into(),
        }),
        ErrorKind::CudaContextLost,
        &context_lost(),
    );
    assert_kind(
        &Error::from(EngineError::CudaPrivateBuildRefused {
            ordinal: 1,
            message: "refused".into(),
        }),
        ErrorKind::CudaPrivateBuildRefused,
        &build_refused(),
    );
}

#[test]
fn reranker_policy_errors_of_each_kind_are_not_swallowed_by_the_policy_class() {
    assert_kind(
        &Error::from(EngineError::RerankerDevicePolicy(
            RerankerDevicePolicyError::CudaPoolExhausted {
                ordinal: 0,
                max_size_bytes: 3 << 30,
                message: "oom".into(),
            },
        )),
        ErrorKind::CudaPoolExhausted,
        &pool_exhausted(),
    );
    assert_kind(
        &Error::from(EngineError::RerankerDevicePolicy(
            RerankerDevicePolicyError::CudaContextLost {
                recorded_context_id: u64::MAX,
                current_context_id: None,
                driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".into(),
                operation: "forward".into(),
            },
        )),
        ErrorKind::CudaContextLost,
        &context_lost(),
    );
    assert_kind(
        &Error::from(EngineError::RerankerDevicePolicy(
            RerankerDevicePolicyError::CudaPrivateBuildRefused {
                ordinal: 1,
                message: "refused".into(),
            },
        )),
        ErrorKind::CudaPrivateBuildRefused,
        &build_refused(),
    );
}

#[test]
fn open_errors_of_each_kind_carry_their_details() {
    assert_kind(
        &Error::from(EngineOpenError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "oom".into(),
        }),
        ErrorKind::CudaPoolExhausted,
        &pool_exhausted(),
    );
    assert_kind(
        &Error::from(EngineOpenError::CudaContextLost {
            recorded_context_id: u64::MAX,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".into(),
            operation: "forward".into(),
        }),
        ErrorKind::CudaContextLost,
        &context_lost(),
    );
    assert_kind(
        &Error::from(EngineOpenError::CudaPrivateBuildRefused {
            ordinal: 1,
            message: "refused".into(),
        }),
        ErrorKind::CudaPrivateBuildRefused,
        &build_refused(),
    );
    assert_kind(
        &Error::from(EngineOpenError::Embedder(EmbedderError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "oom".into(),
        })),
        ErrorKind::CudaPoolExhausted,
        &pool_exhausted(),
    );
}

/// An `Error::Engine` built directly (not through `From`) still reports its
/// CUDA kind: `engine_kind` has explicit arms ahead of its catch-all.
#[test]
fn a_directly_wrapped_engine_variant_still_reports_its_kind() {
    let error = Error::Engine(EngineError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 1,
        message: "m".into(),
    });
    assert_eq!(error.kind(), ErrorKind::CudaPoolExhausted);
    let error = Error::Engine(EngineError::CudaContextLost {
        recorded_context_id: 1,
        current_context_id: Some(2),
        driver_error: "d".into(),
        operation: "o".into(),
    });
    assert_eq!(error.kind(), ErrorKind::CudaContextLost);
    let error =
        Error::Engine(EngineError::CudaPrivateBuildRefused { ordinal: 0, message: "m".into() });
    assert_eq!(error.kind(), ErrorKind::CudaPrivateBuildRefused);
    let error = Error::Open(EngineOpenError::CudaPoolExhausted {
        ordinal: 0,
        max_size_bytes: 1,
        message: "m".into(),
    });
    assert_eq!(error.kind(), ErrorKind::CudaPoolExhausted);
}

#[test]
fn other_errors_have_no_cuda_details() {
    assert_eq!(Error::from(EngineError::Closing).cuda_details(), None);
    assert_eq!(Error::from(EngineError::Embedder).kind(), ErrorKind::Embedder);
    assert_eq!(
        Error::from(EngineOpenError::Embedder(EmbedderError::Timeout)).kind(),
        ErrorKind::Embedder
    );
}

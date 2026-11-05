use std::error::Error as _;

use fathomdb_sdk::{
    EngineOpenError, Error, ErrorKind, RuntimeEmbedderError, VectorVerificationFailure,
};

#[test]
fn numeric_refusal_retains_existing_class_and_reason() {
    let error =
        Error::from(EngineOpenError::VectorEquivalenceMismatch { reason: "P1 flips=3".into() });
    assert_eq!(error.kind(), ErrorKind::VectorEquivalenceMismatch);
    assert!(
        matches!(error, Error::Open(EngineOpenError::VectorEquivalenceMismatch { ref reason }) if reason == "P1 flips=3")
    );
    assert!(error.source().is_some());
}

#[test]
fn cannot_verify_preserves_original_provider_payload_and_class() {
    let provider = RuntimeEmbedderError::CudaPoolExhausted {
        ordinal: 2,
        max_size_bytes: 12345,
        message: "original allocator detail".into(),
    };
    let error = Error::from(EngineOpenError::VectorVerification {
        probe_ordinal: 7,
        cause: VectorVerificationFailure::Provider(provider.clone()),
    });
    assert_eq!(error.kind(), ErrorKind::VectorVerification);
    assert_eq!(error.kind().name(), "VectorVerificationError");
    assert!(error.cuda_details().is_none(), "verification context must not be flattened to CUDA");
    assert!(
        matches!(error, Error::Open(EngineOpenError::VectorVerification { probe_ordinal: 7, cause: VectorVerificationFailure::Provider(ref actual) }) if actual == &provider)
    );
    assert!(error.source().is_some());
}

#[test]
fn each_dispatch_refusal_retains_its_typed_cause() {
    for cause in [
        VectorVerificationFailure::NotConfigured,
        VectorVerificationFailure::Saturated,
        VectorVerificationFailure::QueuedExpired,
        VectorVerificationFailure::StartedTimeout,
        VectorVerificationFailure::InvalidOutput,
        VectorVerificationFailure::Panic { message: "original panic".into() },
        VectorVerificationFailure::Closing,
        VectorVerificationFailure::Cancelled,
    ] {
        let original = cause.clone();
        let error = Error::from(EngineOpenError::VectorVerification { probe_ordinal: 12, cause });
        assert_eq!(error.kind(), ErrorKind::VectorVerification);
        assert!(
            matches!(error, Error::Open(EngineOpenError::VectorVerification { probe_ordinal: 12, ref cause }) if cause == &original)
        );
    }
}

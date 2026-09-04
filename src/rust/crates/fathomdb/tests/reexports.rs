//! Asserts the `fathomdb` facade re-exports the **governed** typed surface owned
//! by `dev/interfaces/rust.md`. The 20
//! operator-seam re-exports are gated behind the `operator` feature and are
//! resolved by `governed_surface.rs::t_074_operator_seam_resolves_with_feature`
//! (Slice 27 fix-1).

#[test]
fn re_exports_compile() {
    let _ = std::any::type_name::<fathomdb::Engine>();
    let _ = std::any::type_name::<fathomdb::OpenedEngine>();
    let _ = std::any::type_name::<fathomdb::OpenReport>();
    let _ = std::any::type_name::<fathomdb::WriteReceipt>();
    let _ = std::any::type_name::<fathomdb::SearchResult>();
    let _ = std::any::type_name::<fathomdb::PreparedWrite>();
    let _ = std::any::type_name::<fathomdb::EngineError>();
    let _ = std::any::type_name::<fathomdb::EngineOpenError>();
    let _ = std::any::type_name::<fathomdb::ArtifactRevisionId>();
    let _ = std::any::type_name::<fathomdb::CanonicalHash>();
    let _ = std::any::type_name::<fathomdb::ProvenanceCompleteness>();
    let _ = std::any::type_name::<fathomdb::ProvenanceError>();
    let _ = std::any::type_name::<fathomdb::ProvenanceErrorReason>();
    let _ = std::any::type_name::<fathomdb::ProvenancedEdgeV1>();
    let _ = std::any::type_name::<fathomdb::ProvenancedNodeV1>();
    let _ = std::any::type_name::<fathomdb::SourceLocator>();
    let _ = std::any::type_name::<fathomdb::SourceRevisionId>();
    let _ = std::any::type_name::<fathomdb::SourceVersionId>();
    let _ = std::any::type_name::<fathomdb::WriteProvenanceV1>();
    let _ = std::any::type_name::<fathomdb::DependencyId>();
    let _ = std::any::type_name::<fathomdb::SourceDependencyRegistrationV1>();
    let _ = std::any::type_name::<fathomdb::DependencySourceLookupV1>();
    let _ = std::any::type_name::<fathomdb::DependencyDerivedLookupV1>();
    let _ = std::any::type_name::<fathomdb::SourceDependencyV1>();
    let _ = std::any::type_name::<fathomdb::DependencyListV1>();
    let _ = std::any::type_name::<fathomdb::DependencyError>();
    let _ = std::any::type_name::<fathomdb::DependencyErrorReason>();

    let _ = std::any::type_name::<fathomdb::CorruptionDetail>();
    let _ = std::any::type_name::<fathomdb::CorruptionKind>();
    let _ = std::any::type_name::<fathomdb::CorruptionLocator>();
    let _ = std::any::type_name::<fathomdb::OpenStage>();
    let _ = std::any::type_name::<fathomdb::RecoveryHint>();

    let _ = std::any::type_name::<fathomdb::SoftFallback>();
    let _ = std::any::type_name::<fathomdb::SoftFallbackBranch>();
    let _ = std::any::type_name::<fathomdb::CounterSnapshot>();
    let _ = std::any::type_name::<fathomdb::Subscription>();
    let _ = std::any::type_name::<fathomdb::ProjectionRuntimeStatus>();
    let _ = std::any::type_name::<fathomdb::ProjectionRuntimeStatusEntry>();
    let _ = std::any::type_name::<fathomdb::ProjectionRuntimeUnavailabilityReason>();
    let _ = std::any::type_name::<fathomdb::ProjectionStatusDenseReadiness>();
}

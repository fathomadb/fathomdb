//! The SDK-only consumer can read the closed exact reporting carrier.
use fathomdb_sdk::{
    read, Engine, OpenOptions, StoredHeadCoverageCounts, StoredHeadCoverageFraction,
    StoredHeadCoverageProfile, StoredHeadCoverageUnavailableReason, StoredHeadCoverageV1,
};

fn check_profile(profile: StoredHeadCoverageProfile) {
    assert_eq!(profile.key, "default");
    assert!(!profile.name.is_empty());
    assert!(!profile.revision.is_empty());
    assert!(profile.dimension > 0);
}

fn check_empty_share(share: Option<StoredHeadCoverageFraction>) {
    assert_eq!(share, None);
}

fn check_empty_counts(counts: StoredHeadCoverageCounts) {
    assert_eq!(counts.records, 0);
    assert_eq!(counts.known_records, 0);
    assert_eq!(counts.unknown_records, 0);
    assert_eq!(counts.truncated_known_records, 0);
    assert_eq!(counts.known_body_bytes, 0);
    assert_eq!(counts.unrepresented_known_bytes, 0);
    assert_eq!(counts.unattributed_heads, 0);
    check_empty_share(counts.truncated_known_share);
    check_empty_share(counts.unrepresented_known_byte_share);
}

#[test]
fn provider_free_sdk_consumer_reads_exact_stored_head_coverage() {
    let directory = tempfile::tempdir().unwrap();
    let engine =
        Engine::open(directory.path().join("consumer.sqlite"), OpenOptions::default()).unwrap();
    let status = read::projection_status(&engine).unwrap();
    assert!(!status.runtime_embedder_available);
    match status.vector_coverage {
        StoredHeadCoverageV1::Available { population, profile, counts } => {
            assert_eq!(population, "stored_mapped_spans");
            check_profile(profile);
            check_empty_counts(counts);
        }
        StoredHeadCoverageV1::Unavailable { population, profile, reason } => {
            assert_eq!(population, "stored_mapped_spans");
            check_profile(profile);
            match reason {
                StoredHeadCoverageUnavailableReason::CacheCold
                | StoredHeadCoverageUnavailableReason::StateChanged => {}
            }
        }
    }
    engine.close().unwrap();
}

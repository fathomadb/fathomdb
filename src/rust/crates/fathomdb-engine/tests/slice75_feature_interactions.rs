#![cfg(feature = "test-hooks")]

use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use std::thread;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, FrozenReadErrorReason, InitialState,
    LifecycleState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice75-fixed", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0_f32; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        if vector.iter().all(|value| *value == 0.0) {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn open(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    let opened = Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("configure vector kind");
    opened
}

fn node(logical_id: &str, body: String, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body,
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn visibility_generation(engine: &Engine) -> i64 {
    engine
        .query_i64_col_for_test(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
        )
        .expect("read visibility generation")[0]
}

fn canonical_hash(body: &str) -> CanonicalHash {
    let digest: String =
        Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
    CanonicalHash::sha256(digest).expect("canonical hash")
}

fn canonical(revision: &str, logical: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).expect("revision"),
            SourceVersionId::new("slice75-v1").expect("source version"),
        ),
    })
}

fn derived(
    revision: &str,
    logical: &str,
    source: &str,
    source_revision: &str,
    source_body: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).expect("revision"),
            SourceVersionId::new("slice75-v1").expect("source version"),
            SourceRevisionId::new(source_revision).expect("source revision"),
            SourceLocator::whole_body(),
            canonical_hash(source_body),
        ),
    })
}

#[test]
fn batch_write_coalesces_visibility_while_custom_triggers_and_projection_rows_remain_per_row() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("batch{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let before = visibility_generation(&opened.engine);
    let writes = (0..8)
        .map(|index| {
            node(
                &format!("batch-{index}"),
                format!("batchneedle document {index}"),
                "slice75-batch",
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("batch write");
    assert_eq!(visibility_generation(&opened.engine), before + 1);
    opened.engine.set_projection_scheduler_frozen_for_test(false);
    opened.engine.drain(10_000).expect("projection drain");
    assert_eq!(opened.engine.vector_row_count_for_test().expect("vector rows"), 8);

    opened
        .engine
        .execute_for_test(
            "CREATE TABLE slice75_trigger_audit(id INTEGER PRIMARY KEY); \
             CREATE TRIGGER slice75_custom_ai AFTER INSERT ON _fathomdb_vector_rows BEGIN \
               INSERT INTO slice75_trigger_audit VALUES(NULL); \
             END;",
        )
        .expect("install custom projection trigger");
    let fallback_writes = (8..12)
        .map(|index| {
            node(
                &format!("batch-{index}"),
                format!("batchneedle document {index}"),
                "slice75-batch",
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&fallback_writes).expect("fallback batch write");
    opened.engine.drain(10_000).expect("fallback projection drain");
    assert_eq!(
        opened
            .engine
            .query_i64_col_for_test("SELECT COUNT(*) FROM slice75_trigger_audit")
            .expect("custom trigger count"),
        vec![4]
    );
    assert_eq!(opened.engine.vector_row_count_for_test().expect("vector rows"), 12);
    opened.engine.close().expect("close");
    let reopened = open(&path);
    assert_eq!(
        reopened.engine.search_with_limit("batchneedle", 20).expect("search").results.len(),
        12
    );
}

#[test]
fn deferred_hybrid_search_matches_full_sort_and_frozen_eligibility_hides_inactive_rows() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("hybrid{SQLITE_SUFFIX}"));
    let route_witness = dir.path().join("routes.jsonl");
    unsafe {
        std::env::set_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness);
        std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    }
    let opened = open(&path);
    let writes = (1..=220)
        .map(|rank| {
            node(
                &format!("hybrid-{rank}"),
                format!("{} document-{rank}", "slice75hybrid ".repeat(rank)),
                "slice75-hybrid",
            )
        })
        .collect::<Vec<_>>();
    opened.engine.write(&writes).expect("seed");
    opened.engine.drain(10_000).expect("drain");
    let optimized = opened.engine.search_with_limit("slice75hybrid", 100).expect("optimized");
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let control = opened.engine.search_with_limit("slice75hybrid", 100).expect("control");
    assert_eq!(optimized, control);
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "slice75hybrid inactive".into(),
            source_id: SourceId::new("slice75-hybrid").expect("source id"),
            logical_id: Some("hybrid-inactive".into()),
            state: InitialState::Pending,
            reason: Some("fixture".into()),
            valid_from: None,
            valid_until: None,
        }])
        .expect("write inactive row");
    let context =
        ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context");
    let frozen = opened.engine.freeze_read_context(&context).expect("freeze");
    let frozen_result = opened
        .engine
        .search_frozen("slice75hybrid", &frozen, 0, false, 0.3, 0, false, 100)
        .expect("frozen search");
    assert_eq!(control.results, frozen_result.results);
    assert!(frozen_result.results.iter().all(|hit| hit.body != "slice75hybrid inactive"));
    let routes = std::fs::read_to_string(route_witness).expect("route witness");
    assert!(routes.contains("hybrid_deferred_identity"));
    assert!(routes.contains("hybrid_full_sort_forced"));
    unsafe {
        std::env::remove_var("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST");
        std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    }
}

#[test]
fn concurrent_writes_projection_and_search_never_duplicate_or_expose_deleted_rows() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("race{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let engine = Arc::new(opened.engine);
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let (checked_tx, checked_rx) = mpsc::sync_channel(0);
    let writer = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || {
            for index in 0..40 {
                let logical = format!("race-{index}");
                let receipt = engine
                    .write(&[node(
                        &logical,
                        format!("raceneedle document {index}"),
                        "slice75-race",
                    )])
                    .expect("race write");
                let deleted = index % 3 == 0;
                if deleted {
                    engine
                        .transition(&logical, LifecycleState::Deleted, Some("race fixture".into()))
                        .expect("race delete");
                }
                engine.drain(10_000).expect("race projection drain");
                ready_tx.send((receipt.cursor, deleted)).expect("publish completed mutation");
                checked_rx.recv().expect("wait for search check");
            }
        })
    };
    for _ in 0..40 {
        let (cursor, deleted) = ready_rx.recv().expect("completed mutation");
        let result = engine.search_with_limit("raceneedle", 100).expect("race search");
        let identities = result.results.iter().map(|hit| hit.id.clone()).collect::<HashSet<_>>();
        assert_eq!(identities.len(), result.results.len(), "search returned duplicate identities");
        assert!(
            result.projection_cursor >= cursor,
            "search cursor {} did not cover drained write {cursor}",
            result.projection_cursor
        );
        if deleted {
            assert!(
                result.results.iter().all(|hit| hit.write_cursor != cursor),
                "deleted cursor {cursor} remained visible after its transition completed"
            );
        }
        checked_tx.send(()).expect("release writer");
    }
    writer.join().expect("writer");
    engine.drain(10_000).expect("drain");
    let final_result = engine.search_with_limit("raceneedle", 100).expect("final search");
    assert_eq!(final_result.results.len(), 26);
}

#[test]
fn source_erasure_removes_dependency_and_search_state_before_safe_recreation_and_reopen() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("erase{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let source_body = "eraseneedle canonical source";
    opened
        .engine
        .write(&[
            canonical("erase-source-r1", "erase-source", "slice75-erasure", source_body),
            derived(
                "erase-derived-r1",
                "erase-derived",
                "slice75-erasure",
                "erase-source-r1",
                source_body,
                "eraseneedle derived value",
            ),
        ])
        .expect("provenanced write");
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "erase-dependency",
                "erase-source-r1",
                "erase-derived-r1",
            )
            .expect("dependency request"),
        )
        .expect("register dependency");
    opened.engine.drain(10_000).expect("drain");
    assert_eq!(opened.engine.search("eraseneedle").expect("pre-erase search").results.len(), 2);
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("context"),
        )
        .expect("freeze");
    opened.engine.erase_source("slice75-erasure").expect("erase source");
    assert!(opened.engine.search("eraseneedle").expect("post-erase search").results.is_empty());
    assert!(matches!(
        opened.engine.search_frozen("eraseneedle", &frozen, 0, false, 0.3, 0, false, 10),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted
    ));
    opened
        .engine
        .write(&[node("erase-recreated", "eraseneedle recreated".into(), "slice75-erasure")])
        .expect("recreate");
    opened.engine.drain(10_000).expect("recreation drain");
    opened.engine.close().expect("close");
    let reopened = open(&path);
    let results = reopened.engine.search("eraseneedle").expect("reopen search").results;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].body, "eraseneedle recreated");
    assert_eq!(reopened.engine.vector_row_count_for_test().expect("vector rows"), 1);
}

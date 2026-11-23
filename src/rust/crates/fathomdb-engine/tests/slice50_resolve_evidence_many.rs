//! Slice 50 (0.8.31) — `resolve_evidence_many` engine contract tests.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_evidence_before_resolve_return_hook_for_test, ArtifactRevisionId, Engine, EngineError,
    EvidenceErrorReasonV1, EvidenceRefV1, EvidenceResolveManyItemV1, EvidenceResolveManyRequestV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, InitialState, LifecycleState, PreparedWrite,
    ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SourceId, SourceVersionId,
    WriteProvenanceV1, EVIDENCE_RESOLVE_MANY_MAX,
};
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice50-many-fixed", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        if vector.iter().all(|value| *value == 0.0) {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn open_engine(dir: &TempDir, name: &str) -> Engine {
    Engine::open_with_embedder_for_test(dir.path().join(name), Arc::new(FixedEmbedder))
        .expect("open test engine")
        .engine
}

fn canonical_node(
    logical: &str,
    revision: &str,
    version: &str,
    source_id: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
        properties: None,
    })
}

fn default_read_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

fn seed_two_docs(engine: &Engine) -> (fathomdb_engine::FrozenReadContextV1, Vec<EvidenceRefV1>) {
    engine
        .write(&[
            canonical_node("doc:a", "rev:a", "ver:a", "src:a", "needle alpha document"),
            canonical_node("doc:b", "rev:b", "ver:b", "src:b", "needle beta document"),
        ])
        .unwrap();
    engine.drain(5_000).unwrap();
    let frozen = engine.freeze_read_context(&default_read_context()).unwrap();
    let searched = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), frozen.clone()))
        .unwrap();
    assert_eq!(searched.evidence.len(), 2);
    let refs = searched.evidence.into_iter().map(|entry| entry.evidence_ref).collect();
    (frozen, refs)
}

#[test]
fn bounds_zero_and_over_max_rejected_before_db_lookup() {
    let dir = TempDir::new().unwrap();
    let engine = open_engine(&dir, "bounds.sqlite");
    let (frozen, refs) = seed_two_docs(&engine);

    assert_eq!(EVIDENCE_RESOLVE_MANY_MAX, 50);

    // 0 refs -> InvalidArgument
    let empty_req = EvidenceResolveManyRequestV1 {
        schema_version: 1,
        evidence_refs: Vec::new(),
        context: frozen.clone(),
    };
    let err = engine.resolve_evidence_many(&empty_req).unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidArgument { .. }),
        "expected InvalidArgument for empty refs, got {err:?}"
    );

    // 51 identical duplicate refs -> InvalidArgument (enforced BEFORE dedup)
    let over_max_req = EvidenceResolveManyRequestV1 {
        schema_version: 1,
        evidence_refs: vec![refs[0].clone(); EVIDENCE_RESOLVE_MANY_MAX + 1],
        context: frozen.clone(),
    };
    let err = engine.resolve_evidence_many(&over_max_req).unwrap_err();
    assert!(
        matches!(err, EngineError::InvalidArgument { .. }),
        "expected InvalidArgument for 51 refs, got {err:?}"
    );

    // Exactly 50 refs succeeds
    let exact_max_req = EvidenceResolveManyRequestV1 {
        schema_version: 1,
        evidence_refs: vec![refs[0].clone(); EVIDENCE_RESOLVE_MANY_MAX],
        context: frozen,
    };
    let items = engine.resolve_evidence_many(&exact_max_req).unwrap();
    assert_eq!(items.len(), EVIDENCE_RESOLVE_MANY_MAX);
    assert!(items.iter().all(|item| matches!(item, EvidenceResolveManyItemV1::Resolved(_))));

    engine.close().unwrap();
}

#[test]
fn schema_version_mismatch_fails_whole_call() {
    let dir = TempDir::new().unwrap();
    let engine = open_engine(&dir, "schema_version.sqlite");
    let (frozen, _) = seed_two_docs(&engine);

    // schema_version != 1 takes precedence even over empty evidence_refs
    let req = EvidenceResolveManyRequestV1 {
        schema_version: 2,
        evidence_refs: Vec::new(),
        context: frozen,
    };
    let err = engine.resolve_evidence_many(&req).unwrap_err();
    match err {
        EngineError::Evidence(ev) => {
            assert_eq!(ev.reason, EvidenceErrorReasonV1::UnsupportedSchemaVersion);
            assert_eq!(ev.field_path, "/schemaVersion");
        }
        other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
    }

    engine.close().unwrap();
}

#[test]
fn duplicates_preserve_order_and_arity_in_single_snapshot() {
    let dir = TempDir::new().unwrap();
    let engine = open_engine(&dir, "duplicates.sqlite");
    let (frozen, refs) = seed_two_docs(&engine);

    let single_0 = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: refs[0].clone(),
            context: frozen.clone(),
        })
        .unwrap();
    let single_1 = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: refs[1].clone(),
            context: frozen.clone(),
        })
        .unwrap();

    let batch = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![refs[0].clone(), refs[1].clone(), refs[0].clone()],
            context: frozen,
        })
        .unwrap();
    assert_eq!(batch.len(), 3);
    assert_eq!(batch[0], EvidenceResolveManyItemV1::Resolved(Box::new(single_0.clone())));
    assert_eq!(batch[1], EvidenceResolveManyItemV1::Resolved(Box::new(single_1)));
    assert_eq!(batch[2], EvidenceResolveManyItemV1::Resolved(Box::new(single_0)));

    engine.close().unwrap();
}

#[test]
fn frozen_context_and_post_hook_drift_fail_whole_call_with_evidence_unavailable() {
    let dir = TempDir::new().unwrap();
    let engine = Arc::new(open_engine(&dir, "drift.sqlite"));
    let (frozen, refs) = seed_two_docs(&engine);

    // 1. Foreign DB frozen context -> whole-call EvidenceUnavailable
    let other_engine = open_engine(&dir, "other.sqlite");
    let other_frozen = other_engine.freeze_read_context(&default_read_context()).unwrap();
    let foreign_err = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![refs[0].clone()],
            context: other_frozen,
        })
        .unwrap_err();
    match foreign_err {
        EngineError::Evidence(ev) => {
            assert_eq!(ev.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
        }
        other => panic!("expected whole-call EvidenceUnavailable, got {other:?}"),
    }
    other_engine.close().unwrap();

    // 2. Stale context after real write -> whole-call EvidenceUnavailable
    engine
        .write(&[canonical_node("doc:c", "rev:c", "ver:c", "src:c", "needle gamma document")])
        .unwrap();
    let stale_err = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![refs[0].clone()],
            context: frozen,
        })
        .unwrap_err();
    match stale_err {
        EngineError::Evidence(ev) => {
            assert_eq!(ev.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
        }
        other => panic!("expected EvidenceUnavailable for drifted snapshot, got {other:?}"),
    }

    // 3. Re-mint fresh frozen context and verify before_resolve_return hook fires
    //    while snapshot linearization stays intact against concurrent external mutation.
    let fresh_frozen = engine.freeze_read_context(&default_read_context()).unwrap();
    let searched = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), fresh_frozen.clone()))
        .unwrap();
    let fresh_ref = searched.evidence[0].evidence_ref.clone();
    let hook_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let hook_fired_clone = Arc::clone(&hook_fired);
    arm_evidence_before_resolve_return_hook_for_test(
        &engine,
        Box::new(move || {
            hook_fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        }),
    );
    let ok = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![fresh_ref],
            context: fresh_frozen,
        })
        .unwrap();
    assert_eq!(ok.len(), 1);
    assert!(hook_fired.load(std::sync::atomic::Ordering::SeqCst));

    engine.close().unwrap();
}

#[test]
fn partial_results_nondisclosure_matrix_and_raw_refs() {
    let dir = TempDir::new().unwrap();
    let engine = open_engine(&dir, "partial.sqlite");
    let (frozen, refs) = seed_two_docs(&engine);

    // Mint a second context with a different eligibility filter to get a wrong-context token
    let mut doc_filter = SearchFilter::default();
    doc_filter.kind = Some("document".into());
    let filtered_ctx = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), doc_filter).unwrap())
        .unwrap();
    let filtered_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), filtered_ctx))
        .unwrap();
    let wrong_context_ref = filtered_search.evidence[0].evidence_ref.clone();

    // Tamper the last char of refs[1]
    let mut tampered = refs[1].as_str().to_owned();
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'a' { 'b' } else { 'a' });
    let tampered_ref = EvidenceRefV1::new(tampered).unwrap();

    let malformed_ref = EvidenceRefV1::new("not-a-valid-evidence-token").unwrap();

    let batch = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![
                refs[0].clone(),
                malformed_ref,
                tampered_ref,
                wrong_context_ref,
                refs[0].clone(),
            ],
            context: frozen.clone(),
        })
        .unwrap();

    assert_eq!(batch.len(), 5);
    assert!(matches!(batch[0], EvidenceResolveManyItemV1::Resolved(_)));
    assert_eq!(batch[0], batch[4]);
    for (slot, item) in [(1, &batch[1]), (2, &batch[2]), (3, &batch[3])] {
        match item {
            EvidenceResolveManyItemV1::Refused(err) => {
                assert_eq!(err.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
                assert_eq!(err.field_path, format!("/evidenceRefs/{slot}"));
            }
            other => panic!("expected Refused at slot {slot}, got {other:?}"),
        }
    }

    // Raw-ref helper handles empty "" and >2048-byte strings as per-slot Refused
    let oversized = "x".repeat(2_049);
    let raw_batch = engine
        .resolve_evidence_many_raw_refs(
            1,
            &[refs[0].as_str().to_owned(), String::new(), oversized, refs[1].as_str().to_owned()],
            &frozen,
        )
        .unwrap();
    assert_eq!(raw_batch.len(), 4);
    assert!(matches!(raw_batch[0], EvidenceResolveManyItemV1::Resolved(_)));
    assert!(matches!(raw_batch[3], EvidenceResolveManyItemV1::Resolved(_)));
    for slot in [1usize, 2] {
        match &raw_batch[slot] {
            EvidenceResolveManyItemV1::Refused(err) => {
                assert_eq!(err.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
                assert_eq!(err.field_path, format!("/evidenceRefs/{slot}"));
            }
            other => panic!("expected Refused at raw slot {slot}, got {other:?}"),
        }
    }

    // Superseded artifact token minted before supersession is refused as
    // EvidenceUnavailable at its slot under a live default view minted after supersession.
    let stable_view = ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() };
    let stable_ctx = ReadContextV1::new(stable_view, SearchFilter::default()).unwrap();
    let pre_sup_frozen = engine.freeze_read_context(&stable_ctx).unwrap();
    let pre_sup_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), pre_sup_frozen))
        .unwrap();
    let pre_sup_ref_a = pre_sup_search
        .evidence
        .iter()
        .find(|e| e.artifact_revision_id == "rev:a")
        .expect("rev:a must be present before supersession")
        .evidence_ref
        .clone();
    engine
        .write(&[canonical_node(
            "doc:a",
            "rev:a2",
            "ver:a2",
            "src:a2",
            "needle alpha superseded replacement",
        )])
        .unwrap();
    engine.drain(5_000).unwrap();
    let live_frozen = engine.freeze_read_context(&stable_ctx).unwrap();
    let live_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new("needle".into(), live_frozen.clone()))
        .unwrap();
    let live_valid_ref = live_search.evidence[0].evidence_ref.clone();
    let sup_batch = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![live_valid_ref, pre_sup_ref_a],
            context: live_frozen,
        })
        .unwrap();
    assert_eq!(sup_batch.len(), 2);
    assert!(matches!(sup_batch[0], EvidenceResolveManyItemV1::Resolved(_)));
    match &sup_batch[1] {
        EvidenceResolveManyItemV1::Refused(err) => {
            assert_eq!(err.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
            assert_eq!(err.field_path, "/evidenceRefs/1");
        }
        other => panic!("expected Refused for superseded ref at slot 1, got {other:?}"),
    }

    // Test erased token in a new snapshot
    let old_ref_b = refs[1].clone();
    // Transition doc:b to deleted and erase src:b -> ref is now inaccessible even if we re-mint
    engine.transition("doc:b", LifecycleState::Deleted, None).unwrap();
    engine.erase_source("src:b").unwrap();
    let after_erase_frozen = engine.freeze_read_context(&default_read_context()).unwrap();
    let after_search = engine
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "needle".into(),
            after_erase_frozen.clone(),
        ))
        .unwrap();
    assert_eq!(after_search.evidence.len(), 1);
    let still_valid_ref = after_search.evidence[0].evidence_ref.clone();

    let after_batch = engine
        .resolve_evidence_many(&EvidenceResolveManyRequestV1 {
            schema_version: 1,
            evidence_refs: vec![still_valid_ref, old_ref_b],
            context: after_erase_frozen,
        })
        .unwrap();
    assert_eq!(after_batch.len(), 2);
    assert!(matches!(after_batch[0], EvidenceResolveManyItemV1::Resolved(_)));
    match &after_batch[1] {
        EvidenceResolveManyItemV1::Refused(err) => {
            assert_eq!(err.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
            assert_eq!(err.field_path, "/evidenceRefs/1");
        }
        other => panic!("expected Refused for erased ref, got {other:?}"),
    }

    engine.close().unwrap();
}

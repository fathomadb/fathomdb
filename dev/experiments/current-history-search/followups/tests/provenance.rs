use current_history_followups::ledger::Ledger;
use fathomdb_engine::*;
use sha2::{Digest, Sha256};

fn source() -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "source".into(),
        body: "source bytes".into(),
        source_id: SourceId::new("source").unwrap(),
        logical_id: Some("authority".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-r1").unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
        ),
    })
}
fn derived(revision: &str, body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("source").unwrap(),
        logical_id: Some("claim".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(
                Sha256::digest(b"source bytes")
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
            )
            .unwrap(),
        ),
    })
}
#[test]
fn exact_intrinsic_revision_and_source_authority_survive_fragment_selection() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("provenance.sqlite")).unwrap();
    ledger.engine.write(&[source()]).unwrap();
    let a =
        ledger.change_prepared(derived("assertion-r1", "oldneedle A"), (0, 100), false).unwrap();
    let b = ledger.change_prepared(derived("assertion-r2", "new B"), (20, 40), false).unwrap();
    assert_eq!(ledger.selected("claim", 10, b as i64).unwrap(), vec![(a, "oldneedle A".into())]);
    let intrinsic:(String,String)=ledger.read.query_row("SELECT ar.revision_id,link.source_revision_id FROM _fathomdb_artifact_revisions ar JOIN _fathomdb_source_links link ON link.artifact_revision_id=ar.revision_id WHERE ar.write_cursor=?1",[a],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(intrinsic, ("assertion-r1".into(), "source-r1".into()));
    ledger.engine.drain(10_000).unwrap();
    let frozen = ledger
        .engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { include_superseded: true, ..Default::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    let request = EvidenceSearchRequestV1 {
        schema_version: 1,
        query: "oldneedle".into(),
        context: frozen.clone(),
        rerank_depth: 0,
        use_graph_arm: false,
        alpha: 0.3,
        pool_n: 0,
        include_explanation: false,
        limit: 10,
    };
    let evidence = ledger.engine.search_with_evidence(&request).unwrap();
    assert_eq!(evidence.evidence[0].artifact_revision_id, "assertion-r1");
    let resolved = ledger
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, "assertion-r1");
    assert_eq!(resolved.source_revision_id, "source-r1");
    assert_eq!(resolved.canonical_source_body, "source bytes");
    ledger.engine.transition("authority", LifecycleState::Deleted, None).unwrap();
    assert!(
        ledger.selected("claim", 10, b as i64).unwrap().is_empty(),
        "historical K cannot restore current source authority"
    );
    ledger.engine.close().unwrap();
}

#[test]
fn declared_derived_revision_requires_positive_source_link() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("missing-link.sqlite")).unwrap();
    ledger.engine.write(&[source()]).unwrap();
    let cut = ledger.change_prepared(derived("assertion-r1", "body"), (0, 100), false).unwrap();
    ledger
        .engine
        .execute_for_test(
            "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='assertion-r1'",
        )
        .unwrap();
    assert!(ledger.selected("claim", 10, cut as i64).unwrap().is_empty());
    ledger.engine.close().unwrap();
}

#[test]
fn temporal_adapter_refuses_independent_canonical_validity_windows() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("double-window.sqlite")).unwrap();
    ledger.engine.write(&[source()]).unwrap();
    let PreparedWrite::ProvenancedNode(mut node) = derived("assertion-r1", "body") else {
        unreachable!()
    };
    node.valid_from = Some(1);
    assert!(ledger.change_prepared(PreparedWrite::ProvenancedNode(node), (0, 100), false).is_err());
    assert_eq!(
        ledger.engine.query_i64_col_for_test("SELECT COUNT(*) FROM poc_claims").unwrap(),
        vec![0]
    );
    ledger.engine.close().unwrap();
}

#[test]
fn current_family_tracks_source_boundaries_and_actual_source_revocation() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("current-source.sqlite")).unwrap();
    let PreparedWrite::ProvenancedNode(mut canonical) = source() else { unreachable!() };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    canonical.valid_until = Some(now + 100);
    store
        .write(&[PreparedWrite::ProvenancedNode(canonical), derived("assertion-r1", "needle")])
        .unwrap();
    store.bootstrap(now).unwrap();
    assert_eq!(store.search("needle", now, 10).unwrap().rows.len(), 1);
    assert!(
        store.search("needle", now + 100, 10).unwrap().rows.is_empty(),
        "source expiry must remove dependent rows"
    );
    store.close().unwrap();

    let store = CurrentStore::open(&dir.path().join("current-revoke.sqlite")).unwrap();
    store.write(&[source(), derived("assertion-r1", "needle")]).unwrap();
    store.bootstrap(now).unwrap();
    store
        .with_writer(|engine| engine.transition("authority", LifecycleState::Deleted, None))
        .unwrap();
    assert!(
        store.search("needle", now, 10).unwrap().rows.is_empty(),
        "source revocation invalidates and refreshes dependencies"
    );
    store.close().unwrap();
}

#[test]
fn temporal_claim_kind_is_immutable() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("kind.sqlite")).unwrap();
    ledger.engine.write(&[source()]).unwrap();
    ledger.change_prepared(derived("assertion-r1", "body"), (0, 100), false).unwrap();
    let PreparedWrite::ProvenancedNode(mut node) = derived("assertion-r2", "other") else {
        unreachable!()
    };
    node.kind = "different-type".into();
    assert!(ledger.change_prepared(PreparedWrite::ProvenancedNode(node), (20, 40), false).is_err());
    ledger.engine.close().unwrap();
}

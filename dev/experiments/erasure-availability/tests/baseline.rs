use erasure_availability::*;
use fathomdb_engine::EngineError;

#[test]
fn shipping_ghost_positive_control_and_exact_family_counts() {
    for family in ["node", "edge", "property"] {
        let dir = tempfile::tempdir().unwrap();
        let f = Fixture::build(&dir.path().join("db.sqlite"), family, false, 4, 1).unwrap();
        let before = f.inventory().unwrap();
        let expected = match family {
            "node" => [4, 0, 0, 4],
            "edge" => [0, 4, 0, 0],
            _ => [4, 0, 4, 4],
        };
        assert_eq!(before.iter().map(|r| r.matches).collect::<Vec<_>>(), expected);
        for (row, count) in before.iter().zip(expected) {
            if count > 0 {
                assert!(row.shadow_markers > 0, "missing positive marker: {}", row.table);
            }
        }
        f.erase().unwrap();
        let after = f.inventory().unwrap();
        assert!(after.iter().all(|r| r.matches == 0));
        assert!(
            after.iter().any(|r| r.shadow_markers > 0),
            "unchanged shipping ghost remains visible"
        );
        assert!(physical_markers(&f.path, MARKER).unwrap() > 0);
        let survivors = inventory(&f.path, "survivorneedle").unwrap();
        assert_eq!(survivors.iter().map(|r| r.matches).collect::<Vec<_>>(), expected);
        f.engine.close().unwrap();
    }
}

#[test]
fn ordinary_pending_wal_allows_unrelated_writer_and_keeps_pinned_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 0).unwrap();
    let pin = reader(&f.path).unwrap();
    pin.execute_batch("BEGIN").unwrap();
    let pre: i64 = pin
        .query_row("SELECT count(*) FROM canonical_nodes WHERE source_id='target'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(pre, 2);
    let error = f.engine.erase_source("target").unwrap_err();
    assert!(
        matches!(error,EngineError::ErasureIncomplete{ref stage,..} if stage=="wal_checkpoint"),
        "{error:?}"
    );
    f.engine
        .write(&[node("other", "other", "unrelated")])
        .expect("no dependency closure was created for a dependency-free source");
    let old: i64 = pin
        .query_row("SELECT count(*) FROM canonical_nodes WHERE source_id='target'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(old, pre, "pre-denial snapshots retain their old cut");
    let new = reader(&f.path).unwrap();
    let current: i64 = new
        .query_row("SELECT count(*) FROM canonical_nodes WHERE source_id='target'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(current, 0);
    drop(new);
    pin.execute_batch("ROLLBACK").unwrap();
    drop(pin);
    f.engine.erase_source("target").unwrap();
    f.engine.write(&[node("other", "other", "unrelated")]).unwrap();
    f.engine.close().unwrap();
}

#[test]
fn busy_checkpoint_cannot_complete_sanitation_and_retry_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let f = Fixture::build(&dir.path().join("db.sqlite"), "node", false, 4, 1).unwrap();
    f.erase().unwrap();
    admit(&f.engine, 100, 130).unwrap();
    let pin = reader(&f.path).unwrap();
    pin.execute_batch("BEGIN").unwrap();
    let _: i64 = pin.query_row("SELECT count(*) FROM search_index_data", [], |r| r.get(0)).unwrap();
    for _ in 0..256 {
        if job(&f.engine).unwrap().phase != "cleaning" {
            break;
        }
        step(&f.engine, 1, 101, false).unwrap();
        f.engine.write(&[node("other", "other", "unrelated progress")]).unwrap();
    }
    assert_eq!(job(&f.engine).unwrap().phase, "proof_pending");
    assert!(!finalize(&f.engine, &f.path, MARKER).unwrap());
    assert_eq!(job(&f.engine).unwrap().phase, "proof_pending");
    pin.execute_batch("ROLLBACK").unwrap();
    drop(pin);
    assert!(finalize(&f.engine, &f.path, MARKER).unwrap());
    assert_eq!(job(&f.engine).unwrap().phase, "complete");
    assert_eq!(physical_markers(&f.path, MARKER).unwrap(), 0);
    f.engine.close().unwrap();
}

#[test]
fn registered_dependency_pending_closure_fences_unrelated_writes() {
    use fathomdb_engine::{
        ArtifactRevisionId, CanonicalHash, InitialState, PreparedWrite, ProvenancedNodeV1,
        SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
        WriteProvenanceV1,
    };
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.sqlite");
    let engine = open(&path).unwrap();
    let source_body = "source original";
    let canonical = WriteProvenanceV1::canonical(
        ArtifactRevisionId::new("source-r1").unwrap(),
        SourceVersionId::new("v1").unwrap(),
    );
    let derived = WriteProvenanceV1::derived(
        ArtifactRevisionId::new("derived-r1").unwrap(),
        SourceVersionId::new("v1").unwrap(),
        SourceRevisionId::new("source-r1").unwrap(),
        SourceLocator::whole_body(),
        CanonicalHash::sha256(
            Sha256::digest(source_body.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        )
        .unwrap(),
    );
    let writes = [("source", source_body, canonical), ("derived", "derived original", derived)]
        .into_iter()
        .map(|(id, body, provenance)| {
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: body.into(),
                source_id: SourceId::new("target").unwrap(),
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance,
            })
        })
        .collect::<Vec<_>>();
    engine.write(&writes).unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    engine.drain(60_000).unwrap();
    let pin = reader(&path).unwrap();
    pin.execute_batch("BEGIN").unwrap();
    let count: i64 =
        pin.query_row("SELECT count(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();
    assert_eq!(count, 2);
    let err = engine.erase_source("target").unwrap_err();
    assert!(
        matches!(err,EngineError::ErasureIncomplete{ref stage,..} if stage=="wal_checkpoint"),
        "{err:?}"
    );
    let err = engine.write(&[node("other", "other", "unrelated")]).unwrap_err();
    assert!(
        matches!(err,EngineError::ErasureIncomplete{ref stage,..} if stage=="dependency_closure"),
        "{err:?}"
    );
    assert!(admit(&engine, 100, 130).is_err());
    pin.execute_batch("ROLLBACK").unwrap();
    drop(pin);
    engine.erase_source("target").unwrap();
    engine.write(&[node("other", "other", "unrelated")]).unwrap();
    engine.close().unwrap();
}

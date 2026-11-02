use current_history_followups::ledger::Ledger;

#[test]
fn real_head_survives_partial_a_b_a_without_fabricating_knowledge() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("ledger.sqlite")).unwrap();
    let a = ledger.change("claim", "A", "source", (0, 100), false).unwrap();
    let b = ledger.change("claim", "B", "source", (20, 40), false).unwrap();
    for (v, expected) in [
        (0, (a, "A")),
        (19, (a, "A")),
        (20, (b, "B")),
        (39, (b, "B")),
        (40, (a, "A")),
        (99, (a, "A")),
    ] {
        assert_eq!(
            ledger.selected("claim", v, b as i64).unwrap(),
            vec![(expected.0, expected.1.into())]
        );
    }
    assert_eq!(ledger.selected("claim", 30, a as i64).unwrap(), vec![(a, "A".into())]);
    assert!(ledger.selected("claim", 100, b as i64).unwrap().is_empty());
    assert!(ledger.selected("claim", 30, a as i64 - 1).unwrap().is_empty());
    assert_eq!(ledger.read.query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='claim' AND superseded_at IS NULL",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    assert_eq!(ledger.read.query_row("SELECT body FROM canonical_nodes WHERE logical_id='claim' AND superseded_at IS NULL",[],|r|r.get::<_,String>(0)).unwrap(),"B");
    ledger.engine.close().unwrap();
}

#[test]
fn actual_lifecycle_and_erasure_do_not_resurrect_temporal_payloads() {
    use fathomdb_engine::LifecycleState;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lifecycle.sqlite");
    let ledger = Ledger::open(&path).unwrap();
    let a = ledger.change("claim", "A", "source", (0, 100), false).unwrap();
    ledger.engine.transition("claim", LifecycleState::Deleted, None).unwrap();
    assert!(
        ledger.selected("claim", 10, a as i64).unwrap().is_empty(),
        "past knowledge cannot bypass current per-revision deletion"
    );
    ledger.engine.erase_source("source").unwrap();
    assert_eq!(
        ledger
            .read
            .query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    ledger.engine.close().unwrap();
    drop(ledger);
    let reopened = Ledger::open(&path).unwrap();
    reopened.engine.rebuild_projections().unwrap();
    assert!(reopened.selected("claim", 10, a as i64).unwrap().is_empty());
    reopened.engine.close().unwrap();
}

#[test]
fn nested_retraction_late_future_and_atomic_fault_preserve_prior_account() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested.sqlite");
    let ledger = Ledger::open(&path).unwrap();
    let a = ledger.change("claim", "A", "source", (0, 100), false).unwrap();
    let b = ledger.change("claim", "B", "source", (20, 80), false).unwrap();
    let retract = ledger.change("claim", "withdrawn", "source", (30, 60), true).unwrap();
    assert!(ledger.selected("claim", 40, retract as i64).unwrap().is_empty());
    assert_eq!(ledger.selected("claim", 40, b as i64).unwrap(), vec![(b, "B".into())]);
    assert_eq!(ledger.selected("claim", 10, retract as i64).unwrap(), vec![(a, "A".into())]);
    let future = ledger.change("claim", "future", "source", (100, 200), false).unwrap();
    assert_eq!(
        ledger.selected("claim", 150, future as i64).unwrap(),
        vec![(future, "future".into())]
    );
    assert!(ledger.selected("claim", 150, future as i64 - 1).unwrap().is_empty());
    let late = ledger.change("claim", "late", "source", (5, 10), false).unwrap();
    assert_eq!(ledger.selected("claim", 5, late as i64).unwrap(), vec![(late, "late".into())]);
    assert_eq!(ledger.selected("claim", 5, a as i64).unwrap(), vec![(a, "A".into())]);
    let before: i64 =
        ledger.read.query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get(0)).unwrap();
    ledger.engine.execute_for_test("CREATE TRIGGER poc_fault AFTER INSERT ON poc_fragments WHEN NEW.claim='claim' BEGIN SELECT RAISE(ABORT,'injected ledger fault'); END;").unwrap();
    assert!(ledger.change("claim", "mustrollback", "source", (0, 200), false).is_err());
    assert_eq!(
        ledger
            .read
            .query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert_eq!(ledger.read.query_row("SELECT body FROM canonical_nodes WHERE logical_id='claim' AND superseded_at IS NULL",[],|r|r.get::<_,String>(0)).unwrap(),"late");
    ledger.engine.execute_for_test("DROP TRIGGER poc_fault").unwrap();
    ledger.engine.close().unwrap();
    drop(ledger);
    let reopened = Ledger::open(&path).unwrap();
    reopened.engine.rebuild_projections().unwrap();
    assert_eq!(reopened.selected("claim", 5, late as i64).unwrap(), vec![(late, "late".into())]);
    reopened.engine.close().unwrap();
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(16))]
    #[test]
    fn fragment_roundtrip_matches_last_explicit_covering_assertion(ops in proptest::collection::vec((0i64..20,1i64..20,proptest::bool::ANY),1..14)) {
        let dir=tempfile::tempdir().unwrap();
        let path=dir.path().join("property.sqlite");
        let ledger=Ledger::open(&path).unwrap();
        let mut changes=Vec::new();
        for (i,(start,len,withdraw)) in ops.into_iter().enumerate() {
            let body=format!("payload{i}");
            let cursor=ledger.change("claim",&body,"source",(start,start+len),withdraw).unwrap();
            changes.push((cursor,start,start+len,withdraw,body));
        }
        ledger.engine.close().unwrap();drop(ledger);
        let ledger=Ledger::open(&path).unwrap();
        ledger.engine.rebuild_projections().unwrap();
        for known in changes.iter().map(|c|c.0) {
            for valid in 0..40 {
                let expected=changes.iter().rev().find(|c|c.0<=known && c.1<=valid && valid<c.2)
                    .filter(|c|!c.3).map(|c|(c.0,c.4.clone())).into_iter().collect::<Vec<_>>();
                proptest::prop_assert_eq!(ledger.selected("claim",valid,known as i64).unwrap(),expected);
            }
        }
        ledger.engine.close().unwrap();
    }
}

fn ordinary(claim: &str, body: &str) -> fathomdb_engine::PreparedWrite {
    fathomdb_engine::PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: fathomdb_engine::SourceId::new("source").unwrap(),
        logical_id: Some(claim.into()),
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn invalid_temporal_input_cannot_attach_intent_to_a_later_ordinary_write() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("invalid.sqlite")).unwrap();
    assert!(ledger.change("claim", "intended", "", (0, 100), false).is_err());
    ledger.engine.write(&[ordinary("claim", "ordinary")]).unwrap();
    assert_eq!(
        ledger
            .read
            .query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    ledger.engine.close().unwrap();
}

#[test]
fn interrupted_staging_cannot_survive_reopen_or_bind_an_unrelated_payload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("interrupted.sqlite");
    let ledger = Ledger::open(&path).unwrap();
    ledger.engine.execute_for_test("INSERT INTO poc_intent VALUES('claim',0,100,0)").unwrap();
    ledger.engine.close().unwrap();
    drop(ledger);
    let ledger = Ledger::open(&path).unwrap();
    ledger.engine.write(&[ordinary("claim", "ordinary")]).unwrap();
    assert_eq!(
        ledger
            .read
            .query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    ledger.engine.close().unwrap();
}

#[test]
fn claim_source_is_immutable_and_unknown_batch_interior_is_not_a_knowledge_cut() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("cuts.sqlite")).unwrap();
    ledger.change("claim", "A", "source", (0, 100), false).unwrap();
    assert!(ledger.change("claim", "source hijack", "other-source", (0, 100), false).is_err());
    let receipt =
        ledger.engine.write(&[ordinary("first", "first"), ordinary("second", "second")]).unwrap();
    assert!(receipt.row_cursors[0] < receipt.cursor);
    assert!(
        ledger.selected("claim", 10, receipt.row_cursors[0] as i64).is_err(),
        "row cursor inside an atomic batch is not an authenticated temporal commit cut"
    );
    ledger.engine.close().unwrap();
}

#[test]
fn erased_source_leaves_no_candidate_identity_and_staging_failure_releases_writer() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("ownership.sqlite")).unwrap();
    ledger.change("secret-claim", "A", "secret-source", (0, 100), false).unwrap();
    ledger.engine.erase_source("secret-source").unwrap();
    assert_eq!(ledger.read.query_row("SELECT COUNT(*) FROM poc_claims WHERE claim='secret-claim' OR source='secret-source'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    ledger
        .engine
        .execute_for_test("INSERT INTO poc_operation VALUES('claim','different','source')")
        .unwrap();
    assert!(ledger.change("claim", "intended", "source", (0, 100), false).is_err());
    ledger
        .engine
        .write(&[ordinary("other", "ordinary")])
        .expect("failed staging must roll back and release writer transaction");
    ledger.engine.close().unwrap();
}

#[test]
fn same_connection_wrong_payload_does_not_consume_temporal_intent() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("binding.sqlite")).unwrap();
    ledger.engine.execute_for_test("INSERT INTO poc_intent VALUES('claim',0,100,0); INSERT INTO poc_operation VALUES('claim','expected','source');").unwrap();
    ledger.engine.write(&[ordinary("claim", "different")]).unwrap();
    assert_eq!(
        ledger
            .read
            .query_row("SELECT COUNT(*) FROM poc_fragments", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    ledger.engine.close().unwrap();
}

#[test]
fn failed_staging_rolls_back_before_releasing_admission() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = Ledger::open(&dir.path().join("rollback.sqlite")).unwrap();
    ledger
        .engine
        .execute_for_test("INSERT INTO poc_operation VALUES('claim','different','source')")
        .unwrap();
    assert!(ledger.change("claim", "intended", "source", (0, 100), false).is_err());
    ledger
        .engine
        .write(&[ordinary("other", "ordinary")])
        .expect("failed stage left writer transaction open");
    ledger.engine.close().unwrap();
}

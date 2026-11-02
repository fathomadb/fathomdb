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

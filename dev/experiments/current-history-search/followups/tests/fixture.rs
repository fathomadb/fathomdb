use current_history_followups::{fixture, history};
#[test]
fn actual_family_counts_payloads_and_selection_axes_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    for family in ["node", "edge", "property"] {
        let path = dir.path().join(format!("{family}.sqlite"));
        fixture::build(&path, family, 4, 128, 100).unwrap();
        let main = rusqlite::Connection::open(&path).unwrap();
        for (view, expected) in [
            ("current", if family == "edge" { 100 } else { 99 }),
            ("corrected", if family == "edge" { 100 } else { 99 }),
            ("prior", if family == "edge" { 100 } else { 99 }),
            ("archive", 500),
            ("inactive", 100),
            ("superseded", if family == "edge" { 500 } else { 495 }),
            ("both", 500),
        ] {
            let (select, hydrate) = fixture::queries(family, 4, view);
            let tx = main.unchecked_transaction().unwrap();
            let result =
                history::search(&tx, &select, &hydrate, "needle", 10, true, || Ok(())).unwrap();
            let expected = expected * if family == "property" { 2 } else { 1 };
            assert_eq!(result.documents, expected, "{family}/{view}");
            assert_eq!(result.indexed_bytes, expected * 128);
            assert_eq!(result.hits.len(), 10);
        }
        if family == "edge" {
            assert_eq!(
                main.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r
                    .get::<_, usize>(0))
                    .unwrap(),
                101
            );
        }
    }
}

#[test]
fn interrupted_fixture_is_never_reused_as_complete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("partial.sqlite");
    let engine = fathomdb_engine::Engine::open(&path).unwrap().engine;
    engine.close().unwrap();
    assert!(fixture::build(&path, "node", 4, 128, 100).is_err());
}

#[test]
fn corrected_and_prior_select_explicit_intervals_and_committed_receipt_cuts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("temporal.sqlite");
    fixture::build(&path, "node", 4, 128, 100).unwrap();
    let main = rusqlite::Connection::open(path).unwrap();
    let (selection, _) = fixture::queries("node", 4, "corrected");
    let selected = main
        .prepare(&selection)
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for cursor in selected {
        let (ordinal, version): (i64, i64) = main
            .query_row("SELECT ordinal,version FROM poc_versions WHERE cursor=?1", [cursor], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(version, if ordinal < 10 { 3 } else { 4 });
    }
    let cut: i64 = main
        .query_row("SELECT MAX(commit_cut) FROM poc_versions WHERE version=2", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        main.query_row(
            "SELECT COUNT(DISTINCT commit_cut) FROM poc_versions WHERE commit_cut=?1",
            [cut],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    let count:i64=main.query_row("SELECT COUNT(*) FROM poc_temporal_fragments WHERE v0<=10 AND 10<v1 AND k0<=?1 AND (k1 IS NULL OR ?1<k1)",[cut],|r|r.get(0)).unwrap();
    assert_eq!(count, 100);
}

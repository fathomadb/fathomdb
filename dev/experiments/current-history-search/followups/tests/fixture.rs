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

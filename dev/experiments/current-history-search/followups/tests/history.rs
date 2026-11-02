use current_history_followups::history;
use rusqlite::Connection;
#[test]
fn contentless_native_preserves_full_compiler_statistics_and_snapshot_identity() {
    let main = Connection::open_in_memory().unwrap();
    main.execute_batch("CREATE TABLE docs(id INTEGER PRIMARY KEY,body TEXT,eligible INTEGER);")
        .unwrap();
    let bodies = [
        "running runs runner",
        "café naïve 東京",
        "the and or",
        "!!!",
        "",
        "alpha beta alpha",
        "alpha",
        "quoted words",
        "other property alpha",
    ];
    for (i, body) in bodies.iter().enumerate() {
        main.execute("INSERT INTO docs VALUES(?1,?2,?3)", rusqlite::params![i + 1, body, i != 8])
            .unwrap();
    }
    let tx = main.unchecked_transaction().unwrap();
    for query in [
        "running",
        "run runs",
        "café 東京",
        "the and",
        "!!!",
        "\"quoted words\"",
        "alpha",
        "alpha OR beta",
        "",
        "a-b",
    ] {
        let regular = history::search(
            &tx,
            "SELECT id,body,eligible FROM docs",
            "SELECT body FROM docs WHERE id=?1",
            query,
            20,
            false,
            || Ok(()),
        )
        .unwrap();
        let compact = history::search(
            &tx,
            "SELECT id,body,eligible FROM docs",
            "SELECT body FROM docs WHERE id=?1",
            query,
            20,
            true,
            || Ok(()),
        )
        .unwrap();
        assert_eq!(compact.documents, bodies.len());
        assert_eq!(compact.indexed_bytes, bodies.iter().map(|s| s.len()).sum::<usize>());
        assert_eq!(regular.hits, compact.hits, "compiler query {query:?}");
        assert_eq!(compact.temp_store, 2);
        assert!(compact.private_files.iter().all(String::is_empty));
        if query == "alpha" {
            assert!(!compact.hits.is_empty());
            assert!(compact.hits.iter().all(|h| h.id != 9));
        }
    }
}

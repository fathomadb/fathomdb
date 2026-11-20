use std::path::Path;

/// Create a readable test store with one independently known index inconsistency.
/// The product tables are untouched. Quick checking omits index/table agreement.
pub fn missing_index_entry(path: &Path) {
    let conn = rusqlite::Connection::open(path).expect("fixture connection");
    conn.execute_batch(
        "CREATE TABLE slice55_witness(v INTEGER); \
         CREATE INDEX slice55_witness_gap ON slice55_witness(v) WHERE v=1; \
         INSERT INTO slice55_witness VALUES(1),(2); \
         PRAGMA writable_schema=ON; \
         UPDATE sqlite_schema SET sql=\
           'CREATE INDEX slice55_witness_gap ON slice55_witness(v) WHERE v>0' \
           WHERE name='slice55_witness_gap'; \
         PRAGMA writable_schema=OFF; \
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .expect("install owned index inconsistency");
    drop(conn);
    let conn = rusqlite::Connection::open(path).expect("fresh fixture connection");
    let quick: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0)).unwrap();
    assert_eq!(quick, "ok", "fixture must be readable and pass quick check");
    let rows = conn
        .prepare("PRAGMA integrity_check")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(rows.iter().any(|row| row.contains("row 2 missing from index slice55_witness_gap")));
}

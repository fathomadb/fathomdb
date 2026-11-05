use fathomdb_schema::migrate;
use proptest::prelude::*;
use rusqlite::{params, Connection};

#[test]
fn spans_support_multiple_windows_and_mapping_deletion_without_foreign_keys() {
    let conn = Connection::open_in_memory().unwrap();
    migrate(&conn).unwrap();
    conn.execute_batch(
        "PRAGMA foreign_keys=OFF;
      INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(1,'doc',10),(2,'doc',10);
      INSERT INTO _fathomdb_vector_spans VALUES(1,10,0,0,5,12),(2,10,1,5,12,12);",
    )
    .unwrap();
    assert!(conn.execute("INSERT INTO _fathomdb_vector_spans VALUES(3,10,1,5,12,12)", []).is_err());
    conn.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=10", []).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM _fathomdb_vector_spans", [], |r| r.get::<_, u64>(0))
            .unwrap(),
        0
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]
    #[test]
    fn span_ranges_round_trip_and_reject_out_of_bounds(body in ".{0,40}", end in 0usize..200) {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        conn.execute("INSERT INTO _fathomdb_vector_rows VALUES(1,'doc',1)", []).unwrap();
        let length = body.len();
        let result = conn.execute("INSERT INTO _fathomdb_vector_spans VALUES(1,1,0,0,?1,?2)", params![end, length]);
        prop_assert_eq!(result.is_ok(), end <= length);
        if end <= length {
            let actual: (usize,usize) = conn.query_row("SELECT byte_end,body_length FROM _fathomdb_vector_spans", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            prop_assert_eq!(actual, (end,length));
        }
    }
}

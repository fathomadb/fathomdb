use super::*;

pub(crate) fn validate(connection: &Connection, cursor: Option<u64>) -> Result<(), EngineError> {
    // Bulk readiness is cached by the visibility nonce; scalar callers restrict
    // both indexed identities so misplaced spans cannot hide under another owner.
    let mapping_scope =
        if cursor.is_some() { " AND (r.write_cursor=?1 OR r.rowid=?1)" } else { "" };
    let span_scope =
        if cursor.is_some() { " AND (s.write_cursor=?1 OR s.vector_rowid=?1)" } else { "" };
    let sql = format!(
        "SELECT EXISTS(
          SELECT 1 FROM _fathomdb_vector_rows r
          LEFT JOIN _fathomdb_vector_spans s ON s.vector_rowid=r.rowid
          LEFT JOIN canonical_nodes n ON n.write_cursor=r.write_cursor
          LEFT JOIN canonical_edges e ON e.write_cursor=r.write_cursor
          WHERE (s.vector_rowid IS NULL OR s.write_cursor!=r.write_cursor
            OR r.rowid!=r.write_cursor OR s.window_number!=0 OR s.byte_start!=0
            OR typeof(s.body_length)!='integer' OR s.body_length<0
            OR (COALESCE(n.body,e.body) IS NOT NULL
              AND s.body_length!=length(CAST(COALESCE(n.body,e.body) AS BLOB)))
            OR (s.byte_end IS NOT NULL AND (typeof(s.byte_end)!='integer'
              OR s.byte_end<0 OR s.byte_end>s.body_length
              OR hex(substr(CAST(COALESCE(n.body,e.body) AS BLOB),s.byte_end+1,1)) BETWEEN '80' AND 'BF')))
            {mapping_scope}
          UNION ALL
          SELECT 1 FROM _fathomdb_vector_spans s
          WHERE NOT EXISTS(SELECT 1 FROM _fathomdb_vector_rows r
            WHERE r.rowid=s.vector_rowid AND r.write_cursor=s.write_cursor) {span_scope}
        )"
    );
    let corrupt: bool = match cursor {
        Some(cursor) => connection.query_row(&sql, [cursor], |row| row.get(0)),
        None => connection.query_row(&sql, [], |row| row.get(0)),
    }
    .map_err(|_| EngineError::Storage)?;
    if corrupt {
        Err(projection_generation::corruption_error())
    } else {
        Ok(())
    }
}

pub(crate) fn count_for_cursor(connection: &Connection, cursor: u64) -> rusqlite::Result<u64> {
    connection.query_row(
        "SELECT COUNT(*) FROM _fathomdb_vector_spans WHERE vector_rowid=?1 OR write_cursor=?1",
        [cursor],
        |row| row.get(0),
    )
}

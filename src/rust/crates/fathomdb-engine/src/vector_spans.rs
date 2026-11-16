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

pub(crate) fn cleanup_trigger_is_valid(connection: &Connection) -> rusqlite::Result<bool> {
    let sql: Option<String> = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='trigger' AND name='_fathomdb_vector_spans_mapping_delete'",
        [], |row| row.get(0),
    ).optional()?;
    let expected = "CREATE TRIGGER _fathomdb_vector_spans_mapping_delete AFTER DELETE ON _fathomdb_vector_rows BEGIN DELETE FROM _fathomdb_vector_spans WHERE vector_rowid=OLD.rowid; END";
    let normalize = |sql: &str| sql.split_whitespace().collect::<Vec<_>>().join(" ");
    Ok(sql.as_deref().map(normalize).as_deref() == Some(&normalize(expected)))
}

pub(crate) fn capture(connection: &Connection) -> Result<StoredHeadCoverageCounts, EngineError> {
    let mut statement = connection.prepare_cached(
        "SELECT (s.vector_rowid IS NULL OR s.write_cursor!=r.write_cursor
            OR r.rowid!=r.write_cursor OR s.window_number!=0 OR s.byte_start!=0
            OR typeof(s.body_length)!='integer' OR s.body_length<0
            OR (COALESCE(n.body,e.body) IS NOT NULL
              AND s.body_length!=length(CAST(COALESCE(n.body,e.body) AS BLOB)))
            OR (s.byte_end IS NOT NULL AND (typeof(s.byte_end)!='integer'
              OR s.byte_end<0 OR s.byte_end>s.body_length
              OR hex(substr(CAST(COALESCE(n.body,e.body) AS BLOB),s.byte_end+1,1)) BETWEEN '80' AND 'BF'))),
            ((n.write_cursor IS NOT NULL)+(e.write_cursor IS NOT NULL)=1 AND COALESCE(n.body,e.body) IS NOT NULL),
            s.body_length,s.byte_end
         FROM _fathomdb_vector_rows r
         LEFT JOIN _fathomdb_vector_spans s ON s.vector_rowid=r.rowid
         LEFT JOIN canonical_nodes n ON n.write_cursor=r.write_cursor
         LEFT JOIN canonical_edges e ON e.write_cursor=r.write_cursor
         UNION ALL
         SELECT 1,0,0,NULL FROM _fathomdb_vector_spans s
         WHERE NOT EXISTS(SELECT 1 FROM _fathomdb_vector_rows r
            WHERE r.rowid=s.vector_rowid AND r.write_cursor=s.write_cursor)"
    ).map_err(|_| EngineError::Storage)?;
    let mut rows = statement.query([]).map_err(|_| EngineError::Storage)?;
    let mut counts = StoredHeadCoverageCounts::default();
    let corrupt = projection_generation::corruption_error;
    let increment = |value: &mut u64| -> Result<(), EngineError> {
        *value = value.checked_add(1).filter(|v| *v <= i64::MAX as u64).ok_or_else(corrupt)?;
        Ok(())
    };
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        if row.get::<_, bool>(0).map_err(|_| EngineError::Storage)? {
            return Err(corrupt());
        }
        if !row.get::<_, bool>(1).map_err(|_| EngineError::Storage)? {
            increment(&mut counts.unattributed_heads)?;
            continue;
        }
        let length: u64 = row.get(2).map_err(|_| corrupt())?;
        let end: Option<u64> = row.get(3).map_err(|_| corrupt())?;
        increment(&mut counts.records)?;
        if let Some(end) = end {
            increment(&mut counts.known_records)?;
            if end < length {
                increment(&mut counts.truncated_known_records)?;
            }
            counts.known_body_bytes =
                counts.known_body_bytes.checked_add(u128::from(length)).ok_or_else(corrupt)?;
            counts.unrepresented_known_bytes = counts
                .unrepresented_known_bytes
                .checked_add(u128::from(length.checked_sub(end).ok_or_else(corrupt)?))
                .ok_or_else(corrupt)?;
        } else {
            increment(&mut counts.unknown_records)?;
        }
    }
    counts.truncated_known_share =
        (counts.known_records != 0).then_some(StoredHeadCoverageFraction {
            numerator: u128::from(counts.truncated_known_records),
            denominator: u128::from(counts.known_records),
        });
    counts.unrepresented_known_byte_share =
        (counts.known_body_bytes != 0).then_some(StoredHeadCoverageFraction {
            numerator: counts.unrepresented_known_bytes,
            denominator: counts.known_body_bytes,
        });
    Ok(counts)
}

pub(crate) const SCORED_EXTENT_SQL: &str = "
    SELECT 0,r.rowid,r.write_cursor,NULL,NULL,NULL,NULL
      FROM _fathomdb_vector_rows r WHERE r.rowid=?1 OR r.write_cursor=?1
    UNION ALL
    SELECT 1,s.vector_rowid,s.write_cursor,s.window_number,s.byte_start,s.body_length,s.byte_end
      FROM _fathomdb_vector_spans s WHERE s.vector_rowid=?1 OR s.write_cursor=?1
    UNION ALL
    SELECT 2,n.write_cursor,n.body IS NOT NULL,COALESCE(CAST(n.body AS BLOB)=CAST(?2 AS BLOB),0),NULL,NULL,NULL
      FROM canonical_nodes n WHERE n.write_cursor=?1
    UNION ALL
    SELECT 3,e.write_cursor,e.body IS NOT NULL,COALESCE(CAST(e.body AS BLOB)=CAST(?2 AS BLOB),0),NULL,NULL,NULL
      FROM canonical_edges e WHERE e.write_cursor=?1
    UNION ALL
    SELECT 4,rowid,NULL,NULL,NULL,NULL,NULL FROM vector_default WHERE rowid=?1";

pub(crate) fn scored_extent(
    connection: &Connection,
    key: crate::fusion::RecordKey,
    body: &str,
) -> Result<Option<u32>, EngineError> {
    use crate::fusion::RecordKey;
    let (cursor, expected_owner) = match key {
        RecordKey::CanonicalNode(cursor) => (cursor, 2_u8),
        RecordKey::CanonicalEdge(cursor) => (cursor, 3_u8),
        RecordKey::OwnerlessNodeFts(_) => return Ok(None),
    };
    let corrupt = projection_generation::corruption_error;
    let mut statement = crate::search::prepare_search_statement(connection, SCORED_EXTENT_SQL)
        .map_err(|_| EngineError::Storage)?;
    let mut rows =
        statement.query(rusqlite::params![cursor, body]).map_err(|_| EngineError::Storage)?;
    let mut mapping = false;
    let mut span = None;
    let mut owner = false;
    let mut physical = false;
    while let Some(row) = rows.next().map_err(|_| EngineError::Storage)? {
        let tag: u8 = row.get(0).map_err(|_| corrupt())?;
        let rowid: u64 = row.get(1).map_err(|_| corrupt())?;
        if rowid != cursor {
            return Err(corrupt());
        }
        match tag {
            0 => {
                if mapping || row.get::<_, u64>(2).map_err(|_| corrupt())? != cursor {
                    return Err(corrupt());
                }
                mapping = true;
            }
            1 => {
                if span.is_some()
                    || row.get::<_, u64>(2).map_err(|_| corrupt())? != cursor
                    || row.get::<_, u64>(3).map_err(|_| corrupt())? != 0
                    || row.get::<_, u64>(4).map_err(|_| corrupt())? != 0
                {
                    return Err(corrupt());
                }
                let length: u64 = row.get(5).map_err(|_| corrupt())?;
                let end: Option<u64> = row.get(6).map_err(|_| corrupt())?;
                if length != body.len() as u64 {
                    return Err(corrupt());
                }
                if let Some(end) = end {
                    if end > length
                        || !body.is_char_boundary(usize::try_from(end).map_err(|_| corrupt())?)
                    {
                        return Err(corrupt());
                    }
                }
                span = Some(end.map(u32::try_from).transpose().map_err(|_| corrupt())?);
            }
            2 | 3 => {
                if owner
                    || tag != expected_owner
                    || !row.get::<_, bool>(2).map_err(|_| corrupt())?
                    || !row.get::<_, bool>(3).map_err(|_| corrupt())?
                {
                    return Err(corrupt());
                }
                owner = true;
            }
            4 => {
                if physical {
                    return Err(corrupt());
                }
                physical = true;
            }
            _ => return Err(corrupt()),
        }
    }
    if !mapping || !owner || !physical {
        return Err(corrupt());
    }
    span.ok_or_else(corrupt)
}

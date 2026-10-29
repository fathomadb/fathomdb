//! Experiment-only alternatives to the phase-1 vector rerank
//! (`dev/experiments/ac013-vector-reads/`). Nothing here is active unless
//! `FATHOMDB_PERF_EXPERIMENTS` is set, and each knob is read once per process.
//!
//! - `FATHOMDB_PERF_VECTOR_RERANK=chunked`: run the bit-KNN alone, then read
//!   each candidate's f32 vector straight from the vec0 chunk blob, grouping
//!   candidates by chunk and reading in offset order through ONE blob handle
//!   per chunk, so SQLite's per-cursor overflow-page cache makes each chunk's
//!   chain walk once per search instead of once per candidate. Distances are
//!   computed by the same `vec_distance_l2` on the same bytes. No storage
//!   change.
//! - `FATHOMDB_PERF_VECTOR_RERANK=f32table`: rerank from the rowid-keyed
//!   `_fathomdb_vector_f32` table (one ~3 KB row per vector, stored in its
//!   own leaf cell), falling back to the vec0 join when the table is absent
//!   or a candidate has no row there.
//! - `FATHOMDB_PERF_VECTOR_F32_TABLE=1`: create `_fathomdb_vector_f32` with
//!   the vector partition and write each projected f32 vector to it next to
//!   the vec0 insert. (Prototype: deletes, erasure and rebuild paths do not
//!   maintain it; the rerank reads it only for vec0-surfaced candidates.)
//! - `FATHOMDB_PERF_VEC0_CHUNK_SIZE=N`: create new vec0 partitions with
//!   `chunk_size=N` (vec0 default 1,024; must be a positive multiple of 8).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use rusqlite::{Connection, MAIN_DB};

use crate::filter::{build_vector_phase1_sql, vector_candidates_sql};

pub(crate) const F32_TABLE: &str = "_fathomdb_vector_f32";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RerankMode {
    Join,
    Chunked,
    F32Table,
}

fn knob(name: &str) -> Option<String> {
    std::env::var_os("FATHOMDB_PERF_EXPERIMENTS")?;
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

pub(crate) fn rerank_mode() -> RerankMode {
    static MODE: OnceLock<RerankMode> = OnceLock::new();
    *MODE.get_or_init(|| match knob("FATHOMDB_PERF_VECTOR_RERANK").as_deref() {
        Some("chunked") => RerankMode::Chunked,
        Some("f32table") => RerankMode::F32Table,
        Some(other) => {
            eprintln!("perf-experiment: unknown FATHOMDB_PERF_VECTOR_RERANK={other}; using join");
            RerankMode::Join
        }
        None => RerankMode::Join,
    })
}

pub(crate) fn f32_table_writes_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| knob("FATHOMDB_PERF_VECTOR_F32_TABLE").is_some())
}

/// The `chunk_size=N` vec0 table option for new partitions, if requested.
pub(crate) fn vec0_chunk_size_option() -> Option<u32> {
    static SIZE: OnceLock<Option<u32>> = OnceLock::new();
    *SIZE.get_or_init(|| {
        let raw = knob("FATHOMDB_PERF_VEC0_CHUNK_SIZE")?;
        match raw.parse::<u32>() {
            Ok(n) if n > 0 && n % 8 == 0 => Some(n),
            _ => {
                eprintln!("perf-experiment: bad FATHOMDB_PERF_VEC0_CHUNK_SIZE={raw}; ignored");
                None
            }
        }
    })
}

pub(crate) fn ensure_f32_table(connection: &Connection) -> rusqlite::Result<()> {
    if !f32_table_writes_enabled() {
        return Ok(());
    }
    connection.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {F32_TABLE}(rowid INTEGER PRIMARY KEY, embedding BLOB NOT NULL)"
    ))
}

pub(crate) fn write_f32_row(
    connection: &Connection,
    rowid: i64,
    embedding: &[u8],
) -> rusqlite::Result<()> {
    if !f32_table_writes_enabled() {
        return Ok(());
    }
    connection
        .prepare_cached(&format!(
            "INSERT OR REPLACE INTO {F32_TABLE}(rowid, embedding) VALUES(?1, ?2)"
        ))?
        .execute(rusqlite::params![rowid, embedding])?;
    Ok(())
}

fn run_join(
    connection: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<Vec<(i64, f64)>> {
    let mut statement = connection.prepare_cached(sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
    })?;
    rows.collect()
}

/// Phase 1 under the experiment rerank modes. `params` is the same list the
/// shipped statement binds (`?1` sign-quant query, `?2` f32 query, filters
/// from `?3`). Returns `(rowid, l2)` nearest first, at most `final_limit`.
pub(crate) fn phase1(
    mode: RerankMode,
    connection: &Connection,
    filter: Option<&crate::SearchFilter>,
    final_limit: usize,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<Vec<(i64, f64)>> {
    match mode {
        RerankMode::Join => {
            run_join(connection, &build_vector_phase1_sql(filter, final_limit), params)
        }
        RerankMode::F32Table => phase1_f32_table(connection, filter, final_limit, params),
        RerankMode::Chunked => phase1_chunked(connection, filter, final_limit, params),
    }
}

fn candidates(
    connection: &Connection,
    filter: Option<&crate::SearchFilter>,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<Vec<i64>> {
    let mut statement = connection.prepare_cached(&vector_candidates_sql(filter))?;
    // Without filter terms the candidate SELECT has no `?2`; with them `?2`
    // is an unused slot below `?3`.
    let bound = if params.len() > 2 { params } else { &params[..1] };
    let rows = statement
        .query_map(rusqlite::params_from_iter(bound.iter()), |row| row.get::<_, i64>(0))?;
    rows.collect()
}

fn sort_and_cut(mut scored: Vec<(i64, f64)>, final_limit: usize) -> Vec<(i64, f64)> {
    // Stable: ties keep the KNN order, as the shipped sorter does.
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(final_limit);
    scored
}

fn phase1_f32_table(
    connection: &Connection,
    filter: Option<&crate::SearchFilter>,
    final_limit: usize,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<Vec<(i64, f64)>> {
    let present: bool = connection
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        )?
        .query_row([F32_TABLE], |row| row.get(0))?;
    if !present {
        return run_join(connection, &build_vector_phase1_sql(filter, final_limit), params);
    }
    let ids = candidates(connection, filter, params)?;
    let query: Vec<u8> = connection
        .prepare_cached("SELECT vec_f32(?1)")?
        .query_row([&params[1]], |row| row.get(0))?;
    let mut lookup = connection.prepare_cached(&format!(
        "SELECT vec_distance_l2(embedding, ?2) FROM {F32_TABLE} WHERE rowid = ?1"
    ))?;
    let mut scored = Vec::with_capacity(ids.len());
    for rowid in ids {
        match lookup.query_row(rusqlite::params![rowid, query], |row| row.get::<_, f64>(0)) {
            Ok(l2) => scored.push((rowid, l2)),
            // A candidate the table does not cover yet: the shipped path.
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return run_join(connection, &build_vector_phase1_sql(filter, final_limit), params);
            }
            Err(err) => return Err(err),
        }
    }
    Ok(sort_and_cut(scored, final_limit))
}

fn phase1_chunked(
    connection: &Connection,
    filter: Option<&crate::SearchFilter>,
    final_limit: usize,
    params: &[rusqlite::types::Value],
) -> rusqlite::Result<Vec<(i64, f64)>> {
    let ids = candidates(connection, filter, params)?;
    let query: Vec<u8> = connection
        .prepare_cached("SELECT vec_f32(?1)")?
        .query_row([&params[1]], |row| row.get(0))?;
    let width = query.len();
    // chunk_id -> [(chunk_offset, candidate index)]
    let mut by_chunk: BTreeMap<i64, Vec<(i64, usize)>> = BTreeMap::new();
    {
        let mut position = connection.prepare_cached(
            "SELECT chunk_id, chunk_offset FROM vector_default_rowids WHERE rowid = ?1",
        )?;
        for (index, rowid) in ids.iter().enumerate() {
            let (chunk, offset): (i64, i64) =
                position.query_row([rowid], |row| Ok((row.get(0)?, row.get(1)?)))?;
            by_chunk.entry(chunk).or_default().push((offset, index));
        }
    }
    let mut vectors: Vec<Vec<u8>> = vec![Vec::new(); ids.len()];
    let mut blob: Option<rusqlite::blob::Blob<'_>> = None;
    for (chunk, mut members) in by_chunk {
        members.sort_unstable();
        let handle = match blob.as_mut() {
            Some(handle) => {
                handle.reopen(chunk)?;
                handle
            }
            None => blob.insert(connection.blob_open(
                MAIN_DB,
                c"vector_default_vector_chunks00",
                c"vectors",
                chunk,
                true,
            )?),
        };
        for (offset, index) in members {
            let mut buf = vec![0u8; width];
            handle.read_at_exact(&mut buf, offset as usize * width)?;
            vectors[index] = buf;
        }
    }
    drop(blob);
    let mut distance = connection.prepare_cached("SELECT vec_distance_l2(?1, ?2)")?;
    let mut scored = Vec::with_capacity(ids.len());
    for (rowid, vector) in ids.iter().zip(&vectors) {
        let l2: f64 = distance.query_row(rusqlite::params![vector, query], |row| row.get(0))?;
        scored.push((*rowid, l2));
    }
    Ok(sort_and_cut(scored, final_limit))
}

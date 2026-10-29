//! Copy a seeded FathomDB store into the physical layout of a vector-read
//! candidate (`dev/experiments/ac013-vector-reads/`), without re-embedding.
//!
//! Experiment tooling only: it rewrites `vector_default` with plain SQL
//! (stage, drop, recreate, refill), which is exactly the data migration
//! `AGENTS.md` § 10 forbids in a product release. It exists so the MCP arm,
//! whose store takes about 30 minutes of CPU embedding to seed, can measure a
//! storage candidate on the same vectors.
//!
//! ```text
//! cargo run --release -p fathomdb-engine --example ac013_relayout -- \
//!   <src.sqlite> <dst.sqlite> [--chunk-size N] [--f32-table]
//! ```
//!
//! - `--chunk-size N`: recreate `vector_default` with the vec0 option
//!   `chunk_size=N` (the engine reads the table's own SQL at open, so the
//!   option persists).
//! - `--f32-table`: create and fill `_fathomdb_vector_f32(rowid INTEGER
//!   PRIMARY KEY, embedding BLOB)` from `vector_default.embedding`, as the
//!   `FATHOMDB_PERF_VECTOR_F32_TABLE` write path would have.
//!
//! The source must be closed (no live WAL). The destination is overwritten.
//! Prints one JSON line: row counts before and after, the vec0 SQL, and the
//! file size after `VACUUM`.

use std::path::Path;

use rusqlite::Connection;

fn register_vec() {
    // SAFETY: the sqlite-vec entry point has the auto-extension signature.
    unsafe {
        let entrypoint: unsafe extern "C" fn(
            *mut rusqlite::ffi::sqlite3,
            *mut *mut std::os::raw::c_char,
            *const rusqlite::ffi::sqlite3_api_routines,
        ) -> std::os::raw::c_int = std::mem::transmute(sqlite_vec::sqlite3_vec_init as *const ());
        rusqlite::ffi::sqlite3_auto_extension(Some(entrypoint));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: ac013_relayout <src> <dst> [--chunk-size N] [--f32-table]");
        std::process::exit(2);
    }
    let (src, dst) = (Path::new(&args[1]), Path::new(&args[2]));
    let mut chunk_size: Option<u32> = None;
    let mut f32_table = false;
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--chunk-size" => {
                chunk_size = Some(args[i + 1].parse().expect("chunk size"));
                i += 1;
            }
            "--f32-table" => f32_table = true,
            other => panic!("unknown argument {other}"),
        }
        i += 1;
    }
    for suffix in ["", "-wal", "-shm"] {
        let mut p = dst.as_os_str().to_owned();
        p.push(suffix);
        let _ = std::fs::remove_file(&p);
    }
    std::fs::copy(src, dst).expect("copy store");
    register_vec();
    let mut conn = Connection::open(dst).expect("open copy");
    let rows: i64 =
        conn.query_row("SELECT count(*) FROM vector_default", [], |r| r.get(0)).unwrap();
    let mut sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='vector_default'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!sql.contains("attr_"), "stores with filterable attribute columns are not handled");
    if let Some(n) = chunk_size {
        assert!(!sql.contains("chunk_size"), "source already sets chunk_size");
        let close = sql.rfind(')').expect("vec0 sql");
        sql = format!("{},chunk_size={n})", &sql[..close]);
        let tx = conn.transaction().unwrap();
        tx.execute_batch(
            "CREATE TABLE _ac013_stage AS SELECT rowid AS rid, embedding, embedding_bin, \
             source_type, kind, created_at, status FROM vector_default;
             DROP TABLE vector_default;",
        )
        .unwrap();
        tx.execute_batch(&sql).unwrap();
        // No ORDER BY: a sorter drops the bit subtype vec_bit() sets.
        tx.execute_batch(
            "INSERT INTO vector_default(rowid, embedding, embedding_bin, source_type, kind, \
             created_at, status) SELECT rid, embedding, vec_bit(embedding_bin), source_type, \
             kind, created_at, status FROM _ac013_stage;
             DROP TABLE _ac013_stage;",
        )
        .unwrap();
        tx.commit().unwrap();
    }
    if f32_table {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _fathomdb_vector_f32(rowid INTEGER PRIMARY KEY, \
             embedding BLOB NOT NULL);
             INSERT OR REPLACE INTO _fathomdb_vector_f32(rowid, embedding) \
             SELECT rowid, embedding FROM vector_default ORDER BY rowid;",
        )
        .unwrap();
    }
    conn.execute_batch("VACUUM").unwrap();
    let after: i64 =
        conn.query_row("SELECT count(*) FROM vector_default", [], |r| r.get(0)).unwrap();
    let chunks: i64 =
        conn.query_row("SELECT count(*) FROM vector_default_chunks", [], |r| r.get(0)).unwrap();
    let has_f32: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='_fathomdb_vector_f32')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let f32_rows: i64 = if has_f32 {
        conn.query_row("SELECT count(*) FROM _fathomdb_vector_f32", [], |r| r.get(0)).unwrap()
    } else {
        -1
    };
    let final_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='vector_default'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    drop(conn);
    let bytes = std::fs::metadata(dst).map(|m| m.len()).unwrap_or(0);
    println!(
        "{}",
        serde_json::json!({
            "command": "relayout", "src": args[1], "dst": args[2],
            "rows_before": rows, "rows_after": after, "chunks": chunks,
            "f32_rows": f32_rows, "vec0_sql": final_sql, "file_bytes": bytes,
        })
    );
    assert_eq!(rows, after, "row count changed");
}

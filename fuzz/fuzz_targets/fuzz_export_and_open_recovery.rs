#![no_main]

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use arbitrary::Unstructured;
use fathomdb_engine::{
    decode_logical_export_v1, restore_logical_archive_v1, EmbedderChoice, Engine,
    LOGICAL_EXPORT_FORMAT_V1,
};
use libfuzzer_sys::fuzz_target;
use rusqlite::Connection;
use tempfile::TempDir;

struct RecoveryFixture {
    _dir: TempDir,
    template_bytes: Vec<u8>,
    restore_db_path: PathBuf,
    open_db_path: PathBuf,
}

fn fixture() -> &'static Mutex<RecoveryFixture> {
    static FIXTURE: OnceLock<Mutex<RecoveryFixture>> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = TempDir::new().expect("tempdir for fuzz_export_and_open_recovery");
        let template_path = dir.path().join("template_schema39.sqlite");
        let engine = Engine::open_with_choice(&template_path, EmbedderChoice::None)
            .expect("bootstrap template schema 39 db")
            .engine;
        engine.close().expect("close template engine");
        // Checkpoint to flush WAL cleanly into the main file so template_bytes is self-contained.
        {
            let conn = Connection::open(&template_path).expect("open template for checkpoint");
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
        let template_bytes = fs::read(&template_path).expect("read template db bytes");
        let restore_db_path = dir.path().join("restore_target.sqlite");
        let open_db_path = dir.path().join("open_target.sqlite");
        Mutex::new(RecoveryFixture { _dir: dir, template_bytes, restore_db_path, open_db_path })
    })
}

fn cleanup_sidecars(db_path: &std::path::Path) {
    let base = db_path.as_os_str().to_string_lossy();
    for suffix in ["", "-wal", "-shm", "-journal", ".lock"] {
        let p = PathBuf::from(format!("{base}{suffix}"));
        let _ = fs::remove_file(p);
    }
}

fn arb_str(u: &mut Unstructured<'_>, max_len: usize) -> String {
    let raw: String = u.arbitrary().unwrap_or_default();
    raw.chars().take(max_len).collect()
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=3).unwrap_or(0);
    let guard = fixture().lock().unwrap_or_else(|p| p.into_inner());

    match mode {
        0 => {
            // Raw or truncated fathomdb.logical-export/v1 payload decode + restore attempt.
            let payload = &data[1..];
            if let Ok(archive) = decode_logical_export_v1(payload) {
                cleanup_sidecars(&guard.restore_db_path);
                if fs::write(&guard.restore_db_path, &guard.template_bytes).is_ok() {
                    if let Ok(mut conn) = Connection::open(&guard.restore_db_path) {
                        let _ = restore_logical_archive_v1(&mut conn, &archive);
                    }
                }
                cleanup_sidecars(&guard.restore_db_path);
            }
        }
        1 => {
            // Structured semi-valid LogicalExportV1 archive with adversarial fields
            // (invalid properties JSON, bad row_kind/state, constraint conflicts, bad vectors)
            // restored against a fresh Schema 39 database.
            let include_vectors = u.arbitrary().unwrap_or(false);
            let dim = u.int_in_range(1..=4).unwrap_or(4);
            let archive_json = serde_json::json!({
                "format_version": LOGICAL_EXPORT_FORMAT_V1,
                "schema_version": 39,
                "include_audit": false,
                "include_vectors": include_vectors,
                "physical_erasure_policy": "autonomous",
                "dependency_generation": u.int_in_range(0..=10).unwrap_or(0),
                "authority": {
                    "canonical_nodes": [{
                        "write_cursor": u.int_in_range(1..=100).unwrap_or(1),
                        "kind": arb_str(&mut u, 24),
                        "body": arb_str(&mut u, 64),
                        "source_id": Some("src-1"),
                        "logical_id": if u.arbitrary().unwrap_or(true) { Some(arb_str(&mut u, 24)) } else { None },
                        "superseded_at": Option::<i64>::None,
                        "row_kind": match u.int_in_range(0..=3).unwrap_or(0) {
                            0 => "leaf".to_string(),
                            1 => "coverage".to_string(),
                            2 => "graph".to_string(),
                            _ => arb_str(&mut u, 16),
                        },
                        "importance": Option::<f64>::None,
                        "state": match u.int_in_range(0..=3).unwrap_or(0) {
                            0 => "active".to_string(),
                            1 => "pending".to_string(),
                            2 => "deleted".to_string(),
                            _ => arb_str(&mut u, 16),
                        },
                        "reason": Option::<String>::None,
                        "valid_from": if u.arbitrary().unwrap_or(false) { Some(u.arbitrary().unwrap_or(0)) } else { None },
                        "valid_until": if u.arbitrary().unwrap_or(false) { Some(u.arbitrary().unwrap_or(0)) } else { None },
                        "properties": if u.arbitrary().unwrap_or(false) { Some(arb_str(&mut u, 64)) } else { None },
                        "scope": if u.arbitrary().unwrap_or(true) { "".to_string() } else { arb_str(&mut u, 24) },
                    }],
                    "canonical_edges": [],
                    "operational_collections": [],
                    "operational_mutations": [],
                    "operational_state": [],
                    "artifact_revisions": [],
                    "source_versions": [],
                    "source_links": [],
                    "source_dependencies": [],
                    "projection_registry": [],
                    "vector_kinds": [],
                    "commit_timestamps": []
                },
                "audit": null,
                "vectors": if include_vectors {
                    Some(serde_json::json!({
                        "embedder": {
                            "profile": "default",
                            "name": "fuzz-embedder",
                            "revision": "rev-1",
                            "dimension": dim
                        },
                        "rows": [{
                            "rowid": 1,
                            "write_cursor": 1,
                            "kind": "note",
                            "created_at": 1700000000,
                            "window_number": 0,
                            "byte_start": 0,
                            "byte_end": 5,
                            "body_length": 10,
                            "embedding": vec![0.1_f32; dim as usize]
                        }]
                    }))
                } else {
                    None
                }
            })
            .to_string();
            if let Ok(archive) = decode_logical_export_v1(archive_json.as_bytes()) {
                cleanup_sidecars(&guard.restore_db_path);
                if fs::write(&guard.restore_db_path, &guard.template_bytes).is_ok() {
                    if let Ok(mut conn) = Connection::open(&guard.restore_db_path) {
                        let _ = restore_logical_archive_v1(&mut conn, &archive);
                    }
                }
                cleanup_sidecars(&guard.restore_db_path);
            }
        }
        2 => {
            // Malformed / truncated raw SQLite file or corrupted Schema 39 header/WAL bytes
            // passed to Engine::open_with_choice. Must return Ok (cleanly closed) or typed Err,
            // never panic or trigger sanitizer violations.
            cleanup_sidecars(&guard.open_db_path);
            let submode = u.int_in_range(0..=2).unwrap_or(0);
            match submode {
                0 => {
                    // Pure arbitrary / truncated bytes written directly as the DB file.
                    let rest = &data[1..data.len().min(4096)];
                    let _ = fs::write(&guard.open_db_path, rest);
                }
                1 => {
                    // Truncate valid Schema 39 template at an arbitrary offset.
                    let cut = u.int_in_range(0..=guard.template_bytes.len()).unwrap_or(0);
                    let _ = fs::write(&guard.open_db_path, &guard.template_bytes[..cut]);
                }
                _ => {
                    // Flip up to 4 bytes inside a copy of the Schema 39 template, optionally
                    // attaching a fuzzed -wal sidecar.
                    let mut buf = guard.template_bytes.clone();
                    let flips = u.int_in_range(1..=4).unwrap_or(1);
                    for _ in 0..flips {
                        if !buf.is_empty() {
                            let idx = u.int_in_range(0..=(buf.len() - 1)).unwrap_or(0);
                            let xor_byte: u8 = u.arbitrary().unwrap_or(0xFF);
                            buf[idx] ^= xor_byte;
                        }
                    }
                    let _ = fs::write(&guard.open_db_path, &buf);
                    if u.arbitrary().unwrap_or(false) {
                        let wal_path =
                            PathBuf::from(format!("{}-wal", guard.open_db_path.to_string_lossy()));
                        let wal_bytes: Vec<u8> = u.arbitrary().unwrap_or_default();
                        let _ = fs::write(wal_path, &wal_bytes[..wal_bytes.len().min(2048)]);
                    }
                }
            }
            if let Ok(opened) = Engine::open_with_choice(&guard.open_db_path, EmbedderChoice::None)
            {
                let _ = opened.engine.close();
            }
            cleanup_sidecars(&guard.open_db_path);
        }
        _ => {
            // Targeted recovery metadata corruption inside a valid Schema 39 SQLite container
            // (_fathomdb_open_state, _fathomdb_read_visibility_state, user_version, triggers).
            cleanup_sidecars(&guard.open_db_path);
            if fs::write(&guard.open_db_path, &guard.template_bytes).is_ok() {
                if let Ok(conn) = Connection::open(&guard.open_db_path) {
                    let val = arb_str(&mut u, 96);
                    match u.int_in_range(0..=7).unwrap_or(0) {
                        0 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_open_state SET value = ?1 WHERE key = '_fathomdb_database_id'",
                                [&val],
                            );
                        }
                        1 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_open_state SET value = ?1 WHERE key = '_fathomdb_read_context_key'",
                                [&val],
                            );
                        }
                        2 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_open_state SET value = ?1 WHERE key = '_fathomdb_physical_erasure_policy'",
                                [&val],
                            );
                        }
                        3 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_open_state SET value = ?1 WHERE key = '_fathomdb_dependency_generation'",
                                [&val],
                            );
                        }
                        4 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_read_visibility_state SET coalescing = 1 WHERE singleton = 1",
                                [],
                            );
                        }
                        5 => {
                            let _ = conn.execute(
                                "UPDATE _fathomdb_read_visibility_state SET state_nonce = ?1, global_struct_nonce = ?1 WHERE singleton = 1",
                                [&val],
                            );
                        }
                        6 => {
                            let ver: u32 = match u.int_in_range(0..=3).unwrap_or(0) {
                                0 => 0,
                                1 => 37,
                                2 => 40,
                                _ => 999,
                            };
                            let _ = conn.execute_batch(&format!("PRAGMA user_version = {ver};"));
                        }
                        _ => {
                            let _ = conn.execute_batch(
                                "DROP TABLE IF EXISTS _fathomdb_scope_visibility_watermarks;",
                            );
                        }
                    }
                }
                if let Ok(opened) =
                    Engine::open_with_choice(&guard.open_db_path, EmbedderChoice::None)
                {
                    let _ = opened.engine.close();
                }
            }
            cleanup_sidecars(&guard.open_db_path);
        }
    }
});

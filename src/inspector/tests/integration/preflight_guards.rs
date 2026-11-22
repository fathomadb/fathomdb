use crate::{admission::preflight, fixtures::database};
use std::{fs, path::Path};
fn clone_store(base: &Path, dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(format!("{name}.sqlite"));
    fs::copy(base, &path).unwrap();
    fs::write(format!("{}.lock", path.display()), b"existing lock sentinel").unwrap();
    path
}
#[test]
fn compiled_profile_vector_shape_markers_and_mean_recovery_refuse_before_open() {
    let dir = tempfile::tempdir().unwrap();
    let base = database(dir.path());
    let mean = clone_store(&base, dir.path(), "mean");
    let c = rusqlite::Connection::open(&mean).unwrap();
    let mut vector = vec![0_u8; 384 * 4];
    vector[..4].copy_from_slice(&1_f32.to_le_bytes());
    let binary = vec![0_u8; 384 / 8];
    for id in 1..=fathomdb_engine::MEAN_VEC_PIN_THRESHOLD {
        c.execute("INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status,lifecycle,valid_from,valid_until) VALUES(?1,?2,vec_bit(?3),'canonical','document',0,'',0,-9223372036854775808,9223372036854775807)",rusqlite::params![id,vector,binary]).unwrap();
    }
    drop(c);
    let before = fs::read(&mean).unwrap();
    assert_eq!(preflight(&mean).unwrap_err(), "mean_recovery_required");
    assert_eq!(fs::read(&mean).unwrap(), before);
    let profile = clone_store(&base, dir.path(), "profile");
    let c = rusqlite::Connection::open(&profile).unwrap();
    c.execute(
        "UPDATE _fathomdb_embedder_profiles SET name='unsupported-profile' WHERE profile='default'",
        [],
    )
    .unwrap();
    drop(c);
    let before = fs::read(&profile).unwrap();
    assert_eq!(preflight(&profile).unwrap_err(), "incompatible_profile");
    assert_eq!(fs::read(&profile).unwrap(), before);
    let shape = clone_store(&base, dir.path(), "shape");
    let c = rusqlite::Connection::open(&shape).unwrap();
    c.execute_batch("DROP TABLE vector_default; CREATE TABLE vector_default(embedding BLOB,embedding_bin BLOB,source_type TEXT,kind TEXT,created_at INTEGER,status TEXT)").unwrap();
    drop(c);
    let before = fs::read(&shape).unwrap();
    assert_eq!(preflight(&shape).unwrap_err(), "malformed_vector_shape");
    assert_eq!(fs::read(&shape).unwrap(), before);
    for key in ["search_index_tokenizer_reproject_complete", "tc33_edge_vector_prune_complete"] {
        let path = clone_store(&base, dir.path(), key);
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute("DELETE FROM _fathomdb_open_state WHERE key=?1", [key]).unwrap();
        drop(c);
        let before = fs::read(&path).unwrap();
        assert_eq!(preflight(&path).unwrap_err(), "missing_marker");
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[test]
fn incremental_text_support_refuses_other_encoding_headers_before_sqlite() {
    use crate::admission::preflight;
    let dir = tempfile::tempdir().unwrap();
    let path = crate::fixtures::database(dir.path());
    let original = std::fs::read(&path).unwrap();
    assert!(preflight(&path).is_ok());
    assert_eq!(u32::from_be_bytes(original[56..60].try_into().unwrap()), 1);
    for encoding in [2_u32, 3] {
        let target = dir.path().join(format!("encoding-{encoding}.sqlite"));
        let mut bytes = original.clone();
        bytes[56..60].copy_from_slice(&encoding.to_be_bytes());
        std::fs::write(&target, &bytes).unwrap();
        std::fs::write(format!("{}.lock", target.display()), b"lock sentinel").unwrap();
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!std::path::Path::new(&format!("{}{suffix}", target.display())).exists());
        }
        assert_eq!(preflight(&target).unwrap_err(), "unsupported_encoding");
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(!std::path::Path::new(&format!("{}{suffix}", target.display())).exists());
        }
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert_eq!(std::fs::read(format!("{}.lock", target.display())).unwrap(), b"lock sentinel");
    }
}

#[test]
fn dirty_coalesced_visibility_refuses_before_engine_startup_without_effects() {
    let dir = tempfile::tempdir().unwrap();
    let path = crate::fixtures::database(dir.path());
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1 WHERE singleton=1", [])
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert_eq!(preflight(&path).unwrap_err(), "invalid_visibility");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

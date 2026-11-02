use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(id).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn scalar(db: &Connection, sql: &str, marker: &str) -> i64 {
    db.query_row(sql, [marker], |r| r.get(0)).unwrap()
}

#[test]
#[ignore = "shipping residual-term diagnostic; intentionally asserts complete physical erasure"]
fn ordinary_engine_erasure_removes_live_fts_shadow_term() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shipping.sqlite");
    let marker = "qzxvplmnbvcxzasdfghjklqwertx";
    let engine = Engine::open(&path).unwrap().engine;
    engine.write(&[node("target", marker), node("survivor", "surviving distinct text")]).unwrap();
    engine.drain(10_000).unwrap();
    let read = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let shadow = "SELECT COUNT(*) FROM search_index_data WHERE instr(block,CAST(?1 AS BLOB))>0";
    let before = scalar(&read, shadow, marker);
    assert!(before > 0, "diagnostic must witness the literal token before erasure");
    engine.erase_source("target").unwrap();
    let canonical =
        scalar(&read, "SELECT COUNT(*) FROM canonical_nodes WHERE source_id=?1", "target");
    let content = scalar(&read, "SELECT COUNT(*) FROM search_index WHERE body=?1", marker);
    let matches =
        scalar(&read, "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1", marker);
    let residual = scalar(&read, shadow, marker);
    let survivor =
        scalar(&read, "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1", "surviving");
    let wal_bytes =
        std::fs::metadata(path.with_extension("sqlite-wal")).map(|m| m.len()).unwrap_or(0);
    println!(
        "{}",
        serde_json::json!({"erase_return":"success","shadow_before":before,"shadow_after":residual,"canonical_after":canonical,"content_after":content,"match_after":matches,"survivor_after":survivor,"wal_bytes":wal_bytes})
    );
    assert_eq!((canonical, content, matches), (0, 0, 0));
    assert_eq!(survivor, 1);
    assert_eq!(wal_bytes, 0);
    assert_eq!(
        residual, 0,
        "successful erasure must remove target token from live FTS shadow blocks"
    );
    engine.close().unwrap();
}

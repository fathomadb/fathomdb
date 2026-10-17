//! Read-only (design §9 order 8b; MR-060.1): after a session that calls
//! every listed tool, the canonical write cursor and a digest of canonical
//! and op-store rows are unchanged.

mod common;

use std::path::Path;

use common::{edge, node, refs, Fixture, Harness};
use fathomdb_sdk::{Engine, OpenOptions, PreparedWrite};
use rusqlite::types::ValueRef;
use serde_json::json;
use sha2::{Digest, Sha256};

/// The collection must be registered in its own batch before op-store rows.
fn write_fixture(path: &Path) {
    let engine = Engine::open(path, OpenOptions::default()).unwrap();
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "events".into(),
            kind: "append_only_log".into(),
            schema_json: "{}".into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    engine
        .write(&[
            PreparedWrite::OpStore {
                collection: "events".into(),
                record_key: "k1".into(),
                schema_id: None,
                body: r#"{"n":1}"#.into(),
            },
            node("note:1", "note", "alpha river crossing"),
            node("note:2", "note", "beta river crossing"),
            edge("link", "note:1", "note:2"),
        ])
        .unwrap();
    engine.close().unwrap();
}

/// (max canonical write cursor, digest of canonical and op-store rows).
fn snapshot(path: &Path) -> (i64, String) {
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mut tables: Vec<String> = connection
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND (name LIKE 'canonical\\_%' ESCAPE '\\' OR name LIKE 'operational\\_%' ESCAPE '\\') \
             ORDER BY name",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    tables.sort();
    assert!(tables.iter().any(|table| table == "canonical_nodes"), "{tables:?}");
    assert!(tables.iter().any(|table| table.starts_with("operational_")), "{tables:?}");
    let mut hasher = Sha256::new();
    for table in &tables {
        let mut statement = connection.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
        let columns = statement.column_count();
        let mut rows: Vec<String> = statement
            .query_map([], |row| {
                let mut rendered = String::new();
                for index in 0..columns {
                    let value = match row.get_ref(index)? {
                        ValueRef::Null => "null".to_string(),
                        ValueRef::Integer(value) => value.to_string(),
                        ValueRef::Real(value) => value.to_string(),
                        ValueRef::Text(value) => format!("t:{}", String::from_utf8_lossy(value)),
                        ValueRef::Blob(value) => format!("b:{value:?}"),
                    };
                    rendered.push_str(&value);
                    rendered.push('\u{1f}');
                }
                Ok(rendered)
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        rows.sort();
        hasher.update(table.as_bytes());
        for row in rows {
            hasher.update(row.as_bytes());
            hasher.update(b"\n");
        }
    }
    let cursor: i64 = connection
        .query_row(
            "SELECT MAX(c) FROM (SELECT MAX(write_cursor) AS c FROM canonical_nodes \
             UNION ALL SELECT MAX(write_cursor) FROM canonical_edges)",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .unwrap()
        .unwrap_or(0);
    let digest = hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect();
    (cursor, digest)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_calling_every_tool_changes_no_canonical_or_op_store_row() {
    let fixture = Fixture::new();
    let path = fixture.path("a.sqlite");
    write_fixture(&path);
    let before = snapshot(&path);
    assert!(before.0 > 0);

    let harness = Harness::start(fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "telemetry_dir": "telemetry"
    })))
    .await;
    harness.ok("list_stores", json!({})).await;
    harness.ok("use_store", json!({"store": "a"})).await;
    harness.ok("orient", json!({})).await;
    let text = harness.ok("search", json!({"query": "river", "limit": 1})).await;
    let hybrid =
        harness.ok("search", json!({"query": "river", "mode": "hybrid", "explain": true})).await;
    harness.ok("search", json!({"continuation": text["continuation"]})).await;
    harness.ok("fetch", json!({"refs": refs(&text), "include_inactive": true})).await;
    harness.ok("expand", json!({"refs": ["l:note:1"], "direction": "both"})).await;
    let feedback = harness
        .ok("feedback", json!({"response_id": hybrid["response_id"], "used": [refs(&hybrid)[0]]}))
        .await;
    assert_eq!(feedback["status"], "recorded", "MR-024.4: feedback works in read-only mode");
    harness.shutdown().await;

    assert_eq!(snapshot(&path), before, "MR-060.1");
}

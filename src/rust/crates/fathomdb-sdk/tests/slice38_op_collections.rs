//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 4 RED/GREEN SDK parity tests for
//! `fathomdb_sdk::read::{operational_collections, operational_collection}`.

use fathomdb_sdk::{
    admin, read, Engine, ErrorKind, OpenOptions, OperationalCollectionInfo, PreparedWrite,
};
use tempfile::TempDir;

#[test]
fn slice38_sdk_read_operational_collections_parity_and_guards() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("sdk-op-collections.sqlite");
    let engine = Engine::open(&path, OpenOptions::default()).expect("open sdk engine");

    let initial = read::operational_collections(&engine).expect("initial list");
    assert_eq!(
        initial.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["projection_failures"]
    );
    assert_eq!(read::operational_collection(&engine, "missing").expect("missing"), None);

    admin::configure(&engine, "zeta", r#"{"type":"object"}"#).expect("register zeta");
    engine
        .write(&[PreparedWrite::AdminSchema {
            name: "alpha".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: r#"{"max_rows":10}"#.into(),
        }])
        .expect("register alpha");

    let list = read::operational_collections(&engine).expect("list collections");
    assert_eq!(
        list,
        vec![
            OperationalCollectionInfo {
                name: "alpha".into(),
                kind: "append_only_log".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: r#"{"max_rows":10}"#.into(),
                format_version: 1,
                created_at: 0,
            },
            OperationalCollectionInfo {
                name: "projection_failures".into(),
                kind: "append_only_log".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: r#"{}"#.into(),
                format_version: 1,
                created_at: 0,
            },
            OperationalCollectionInfo {
                name: "zeta".into(),
                kind: "latest_state".into(),
                schema_json: r#"{"type":"object"}"#.into(),
                retention_json: r#"{}"#.into(),
                format_version: 1,
                created_at: 0,
            },
        ]
    );

    assert_eq!(
        read::operational_collection(&engine, "alpha").expect("get alpha"),
        Some(list[0].clone())
    );
    assert_eq!(
        read::operational_collection(&engine, "zeta").expect("get zeta"),
        Some(list[2].clone())
    );
    assert_eq!(read::operational_collection(&engine, "absent").expect("get absent"), None);

    // Guard rejections on empty, whitespace, embedded NUL, and control characters.
    assert_eq!(
        read::operational_collection(&engine, "").unwrap_err().kind(),
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        read::operational_collection(&engine, "   ").unwrap_err().kind(),
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        read::operational_collection(&engine, "bad\0name").unwrap_err().kind(),
        ErrorKind::WriteValidation
    );
    assert_eq!(
        read::operational_collection(&engine, "bad\nname").unwrap_err().kind(),
        ErrorKind::InvalidArgument
    );
    assert_eq!(
        read::operational_collection(&engine, "bad\x1ename").unwrap_err().kind(),
        ErrorKind::InvalidArgument
    );

    // Kind conflict surfaces as WriteValidation; malformed schema surfaces as SchemaValidation.
    let conflict = engine
        .write(&[PreparedWrite::AdminSchema {
            name: "alpha".into(),
            kind: "latest_state".into(),
            schema_json: r#"{}"#.into(),
            retention_json: r#"{}"#.into(),
        }])
        .unwrap_err();
    assert_eq!(conflict.kind(), ErrorKind::WriteValidation);

    let malformed = engine
        .write(&[PreparedWrite::AdminSchema {
            name: "broken".into(),
            kind: "unknown_kind".into(),
            schema_json: r#"{}"#.into(),
            retention_json: r#"{}"#.into(),
        }])
        .unwrap_err();
    assert_eq!(malformed.kind(), ErrorKind::SchemaValidation);

    engine.close().expect("close");
}

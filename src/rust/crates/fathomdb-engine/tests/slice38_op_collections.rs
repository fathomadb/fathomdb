//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 4 RED/GREEN suite for
//! `read_operational_collections`, `read_operational_collection_info`, and
//! `AdminSchema` kind-conflict refusal.

use fathomdb_engine::{Engine, EngineError, OperationalCollectionInfo, PreparedWrite};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn admin_schema(name: &str, kind: &str, schema_json: &str, retention_json: &str) -> PreparedWrite {
    PreparedWrite::AdminSchema {
        name: name.to_string(),
        kind: kind.to_string(),
        schema_json: schema_json.to_string(),
        retention_json: retention_json.to_string(),
    }
}

#[test]
fn slice38_read_operational_collections_sorted_and_kind_conflict_refused() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("op-collections{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    // Step 4 seeds `projection_failures` in `operational_collections`.
    let initial = engine.read_operational_collections().expect("list initial");
    assert_eq!(
        initial.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
        vec!["projection_failures"]
    );
    assert_eq!(engine.read_operational_collection_info("missing").expect("get missing"), None);

    // Register out of lexicographic order to verify deterministic `name ASC` sorting.
    engine
        .write(&[
            admin_schema(
                "zeta_state",
                "latest_state",
                r#"{"type":"object","properties":{"v":{"type":"integer"}}}"#,
                r#"{"max_rows":50}"#,
            ),
            admin_schema("alpha_log", "append_only_log", r#"{"type":"object"}"#, r#"{}"#),
            admin_schema("mid_state", "latest_state", r#"{"type":"object"}"#, r#"{"ttl_days":7}"#),
        ])
        .expect("register initial collections");

    let list = engine.read_operational_collections().expect("list collections");
    let names: Vec<&str> = list.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["alpha_log", "mid_state", "projection_failures", "zeta_state"]);

    assert_eq!(
        engine.read_operational_collection_info("alpha_log").expect("get alpha_log"),
        Some(OperationalCollectionInfo {
            name: "alpha_log".into(),
            kind: "append_only_log".into(),
            schema_json: r#"{"type":"object"}"#.into(),
            retention_json: r#"{}"#.into(),
            format_version: 1,
            created_at: 0,
        })
    );
    assert_eq!(
        engine.read_operational_collection_info("zeta_state").expect("get zeta_state"),
        Some(OperationalCollectionInfo {
            name: "zeta_state".into(),
            kind: "latest_state".into(),
            schema_json: r#"{"type":"object","properties":{"v":{"type":"integer"}}}"#.into(),
            retention_json: r#"{"max_rows":50}"#.into(),
            format_version: 1,
            created_at: 0,
        })
    );
    assert_eq!(engine.read_operational_collection_info("absent").expect("get absent"), None);

    // Idempotent update on matching `kind` updates `schema_json` and `retention_json`.
    engine
        .write(&[admin_schema(
            "alpha_log",
            "append_only_log",
            r#"{"type":"object","required":["event"]}"#,
            r#"{"max_rows":500}"#,
        )])
        .expect("idempotent schema/retention update");
    let updated = engine
        .read_operational_collection_info("alpha_log")
        .expect("get updated")
        .expect("alpha_log exists");
    assert_eq!(updated.kind, "append_only_log");
    assert_eq!(updated.schema_json, r#"{"type":"object","required":["event"]}"#);
    assert_eq!(updated.retention_json, r#"{"max_rows":500}"#);

    // Conflicting `kind` against already-committed collection fails atomically with WriteValidation.
    let err_log_to_state = engine
        .write(&[
            admin_schema("untouched_new", "latest_state", r#"{}"#, r#"{}"#),
            admin_schema("alpha_log", "latest_state", r#"{"type":"object"}"#, r#"{}"#),
        ])
        .expect_err("append_only_log -> latest_state conflict must fail");
    assert!(
        matches!(err_log_to_state, EngineError::WriteValidation),
        "expected WriteValidation, got {err_log_to_state:?}"
    );
    // Zero mutation from atomic refusal: neither `untouched_new` nor `alpha_log` changed.
    assert_eq!(
        engine.read_operational_collection_info("untouched_new").expect("check rollback"),
        None
    );
    assert_eq!(
        engine.read_operational_collection_info("alpha_log").expect("check unchanged"),
        Some(updated.clone())
    );

    // Reverse direction (`latest_state` -> `append_only_log`) also fails with WriteValidation.
    let err_state_to_log = engine
        .write(&[admin_schema("zeta_state", "append_only_log", r#"{}"#, r#"{}"#)])
        .expect_err("latest_state -> append_only_log conflict must fail");
    assert!(
        matches!(err_state_to_log, EngineError::WriteValidation),
        "expected WriteValidation, got {err_state_to_log:?}"
    );
    assert_eq!(
        engine
            .read_operational_collection_info("zeta_state")
            .expect("zeta_state unchanged")
            .unwrap()
            .kind,
        "latest_state"
    );

    // Same-batch conflicting `kind` on a net-new collection fails atomically with WriteValidation.
    let err_same_batch = engine
        .write(&[
            admin_schema("same_batch_conflict", "append_only_log", r#"{}"#, r#"{}"#),
            admin_schema("same_batch_conflict", "latest_state", r#"{}"#, r#"{}"#),
        ])
        .expect_err("same-batch kind conflict must fail");
    assert!(
        matches!(err_same_batch, EngineError::WriteValidation),
        "expected WriteValidation for same-batch conflict, got {err_same_batch:?}"
    );
    assert_eq!(
        engine.read_operational_collection_info("same_batch_conflict").expect("check rollback"),
        None
    );

    // Malformed `AdminSchema` payloads still return `EngineError::SchemaValidation`.
    for bad in [
        admin_schema("", "append_only_log", r#"{}"#, r#"{}"#),
        admin_schema("bad_kind", "invalid_kind", r#"{}"#, r#"{}"#),
        admin_schema("bad_schema", "append_only_log", "{not-json", r#"{}"#),
        admin_schema("bad_retention", "latest_state", r#"{}"#, "{not-json"),
        admin_schema(
            "ext_ref",
            "latest_state",
            r#"{"$ref":"https://example.com/schema.json"}"#,
            r#"{}"#,
        ),
    ] {
        let err = engine.write(&[bad]).expect_err("malformed AdminSchema must fail");
        assert!(
            matches!(err, EngineError::SchemaValidation),
            "expected SchemaValidation for malformed AdminSchema, got {err:?}"
        );
    }

    engine.close().expect("close");
}

//! Slice 103 operator contract for an owed governed physical erasure.

use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use clap::Parser;
use fathomdb::{
    ArtifactRevisionId, CanonicalHash, Engine, InitialState, PreparedWrite, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_cli::{exit_code, Cli, Command as CliCommand};
use rusqlite::Connection;
use serde_json::Value;
use tempfile::TempDir;

fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fathomdb")).args(args).output().expect("run operator CLI")
}

fn body(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "JSON output: {error}; stdout={}; stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn node(
    revision: &str,
    version: &str,
    logical: &str,
    source: &str,
    content: &str,
    provenance: WriteProvenanceV1,
) -> PreparedWrite {
    let _ = (revision, version);
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: content.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance,
    })
}

fn governed_owed_fixture() -> (TempDir, PathBuf, String) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("owed erasure.sqlite");
    let source_body = "source-erasure-sentinel";
    let hash = "8aa6ca24cef929aff267326b6ee659ae50a782ed0579ec213658cb7029e50d92";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[
            node(
                "original-r1",
                "original-v1",
                "original",
                "source-secret",
                source_body,
                WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("original-r1").unwrap(),
                    SourceVersionId::new("original-v1").unwrap(),
                ),
            ),
            node(
                "dependent-r1",
                "original-v1",
                "dependent",
                "source-secret",
                "derived-secret",
                WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("dependent-r1").unwrap(),
                    SourceVersionId::new("original-v1").unwrap(),
                    SourceRevisionId::new("original-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(hash).unwrap(),
                ),
            ),
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "unrelated-survivor-sentinel".into(),
                source_id: SourceId::new("other-source").unwrap(),
                logical_id: Some("unrelated-survivor".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "original-r1", "dependent-r1").unwrap(),
        )
        .unwrap();
    opened.engine.erase_source("source-secret").unwrap();
    opened.engine.close().unwrap();
    drop(opened);

    let connection = Connection::open(&path).unwrap();
    let closure_id: String = connection
        .query_row(
            "SELECT closure_operation_id FROM _fathomdb_dependency_closures \
         WHERE cause='source_erased' ORDER BY closure_sequence LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    // Recreate the durable phase left by an interrupted checkpoint after a
    // real governed erasure committed its structural zero proof.
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
         SET phase='incomplete', blocker_code='wal_checkpoint' \
         WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);
    (dir, path, closure_id)
}

fn closure_phase(path: &Path, id: &str) -> String {
    Connection::open(path)
        .unwrap()
        .query_row(
            "SELECT phase FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1",
            [id],
            |row| row.get(0),
        )
        .unwrap()
}

fn closure_and_queue_state(connection: &Connection, id: &str) -> (String, String, String, i64) {
    connection
        .query_row(
            "SELECT phase,blocker_code,proof_json,\
                    (SELECT COUNT(*) FROM operational_mutations \
                     WHERE collection_name='erasure_pending_redaction') \
             FROM _fathomdb_dependency_closures WHERE closure_operation_id=?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
}

#[test]
fn doctor_reports_owed_closure_and_remediation_round_trips_with_spaces() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("PRAGMA wal_autocheckpoint=0").unwrap();
    raw.execute(
        "UPDATE _fathomdb_dependency_closures SET blocker_code='wal_checkpoint' \
         WHERE closure_operation_id=?1",
        [&closure_id],
    )
    .unwrap();
    let wal_path = PathBuf::from(format!("{}-wal", path.display()));
    let wal_before = std::fs::read(&wal_path).expect("live WAL fixture");
    let state_before = closure_and_queue_state(&raw, &closure_id);
    let output = command(&["doctor", "check-integrity", "--json", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{output:#?}");
    let report = body(&output);
    let findings = report["logical"]["findings"].as_array().unwrap();
    let finding = findings
        .iter()
        .find(|item| item["code"] == "E_ERASURE_INCOMPLETE")
        .expect("owed physical erasure finding");
    assert_eq!(finding["closure_id"], closure_id);
    assert_eq!(finding["cause"], "source_erased");
    assert_eq!(finding["phase"], "incomplete");
    assert_eq!(finding["blocker"], "wal_checkpoint");
    assert!(finding["sequence"].as_u64().unwrap() > 0);
    assert!(finding["wal_frames"].as_u64().is_some());
    assert_eq!(finding["source_id"], "[redacted]");
    assert!(finding["doc_anchor"].as_str().unwrap().contains("erasure-incomplete"));
    let argv = finding["remediation"]["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let parsed = Cli::try_parse_from(argv).expect("doctor-generated command must parse");
    let CliCommand::Recover(args) = parsed.command else { panic!("recover command") };
    assert!(args.accept_data_loss);
    assert!(format!("{args:?}").contains("complete_erasures: true"));
    assert_eq!(args.db_path, path);
    assert_eq!(closure_phase(&path, &closure_id), "incomplete", "doctor must be read only");
    assert_eq!(closure_and_queue_state(&raw, &closure_id), state_before);
    assert_eq!(std::fs::read(&wal_path).unwrap(), wal_before, "finding must not checkpoint WAL");
}

#[test]
fn offline_recovery_completes_proven_erasure_and_clears_write_fence() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::RECOVERY_ACCEPTED_LOSS), "{output:#?}");
    assert_eq!(body(&output)["status"], "done");
    assert_eq!(closure_phase(&path, &closure_id), "complete");
    let database_bytes = std::fs::read(&path).unwrap();
    let wal_path = PathBuf::from(format!("{}-wal", path.display()));
    let wal_bytes = std::fs::read(&wal_path).unwrap_or_default();
    for erased in ["source-erasure-sentinel", "derived-secret"] {
        assert!(!database_bytes.windows(erased.len()).any(|window| window == erased.as_bytes()));
        assert!(!wal_bytes.windows(erased.len()).any(|window| window == erased.as_bytes()));
    }
    let survivor: String = Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT body FROM canonical_nodes WHERE logical_id='unrelated-survivor'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(survivor, "unrelated-survivor-sentinel");
    let reopened = Engine::open(&path).unwrap();
    reopened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "survivor".into(),
            source_id: SourceId::new("other-source").unwrap(),
            logical_id: Some("other".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("fence lifted only after completion");
    reopened.engine.close().unwrap();
}

#[test]
fn recovery_requires_acknowledgement_and_refuses_while_engine_open() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let no_ack = command(&["recover", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(no_ack.status.code(), Some(exit_code::UNRECOVERABLE));
    assert_eq!(closure_phase(&path, &closure_id), "incomplete");

    let opened = Engine::open(&path).unwrap();
    let held =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(held.status.code(), Some(exit_code::LOCK_HELD), "{held:#?}");
    assert_eq!(closure_phase(&path, &closure_id), "incomplete");
    opened.engine.close().unwrap();
}

#[test]
fn missing_original_telemetry_sink_preserves_owed_closure() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let connection = Connection::open(&path).unwrap();
    let boundary: i64 = connection
        .query_row(
            "SELECT admitted_write_boundary FROM _fathomdb_dependency_closures \
             WHERE closure_operation_id=?1",
            [&closure_id],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO operational_mutations(
           collection_name,record_key,op_kind,payload_json,schema_id,write_cursor
         ) VALUES('erasure_pending_redaction','erase_source','append',
                  '{\"erased_stable_ids\":[\"l:owed\"]}',NULL,?1)",
            [boundary],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
         SET blocker_code='telemetry_redaction' WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    let before = closure_and_queue_state(&Connection::open(&path).unwrap(), &closure_id);
    let diagnosis = command(&["doctor", "check-integrity", "--json", path.to_str().unwrap()]);
    assert_eq!(diagnosis.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES));
    let findings = body(&diagnosis)["logical"]["findings"].as_array().unwrap().clone();
    let finding = findings.iter().find(|finding| finding["closure_id"] == closure_id).unwrap();
    assert!(
        finding["remediation"]["argv"].is_null(),
        "offline action cannot finish telemetry: {finding}"
    );
    assert!(finding["detail"].as_str().unwrap().contains("original telemetry sink"));

    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::UNRECOVERABLE), "{output:#?}");
    assert_eq!(closure_phase(&path, &closure_id), "incomplete");
    assert_eq!(closure_and_queue_state(&Connection::open(&path).unwrap(), &closure_id), before);
}

#[test]
fn doctor_sees_telemetry_queue_before_blocker_is_recorded() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let connection = Connection::open(&path).unwrap();
    let boundary: i64 = connection
        .query_row(
            "SELECT admitted_write_boundary FROM _fathomdb_dependency_closures \
         WHERE closure_operation_id=?1",
            [&closure_id],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO operational_mutations(
           collection_name,record_key,op_kind,payload_json,schema_id,write_cursor
         ) VALUES('erasure_pending_redaction','erase_source','append',
                  '{\"erased_stable_ids\":[\"l:owed\"]}',NULL,?1)",
            [boundary],
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
         SET phase='at_rest_pending',blocker_code=NULL WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    let diagnosis = command(&["doctor", "check-integrity", "--json", path.to_str().unwrap()]);
    assert_eq!(diagnosis.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES));
    let findings = body(&diagnosis)["logical"]["findings"].as_array().unwrap().clone();
    let finding = findings.iter().find(|finding| finding["closure_id"] == closure_id).unwrap();
    assert_eq!(finding["blocker"], "telemetry_redaction");
    assert!(finding["remediation"]["argv"].is_null(), "queue is still owed: {finding}");
    assert!(finding["detail"].as_str().unwrap().contains("original telemetry sink"));
}

#[test]
fn no_owed_closure_does_not_truncate_unrelated_wal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("clean.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let raw = Connection::open(&path).unwrap();
    raw.execute_batch("PRAGMA wal_autocheckpoint=0; CREATE TABLE _recovery_probe(value INTEGER)")
        .unwrap();
    let wal_path = PathBuf::from(format!("{}-wal", path.display()));
    let wal_before = std::fs::read(&wal_path).expect("unrelated WAL fixture");

    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::OK), "{output:#?}");
    assert_eq!(body(&output)["status"], "clean");
    assert_eq!(std::fs::read(&wal_path).unwrap(), wal_before, "unrelated WAL must be untouched");
}

#[test]
fn malformed_wal_refusal_does_not_open_writer_or_change_sidecars() {
    for len in [16, 32] {
        let (_dir, path, _closure_id) = governed_owed_fixture();
        let wal_path = PathBuf::from(format!("{}-wal", path.display()));
        let shm_path = PathBuf::from(format!("{}-shm", path.display()));
        std::fs::write(&wal_path, vec![0xff; len]).unwrap();
        let db_before = std::fs::read(&path).unwrap();
        let wal_before = std::fs::read(&wal_path).unwrap();
        let shm_before = std::fs::read(&shm_path).ok();

        let output = command(&[
            "recover",
            "--accept-data-loss",
            "--complete-erasures",
            path.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(exit_code::UNRECOVERABLE), "{output:#?}");
        assert_eq!(std::fs::read(&path).unwrap(), db_before);
        assert_eq!(std::fs::read(&wal_path).unwrap(), wal_before);
        assert_eq!(std::fs::read(&shm_path).ok(), shm_before);
    }
}

#[test]
fn invalid_physical_zero_proof_refuses_without_completing_closure() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
             SET admitted_dependency_generation=0 WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    let before = closure_and_queue_state(&connection, &closure_id);
    drop(connection);

    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::UNRECOVERABLE), "{output:#?}");
    assert_eq!(closure_phase(&path, &closure_id), "incomplete");
    assert_eq!(closure_and_queue_state(&Connection::open(&path).unwrap(), &closure_id), before);
}

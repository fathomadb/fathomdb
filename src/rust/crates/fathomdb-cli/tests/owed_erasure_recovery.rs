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

#[test]
fn doctor_reports_owed_closure_and_remediation_round_trips_with_spaces() {
    let (_dir, path, closure_id) = governed_owed_fixture();
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
}

#[test]
fn offline_recovery_completes_proven_erasure_and_clears_write_fence() {
    let (_dir, path, closure_id) = governed_owed_fixture();
    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::RECOVERY_ACCEPTED_LOSS), "{output:#?}");
    assert_eq!(body(&output)["status"], "done");
    assert_eq!(closure_phase(&path, &closure_id), "complete");
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
    connection
        .execute(
            "UPDATE _fathomdb_dependency_closures \
         SET blocker_code='telemetry_redaction' WHERE closure_operation_id=?1",
            [&closure_id],
        )
        .unwrap();
    drop(connection);

    let output =
        command(&["recover", "--accept-data-loss", "--complete-erasures", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(exit_code::UNRECOVERABLE), "{output:#?}");
    assert_eq!(closure_phase(&path, &closure_id), "incomplete");
}

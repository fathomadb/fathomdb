use fathomdb::Engine;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::process::{Child, Command, Output};
use std::time::{Duration, Instant};

#[path = "../../fathomdb-engine/tests/support/integrity_findings.rs"]
mod integrity_findings;

struct SupervisedChild(Option<Child>);
impl Drop for SupervisedChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn bounded_output(mut command: Command) -> Output {
    let mut stdout = tempfile::tempfile().unwrap();
    let mut stderr = tempfile::tempfile().unwrap();
    let mut child = SupervisedChild(Some(
        command
            .stdout(stdout.try_clone().unwrap())
            .stderr(stderr.try_clone().unwrap())
            .spawn()
            .unwrap(),
    ));
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.0.as_mut().unwrap().try_wait().unwrap() {
            child.0.take();
            break status;
        }
        assert!(Instant::now() < deadline, "owned command watchdog");
        std::thread::sleep(Duration::from_millis(10));
    };
    stdout.seek(SeekFrom::Start(0)).unwrap();
    stderr.seek(SeekFrom::Start(0)).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout.read_to_end(&mut out).unwrap();
    stderr.read_to_end(&mut err).unwrap();
    Output { status, stdout: out, stderr: err }
}
fn cli(args: &[&str]) -> (i32, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command.args(args);
    let output = bounded_output(command);
    let body = String::from_utf8(output.stdout).unwrap();
    let value = serde_json::from_str(&body)
        .unwrap_or_else(|error| panic!("{error}: {body}; stderr={:?}", output.stderr));
    (output.status.code().expect("normal CLI exit"), value)
}

#[test]
fn full_integrity_reports_actual_index_finding_and_exact_cli_class() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("findings.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    integrity_findings::missing_index_entry(&path);
    for _ in 0..2 {
        let (code, result) = cli(&["doctor", "check-integrity", "--full", path.to_str().unwrap()]);
        assert_eq!(code, 65, "{result}");
        assert_eq!(result["verb"], "check-integrity");
        let findings = result["physical"]["findings"].as_array().expect("physical findings");
        assert!(
            findings.iter().any(|finding| {
                finding["code"] == "E_CORRUPT_INTEGRITY_CHECK"
                    && finding["stage"] == "IntegrityCheck"
                    && finding["detail"]
                        .as_str()
                        .unwrap()
                        .contains("row 2 missing from index slice55_witness_gap")
                    && finding["doc_anchor"] == "design/recovery.md#integrity-check-full-findings"
            }),
            "{result}"
        );
    }
}

mod wal_only {
    use super::*;
    use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
    use fathomdb_engine::{InitialState, PreparedWrite, ReadView, SourceId};
    use rusqlite::Connection;
    use sha2::{Digest, Sha256};
    use std::path::Path;
    use std::sync::Arc;

    struct KnownVector;
    impl Embedder for KnownVector {
        fn identity(&self) -> EmbedderIdentity {
            EmbedderIdentity::new("cli-wal", "authored-v1", 8)
        }
        fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
            Ok(if text.contains("walneedle") {
                vec![0., 1., 0., 0., 0., 0., 0., 0.]
            } else {
                vec![1., 0., 0., 0., 0., 0., 0., 0.]
            })
        }
    }
    fn node(id: &str, body: &str) -> PreparedWrite {
        PreparedWrite::Node {
            kind: "doc".into(),
            body: body.into(),
            source_id: SourceId::new("wal-source").unwrap(),
            logical_id: Some(id.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }
    }
    fn copy(path: &Path, main: &[u8], wal: Option<&[u8]>) {
        std::fs::write(path, main).unwrap();
        if let Some(wal) = wal {
            std::fs::write(format!("{}-wal", path.display()), wal).unwrap();
        }
    }
    fn witness(path: &Path, selected: bool) {
        let engine =
            fathomdb_sdk::Engine::open(path, fathomdb_sdk::OpenOptions::default()).unwrap();
        assert_eq!(
            fathomdb_sdk::read::get(&engine, "control", None).unwrap().unwrap().body,
            "independent control"
        );
        assert_eq!(
            fathomdb_sdk::read::get(&engine, "selected", None).unwrap().map(|r| r.body),
            selected.then(|| "walneedle café 猫".to_owned())
        );
        engine.close().unwrap();
    }
    fn inspect(path: &Path) {
        witness(path, true);
        let engine =
            fathomdb_sdk::Engine::open(path, fathomdb_sdk::OpenOptions::default()).unwrap();
        let hits = engine
            .search_text_only("walneedle", fathomdb_sdk::TextSearchOptions::default())
            .unwrap();
        assert_eq!(hits.results.len(), 1);
        assert_eq!(hits.results[0].id.value, "selected");
        assert_eq!(hits.results[0].source_id.as_deref(), Some("wal-source"));
        engine.close().unwrap();
        let conn = Connection::open(path).unwrap();
        let profile:(String,String,i64)=conn.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(profile, ("cli-wal".into(), "authored-v1".into(), 8));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM _fathomdb_vector_rows", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        let rows=conn.prepare("SELECT n.logical_id,n.write_cursor,m.rowid,m.write_cursor,v.embedding FROM canonical_nodes n JOIN _fathomdb_vector_rows m ON m.write_cursor=n.write_cursor JOIN vector_default v ON v.rowid=m.rowid ORDER BY n.write_cursor").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,Vec<u8>>(4)?))).unwrap().collect::<Result<Vec<_>,_>>().unwrap();
        let control = [1_f32, 0., 0., 0., 0., 0., 0., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        let selected = [0_f32, 1., 0., 0., 0., 0., 0., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![("control".into(), 1, 1, 1, control), ("selected".into(), 2, 2, 2, selected)]
        );
    }
    #[test]
    #[ignore = "supervised fresh-process SDK output inspection"]
    fn inspect_child() {
        let path = std::env::var("SLICE55_OUTPUT").unwrap();
        inspect(Path::new(&path));
        std::fs::write(format!("{path}.verified"), b"inspected-and-closed").unwrap();
    }
    fn fresh_inspect(path: &Path) {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "wal_only::inspect_child", "--ignored", "--nocapture"])
            .env("SLICE55_OUTPUT", path);
        let output = bounded_output(command);
        assert!(
            output.status.success(),
            "fresh-process inspection: {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            std::fs::read(format!("{}.verified", path.display())).unwrap(),
            b"inspected-and-closed",
            "actual helper body completed"
        );
    }
    #[test]
    fn committed_wal_only_record_survives_cli_export_and_acknowledged_wal_recovery() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("source.sqlite");
        let opened =
            fathomdb_engine::Engine::open_with_embedder_for_test(&path, Arc::new(KnownVector))
                .unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        opened.engine.write(&[node("control", "independent control")]).unwrap();
        opened.engine.drain(10_000).unwrap();
        let checkpointer = Connection::open(&path).unwrap();
        checkpointer.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
        let mut reader = Connection::open(&path).unwrap();
        let tx = reader.transaction().unwrap();
        assert_eq!(
            tx.query_row("SELECT count(*) FROM canonical_nodes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        opened.engine.write(&[node("selected", "walneedle café 猫")]).unwrap();
        opened.engine.drain(10_000).unwrap();
        assert_eq!(
            opened.engine.read_get("selected", &ReadView::default()).unwrap().unwrap().body,
            "walneedle café 猫"
        );
        let main = std::fs::read(&path).unwrap();
        let wal = std::fs::read(format!("{}-wal", path.display())).unwrap();
        assert!(wal.len() > 32);
        // Snapshot only while the product is quiescent and the held reader prevents backfill.
        let with = dir.path().join("witness-with.sqlite");
        let without = dir.path().join("witness-main.sqlite");
        copy(&with, &main, Some(&wal));
        copy(&without, &main, None);
        witness(&with, true);
        witness(&without, false);
        let export_input = dir.path().join("export-input.sqlite");
        let recover_input = dir.path().join("recover-input.sqlite");
        copy(&export_input, &main, Some(&wal));
        copy(&recover_input, &main, Some(&wal));
        assert_eq!(std::fs::read(&export_input).unwrap(), main);
        assert_eq!(std::fs::read(&recover_input).unwrap(), main);
        let out = dir.path().join("export.sqlite");
        let manifest = dir.path().join("export.manifest.json");
        let (code, result) = cli(&[
            "doctor",
            "safe-export",
            out.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            export_input.to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["verb"], "safe-export");
        let bytes = std::fs::read(&out).unwrap();
        let digest = Sha256::digest(&bytes).iter().map(|v| format!("{v:02x}")).collect::<String>();
        let metadata: Value = serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        assert_eq!(result["manifest_sha256"], digest);
        assert_eq!(metadata["sha256"], digest);
        assert_eq!(metadata["byte_count"], bytes.len() as u64);
        for _ in 0..2 {
            let (code, profile) = cli(&["doctor", "dump-profile", "--json", out.to_str().unwrap()]);
            assert_eq!(code, 0);
            assert_eq!(profile["embedder_identity"], "cli-wal:authored-v1");
            assert_eq!(profile["embedder_dimension"], 8);
            assert_eq!(profile["vectorized_kinds"], serde_json::json!(["doc"]));
            let (code, counts) =
                cli(&["doctor", "dump-row-counts", "--json", out.to_str().unwrap()]);
            assert_eq!(code, 0);
            let rows = counts["counts"].as_array().unwrap();
            assert_eq!(rows.iter().find(|r| r["name"] == "canonical_nodes").unwrap()["rows"], 2);
            assert_eq!(rows.iter().find(|r| r["name"] == "canonical_edges").unwrap()["rows"], 0);
            let (code, schema) = cli(&["doctor", "dump-schema", "--json", out.to_str().unwrap()]);
            assert_eq!(code, 0);
            assert_eq!(schema["user_version"], 38);
            let tables = schema["tables"].as_array().unwrap();
            assert!(tables.iter().any(|t| t["name"] == "canonical_nodes"));
            assert!(tables.iter().any(|t| t["name"] == "canonical_edges"));
            assert!(tables.iter().all(|t| !t["name"].as_str().unwrap().starts_with("sqlite_")));
        }
        fresh_inspect(&out);
        let (code, result) = cli(&[
            "recover",
            "--accept-data-loss",
            "--truncate-wal",
            recover_input.to_str().unwrap(),
        ]);
        assert_eq!(code, 64, "{result}");
        assert_eq!(result["verb"], "truncate-wal");
        assert_eq!(result["status"], "done");
        assert_eq!(result["discarded_corrupt_wal"], false);
        let remaining_wal = std::fs::metadata(format!("{}-wal", recover_input.display()))
            .map(|m| m.len())
            .unwrap_or(0);
        assert_eq!(remaining_wal, 0, "successful truncate-wal must actually checkpoint/truncate");
        let recovered_main = dir.path().join("recovered-main-only.sqlite");
        std::fs::copy(&recover_input, &recovered_main).unwrap();
        fresh_inspect(&recovered_main);
        tx.commit().unwrap();
        drop(reader);
        opened.engine.close().unwrap();
        drop(opened);
        drop(checkpointer);
        let tampered = bytes
            .iter()
            .enumerate()
            .map(|(i, b)| if i == bytes.len() - 1 { b ^ 1 } else { *b })
            .collect::<Vec<_>>();
        assert_ne!(
            Sha256::digest(tampered).to_vec(),
            Sha256::digest(bytes).to_vec(),
            "independent manifest oracle detects tampering; no verifier command exists"
        );
    }
}

#[test]
fn doctor_inventory_and_selected_refusal_effects_are_exact() {
    use clap::CommandFactory;
    let command = fathomdb_cli::Cli::command();
    let doctor = command.find_subcommand("doctor").unwrap();
    let mut names =
        doctor.get_subcommands().map(|c| c.get_name()).filter(|n| *n != "help").collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        vec![
            "check-integrity",
            "cuda-allocator",
            "cuda-libraries",
            "data-plane-integrity",
            "dump-mutations",
            "dump-profile",
            "dump-row-counts",
            "dump-schema",
            "gpu",
            "orphan-provenance",
            "platform",
            "recompute-mean",
            "reranker-gpu",
            "safe-export",
            "trace",
            "verify-embedder",
            "warm-cache"
        ]
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effects.sqlite");
    let engine = fathomdb_sdk::Engine::open(&path, fathomdb_sdk::OpenOptions::default()).unwrap();
    engine
        .write(&[fathomdb_sdk::PreparedWrite::Node {
            kind: "doc".into(),
            body: "independent survivor".into(),
            source_id: fathomdb_sdk::SourceId::new("doctor-source").unwrap(),
            logical_id: Some("survivor".into()),
            state: fathomdb_sdk::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.close().unwrap();
    for _ in 0..2 {
        let (code, value) = cli(&[
            "doctor",
            "verify-embedder",
            "--identity",
            "wrong:identity",
            "--dimension",
            "999",
            path.to_str().unwrap(),
        ]);
        assert_eq!(code, 0, "mismatch is a diagnostic status, not command failure: {value}");
        assert_eq!(value["status"], "both_mismatch");
        assert_eq!(value["supplied_identity"], "wrong:identity");
        assert_eq!(value["supplied_dimension"], 999);
        let (code, value) =
            cli(&["doctor", "trace", "--source-ref", "missing-source", path.to_str().unwrap()]);
        assert_eq!(code, 0);
        assert_eq!(value["source_ref"], "missing-source");
        assert_eq!(value["events"], serde_json::json!([]));
        let out = dir.path().join("occupied.sqlite");
        std::fs::write(&out, b"owned sentinel").unwrap();
        let manifest = dir.path().join("never-created.json");
        let (code, value) = cli(&[
            "doctor",
            "safe-export",
            out.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            path.to_str().unwrap(),
        ]);
        assert_eq!(code, 66);
        assert_eq!(value["verb"], "safe-export");
        assert_eq!(value["status"], "error");
        assert_eq!(std::fs::read(out).unwrap(), b"owned sentinel");
        assert!(!manifest.exists());
    }
    let engine = fathomdb_sdk::Engine::open(&path, fathomdb_sdk::OpenOptions::default()).unwrap();
    assert_eq!(
        fathomdb_sdk::read::list(&engine, "doc", fathomdb_sdk::ListOptions::default())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        fathomdb_sdk::read::get(&engine, "survivor", None).unwrap().unwrap().body,
        "independent survivor"
    );
    engine.close().unwrap();
}

#[cfg(not(feature = "default-embedder"))]
#[test]
fn warm_cache_no_feature_is_exact_database_free_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command.args(["doctor", "warm-cache", "--json"]).current_dir(dir.path());
    let output = bounded_output(command);
    assert_eq!(output.status.code(), Some(70));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"verb":"warm-cache","status":"error","code":"DefaultEmbedderFeatureDisabled","detail":"fathomdb CLI was built without the `default-embedder` feature; rebuild with --features default-embedder"})
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
#[cfg(not(target_arch = "aarch64"))]
#[test]
fn platform_native_non_aarch64_is_exact_database_free_success() {
    let dir = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command.args(["doctor", "platform", "--json"]).current_dir(dir.path());
    let output = bounded_output(command);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"schema_version":"fathomdb.doctor.platform.v1","platform_class":"non_aarch64","tegra_family":false,"sbsa_capable":false,"l4t_release":null,"reason":null})
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn full_integrity_early_header_refusal_is_distinct_from_findings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("header.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[..16].copy_from_slice(b"not-a-sqlite-db!");
    std::fs::write(&path, &bytes).unwrap();
    let (code, value) = cli(&["doctor", "check-integrity", "--full", path.to_str().unwrap()]);
    assert_eq!(code, 70);
    assert_eq!(value["verb"], "check-integrity");
    assert_eq!(value["status"], "error");
    assert_eq!(value["code"], "E_CORRUPT_HEADER");
    assert!(value.get("physical").is_none());
}

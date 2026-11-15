use super::entry::{inherited_settings, run_cell};
use serde_json::json;
use std::process::Command;
use std::time::{Duration, Instant};

const KEYS: [&str; 5] = [
    "FATHOMDB_PROJECTION_BATCH",
    "FATHOMDB_PERF_EXPERIMENTS",
    "FATHOMDB_PERF_READER_PRAGMAS",
    "FATHOMDB_PERF_WRITER_PRAGMAS",
    "FATHOMDB_PERF_SEARCH_LIMIT",
];

#[test]
fn native_entry_refuses_inherited_treatments_before_store() {
    if let Ok(output) = std::env::var("S30_D_ENV_CHILD") {
        if std::env::var("S30_D_ENV_DEFAULT").as_deref() == Ok("1") {
            assert!(inherited_settings().is_none(), "inert settings must retain default admission");
            return;
        }
        let origin = Instant::now();
        let identity = json!({"cell_id":"tiny-t-held-n2-r0","attempt_id":"env-one",
            "n":2,"mode":"held","layer":"T","repeat":0});
        let report = run_cell(
            &identity,
            std::path::Path::new(&output),
            origin,
            origin + Duration::from_secs(60),
        )
        .expect("native admission returns a retained refusal");
        assert!(
            !std::path::Path::new(&output).join("store.sqlite").exists(),
            "inherited treatment reached native store preparation"
        );
        assert_eq!(report["complete"], false);
        assert_eq!(report["state"], "held");
        let expected = KEYS
            .iter()
            .filter_map(|key| {
                std::env::var(key).ok().map(|value| ((*key).to_owned(), json!(value)))
            })
            .collect::<serde_json::Map<_, _>>();
        assert_eq!(report["inherited_settings"], json!(expected));
        return;
    }
    let cases = [
        vec![("FATHOMDB_PROJECTION_BATCH", "1")],
        vec![("FATHOMDB_PROJECTION_BATCH", "true")],
        vec![("FATHOMDB_PROJECTION_BATCH", "on")],
        vec![
            ("FATHOMDB_PERF_EXPERIMENTS", "1"),
            ("FATHOMDB_PERF_READER_PRAGMAS", "cache_size=-64"),
        ],
        vec![
            ("FATHOMDB_PERF_EXPERIMENTS", "1"),
            ("FATHOMDB_PERF_WRITER_PRAGMAS", "synchronous=OFF"),
        ],
        vec![("FATHOMDB_PERF_EXPERIMENTS", "1"), ("FATHOMDB_PERF_SEARCH_LIMIT", "1")],
        vec![("FATHOMDB_PERF_EXPERIMENTS", "0"), ("FATHOMDB_PERF_SEARCH_LIMIT", "0")],
    ];
    for settings in cases {
        let directory = tempfile::tempdir().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "entry_env_tests::native_entry_refuses_inherited_treatments_before_store",
                "--test-threads=1",
            ])
            .env("S30_D_ENV_CHILD", directory.path())
            .env("S30_D_ENV_DEFAULT", "0");
        for key in KEYS {
            command.env_remove(key);
        }
        for (key, value) in &settings {
            command.env(key, value);
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "settings={settings:?}\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for settings in [
        vec![("FATHOMDB_PROJECTION_BATCH", "0")],
        vec![
            ("FATHOMDB_PERF_READER_PRAGMAS", "cache_size=-64"),
            ("FATHOMDB_PERF_SEARCH_LIMIT", "1"),
        ],
        vec![("FATHOMDB_PERF_EXPERIMENTS", "1"), ("FATHOMDB_PERF_SEARCH_LIMIT", "invalid")],
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "entry_env_tests::native_entry_refuses_inherited_treatments_before_store",
                "--test-threads=1",
            ])
            .env("S30_D_ENV_CHILD", directory.path())
            .env("S30_D_ENV_DEFAULT", "1");
        for key in KEYS {
            command.env_remove(key);
        }
        for (key, value) in &settings {
            command.env(key, value);
        }
        let result = command.output().unwrap();
        assert!(
            result.status.success(),
            "inert settings={settings:?}\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

//! Slice 41 (0.8.28) phase 2 AC41-36/38 — query stop words through the Rust SDK.
//!
//! A refused setting is `ErrorKind::InvalidArgument` at open, naming the field;
//! the open report carries the effective hash, and the setting is echoed by
//! `Engine::config`. The refusal cases are the shared fixture's.

use fathomdb_sdk::{
    Engine, EngineConfig, ErrorKind, OpenOptions, QueryStopWords, StopWordBase,
    DEFAULT_STOP_WORDS_SHA256,
};
use serde_json::Value;
use tempfile::TempDir;

const FIXTURES: &str = include_str!("../../../../conformance/query-stop-words-fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).expect("fixture JSON")
}

fn setting_from(v: &Value) -> QueryStopWords {
    let words = |key: &str| -> Vec<String> {
        v[key]
            .as_array()
            .map(|a| a.iter().map(|w| w.as_str().expect("word").to_string()).collect())
            .unwrap_or_default()
    };
    QueryStopWords {
        base: if v["base"] == "none" { StopWordBase::None } else { StopWordBase::Fathomdb },
        extra: words("extra"),
        keep: words("keep"),
    }
}

fn options(setting: Option<QueryStopWords>) -> OpenOptions {
    OpenOptions {
        config: EngineConfig { query_stop_words: setting, ..EngineConfig::default() },
        ..OpenOptions::default()
    }
}

#[test]
fn invalid_query_stop_words_are_invalid_argument() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("refused.sqlite");
    for case in fixtures()["invalid_settings"].as_array().expect("invalid_settings") {
        let err = Engine::open(&path, options(Some(setting_from(&case["setting"]))))
            .expect_err("must be refused");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument, "case {}", case["name"]);
        let needle = case["message_contains"].as_str().expect("needle");
        assert!(err.to_string().contains(needle), "case {}: {err}", case["name"]);
        assert!(!path.exists(), "case {}", case["name"]);
    }
}

#[test]
fn open_report_and_config_carry_the_setting() {
    let f = fixtures();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("hash.sqlite");
    assert_eq!(DEFAULT_STOP_WORDS_SHA256, f["default_sha256"]);
    let plain = Engine::open(&path, options(None)).expect("open");
    assert_eq!(plain.open_report().stop_words_sha256, DEFAULT_STOP_WORDS_SHA256);
    plain.close().expect("close");
    let setting = setting_from(&f["setting"]);
    let configured = Engine::open(&path, options(Some(setting.clone()))).expect("open");
    assert_eq!(configured.open_report().stop_words_sha256, f["setting_sha256"]);
    assert_eq!(configured.config().query_stop_words, Some(setting));
    configured.close().expect("close");
}

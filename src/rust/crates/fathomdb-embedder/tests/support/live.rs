//! Test-only handling of a missing live prerequisite (model weights, a GPU,
//! ONNX Runtime assets, or a runner environment).
// Included by test targets whose callers are compiled only under some features.
#![allow(dead_code)]

/// Reports that a live prerequisite is missing. With `FATHOMDB_REQUIRE_LIVE=1`
/// (set by the feature-complete gate, which provisions every such
/// prerequisite) it panics with `message`; otherwise it prints `message` and
/// the caller skips.
pub fn require_live_or_skip(message: &str) {
    if std::env::var_os("FATHOMDB_REQUIRE_LIVE").is_some_and(|value| value == "1") {
        panic!("FATHOMDB_REQUIRE_LIVE=1 and a live prerequisite is missing: {message}");
    }
    eprintln!("{message}");
}

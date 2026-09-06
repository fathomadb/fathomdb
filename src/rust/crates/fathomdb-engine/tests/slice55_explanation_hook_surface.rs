//! Default-build compile-surface proof for Slice 55 explanation test hooks.

use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn slice55_explanation_hooks_do_not_compile_for_default_consumers() {
    let dir = TempDir::new().unwrap();
    let manifest_dir = env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\");
    fs::create_dir(dir.path().join("src")).unwrap();
    fs::write(
        dir.path().join("Cargo.toml"),
        format!(
            "[package]\nname='slice55-hook-surface'\nversion='0.0.0'\nedition='2021'\n\
             [dependencies]\nfathomdb-engine={{path='{manifest_dir}',default-features=false}}\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.path().join("src/main.rs"),
        "use fathomdb_engine::{\n\
             arm_explanation_after_telemetry_lock_hook_for_test,\n\
             arm_explanation_before_telemetry_lock_hook_for_test,\n\
         };\n\
         fn main() {\n\
             arm_explanation_before_telemetry_lock_hook_for_test(Box::new(|| {}));\n\
             arm_explanation_after_telemetry_lock_hook_for_test(Box::new(|| {}));\n\
         }\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--offline"])
        .env("CARGO_TARGET_DIR", dir.path().join("target"))
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success(), "default consumer compiled test-only hooks");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unresolved imports"), "unexpected compiler diagnostic: {stderr}");
    assert!(
        stderr.contains("arm_explanation_before_telemetry_lock_hook_for_test"),
        "missing before-hook diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("arm_explanation_after_telemetry_lock_hook_for_test"),
        "missing after-hook diagnostic: {stderr}"
    );
}

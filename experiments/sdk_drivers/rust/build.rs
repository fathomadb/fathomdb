use std::process::Command;

fn sdk_version() -> String {
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..").join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    let source = std::fs::read_to_string(manifest).expect("read repository Cargo.toml");
    let workspace = source
        .split("[workspace.package]")
        .nth(1)
        .expect("workspace package table")
        .split('\n')
        .take_while(|line| !line.starts_with('['));
    workspace
        .filter_map(|line| line.trim().strip_prefix("version = \"")?.split('"').next())
        .next()
        .expect("workspace version")
        .to_string()
}

fn main() {
    println!("cargo:rustc-env=DRIVER_SDK_VERSION={}", sdk_version());
    let rustc = std::env::var("RUSTC").expect("Cargo sets RUSTC");
    let output = Command::new(rustc).arg("--version").output().expect("run rustc --version");
    assert!(output.status.success(), "rustc --version failed");
    let version = String::from_utf8(output.stdout).expect("rustc version is UTF-8");
    println!("cargo:rustc-env=DRIVER_RUSTC_VERSION={}", version.trim());
    for name in ["TARGET", "PROFILE"] {
        let value = std::env::var(name).expect("Cargo sets build identity");
        println!("cargo:rustc-env=DRIVER_{name}={value}");
    }
}

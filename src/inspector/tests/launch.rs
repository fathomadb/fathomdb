#![cfg(target_os = "linux")]
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
fn binary() -> std::path::PathBuf {
    let binary = std::env::var_os("INSPECTOR_QUALIFY_BINARY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_fathomdb-inspector").into());
    assert!(binary.is_absolute() && binary.is_file());
    binary
}
#[test]
fn missing_gui_is_actionable_and_creates_no_private_session() {
    let scratch = tempfile::tempdir().unwrap();
    let binary = binary();
    let missing = Command::new(binary)
        .arg(scratch.path().join("missing.sqlite"))
        .env_clear()
        .env("TMPDIR", scratch.path())
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert_eq!(String::from_utf8(missing.stderr).unwrap().trim(), "gui_unavailable");
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}
#[test]
#[ignore = "mandatory native GUI qualification; run explicitly on selected graphical executor"]
fn production_launch_owns_browser_bootstrap_until_exit() {
    let scratch = tempfile::tempdir().unwrap();
    let binary = binary();
    use sha2::Digest;
    let digest: String = sha2::Sha256::digest(std::fs::read(&binary).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let description =
        Command::new(&binary).env_clear().arg("--build-description").output().unwrap();
    assert!(description.status.success());
    let description: serde_json::Value = serde_json::from_slice(&description.stdout).unwrap();
    println!(
        "native GUI artifact: sha256={digest} build={} profile={} fixture_control={}",
        description["build"], description["profile"], description["fixture_control"]
    );
    let mut command = Command::new(&binary);
    command.env("TMPDIR", scratch.path()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "Inspector is running locally.");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut consumed = false;
    while Instant::now() < deadline {
        let dirs = std::fs::read_dir(scratch.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        if dirs.iter().any(|p| p.join("browser").is_dir() && !p.join("bootstrap.html").exists()) {
            consumed = true;
            break;
        }
        if child.try_wait().unwrap().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let stop = Instant::now();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(stop.elapsed() < Duration::from_secs(30));
    assert!(consumed, "real selected browser must consume the private bootstrap");
    assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
}
#[test]
fn installed_identity_exposes_own_version_in_help_and_description() {
    let executable = binary();
    let help = Command::new(&executable).env_clear().arg("--help").output().unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains(concat!("fathomdb-inspector ", env!("CARGO_PKG_VERSION"))),
        "help must identify the Inspector package version"
    );
    let description =
        Command::new(executable).env_clear().arg("--build-description").output().unwrap();
    assert!(description.status.success());
    let description: serde_json::Value = serde_json::from_slice(&description.stdout).unwrap();
    assert_eq!(
        description["version"],
        env!("CARGO_PKG_VERSION"),
        "build description must identify the Inspector package version"
    );
}

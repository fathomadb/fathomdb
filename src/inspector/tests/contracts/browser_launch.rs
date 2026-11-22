use crate::process::browser_command;
use std::{collections::BTreeMap, path::Path};
#[test]
fn browser_launch_has_separate_closed_gui_environment_and_private_profile() {
    let scratch = tempfile::tempdir().unwrap();
    let profile = scratch.path().join("browser");
    std::fs::create_dir(&profile).unwrap();
    let gui = BTreeMap::from([
        ("DISPLAY".into(), ":0".into()),
        ("XDG_RUNTIME_DIR".into(), scratch.path().display().to_string()),
        ("DBUS_SESSION_BUS_ADDRESS".into(), format!("unix:path={}/bus", scratch.path().display())),
        ("HTTPS_PROXY".into(), "http://hostile.invalid".into()),
        ("FATHOMDB_PERF_WRITER_PRAGMAS".into(), "application_id=999".into()),
    ]);
    let command =
        browser_command(Path::new("/usr/bin/env"), scratch.path(), &profile, &gui).unwrap();
    let entries: BTreeMap<_, _> = command
        .get_envs()
        .map(|(k, v)| (k.to_string_lossy().to_string(), v.unwrap().to_string_lossy().to_string()))
        .collect();
    assert_eq!(entries.len(), 6);
    assert_eq!(entries["DISPLAY"], ":0");
    assert_eq!(entries["TMPDIR"], scratch.path().display().to_string());
    assert_eq!(entries["LANG"], "C.UTF-8");
    assert!(!entries.contains_key("HTTPS_PROXY"));
    assert!(!entries.contains_key("FATHOMDB_PERF_WRITER_PRAGMAS"));
    let args = command.get_args().map(|v| v.to_string_lossy().to_string()).collect::<Vec<_>>();
    assert!(args.contains(&format!("--user-data-dir={}", profile.display())));
    assert!(args.contains(&"--disable-extensions".into()));
    let mut bad = gui.clone();
    bad.insert("DISPLAY".into(), "remote.invalid:0".into());
    assert!(browser_command(Path::new("/usr/bin/env"), scratch.path(), &profile, &bad).is_err());
    assert!(browser_command(Path::new("env"), scratch.path(), &profile, &gui).is_err());
}

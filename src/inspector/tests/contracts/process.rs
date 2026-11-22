use crate::process::worker_command;
use std::path::Path;

#[test]
fn cleared_workers_execute_with_exact_allowlist_before_initialization() {
    let scratch = tempfile::tempdir().unwrap();
    for model in [false, true] {
        let command = worker_command(Path::new("/usr/bin/env"), scratch.path(), model).unwrap();
        let entries: std::collections::BTreeMap<_, _> = command
            .get_envs()
            .map(|(k, v)| {
                (k.to_string_lossy().to_string(), v.map(|v| v.to_string_lossy().to_string()))
            })
            .collect();
        assert_eq!(entries.len(), if model { 5 } else { 3 });
        let output = worker_command(Path::new("/usr/bin/env"), scratch.path(), model)
            .unwrap()
            .output()
            .unwrap();
        assert!(output.status.success());
        let actual: std::collections::BTreeSet<_> =
            String::from_utf8(output.stdout).unwrap().lines().map(String::from).collect();
        let mut expected = std::collections::BTreeSet::from([
            format!("TMPDIR={}", scratch.path().display()),
            "LANG=C.UTF-8".into(),
            "LC_ALL=C.UTF-8".into(),
        ]);
        if model {
            expected.extend(["FATHOMDB_EMBED_DEVICE=cpu".into(), "RAYON_NUM_THREADS=1".into()]);
        }
        assert_eq!(actual, expected);
    }
    assert!(worker_command(Path::new("env"), scratch.path(), false).is_err());
    assert!(worker_command(Path::new("/usr/bin/env"), Path::new("relative"), false).is_err());
}

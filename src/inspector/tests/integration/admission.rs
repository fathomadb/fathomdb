use crate::{admission::preflight, fixtures::database};
use std::fs;
use std::path::Path;
fn bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap()
}

#[test]
fn current_existing_admission_and_refusals_preserve_product() {
    let dir = tempfile::tempdir().unwrap();
    let path = database(dir.path());
    let original = bytes(&path);
    assert_eq!(preflight(&path).unwrap().path, path);
    assert_eq!(bytes(&path), original);
    assert_eq!(preflight(&dir.path().join("missing.sqlite")).unwrap_err(), "missing");
    let empty = dir.path().join("empty.sqlite");
    fs::write(&empty, b"").unwrap();
    assert_eq!(preflight(&empty).unwrap_err(), "empty");
    assert!(bytes(&empty).is_empty());
    for suffix in ["-wal", "-journal"] {
        let sidecar = format!("{}{suffix}", path.display());
        fs::write(&sidecar, b"preserve sidecar").unwrap();
        assert_eq!(preflight(&path).unwrap_err(), "initial_sidecar");
        assert_eq!(bytes(&path), original);
        assert_eq!(bytes(Path::new(&sidecar)), b"preserve sidecar");
        fs::write(&sidecar, b"").unwrap();
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("{}.lock", path.display()))
        .unwrap();
    lock.lock().unwrap();
    assert_eq!(preflight(&path).unwrap_err(), "locked");
    drop(lock);
    for version in [33_u32, 35] {
        let target = dir.path().join(format!("schema{version}.sqlite"));
        let mut contents = original.clone();
        contents[60..64].copy_from_slice(&version.to_be_bytes());
        fs::write(&target, &contents).unwrap();
        fs::write(format!("{}.lock", target.display()), b"lock sentinel").unwrap();
        assert_eq!(preflight(&target).unwrap_err(), "unsupported_schema");
        assert_eq!(bytes(&target), contents);
    }
    let no_lock = dir.path().join("no-lock.sqlite");
    fs::write(&no_lock, &original).unwrap();
    assert_eq!(preflight(&no_lock).unwrap_err(), "missing_lock");
    assert_eq!(bytes(&no_lock), original);
}

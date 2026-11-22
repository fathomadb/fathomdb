use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::FromRawFd;
use std::path::Path;
fn seal(path: &Path, pin: &str, max: u64) -> Result<File, &'static str> {
    let mut source = File::open(path).map_err(|_| "model_unavailable")?;
    if !source.metadata().map_err(|_| "model_unavailable")?.is_file() {
        return Err("model_unavailable");
    }
    let fd = unsafe {
        libc::memfd_create(
            c"inspector-pinned-asset".as_ptr(),
            libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
        )
    };
    if fd < 0 {
        return Err("model_unavailable");
    }
    let mut target = unsafe { File::from_raw_fd(fd) };
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        let n = source.read(&mut buffer).map_err(|_| "model_unavailable")?;
        if n == 0 {
            break;
        }
        bytes = bytes.checked_add(n as u64).ok_or("model_unavailable")?;
        if bytes > max {
            return Err("model_unavailable");
        }
        target.write_all(&buffer[..n]).map_err(|_| "model_unavailable")?;
    }
    let seals = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;
    if unsafe { libc::fcntl(fd, libc::F_ADD_SEALS, seals) } != 0
        || unsafe { libc::fcntl(fd, libc::F_GET_SEALS) } != seals
    {
        return Err("model_unavailable");
    }
    target.seek(SeekFrom::Start(0)).map_err(|_| "model_unavailable")?;
    let mut hash = Sha256::new();
    loop {
        let n = target.read(&mut buffer).map_err(|_| "model_unavailable")?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if actual != pin {
        return Err("model_unavailable");
    }
    target.seek(SeekFrom::Start(0)).map_err(|_| "model_unavailable")?;
    Ok(target)
}
pub(crate) struct Snapshot {
    _files: Vec<File>,
}
impl Snapshot {
    pub(crate) fn new(source: &Path, view: &Path) -> Result<Self, &'static str> {
        use std::os::fd::AsRawFd;
        let description: serde_json::Value =
            serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/build-description.json")))
                .map_err(|_| "model_unavailable")?;
        let identity = format!(
            "{}@{}",
            description["model_repository"].as_str().ok_or("model_unavailable")?,
            description["model_revision"].as_str().ok_or("model_unavailable")?
        );
        let digest: String =
            Sha256::digest(identity.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        let relative = Path::new("fathomdb/embedders").join(&digest[..12]);
        let target = view.join(&relative);
        std::fs::create_dir_all(&target).map_err(|_| "model_unavailable")?;
        let mut files = vec![];
        for (name, max) in [
            ("config.json", 65536),
            ("tokenizer.json", 4 * 1024 * 1024),
            ("model.safetensors", 150 * 1024 * 1024),
        ] {
            let file = seal(
                &source.join(&relative).join(name),
                description["assets"][name].as_str().ok_or("model_unavailable")?,
                max,
            )?;
            std::os::unix::fs::symlink(
                format!("/proc/self/fd/{}", file.as_raw_fd()),
                target.join(name),
            )
            .map_err(|_| "model_unavailable")?;
            files.push(file);
        }
        Ok(Self { _files: files })
    }
}
pub(crate) fn ready(path: &Path) -> bool {
    let Ok(mut c) = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return false;
    };
    let start = std::time::Instant::now();
    let mut ticks = 0;
    if c.progress_handler(
        1000,
        Some(move || {
            ticks += 1;
            ticks > 100000 || start.elapsed().as_secs() >= 5
        }),
    )
    .is_err()
    {
        return false;
    }
    let identity:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM _fathomdb_embedder_profiles WHERE profile='default' AND name=?1 AND revision=?2 AND dimension=?3)",rusqlite::params![env!("INSPECTOR_DEFAULT_NAME"),env!("INSPECTOR_DEFAULT_REVISION"),env!("INSPECTOR_DEFAULT_DIMENSION").parse::<u32>().unwrap()],|r|r.get(0)).unwrap_or(false);
    if !identity {
        return false;
    }
    let probes: i64 = c
        .query_row("SELECT count(*) FROM (SELECT 1 FROM _fathomdb_embed_probe LIMIT 46)", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    if probes != 45 {
        return false;
    }
    let debt:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM canonical_nodes n JOIN _fathomdb_vector_kinds k ON k.kind=n.kind WHERE octet_length(n.body)>0 AND (NOT EXISTS(SELECT 1 FROM _fathomdb_projection_terminal t WHERE t.write_cursor=n.write_cursor AND t.state='up_to_date') OR NOT EXISTS(SELECT 1 FROM _fathomdb_vector_rows r JOIN _fathomdb_vector_spans s ON s.vector_rowid=r.rowid JOIN vector_default v ON v.rowid=r.rowid WHERE r.write_cursor=n.write_cursor)))",[],|r|r.get(0)).unwrap_or(true);
    !debt
}
#[repr(C)]
struct Ruleset {
    access: u64,
}
#[repr(C, packed)]
struct Beneath {
    access: u64,
    fd: i32,
}
pub(crate) fn confine(writable: &[&Path], snapshot: &Path) -> Result<(), &'static str> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;
    let snapshot = snapshot.canonicalize().map_err(|_| "model_confinement")?;
    let roots = writable
        .iter()
        .map(|p| p.canonicalize().map_err(|_| "model_confinement"))
        .collect::<Result<Vec<_>, _>>()?;
    if roots.iter().any(|p| snapshot.starts_with(p)) {
        return Err("model_confinement");
    }
    let abi =
        unsafe { libc::syscall(libc::SYS_landlock_create_ruleset, std::ptr::null::<u8>(), 0, 1) };
    if abi < 3 {
        return Err("model_confinement");
    }
    let rights = (1u64 << 1) | (((1u64 << 15) - 1) & !((1u64 << 4) - 1));
    let ruleset = Ruleset { access: rights };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &ruleset,
            std::mem::size_of::<Ruleset>(),
            0,
        )
    };
    if fd < 0 {
        return Err("model_confinement");
    }
    let ruleset_fd = unsafe { File::from_raw_fd(fd as i32) };
    for root in &roots {
        let dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(root)
            .map_err(|_| "model_confinement")?;
        let rule = Beneath { access: rights, fd: dir.as_raw_fd() };
        if unsafe {
            libc::syscall(libc::SYS_landlock_add_rule, ruleset_fd.as_raw_fd(), 1, &rule, 0)
        } != 0
        {
            return Err("model_confinement");
        }
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
        || unsafe { libc::syscall(libc::SYS_landlock_restrict_self, ruleset_fd.as_raw_fd(), 0) }
            != 0
    {
        return Err("model_confinement");
    }
    deny_network()
}
fn deny_network() -> Result<(), &'static str> {
    #[cfg(target_arch = "x86_64")]
    const ARCH: u32 = 0xc000003e;
    #[cfg(target_arch = "aarch64")]
    const ARCH: u32 = 0xc00000b7;
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    return Err("model_confinement");
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    {
        let instruction = |code, jt, jf, k| libc::sock_filter { code, jt, jf, k };
        let mut filter = vec![
            instruction(0x20, 0, 0, 4),
            instruction(0x15, 1, 0, ARCH),
            instruction(0x06, 0, 0, 0x80000000),
            instruction(0x20, 0, 0, 0),
            instruction(0x35, 0, 1, 0x40000000),
            instruction(0x06, 0, 0, 0x00050000 | libc::EACCES as u32),
        ];
        for syscall in [libc::SYS_socket, libc::SYS_socketpair, libc::SYS_connect] {
            filter.push(instruction(0x15, 0, 1, syscall as u32));
            filter.push(instruction(0x06, 0, 0, 0x00050000 | libc::EACCES as u32));
        }
        filter.push(instruction(0x06, 0, 0, 0x7fff0000));
        let program = libc::sock_fprog { len: filter.len() as u16, filter: filter.as_mut_ptr() };
        if unsafe { libc::prctl(libc::PR_SET_SECCOMP, 2, &program, 0, 0) } != 0 {
            return Err("model_confinement");
        }
        Ok(())
    }
}
fn protected(name: *const std::os::raw::c_char) -> bool {
    if name.is_null() {
        return false;
    }
    let name = unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes();
    name.starts_with(b"vector_default")
        || name.starts_with(b"_fathomdb_vector_")
        || name == b"_fathomdb_embedder_profiles"
        || name == b"_fathomdb_embed_probe"
}
unsafe extern "C" fn authorize(
    _context: *mut std::ffi::c_void,
    action: i32,
    a: *const std::os::raw::c_char,
    b: *const std::os::raw::c_char,
    _db: *const std::os::raw::c_char,
    _source: *const std::os::raw::c_char,
) -> i32 {
    use rusqlite::ffi::*;
    // Production boot graft always prepares an idempotent enrollment INSERT.
    // This table has no embedding bytes; all other dense/probe writes stay denied.
    if action == SQLITE_INSERT
        && !a.is_null()
        && unsafe { std::ffi::CStr::from_ptr(a) }.to_bytes() == b"_fathomdb_vector_kinds"
    {
        return SQLITE_OK;
    }
    let denied = match action {
        SQLITE_INSERT | SQLITE_UPDATE | SQLITE_DELETE | SQLITE_DROP_TABLE | SQLITE_DROP_VTABLE => {
            protected(a)
        }
        SQLITE_ALTER_TABLE | SQLITE_DROP_INDEX | SQLITE_DROP_TRIGGER => protected(b),
        _ => false,
    };
    if denied {
        SQLITE_DENY
    } else {
        SQLITE_OK
    }
}
unsafe extern "C" fn guard_connection(
    connection: *mut rusqlite::ffi::sqlite3,
    _error: *mut *mut std::os::raw::c_char,
    _api: *const rusqlite::ffi::sqlite3_api_routines,
) -> i32 {
    unsafe {
        rusqlite::ffi::sqlite3_set_authorizer(connection, Some(authorize), std::ptr::null_mut())
    }
}
pub(crate) fn deny_dense_writes() -> Result<(), &'static str> {
    let entry: unsafe extern "C" fn(
        *mut rusqlite::ffi::sqlite3,
        *mut *mut std::os::raw::c_char,
        *const rusqlite::ffi::sqlite3_api_routines,
    ) -> std::os::raw::c_int = guard_connection;
    if unsafe { rusqlite::ffi::sqlite3_auto_extension(Some(entry)) } != rusqlite::ffi::SQLITE_OK {
        return Err("model_confinement");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Seek, SeekFrom, Write};
    const PIN: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    #[test]
    fn sealed_asset_stays_pinned_after_source_replacement_and_refuses_all_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("asset");
        std::fs::write(&path, b"abc").unwrap();
        let result = seal(&path, PIN, 3);
        assert!(result.is_ok(), "pinned asset must be sealed: {result:?}");
        let mut file = result.unwrap();
        std::fs::write(&path, b"xyz").unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let mut bytes = vec![];
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abc");
        assert!(file.write_all(b"x").is_err());
        assert!(file.set_len(0).is_err());
        assert!(file.set_len(100).is_err());
        assert!(seal(&path, PIN, 3).is_err());
        std::fs::write(&path, b"abcx").unwrap();
        assert!(seal(&path, PIN, 3).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(seal(&path, PIN, 3).is_err());
    }
}

#[cfg(test)]
mod boundary_tests {
    #[test]
    fn actual_confinement_blocks_cache_network_and_dense_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = crate::fixtures::database(dir.path());
        let scratch = tempfile::tempdir().unwrap();
        let cache = scratch.path().join("model");
        let work = scratch.path().join("work");
        std::fs::create_dir(&cache).unwrap();
        std::fs::create_dir(&work).unwrap();
        std::fs::write(cache.join("asset"), b"abc").unwrap();
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "worker::model::boundary_tests::confinement_child",
                "--ignored",
                "--nocapture",
            ])
            .env("INSPECTOR_BOUNDARY_DB", &path)
            .env("INSPECTOR_BOUNDARY_CACHE", &cache)
            .env("INSPECTOR_BOUNDARY_WORK", &work)
            .output()
            .unwrap();
        assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stdout));
        assert_eq!(std::fs::read(cache.join("asset")).unwrap(), b"abc");
    }
    #[test]
    #[ignore = "owned subprocess only; mutates its process security boundary"]
    fn confinement_child() {
        let path = std::path::PathBuf::from(std::env::var_os("INSPECTOR_BOUNDARY_DB").unwrap());
        let cache = std::path::PathBuf::from(std::env::var_os("INSPECTOR_BOUNDARY_CACHE").unwrap());
        let work = std::path::PathBuf::from(std::env::var_os("INSPECTOR_BOUNDARY_WORK").unwrap());
        super::confine(&[path.parent().unwrap(), &work], &cache).unwrap();
        assert!(
            std::fs::write(cache.join("asset"), b"bad").is_err(),
            "cache mutation must be denied"
        );
        assert!(std::fs::write(cache.join("new"), b"new").is_err());
        assert!(std::fs::remove_file(cache.join("asset")).is_err());
        std::fs::write(work.join("allowed"), b"ok").unwrap();
        for domain in [libc::AF_INET, libc::AF_INET6, libc::AF_UNIX] {
            assert_eq!(
                unsafe { libc::socket(domain, libc::SOCK_STREAM, 0) },
                -1,
                "new sockets must be denied"
            );
        }
        super::deny_dense_writes().unwrap();
        let c = rusqlite::Connection::open(&path).unwrap();
        let result = c.execute("DELETE FROM _fathomdb_embed_probe", []);
        assert!(result.is_err(), "probe embedding mutations must be denied");
        assert!(c.execute("DELETE FROM _fathomdb_vector_rows", []).is_err());
        assert!(c.execute("DROP TABLE _fathomdb_vector_spans", []).is_err());
        let _: i64 = c.query_row("SELECT count(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();
    }
}

use crate::{
    policy::Policy,
    process::{browser_command, GUI_KEYS},
    server::Server,
    storage::nonce,
};
use std::{
    collections::BTreeMap,
    fs::DirBuilder,
    os::unix::{
        fs::{DirBuilderExt, MetadataExt},
        process::CommandExt,
    },
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
static EXIT: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: libc::c_int) {
    EXIT.store(true, Ordering::SeqCst);
}
struct Scratch {
    path: PathBuf,
    device: u64,
    inode: u64,
}
impl Scratch {
    fn new() -> Result<Self, &'static str> {
        let base = std::env::temp_dir().canonicalize().map_err(|_| "scratch_unavailable")?;
        if base.as_os_str().as_encoded_bytes().len() > 39 {
            return Err("scratch_path_too_long");
        }
        let path = base.join(format!("fdi-{}", &nonce()[..16]));
        DirBuilder::new().mode(0o700).create(&path).map_err(|_| "scratch_unavailable")?;
        let metadata = path.metadata().map_err(|_| "scratch_unavailable")?;
        Ok(Self { path, device: metadata.dev(), inode: metadata.ino() })
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.path.metadata().is_ok_and(|m| m.dev() == self.device && m.ino() == self.inode) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
struct Browser {
    child: Child,
}
impl Drop for Browser {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}
fn selected_browser() -> Result<PathBuf, &'static str> {
    let mut candidates = vec![];
    if let Some(home) = std::env::var_os("HOME") {
        let home = Path::new(&home);
        if home.is_absolute() {
            candidates.push(home.join(".cache/ms-playwright/chromium-1248/chrome-linux64/chrome"));
        }
    }
    candidates
        .extend([PathBuf::from("/usr/bin/google-chrome"), PathBuf::from("/usr/bin/chromium")]);
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or("browser_unavailable")?
        .canonicalize()
        .map_err(|_| "browser_unavailable")
}
fn file_url(path: &Path) -> String {
    format!(
        "file://{}",
        path.as_os_str()
            .as_encoded_bytes()
            .iter()
            .map(|b| if b.is_ascii_alphanumeric() || b"/-_.".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            })
            .collect::<String>()
    )
}
pub(crate) fn run(path: Option<String>, policy: Policy) -> Result<(), &'static str> {
    let gui: BTreeMap<_, _> = GUI_KEYS
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key.to_string(), value)))
        .collect();
    if !gui.contains_key("DISPLAY") && !gui.contains_key("WAYLAND_DISPLAY") {
        return Err("gui_unavailable");
    }
    let executable = selected_browser()?;
    let scratch = Scratch::new()?;
    let profile = scratch.path.join("browser");
    DirBuilder::new().mode(0o700).create(&profile).map_err(|_| "scratch_unavailable")?;
    let mut command = browser_command(&executable, &scratch.path, &profile, &gui)?;
    let server = Server::start(&scratch.path, path, policy)?;
    if !server.address.starts_with("127.0.0.1:") {
        return Err("invalid_listener");
    }
    command
        .arg(file_url(&server.bootstrap))
        .arg(format!("--disk-cache-dir={}", profile.join("cache").display()))
        .arg(format!("--crash-dumps-dir={}", profile.join("crash").display()))
        .arg("--disable-crash-reporter")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let supervisor = std::process::id() as libc::pid_t;
    unsafe {
        command.pre_exec(move || {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) < 0
                || libc::getppid() != supervisor
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    EXIT.store(false, Ordering::SeqCst);
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, stop as *const () as libc::sighandler_t);
    }
    let mut browser = Browser { child: command.spawn().map_err(|_| "browser_start_failed")? };
    println!("Inspector is running locally.");
    let result = loop {
        if EXIT.load(Ordering::SeqCst) || server.is_stopped() {
            break Ok(());
        }
        if let Some(status) = browser.child.try_wait().map_err(|_| "browser_failed")? {
            break if status.success() { Ok(()) } else { Err("browser_failed") };
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    drop(browser);
    drop(server);
    drop(scratch);
    result
}

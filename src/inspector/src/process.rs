use crate::failure::Failure;
use std::path::Path;
use std::process::Command;

pub(crate) fn worker_command(
    executable: &Path,
    scratch: &Path,
    model: bool,
) -> Result<Command, &'static str> {
    if !executable.is_absolute()
        || !executable.is_file()
        || !scratch.is_absolute()
        || !scratch.is_dir()
        || scratch.canonicalize().map_err(|_| "invalid_scratch")? != scratch
    {
        return Err("invalid_process_paths");
    }
    let mut command = Command::new(executable);
    command.env_clear().env("TMPDIR", scratch).env("LANG", "C.UTF-8").env("LC_ALL", "C.UTF-8");
    if model {
        command.env("FATHOMDB_EMBED_DEVICE", "cpu").env("RAYON_NUM_THREADS", "1");
    }
    Ok(command)
}

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::{net::UnixStream, process::CommandExt};
use std::process::{Child, Stdio};
use std::time::{Duration, Instant};
pub(crate) const BUILD: &str = env!("INSPECTOR_BUILD");
pub(crate) const MAX_FRAME: usize = 2 * 1024 * 1024;
pub(crate) fn receive(stream: &mut UnixStream) -> Result<Value, &'static str> {
    let mut size = [0_u8; 4];
    stream.read_exact(&mut size).map_err(|_| "worker_disconnected")?;
    let n = u32::from_be_bytes(size) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err("invalid_frame");
    }
    let mut body = vec![0; n];
    stream.read_exact(&mut body).map_err(|_| "worker_disconnected")?;
    serde_json::from_slice(&body).map_err(|_| "invalid_frame")
}
pub(crate) fn send(stream: &mut UnixStream, value: &Value) -> Result<(), &'static str> {
    let bytes = serde_json::to_vec(value).map_err(|_| "invalid_frame")?;
    if bytes.len() > MAX_FRAME {
        return Err("response_byte_cap");
    }
    stream
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .and_then(|_| stream.write_all(&bytes))
        .map_err(|_| "worker_disconnected")
}
struct PrivateScratch(std::path::PathBuf);
impl Drop for PrivateScratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
pub(crate) struct OwnedWorker {
    pub(crate) pid: u32,
    pub(crate) attachment: String,
    child: Child,
    stream: UnixStream,
    stopped: bool,
    _scratch: PrivateScratch,
}
impl OwnedWorker {
    #[allow(dead_code)]
    pub(crate) fn spawn(exe: &Path, scratch: &Path, initial: Value) -> Result<Self, Failure> {
        Self::spawn_registered(exe, scratch, initial, |_| {})
    }
    pub(crate) fn spawn_registered(
        exe: &Path,
        scratch: &Path,
        mut initial: Value,
        register: impl FnOnce(u32),
    ) -> Result<Self, Failure> {
        let (parent, child_socket) = UnixStream::pair().map_err(|_| "ipc_failed")?;
        let fd = child_socket.as_raw_fd();
        let candidate = std::env::var_os("XDG_CACHE_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".cache"))
            })
            .filter(|p| p.is_absolute());
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| "worker_start_failed")?;
        let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let private = scratch.join(format!("session-{suffix}"));
        let work = private.join("work");
        let view = private.join("view");
        std::fs::create_dir(&private).map_err(|_| "worker_start_failed")?;
        let private_scratch = PrivateScratch(private.clone());
        std::fs::create_dir(&work).map_err(|_| "worker_start_failed")?;
        std::fs::create_dir(&view).map_err(|_| "worker_start_failed")?;
        initial["model_cache"] = json!(candidate);
        initial["model_view"] = json!(view);
        let mut command = worker_command(exe, &work, true)?;
        command.env("XDG_CACHE_HOME", &view);
        command
            .arg("--session-worker")
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
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "supervisor unavailable",
                    ));
                }
                if fd != 3 && libc::dup2(fd, 3) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::syscall(libc::SYS_close_range, 4u32, u32::MAX, 0u32) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(3, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                let limit = libc::rlimit {
                    rlim_cur: 4 * 1024 * 1024 * 1024,
                    rlim_max: 4 * 1024 * 1024 * 1024,
                };
                if libc::setrlimit(libc::RLIMIT_AS, &limit) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().map_err(|_| "worker_start_failed")?;
        let mut worker = Self {
            pid: child.id(),
            attachment: String::new(),
            child,
            stream: parent,
            stopped: false,
            _scratch: private_scratch,
        };
        register(worker.pid);
        drop(child_socket);
        worker.stream.set_read_timeout(Some(Duration::from_secs(30))).map_err(|_| "ipc_failed")?;
        worker.stream.set_write_timeout(Some(Duration::from_secs(30))).map_err(|_| "ipc_failed")?;
        initial["build"] = Value::String(BUILD.into());
        initial["protocol"] = Value::from(1);
        let ready = worker.exchange(initial)?;
        if ready["status"] != "complete" {
            return Err(Failure::from("startup_failed_effects_possible"));
        }
        worker.attachment = ready["attachment"]
            .as_str()
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or("invalid_pair")?
            .to_string();
        Ok(worker)
    }
    pub(crate) fn call(&mut self, request: Value) -> Result<Value, Failure> {
        if request["op"] == "export" {
            return self.export(request);
        }
        self.exchange(serde_json::json!({"origin":"read","request":request}))
    }
    fn export(&mut self, request: Value) -> Result<Value, Failure> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        if request.as_object().is_none_or(|v| v.len() != 3) {
            return Err(Failure::read("unsupported_operation"));
        }
        let handle = request["handle"].as_str().ok_or("invalid_range")?;
        let destination = request["destination"].as_str().ok_or("invalid_destination")?;
        let prepared = self.exchange(
            json!({"origin":"prepare_export","handle":handle,"destination":destination}),
        )?;
        let destination = Path::new(prepared["destination"].as_str().ok_or("invalid_destination")?);
        let parent = destination.parent().ok_or("invalid_destination")?;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(parent)
            .map_err(|_| "destination_unavailable")?;
        let metadata = directory.metadata().map_err(|_| "destination_unavailable")?;
        if prepared["parent_device"].as_str().and_then(|v| v.parse::<u64>().ok())
            != Some(metadata.dev())
            || prepared["parent_inode"].as_str().and_then(|v| v.parse::<u64>().ok())
                != Some(metadata.ino())
        {
            return Err(Failure::read("invalid_destination"));
        }
        let name = std::ffi::CString::new(
            destination.file_name().ok_or("invalid_destination")?.as_encoded_bytes(),
        )
        .map_err(|_| "invalid_destination")?;
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if fd < 0 {
            return Err(Failure::read("destination_unavailable"));
        }
        let output = unsafe { std::fs::File::from_raw_fd(fd) };
        send(&mut self.stream, &json!({"origin":"export_sink","nonce":prepared["nonce"]}))?;
        send_fd(&self.stream, output.as_raw_fd())?;
        let response = receive(&mut self.stream)?;
        match response["status"].as_str() {
            Some("complete" | "partial") => Ok(response),
            Some(_) => Err(Failure::decode(&response).unwrap_or_else(Failure::from)),
            None => Err(Failure::from("invalid_frame")),
        }
    }
    #[cfg(feature = "fixture-control")]
    pub(crate) fn fixture(&mut self, control: Value) -> Result<Value, Failure> {
        self.exchange(serde_json::json!({"origin":"fixture","control":control}))
    }
    fn exchange(&mut self, request: Value) -> Result<Value, Failure> {
        send(&mut self.stream, &request)?;
        let response = receive(&mut self.stream)?;
        match response["status"].as_str() {
            Some("complete" | "partial") => {}
            Some(_) => return Err(Failure::decode(&response).unwrap_or_else(Failure::from)),
            None => return Err(Failure::from("invalid_frame")),
        }
        Ok(response)
    }
    pub(crate) fn close(mut self) -> Result<(), Failure> {
        self.stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|_| "ipc_failed")?;
        let response = self.call(serde_json::json!({"op":"close"}));
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.child.try_wait().map_err(|_| "reap_failed")?.is_some() {
                self.stopped = true;
                return response.map(|_| ());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(Failure::from("forced_shutdown"))
    }
}
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if !self.stopped {
            unsafe {
                libc::kill(-(self.pid as i32), libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
    }
}
pub(crate) fn inherited_socket() -> Result<UnixStream, &'static str> {
    let mut kind = 0_i32;
    let mut n = std::mem::size_of::<i32>() as libc::socklen_t;
    if unsafe {
        libc::getsockopt(3, libc::SOL_SOCKET, libc::SO_TYPE, (&mut kind as *mut i32).cast(), &mut n)
    } != 0
        || kind != libc::SOCK_STREAM
    {
        return Err("owned_ipc_required");
    }
    Ok(unsafe { UnixStream::from_raw_fd(3) })
}

pub(crate) const GUI_KEYS: [&str; 5] =
    ["DISPLAY", "WAYLAND_DISPLAY", "XDG_RUNTIME_DIR", "XAUTHORITY", "DBUS_SESSION_BUS_ADDRESS"];
pub(crate) fn browser_command(
    executable: &Path,
    scratch: &Path,
    profile: &Path,
    gui: &std::collections::BTreeMap<String, String>,
) -> Result<Command, &'static str> {
    if !profile.is_absolute()
        || !profile.is_dir()
        || profile.canonicalize().map_err(|_| "invalid_profile")? != profile
        || profile.parent() != Some(scratch)
    {
        return Err("invalid_profile");
    }
    let mut command = worker_command(executable, scratch, false)?;
    for key in GUI_KEYS {
        let Some(value) = gui.get(key) else {
            continue;
        };
        if value.is_empty() || value.len() > 4096 || value.bytes().any(|b| b < 32) {
            return Err("invalid_gui_environment");
        }
        let valid = match key {
            "DISPLAY" => value.strip_prefix(':').is_some_and(|v| {
                !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            }),
            "WAYLAND_DISPLAY" => {
                value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                    && value != "."
                    && value != ".."
            }
            "XDG_RUNTIME_DIR" => {
                let path = Path::new(value);
                path.is_absolute() && path.is_dir() && path.canonicalize().is_ok_and(|p| p == path)
            }
            "XAUTHORITY" => {
                let path = Path::new(value);
                path.is_absolute() && path.is_file() && path.canonicalize().is_ok_and(|p| p == path)
            }
            "DBUS_SESSION_BUS_ADDRESS" => {
                value
                    .strip_prefix("unix:path=")
                    .is_some_and(|v| v.starts_with('/') && !v.contains(';'))
                    || value
                        .strip_prefix("unix:abstract=")
                        .is_some_and(|v| !v.is_empty() && !v.contains([';', ':']))
            }
            _ => false,
        };
        if !valid {
            return Err("invalid_gui_environment");
        }
        command.env(key, value);
    }
    command.arg(format!("--user-data-dir={}", profile.display())).args([
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--disable-background-networking",
        "--disable-sync",
        "--no-proxy-server",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-domain-reliability",
        "--disable-client-side-phishing-detection",
        "--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication",
    ]);
    Ok(command)
}

pub(crate) fn send_fd(stream: &UnixStream, fd: i32) -> Result<(), &'static str> {
    let mut marker = b'F';
    let mut iov = libc::iovec { iov_base: (&mut marker as *mut u8).cast(), iov_len: 1 };
    let mut control = [0usize; 4];
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen =
        unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) } as usize;
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        if header.is_null() {
            return Err("invalid_sink");
        }
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as usize;
        std::ptr::write(libc::CMSG_DATA(header).cast::<i32>(), fd);
        if libc::sendmsg(stream.as_raw_fd(), &message, libc::MSG_NOSIGNAL) != 1 {
            return Err("worker_disconnected");
        }
    }
    Ok(())
}
pub(crate) fn receive_fd(stream: &UnixStream) -> Result<std::fs::File, &'static str> {
    let mut marker = 0u8;
    let mut iov = libc::iovec { iov_base: (&mut marker as *mut u8).cast(), iov_len: 1 };
    let mut control = [0usize; 8];
    let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
    message.msg_iov = &mut iov;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = std::mem::size_of_val(&control);
    let received =
        unsafe { libc::recvmsg(stream.as_raw_fd(), &mut message, libc::MSG_CMSG_CLOEXEC) };
    let mut files = vec![];
    let mut valid = true;
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let base = libc::CMSG_LEN(0) as usize;
                let length = (*header).cmsg_len.saturating_sub(base);
                if !length.is_multiple_of(std::mem::size_of::<i32>()) {
                    valid = false;
                }
                for index in 0..length / std::mem::size_of::<i32>() {
                    files.push(std::fs::File::from_raw_fd(std::ptr::read_unaligned(
                        libc::CMSG_DATA(header).cast::<i32>().add(index),
                    )));
                }
            } else {
                valid = false;
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if received != 1
        || marker != b'F'
        || message.msg_flags & (libc::MSG_CTRUNC | libc::MSG_TRUNC) != 0
        || files.len() != 1
        || !valid
    {
        return Err("invalid_sink");
    }
    let file = files.pop().unwrap();
    if !file.metadata().map_err(|_| "invalid_sink")?.is_file()
        || unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) } & libc::O_ACCMODE
            == libc::O_RDONLY
    {
        return Err("invalid_sink");
    }
    Ok(file)
}

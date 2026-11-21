use crate::failure::Failure;
use crate::{http, policy::Policy, process::BUILD, storage::nonce};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    io::Write,
    net::{TcpListener, TcpStream},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
struct Core {
    secret: Option<String>,
    attachment: Option<String>,
}
enum JobKind {
    Read(Value),
    #[cfg(feature = "fixture-control")]
    Fixture(Value),
}
struct Job {
    kind: JobKind,
    attachment: Option<String>,
    epoch: u64,
    deadline: Instant,
    reply: mpsc::SyncSender<Value>,
}
struct State {
    address: String,
    launch: String,
    cookie: String,
    core: Mutex<Core>,
    bootstrap: PathBuf,
    script: PathBuf,
    started: Instant,
    path: Option<String>,
    policy: Policy,
    stop: Arc<AtomicBool>,
    jobs: mpsc::SyncSender<Job>,
    epoch: AtomicU64,
    pid: AtomicI32,
    executing: AtomicBool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    build: String,
    protocol: u32,
    launch: String,
    attachment: Option<String>,
    request: Value,
}
pub(crate) struct Server {
    pub(crate) address: String,
    pub(crate) bootstrap: PathBuf,
    state: Arc<State>,
    thread: Option<JoinHandle<()>>,
}
impl Server {
    #[cfg(feature = "fixture-control")]
    #[allow(dead_code)]
    pub(crate) fn fixture(&self, control: Value) -> Value {
        let (reply, receive) = mpsc::sync_channel(1);
        let job = Job {
            kind: JobKind::Fixture(control),
            attachment: self.state.core.lock().unwrap().attachment.clone(),
            epoch: self.state.epoch.load(Ordering::SeqCst),
            deadline: Instant::now() + Duration::from_secs(30),
            reply,
        };
        if self.state.jobs.try_send(job).is_err() {
            return json!({"status":"busy"});
        }
        receive
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| json!({"status":"budget_exhausted"}))
    }
    pub(crate) fn is_stopped(&self) -> bool {
        self.state.stop.load(Ordering::SeqCst)
    }
    pub(crate) fn start(
        scratch: &Path,
        path: Option<String>,
        policy: Policy,
    ) -> Result<Self, &'static str> {
        Self::start_with_executable(
            scratch,
            path,
            policy,
            &std::env::current_exe().map_err(|_| "executable_unavailable")?,
        )
    }
    pub(crate) fn start_with_executable(
        scratch: &Path,
        path: Option<String>,
        policy: Policy,
        executable: &Path,
    ) -> Result<Self, &'static str> {
        if !scratch.is_absolute()
            || scratch.canonicalize().map_err(|_| "invalid_scratch")? != scratch
        {
            return Err("invalid_scratch");
        }
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|_| "bind_failed")?;
        listener.set_nonblocking(true).map_err(|_| "bind_failed")?;
        let address = listener.local_addr().map_err(|_| "bind_failed")?.to_string();
        let secret = nonce();
        let launch = nonce();
        let cookie = format!("fdb_{}={}", &launch[..16], nonce());
        let bootstrap = scratch.join("bootstrap.html");
        let script = scratch.join("bootstrap.js");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&bootstrap)
            .map_err(|_| "bootstrap_failed")?;
        write!(file,"<!doctype html><meta charset=\"utf-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; script-src 'self'; form-action http://{address}\"><title>Open Inspector</title><form id=\"handoff\" method=\"post\" action=\"http://{address}/bootstrap\"><input type=\"hidden\" name=\"secret\" value=\"{secret}\"><button>Open Inspector</button></form><script src=\"bootstrap.js\"></script>").map_err(|_|"bootstrap_failed")?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&script)
            .map_err(|_| "bootstrap_failed")?;
        file.write_all(b"document.getElementById('handoff').requestSubmit();")
            .map_err(|_| "bootstrap_failed")?;
        let stop = Arc::new(AtomicBool::new(false));
        let (jobs, queue) = mpsc::sync_channel(8);
        let state = Arc::new(State {
            address: address.clone(),
            launch,
            cookie,
            core: Mutex::new(Core { secret: Some(secret), attachment: None }),
            bootstrap: bootstrap.clone(),
            script,
            started: Instant::now(),
            path,
            policy,
            stop: stop.clone(),
            jobs,
            epoch: AtomicU64::new(0),
            pid: AtomicI32::new(0),
            executing: AtomicBool::new(false),
        });
        let executor_state = state.clone();
        let executable = executable.canonicalize().map_err(|_| "executable_unavailable")?;
        let scratch = scratch.to_path_buf();
        let executor =
            thread::spawn(move || execute(queue, &executor_state, &executable, &scratch));
        let server_state = state.clone();
        let active = Arc::new(AtomicUsize::new(0));
        let thread = thread::spawn(move || {
            let mut handlers: Vec<JoinHandle<()>> = vec![];
            while !state.stop.load(Ordering::SeqCst) {
                if state.started.elapsed() > Duration::from_secs(60) {
                    if let Ok(mut core) = state.core.lock() {
                        core.secret = None;
                    }
                    let _ = std::fs::remove_file(&state.bootstrap);
                    let _ = std::fs::remove_file(&state.script);
                }
                match listener.accept() {
                    Ok((stream, peer)) => {
                        if !peer.ip().is_loopback() || active.load(Ordering::SeqCst) >= 16 {
                            let _ = http::respond(
                                stream,
                                503,
                                "application/json",
                                b"{\"status\":\"busy\"}",
                                "",
                            );
                            continue;
                        }
                        active.fetch_add(1, Ordering::SeqCst);
                        let state = state.clone();
                        let active = active.clone();
                        handlers.push(thread::spawn(move || {
                            let _ = handle(stream, &state);
                            active.fetch_sub(1, Ordering::SeqCst);
                        }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => break,
                }
                let mut i = 0;
                while i < handlers.len() {
                    if handlers[i].is_finished() {
                        let _ = handlers.swap_remove(i).join();
                    } else {
                        i += 1;
                    }
                }
            }
            let _ = executor.join();
            for handler in handlers {
                let _ = handler.join();
            }
            let _ = std::fs::remove_file(&state.bootstrap);
            let _ = std::fs::remove_file(&state.script);
        });
        Ok(Self { address, bootstrap, state: server_state, thread: Some(thread) })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::SeqCst);
        if self.state.executing.load(Ordering::SeqCst) {
            kill_session(&self.state);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn handle(mut stream: TcpStream, state: &State) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = match http::parse_socket(&mut stream) {
        Ok(r) => r,
        Err(_) => {
            return http::respond(stream, 400, "application/json", b"{\"status\":\"refused\"}", "")
        }
    };
    if request.headers.get("host") != Some(&state.address) {
        return denied(stream);
    }
    if request.method == "POST" && request.path == "/bootstrap" {
        let mut core = state.core.lock().unwrap();
        let valid = state.started.elapsed() < Duration::from_secs(60)
            && request.headers.get("origin").map(String::as_str) == Some("null")
            && request.headers.get("content-type").map(String::as_str)
                == Some("application/x-www-form-urlencoded")
            && core
                .secret
                .as_ref()
                .is_some_and(|s| request.body == format!("secret={s}").as_bytes());
        if !valid {
            return denied(stream);
        }
        core.secret = None;
        drop(core);
        let _ = std::fs::remove_file(&state.bootstrap);
        let _ = std::fs::remove_file(&state.script);
        return http::respond(
            stream,
            303,
            "text/plain",
            b"",
            &format!(
                "Set-Cookie: {}; HttpOnly; SameSite=Strict; Path=/\r\nLocation: /\r\n",
                state.cookie
            ),
        );
    }
    if request.method == "GET" {
        let asset = match request.path.as_str() {
            "/" => {
                Some(("text/html; charset=utf-8", include_str!("../assets/index.html").to_string()))
            }
            "/build.json" => Some((
                "application/json",
                include_str!(concat!(env!("OUT_DIR"), "/build-description.json")).to_string(),
            )),
            "/style.css" => {
                Some(("text/css; charset=utf-8", include_str!("../assets/style.css").to_string()))
            }
            "/comparison.mjs" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../assets/comparison.mjs").to_string(),
            )),
            "/presentation.mjs" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../assets/presentation.mjs").to_string(),
            )),
            "/app.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../assets/app.js").replace("__INSPECTOR_BUILD__", BUILD),
            )),
            _ => None,
        };
        if let Some((mime, body)) = asset {
            return http::respond(stream, 200, mime, body.as_bytes(), "");
        }
    }
    let authorized = request
        .headers
        .get("cookie")
        .is_some_and(|cookies| cookies.split(';').any(|c| c.trim() == state.cookie));
    let origin = format!("http://{}", state.address);
    if !authorized || request.headers.get("origin").is_some_and(|v| v != &origin) {
        return denied(stream);
    }
    if request.method == "GET" && request.path == "/handshake" {
        let value = json!({"build":BUILD,"protocol":1,"launch":state.launch,"attachment":state.core.lock().unwrap().attachment,"path":state.path,"policy":state.policy});
        return json_response(stream, 200, &value);
    }
    if request.method != "POST"
        || request.path != "/api"
        || request.headers.get("origin") != Some(&origin)
        || request.headers.get("content-type").map(String::as_str) != Some("application/json")
    {
        return denied(stream);
    }
    let envelope: Envelope = match serde_json::from_slice(&request.body) {
        Ok(v) => v,
        Err(_) => {
            return json_response(
                stream,
                400,
                &json!({"status":"refused","error":"invalid_request"}),
            )
        }
    };
    if envelope.build != BUILD
        || envelope.protocol != 1
        || envelope.launch != state.launch
        || envelope.attachment != state.core.lock().unwrap().attachment
    {
        return json_response(stream, 409, &json!({"status":"expired","error":"stale_pair"}));
    }
    if envelope.request == json!({"op":"cancel"}) {
        state.epoch.fetch_add(1, Ordering::SeqCst);
        state.core.lock().unwrap().attachment = None;
        kill_session(state);
        return json_response(
            stream,
            200,
            &json!({"status":"cancelled","attachment":null,"execution":"session_termination_requested"}),
        );
    }
    let (reply, receive) = mpsc::sync_channel(1);
    let job = Job {
        kind: JobKind::Read(envelope.request),
        attachment: envelope.attachment,
        epoch: state.epoch.load(Ordering::SeqCst),
        deadline: Instant::now() + Duration::from_secs(30),
        reply,
    };
    if state.jobs.try_send(job).is_err() {
        return json_response(stream, 200, &json!({"status":"busy","error":"queue_full"}));
    }
    let response = receive.recv_timeout(Duration::from_secs(30)).unwrap_or_else(|_| {
        state.epoch.fetch_add(1, Ordering::SeqCst);
        state.core.lock().unwrap().attachment = None;
        kill_session(state);
        json!({"status":"budget_exhausted","attachment":null,"error":"request_deadline"})
    });
    json_response(stream, 200, &response)
}
fn denied(stream: TcpStream) -> std::io::Result<()> {
    json_response(stream, 403, &json!({"status":"refused","error":"local_access_denied"}))
}
fn json_response(stream: TcpStream, code: u16, value: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    if body.len() > crate::process::MAX_FRAME {
        return http::respond(
            stream,
            503,
            "application/json",
            b"{\"status\":\"budget_exhausted\"}",
            "",
        );
    }
    http::respond(stream, code, "application/json", &body, "")
}

fn kill_session(state: &State) {
    let pid = state.pid.swap(0, Ordering::SeqCst);
    if pid > 0 {
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}
fn execute(queue: mpsc::Receiver<Job>, state: &State, executable: &Path, scratch: &Path) {
    let mut worker: Option<crate::process::OwnedWorker> = None;
    while !state.stop.load(Ordering::SeqCst) {
        let job = match queue.recv_timeout(Duration::from_millis(20)) {
            Ok(job) => job,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        if job.epoch != state.epoch.load(Ordering::SeqCst) || job.deadline <= Instant::now() {
            let _ = job.reply.try_send(json!({"status":"cancelled","attachment":null}));
            continue;
        }
        if job.attachment != state.core.lock().unwrap().attachment {
            let _ = job.reply.try_send(json!({"status":"expired","error":"stale_attachment"}));
            continue;
        }
        state.executing.store(true, Ordering::SeqCst);
        #[cfg(not(feature = "fixture-control"))]
        let JobKind::Read(request) = job.kind;
        #[cfg(feature = "fixture-control")]
        let request = match job.kind {
            JobKind::Read(request) => request,
            #[cfg(feature = "fixture-control")]
            JobKind::Fixture(control) => {
                let response = match worker.as_mut() {
                    Some(w) => w.fixture(control).unwrap_or_else(|e| e.json()),
                    None => Failure::read("not_attached").json(),
                };
                state.executing.store(false, Ordering::SeqCst);
                let _ = job.reply.try_send(response);
                continue;
            }
        };
        let op = request.get("op").and_then(Value::as_str).unwrap_or("");
        let result: Result<Value, Failure> = match op {
            "attach" if request.as_object().is_some_and(|m| m.len() == 2) => {
                if worker.is_some() {
                    Err(Failure::read("already_attached"))
                } else if let Some(path) = request.get("path").and_then(Value::as_str) {
                    match crate::process::OwnedWorker::spawn_registered(
                        executable,
                        scratch,
                        json!({"path":path,"policy":state.policy}),
                        |pid| {
                            state.pid.store(pid as i32, Ordering::SeqCst);
                            if job.epoch != state.epoch.load(Ordering::SeqCst)
                                || state.stop.load(Ordering::SeqCst)
                            {
                                kill_session(state);
                            }
                        },
                    ) {
                        Ok(w) => {
                            state.pid.store(w.pid as i32, Ordering::SeqCst);
                            let attachment = w.attachment.clone();
                            worker = Some(w);
                            state.core.lock().unwrap().attachment = Some(attachment.clone());
                            Ok(
                                json!({"status":"complete","attachment":attachment,"mode":"owned_engine","effects":"qualified startup/background/shutdown effects permitted"}),
                            )
                        }
                        Err(code) => {
                            // Registration precedes readiness; the failed owner is already reaped.
                            state.pid.store(0, Ordering::SeqCst);
                            Err(code)
                        }
                    }
                } else {
                    Err(Failure::read("invalid_request"))
                }
            }
            "detach" | "close" if request.as_object().is_some_and(|m| m.len() == 1) => {
                state.pid.store(0, Ordering::SeqCst);
                let result = worker.take().map(|w| w.close()).unwrap_or(Ok(()));
                state.core.lock().unwrap().attachment = None;
                if op == "close" {
                    state.stop.store(true, Ordering::SeqCst);
                }
                result.map(|_| json!({"status":"complete","attachment":null}))
            }
            "catalog" | "check" | "sample" | "page" | "detail" | "chunk" | "tree" | "graph"
            | "investigate" | "graph_serving" | "associations" | "indexed" | "lexical"
            | "vectors" | "serving" | "neighborhood" | "export" => match worker.as_mut() {
                Some(w) => w.call(request),
                None => Err(Failure::read("not_attached")),
            },
            _ => Err(Failure::read("unsupported_operation")),
        };
        let mut response = match result {
            Ok(value) => value,
            Err(failure) => failure.json(),
        };
        if job.epoch != state.epoch.load(Ordering::SeqCst) {
            worker.take();
            state.pid.store(0, Ordering::SeqCst);
            state.core.lock().unwrap().attachment = None;
            response = json!({"status":"cancelled","attachment":null});
        } else if matches!(
            response["error"].as_str(),
            Some(
                "worker_disconnected" | "worker_error" | "attachment_expired" | "response_byte_cap"
            )
        ) {
            worker.take();
            state.pid.store(0, Ordering::SeqCst);
            state.core.lock().unwrap().attachment = None;
            response["attachment"] = Value::Null;
        }
        state.executing.store(false, Ordering::SeqCst);
        let _ = job.reply.try_send(response);
    }
    state.pid.store(0, Ordering::SeqCst);
    if let Some(w) = worker {
        let _ = w.close();
    }
}

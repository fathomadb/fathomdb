//! Provider-free source-test helper, never an ordinary measurement binary.

#[cfg(not(unix))]
fn main() {
    eprintln!("the Slice30 M private helper requires Unix");
    std::process::exit(2);
}

#[cfg(unix)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    unix::run()
}

#[cfg(unix)]
mod unix {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use fathomdb_mcp::config::{Config, SearchMode};
    use fathomdb_mcp::seams::Seams;
    use fathomdb_mcp::Adapter;
    use fathomdb_sdk::{Engine, InitialState, OpenOptions, PreparedWrite, SourceId};
    use rmcp::service::ServerInitializeError;
    use rmcp::ServiceExt;
    use serde_json::{json, Value};

    const LIFETIME: Duration = Duration::from_secs(120);

    struct Release(Seams);

    impl Drop for Release {
        fn drop(&mut self) {
            self.0.release_all();
        }
    }

    fn seed(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(root)?;
        for (store, id, body) in
            [("a", "alpha", "alpha river crossing\né😀"), ("b", "beta", "beta river crossing")]
        {
            let path = root.join(format!("{store}.sqlite"));
            if path.exists() {
                return Err("fixture path already exists".into());
            }
            let engine = Engine::open(path, OpenOptions::default())?;
            engine.write(&[PreparedWrite::Node {
                kind: "note".into(),
                body: body.into(),
                source_id: SourceId::new("m-source-fixture")?,
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])?;
            engine.close()?;
        }
        eprintln!("{}", json!({"closed":true,"stores":["a.sqlite","b.sqlite"]}));
        Ok(())
    }

    fn control(listener: UnixListener, seams: Seams, stop: Arc<AtomicBool>) -> Result<(), String> {
        let _release = Release(seams.clone());
        let until = Instant::now() + LIFETIME;
        let mut stream = loop {
            if stop.load(Ordering::SeqCst) || Instant::now() >= until {
                return Ok(());
            }
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(error) => return Err(error.to_string()),
            }
        };
        stream.set_read_timeout(Some(Duration::from_millis(100))).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(Duration::from_secs(1))).map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        let mut commands = 0;
        while !stop.load(Ordering::SeqCst) && Instant::now() < until {
            let mut byte = [0];
            match stream.read(&mut byte) {
                Ok(0) => return Ok(()),
                Ok(_) => bytes.push(byte[0]),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue
                }
                Err(error) => return Err(error.to_string()),
            }
            if bytes.len() > 4096 {
                return Err("private control line exceeds4096bytes".into());
            }
            if byte[0] != b'\n' {
                continue;
            }
            commands += 1;
            if commands > 4096 {
                return Err("private control exceeds4096commands".into());
            }
            let request: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            bytes.clear();
            let store = request["store"].as_str().unwrap_or("a");
            let facts = match request["command"].as_str() {
                Some("block_open") => {
                    seams.block_open(store);
                    json!({})
                }
                Some("release_open") => {
                    seams.release_open(store);
                    json!({})
                }
                Some("block_calls") => {
                    seams.block_calls(store);
                    json!({})
                }
                Some("release_calls") => {
                    seams.release_calls(store);
                    json!({})
                }
                Some("release_one_call") => json!({"released":seams.release_one_call(store)}),
                Some("held_calls") => json!({"held_calls":seams.held_calls(store)}),
                Some("release_all") => {
                    seams.release_all();
                    json!({})
                }
                _ => return Err("unknown private control command".into()),
            };
            let mut response =
                serde_json::to_vec(&json!({"id":request["id"],"ok":true,"facts":facts}))
                    .map_err(|e| e.to_string())?;
            response.push(b'\n');
            stream.write_all(&response).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() == 2 && args[0] == "--seed-fixture" {
            return seed(Path::new(&args[1]));
        }
        if args.len() != 4 || args[0] != "--config" || args[2] != "--control-socket" {
            return Err("usage: slice30_m_helper --seed-fixture <root> | --config <file> --control-socket <path>".into());
        }
        let config = Config::load(Path::new(&args[1]))?;
        if config.use_default_embedder || config.search_mode != SearchMode::Text {
            return Err("private helper requires provider-free text configuration".into());
        }
        let listener = UnixListener::bind(Path::new(&args[3]))?;
        listener.set_nonblocking(true)?;
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
        let adapter = {
            let _enter = runtime.enter();
            Adapter::new(config)?
        };
        let _release = Release(adapter.seams().clone());
        let stop = Arc::new(AtomicBool::new(false));
        let actor_stop = stop.clone();
        let seams = adapter.seams().clone();
        let actor = std::thread::spawn(move || control(listener, seams, actor_stop));
        let result = runtime.block_on(async {
            let serving = async {
                match adapter.connection().serve(rmcp::transport::stdio()).await {
                    Ok(running) => running.waiting().await.map(|_| ()).map_err(|e| e.to_string()),
                    Err(ServerInitializeError::ConnectionClosed(_)) => Ok(()),
                    Err(error) => Err(error.to_string()),
                }
            };
            let result = tokio::time::timeout(LIFETIME, serving).await.map_err(|e| e.to_string());
            adapter.seams().release_all();
            adapter.shutdown().await;
            result.and_then(|value| value)
        });
        stop.store(true, Ordering::SeqCst);
        adapter.seams().release_all();
        let control_result = actor.join().map_err(|_| "private control thread panicked")?;
        runtime.shutdown_timeout(Duration::from_secs(5));
        control_result?;
        result?;
        Ok(())
    }
}

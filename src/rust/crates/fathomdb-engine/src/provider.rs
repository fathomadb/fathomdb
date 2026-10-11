use super::*;

/// 0.8.6 Slice 5 (ADR-0.8.6) — the family of caller-supplied provider tasks that
/// ride the one NDJSON-over-stdio transport. Each task maps to a wire protocol
/// string `fathomdb.<task>.v1` and a task discriminator name. `Extract` shipped
/// in 0.8.6; `Consolidate` (0.8.12 Slice 15, OPP-2) is the SECOND consumer of
/// this one transport — it adds only a variant, a payload, and an `EngineError`
/// leaf, WITHOUT a second handshake or a second transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderTask {
    Extract,
    /// 0.8.12 Slice 15 (OPP-2, ADR-0.8.12) — consolidation / recency provider.
    Consolidate,
}

impl ProviderTask {
    /// The wire task discriminator, e.g. `"extract"`. Used for `supported_tasks`
    /// negotiation and as the request envelope `type`.
    fn name(self) -> &'static str {
        match self {
            ProviderTask::Extract => "extract",
            ProviderTask::Consolidate => "consolidate",
        }
    }

    /// The protocol string FathomDB sends in `hello`/requests and requires in
    /// `ready`. For `Extract` this is the UNCHANGED `fathomdb.extract.v1` —
    /// byte-identical back-compat for existing ELPS harnesses (ADR-0.8.6 §2.1).
    /// For `Consolidate` it is `fathomdb.consolidate.v1` (ADR-0.8.12 §2).
    fn protocol(self) -> &'static str {
        match self {
            ProviderTask::Extract => "fathomdb.extract.v1",
            ProviderTask::Consolidate => "fathomdb.consolidate.v1",
        }
    }
}

/// 0.8.6 Slice 5 (ADR-0.8.6) — an open provider transport session: the spawned
/// caller subprocess, the buffered stdin writer, the detached stdout-drain
/// channel, the bounded-recv timeout, and the negotiated handshake state
/// (`model` provenance + `max_docs_per_request`). One session serves one task
/// family; the `request`/framing is identical across tasks. `Drop` reaps the
/// child (sends stdin EOF via the writer field's own drop, then kill/wait),
/// replacing the prior explicit outer kill/wait.
pub(crate) struct ProviderSession {
    task: ProviderTask,
    child: std::process::Child,
    writer: std::io::BufWriter<std::process::ChildStdin>,
    line_rx: Receiver<std::io::Result<String>>,
    io_timeout: Duration,
    /// `ready.model`, recorded as output-row provenance (`extractor_model_id`).
    pub(crate) model: Option<String>,
    pub(crate) max_docs_per_request: usize,
}

impl Drop for ProviderSession {
    fn drop(&mut self) {
        // The detached stdout-drain thread exits when the child's stdout closes;
        // kill() guarantees that even for a child that ignores stdin EOF. The
        // `writer` field drops after this (declaration order) sending EOF too.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl ProviderSession {
    /// Run the `hello` → `ready` handshake and `supported_tasks` negotiation.
    /// Validates protocol + schema_version (fix-23 [P2]); rejects a zero
    /// `max_docs_per_request` (fix-1 [P2]); and, when the harness advertises
    /// `supported_tasks`, refuses to proceed unless this session's task is in it.
    /// When `supported_tasks` is absent, the harness is assumed to serve the
    /// requested task (back-compat: existing extract-only harnesses unchanged).
    fn handshake(&mut self) -> Result<(), EngineError> {
        let protocol = self.task.protocol();
        let hello = serde_json::json!({
            "protocol": protocol,
            "type": "hello",
            "schema_version": 1,
        });
        let hello_line = serde_json::to_string(&hello).map_err(|_| EngineError::Extractor)?;
        writeln!(self.writer, "{hello_line}").map_err(|_| EngineError::Extractor)?;
        self.writer.flush().map_err(|_| EngineError::Extractor)?;

        let line = recv_extractor_line(&self.line_rx, self.io_timeout)?;
        let ready: Value = serde_json::from_str(line.trim()).map_err(|_| EngineError::Extractor)?;
        // fix-23 [P2]: validate protocol + schema_version in the ready message per ADR.
        if ready.get("type").and_then(|v| v.as_str()) != Some("ready")
            || ready.get("protocol").and_then(|v| v.as_str()) != Some(protocol)
            || ready.get("schema_version").and_then(|v| v.as_u64()) != Some(1)
        {
            return Err(EngineError::Extractor);
        }

        // 0.8.6 Slice 5 (ADR-0.8.6 §2.2): additive, optional `supported_tasks`
        // negotiation. If present, the harness must advertise this session's task
        // or FathomDB refuses to dispatch it. If absent, default to "serves the
        // requested task" so extract-only harnesses keep working unchanged.
        if let Some(supported) = ready.get("supported_tasks").and_then(|v| v.as_array()) {
            let task_name = self.task.name();
            let advertised = supported.iter().any(|t| t.as_str() == Some(task_name));
            if !advertised {
                return Err(EngineError::Extractor);
            }
        }

        self.model = ready.get("model").and_then(|v| v.as_str()).map(|s| s.to_string());
        let max_docs =
            ready.get("max_docs_per_request").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
        // fix-1 [P2]: reject zero max_docs_per_request to prevent chunks(0) panic.
        if max_docs == 0 {
            return Err(EngineError::Extractor);
        }
        self.max_docs_per_request = max_docs;
        Ok(())
    }

    /// Send one framed request for this session's task and receive its matching
    /// response. `payload` carries the task-specific fields; the envelope keys
    /// (`protocol`, `type`, `request_id`) are added here. The response must have
    /// `type == "result"` and a matching `request_id` (fix-24 [P2]); anything
    /// else (error, wrong id, missing type) is a protocol fault. For `Extract`
    /// the serialized request bytes are identical to the pre-0.8.6 path (serde_json
    /// serializes map keys sorted, independent of insertion order).
    pub(crate) fn request(
        &mut self,
        request_id: &str,
        payload: Vec<(String, Value)>,
    ) -> Result<Value, EngineError> {
        let mut req = serde_json::Map::new();
        req.insert("protocol".to_string(), Value::from(self.task.protocol()));
        req.insert("type".to_string(), Value::from(self.task.name()));
        req.insert("request_id".to_string(), Value::from(request_id));
        for (k, v) in payload {
            req.insert(k, v);
        }
        let req_line =
            serde_json::to_string(&Value::Object(req)).map_err(|_| EngineError::Extractor)?;
        writeln!(self.writer, "{req_line}").map_err(|_| EngineError::Extractor)?;
        self.writer.flush().map_err(|_| EngineError::Extractor)?;

        let result_line = recv_extractor_line(&self.line_rx, self.io_timeout)?;
        let result: Value =
            serde_json::from_str(result_line.trim()).map_err(|_| EngineError::Extractor)?;
        let resp_type = result.get("type").and_then(|v| v.as_str());
        let resp_id = result.get("request_id").and_then(|v| v.as_str());
        if resp_type != Some("result") || resp_id != Some(request_id) {
            return Err(EngineError::Extractor);
        }
        Ok(result)
    }
}

/// fix-35 [P2]: BYO-LLM extractor I/O timeout. Defaults to 300s to accommodate
/// slow LLM harnesses; override (in milliseconds) via
/// `FATHOMDB_EXTRACTOR_TIMEOUT_MS` (tests use this to exercise the hung-harness
/// path quickly).
fn extractor_io_timeout() -> Duration {
    std::env::var("FATHOMDB_EXTRACTOR_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or_else(|| Duration::from_secs(300))
}

/// fix-35 [P1/P2]: receive one line from the stdout reader thread, bounded by
/// `timeout`. A timeout, a closed channel (reader thread ended / child EOF), or
/// an underlying io error all map to [`EngineError::Extractor`].
fn recv_extractor_line(
    rx: &Receiver<std::io::Result<String>>,
    timeout: Duration,
) -> Result<String, EngineError> {
    match rx.recv_timeout(timeout) {
        Ok(Ok(line)) => Ok(line),
        _ => Err(EngineError::Extractor),
    }
}

impl Engine {
    /// 0.8.6 Slice 5 (ADR-0.8.6) — open a provider session: spawn the caller
    /// subprocess, run the `hello`/`ready` handshake for `task`, and negotiate
    /// `supported_tasks`. The transport (NDJSON over stdio, the detached stdout
    /// drainer, the bounded-recv timeout, the `request_id` framing, and the
    /// catch-all `EngineError::Extractor` mapping) is identical across tasks;
    /// only the protocol string (`fathomdb.<task>.v1`) and the negotiated task
    /// name differ. For `ProviderTask::Extract` the wire is byte-identical to the
    /// pre-0.8.6 `fathomdb.extract.v1` path.
    pub(crate) fn provider_session(
        &self,
        task: ProviderTask,
        cmd: &[&str],
    ) -> Result<ProviderSession, EngineError> {
        let (program, args) = cmd.split_first().ok_or(EngineError::Extractor)?;
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|_| EngineError::Extractor)?;

        let child_stdin = match child.stdin.take() {
            Some(s) => s,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(EngineError::Extractor);
            }
        };
        let child_stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(EngineError::Extractor);
            }
        };

        // fix-35 [P1/P2]: drain stdout on a dedicated thread so (a) every read can
        // be bounded with a timeout — a hung harness can no longer block ingest
        // forever — and (b) the child's stdout pipe is drained continuously,
        // preventing a large-request deadlock (parent blocked writing stdin while
        // the child blocks writing a full stdout pipe). The handle is detached:
        // joining could hang if a misbehaving child holds stdout open past its
        // stdin EOF, so the session's `Drop` (child.kill()) is what guarantees
        // thread exit.
        let io_timeout = extractor_io_timeout();
        let (line_tx, line_rx) = mpsc::channel::<std::io::Result<String>>();
        thread::spawn(move || {
            let mut reader = BufReader::new(child_stdout);
            loop {
                let mut buf = String::new();
                match reader.read_line(&mut buf) {
                    Ok(0) => break,
                    Ok(_) => {
                        if line_tx.send(Ok(buf)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = line_tx.send(Err(e));
                        break;
                    }
                }
            }
        });

        let mut session = ProviderSession {
            task,
            child,
            writer: std::io::BufWriter::new(child_stdin),
            line_rx,
            io_timeout,
            model: None,
            max_docs_per_request: 8,
        };
        // On any handshake/negotiation error the session is dropped here, which
        // reaps the child (Drop) — matching the prior outer kill/wait semantics.
        session.handshake()?;
        Ok(session)
    }
}

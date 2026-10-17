//! Fixed protocol for one Rust SDK benchmark repetition.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Instant;

use fathomdb_sdk::{
    admin, graph, read, Engine, NeighborsOptions, NodeRecord, OpenOptions,
    ProjectedTextSearchOptions, ReadView, RuntimeSqliteMode, SearchFilter, SearchOptions,
    SearchResult, SoftFallbackBranch, TextSearchOptions, TraversalDirection,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const SCHEMA: &str = "sdk.driver.v1";
const OPERATIONS: [&str; 5] = [
    "engine.search_text_only",
    "engine.search_projected_text",
    "engine.search",
    "read.get",
    "graph.neighbors",
];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityRequest {
    schema_version: String,
    mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    schema_version: String,
    mode: String,
    sdk: String,
    operation_id: String,
    operation_version: u32,
    database_path: String,
    queries: Vec<Query>,
    arguments: Arguments,
    warmup_per_query: u32,
    timeout_ms: u64,
    expected_native_sha256: String,
    #[serde(default)]
    normalizer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Arguments {
    Projected(ProjectedArguments),
    Hybrid(HybridArguments),
    Fts(FtsArguments),
    Read(ReadArguments),
    Graph(GraphArguments),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FtsArguments {
    limit: usize,
    view: View,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectedArguments {
    limit: usize,
    view: View,
    name: String,
    filter: Option<FilterArgument>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct FilterArgument {
    attributes: Vec<(String, String)>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HybridArguments {
    limit: usize,
    view: View,
    use_default_embedder: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArguments {
    view: View,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphArguments {
    depth: u32,
    view: View,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct View {
    valid_as_of: i64,
}

enum CallOptions {
    Projected { name: String, options: ProjectedTextSearchOptions },
    Hybrid(SearchOptions),
    Fts(TextSearchOptions),
    Read(ReadView),
    Graph { depth: u32, options: NeighborsOptions },
}

enum NativeResult {
    Fts(SearchResult),
    Read(Option<NodeRecord>),
    Graph(Vec<NodeRecord>),
}

fn branch_name(branch: SoftFallbackBranch) -> &'static str {
    match branch {
        SoftFallbackBranch::Vector => "vector",
        SoftFallbackBranch::Text => "text",
        SoftFallbackBranch::TextEdge => "text_edge",
        SoftFallbackBranch::GraphArm => "graph_arm",
    }
}

impl NativeResult {
    fn hits(&self) -> Option<Vec<Value>> {
        match self {
            Self::Fts(result) => Some(
                result
                    .results
                    .iter()
                    .map(|hit| {
                        json!({
                            "id": hit.id.to_prefixed(),
                            "score": hit.score,
                            "branch": branch_name(hit.branch),
                        })
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    fn ids(self) -> Vec<String> {
        match self {
            Self::Fts(result) => {
                result.results.into_iter().map(|hit| hit.id.to_prefixed()).collect()
            }
            Self::Read(node) => {
                node.into_iter().map(|node| format!("l:{}", node.logical_id)).collect()
            }
            Self::Graph(nodes) => {
                nodes.into_iter().map(|node| format!("l:{}", node.logical_id)).collect()
            }
        }
    }
}

fn public_call(
    engine: &Engine,
    text: &str,
    options: &CallOptions,
) -> fathomdb_sdk::Result<NativeResult> {
    match options {
        CallOptions::Projected { name, options } => {
            engine.search_projected_text(text, name, options.clone()).map(NativeResult::Fts)
        }
        CallOptions::Hybrid(options) => engine.search(text, options.clone()).map(NativeResult::Fts),
        CallOptions::Fts(options) => {
            engine.search_text_only(text, options.clone()).map(NativeResult::Fts)
        }
        CallOptions::Read(view) => read::get(engine, text, Some(view)).map(NativeResult::Read),
        CallOptions::Graph { depth, options } => {
            graph::neighbors(engine, text, *depth, options.clone()).map(NativeResult::Graph)
        }
    }
}

impl Arguments {
    fn call_options(&self) -> CallOptions {
        match self {
            Self::Projected(args) => CallOptions::Projected {
                name: args.name.clone(),
                options: ProjectedTextSearchOptions {
                    filter: args.filter.as_ref().map(|filter| {
                        let mut search_filter = SearchFilter::default();
                        search_filter.attributes = filter.attributes.clone();
                        search_filter
                    }),
                    view: ReadView {
                        valid_as_of: Some(args.view.valid_as_of),
                        ..ReadView::default()
                    },
                    limit: args.limit,
                },
            },
            Self::Hybrid(args) => CallOptions::Hybrid(SearchOptions {
                view: ReadView { valid_as_of: Some(args.view.valid_as_of), ..ReadView::default() },
                limit: args.limit,
                ..SearchOptions::default()
            }),
            Self::Fts(args) => CallOptions::Fts(TextSearchOptions {
                view: ReadView { valid_as_of: Some(args.view.valid_as_of), ..ReadView::default() },
                limit: args.limit,
            }),
            Self::Read(args) => CallOptions::Read(ReadView {
                valid_as_of: Some(args.view.valid_as_of),
                ..ReadView::default()
            }),
            Self::Graph(args) => CallOptions::Graph {
                depth: args.depth,
                options: NeighborsOptions {
                    direction: TraversalDirection::Both,
                    view: Some(ReadView {
                        valid_as_of: Some(args.view.valid_as_of),
                        ..ReadView::default()
                    }),
                },
            },
        }
    }

    fn use_default_embedder(&self) -> bool {
        matches!(self, Self::Hybrid(args) if args.use_default_embedder)
    }

    fn json(&self) -> Value {
        match self {
            Self::Projected(args) => json!({
                "limit": args.limit,
                "view": {"valid_as_of": args.view.valid_as_of},
                "name": args.name,
                "filter": args.filter,
            }),
            Self::Hybrid(args) => json!({
                "limit": args.limit,
                "view": {"valid_as_of": args.view.valid_as_of},
                "use_default_embedder": args.use_default_embedder,
            }),
            Self::Fts(args) => {
                json!({"limit": args.limit, "view": {"valid_as_of": args.view.valid_as_of}})
            }
            Self::Read(args) => json!({"view": {"valid_as_of": args.view.valid_as_of}}),
            Self::Graph(args) => {
                json!({"depth": args.depth, "view": {"valid_as_of": args.view.valid_as_of}})
            }
        }
    }
}

#[derive(Debug)]
pub enum Request {
    Capabilities,
    RunRepetition(Box<RunRequest>),
}

#[derive(Debug)]
struct DriverError {
    kind: String,
    message: String,
}

impl DriverError {
    fn protocol(message: &str) -> Self {
        Self { kind: "ProtocolError".into(), message: message.into() }
    }

    fn sdk(error: &fathomdb_sdk::Error) -> Self {
        let kind = error.kind().name().to_string();
        Self { message: kind.clone(), kind }
    }

    fn json(&self) -> Value {
        json!({"kind": self.kind, "message": self.message})
    }
}

/// Parse and validate one closed driver request before opening a database.
pub fn parse_request(raw: &str) -> Result<Request, String> {
    let value: Value = serde_json::from_str(raw).map_err(|_| "invalid JSON")?;
    let mode = value.get("mode").and_then(Value::as_str).ok_or("missing mode")?;
    match mode {
        "capabilities" => {
            let request: CapabilityRequest =
                serde_json::from_value(value).map_err(|_| "invalid capabilities request")?;
            if request.schema_version != SCHEMA || request.mode != "capabilities" {
                return Err("unsupported protocol version".into());
            }
            Ok(Request::Capabilities)
        }
        "run-repetition" => {
            let request: RunRequest =
                serde_json::from_value(value).map_err(|_| "invalid repetition request")?;
            if request.schema_version != SCHEMA || request.mode != "run-repetition" {
                return Err("unsupported protocol version".into());
            }
            if request.sdk != "rust"
                || !OPERATIONS.contains(&request.operation_id.as_str())
                || request.operation_version != 1
            {
                return Err("unsupported SDK operation".into());
            }
            match (request.operation_id.as_str(), &request.arguments) {
                ("engine.search_text_only", Arguments::Fts(args))
                    if (1..=100).contains(&args.limit) => {}
                ("engine.search_projected_text", Arguments::Projected(args))
                    if (1..=100).contains(&args.limit) && !args.name.is_empty() => {}
                ("engine.search", Arguments::Hybrid(args)) if (1..=100).contains(&args.limit) => {}
                ("read.get", Arguments::Read(_)) => {}
                ("graph.neighbors", Arguments::Graph(args)) if (1..=3).contains(&args.depth) => {}
                _ => return Err("invalid operation arguments".into()),
            }
            if request.database_path.is_empty() || !Path::new(&request.database_path).is_absolute()
            {
                return Err("database_path must be absolute".into());
            }
            if request.queries.is_empty()
                || request.queries.iter().any(|q| q.id.is_empty() || q.text.is_empty())
            {
                return Err("queries require nonempty IDs and text".into());
            }
            let mut seen = HashSet::new();
            if request.queries.iter().any(|q| !seen.insert(&q.id)) {
                return Err("query IDs must be unique".into());
            }
            if request.timeout_ms == 0 || request.warmup_per_query > 1000 {
                return Err("invalid timeout or warmup count".into());
            }
            if !matches!(request.normalizer.as_deref(), None | Some("ordered_ids" | "ordered_hits"))
            {
                return Err("unsupported normalizer".into());
            }
            if request.expected_native_sha256.len() != 64
                || !request.expected_native_sha256.bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err("expected_native_sha256 must be a SHA-256 hex digest".into());
            }
            Ok(Request::RunRepetition(Box::new(request)))
        }
        _ => Err("unsupported mode".into()),
    }
}

fn binary_sha256() -> Result<String, DriverError> {
    let path = std::env::current_exe()
        .map_err(|_| DriverError::protocol("cannot resolve driver binary"))?;
    let bytes = fs::read(path).map_err(|_| DriverError::protocol("cannot read driver binary"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn peak_rss_bytes() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmHWM:")?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()?
            .checked_mul(1024)
    })
}

fn identity(native_sha256: Option<&str>) -> Value {
    let mut features = Vec::new();
    if cfg!(feature = "default-embedder") {
        features.push("default-embedder");
    }
    if cfg!(feature = "embed-cuda") {
        features.push("embed-cuda");
    }
    if cfg!(feature = "embed-metal") {
        features.push("embed-metal");
    }
    if cfg!(feature = "default-reranker") {
        features.push("default-reranker");
    }
    if cfg!(feature = "rerank-cuda") {
        features.push("rerank-cuda");
    }
    if cfg!(feature = "rerank-metal") {
        features.push("rerank-metal");
    }
    json!({
        "sdk_version": env!("DRIVER_SDK_VERSION"),
        "runtime": env!("DRIVER_RUSTC_VERSION"),
        "target": env!("DRIVER_TARGET"),
        "profile": env!("DRIVER_PROFILE"),
        "features": features,
        "native_sha256": native_sha256,
    })
}

fn blocked(error: DriverError, native_sha256: Option<&str>, request: Option<&RunRequest>) -> Value {
    json!({
        "schema_version": SCHEMA,
        "status": "blocked",
        "sdk": "rust",
        "operation_id": request.map(|r| r.operation_id.as_str()),
        "operation_version": request.map(|r| r.operation_version),
        "identity": identity(native_sha256),
        "effective_options": Value::Null,
        "samples": [],
        "error": error.json(),
        "closed": false
    })
}

fn run(request: &RunRequest) -> Value {
    let native_sha256 = match binary_sha256() {
        Ok(hash) => hash,
        Err(error) => return blocked(error, None, Some(request)),
    };
    if !native_sha256.eq_ignore_ascii_case(&request.expected_native_sha256) {
        return blocked(
            DriverError::protocol("driver binary SHA-256 mismatch"),
            Some(&native_sha256),
            Some(request),
        );
    }
    if !Path::new(&request.database_path).is_file() {
        return blocked(
            DriverError::protocol("prepared database does not exist"),
            Some(&native_sha256),
            Some(request),
        );
    }
    if let Err(error) = admin::configure_runtime(RuntimeSqliteMode::Performance) {
        return blocked(DriverError::sdk(&error), Some(&native_sha256), Some(request));
    }
    let open_options = OpenOptions {
        use_default_embedder: request.arguments.use_default_embedder(),
        ..OpenOptions::default()
    };
    let engine = match Engine::open(&request.database_path, open_options) {
        Ok(engine) => engine,
        Err(error) => {
            return blocked(DriverError::sdk(&error), Some(&native_sha256), Some(request))
        }
    };
    let report = engine.open_report();
    let open_report = json!({
        "schema_version_before": report.schema_version_before,
        "schema_version_after": report.schema_version_after,
        "query_backend": report.query_backend,
    });
    let options = request.arguments.call_options();
    let mut samples = Vec::with_capacity(request.queries.len());
    let mut failure = None;
    if let Err(error) = engine.drain(request.timeout_ms) {
        failure = Some(DriverError::sdk(&error));
    }
    if failure.is_none() {
        for query in &request.queries {
            for _ in 0..request.warmup_per_query {
                if let Err(error) = public_call(&engine, &query.text, &options) {
                    failure = Some(DriverError::sdk(&error));
                    break;
                }
            }
            if failure.is_some() {
                break;
            }
            let start = Instant::now();
            let result = public_call(&engine, &query.text, &options);
            let elapsed_ns = start.elapsed().as_nanos();
            match result {
                Ok(result) => {
                    let hits = result.hits();
                    let ids = result.ids();
                    let mut sample =
                        json!({"query_id": query.id, "elapsed_ns": elapsed_ns, "ids": ids});
                    if request.normalizer.as_deref() == Some("ordered_hits") {
                        sample["hits"] = json!(hits);
                    }
                    samples.push(sample);
                }
                Err(error) => {
                    failure = Some(DriverError::sdk(&error));
                    break;
                }
            }
        }
    }
    let close_result = engine.close();
    let closed = close_result.is_ok();
    if let Err(error) = close_result {
        failure = Some(DriverError::sdk(&error));
    }
    if let Some(error) = failure {
        let mut response = blocked(error, Some(&native_sha256), Some(request));
        response["closed"] = json!(closed);
        response["open_report"] = open_report;
        return response;
    }
    let mut response = json!({
        "schema_version": SCHEMA,
        "status": "complete",
        "sdk": "rust",
        "operation_id": request.operation_id,
        "operation_version": 1,
        "identity": identity(Some(&native_sha256)),
        "effective_options": request.arguments.json(),
        "samples": samples,
        "error": Value::Null,
        "open_report": open_report,
        "readiness": {"drained": true},
        "closed": true
    });
    if let Some(bytes) = peak_rss_bytes() {
        response["resources"] = json!({"driver_peak_rss_bytes": bytes});
    }
    response
}

/// Execute one protocol request and return exactly one JSON response value.
pub fn handle_request(raw: &str) -> Value {
    match parse_request(raw) {
        Ok(Request::Capabilities) => {
            let hash = binary_sha256().ok();
            json!({
                "schema_version": SCHEMA,
                "status": "complete",
                "sdk": "rust",
                "identity": identity(hash.as_deref()),
                "operations": OPERATIONS.iter().map(|operation_id| json!({"operation_id": operation_id, "operation_version": 1})).collect::<Vec<_>>(),
                "error": Value::Null
            })
        }
        Ok(Request::RunRepetition(request)) => run(&request),
        Err(message) => blocked(DriverError::protocol(&message), None, None),
    }
}

//! One function per tool (design §7): validate arguments, call the bridge
//! once, shape the governed result. No ranking, filtering or eligibility
//! logic lives here (MR-070).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use fathomdb_sdk::{
    graph, read, ArmDegradation, Explanation, GraphExpandRequestV1,
    GraphExpansionDegradationCodeV1, GraphReadContextV1, GraphSeedV1, IdSpace, IdSpaceKind,
    PerHitExplain, ReadContextV1, ReadView, SearchFilter, SearchFilterArg, SearchHit,
    SearchOptions, SearchResult, SoftFallbackBranch, TextSearchOptions, TraversalDirection,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::bridge::{self, Answered};
use crate::config::{Config, SearchMode};
use crate::error::ToolError;
use crate::hooks::Hooks;
use crate::output::{
    Budget, Degradation, DenseDisabled, EmbeddingReadinessOut, Envelope, ExpandOutput,
    ExplanationOut, FeedbackOutput, FeedbackStatus, FetchItem, FetchOutput, Hit, ListStoresOutput,
    ModeWord, NotRecordedReason, OrientOutput, PerHitOut, ProjectionDenseRow, ProjectionRow,
    ProjectionStatusOut, Readiness, RecordOut, SearchOutput, StoreRow, Target, TraceOut,
    Unavailable, UnavailableWord, UseStoreOutput,
};
use crate::registry::{Admission, Cancel, ProgressFn, Registry, StoreGuard, ToolKind};
use crate::session::{Continuation, ResponseContext, ResponseLookup, Session};
use crate::shape::{self, CanonicalSearch};

/// The engine's ranked-result cap (`MAX_RANKED_LIMIT`).
pub(crate) const MAX_RANKED: usize = 100;
/// Work bound for one expansion: the engine maximum. The run limit bounds
/// the time it may take (interface §4 `expand`).
pub(crate) const EXPAND_WORK_UNITS: u64 = 10_000;
const MAX_FETCH_REFS: usize = 50;
const MAX_EXPAND_REFS: usize = 10;
const MIN_MAX_TOKENS: u64 = 256;

/// State shared by every connection of one adapter.
pub(crate) struct Shared {
    pub(crate) config: Arc<Config>,
    pub(crate) registry: Arc<Registry>,
    pub(crate) hooks: Hooks,
}

/// One tool call's context.
pub(crate) struct Ctx<'a> {
    pub(crate) shared: &'a Shared,
    pub(crate) session: &'a Session,
    /// Present when the request carried a progress token: progress
    /// notifications are sent while the call waits for an opening store
    /// (MR-006.4).
    pub(crate) progress: Option<ProgressFn>,
    /// Fires when the client cancels the call; a queued call then leaves
    /// the queue.
    pub(crate) cancel: Cancel,
}

/// A successful result: structured content and its text rendering.
pub(crate) struct Rendered {
    pub(crate) structured: Value,
    pub(crate) text: String,
}

pub(crate) type Outcome = Result<Rendered, ToolError>;

// ---------------------------------------------------------------- arguments

/// `list_stores` takes no arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListStoresArgs {
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// `use_store` arguments.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UseStoreArgs {
    /// Store name to use as the session default.
    pub store: String,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    // Accepted so every tool takes the same argument; this tool emits no hints.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// `orient` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OrientArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    // Accepted so every tool takes the same argument; this tool emits no hints.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// Search path.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModeArg {
    Text,
    Hybrid,
}

/// `search` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// Query text. Required unless `continuation` is given.
    pub query: Option<String>,
    /// `text` (lexical only) or `hybrid`; defaults to the operator's search_mode.
    pub mode: Option<ModeArg>,
    /// Exact kind filter (hybrid only).
    pub kind: Option<String>,
    /// Exact source type filter (hybrid only).
    pub source_type: Option<String>,
    /// Exact status filter (hybrid only).
    pub status: Option<String>,
    /// Lower bound on creation time, epoch seconds (hybrid only).
    pub created_after: Option<i64>,
    /// Declared filterable attribute equalities (hybrid only).
    pub attributes: Option<BTreeMap<String, String>>,
    /// Validity instant, epoch seconds; defaults to now.
    pub as_of: Option<i64>,
    /// Results per page, 1..=100; default 10.
    #[schemars(range(min = 1, max = 100))]
    pub limit: Option<u64>,
    /// Response budget in estimated tokens (bytes / 4), at least 256.
    #[schemars(range(min = 256))]
    pub max_tokens: Option<u64>,
    /// Include a per-hit explanation.
    pub explain: Option<bool>,
    /// Handle from a previous result; then only store, max_tokens and hints may be given.
    pub continuation: Option<String>,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// `fetch` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FetchArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// 1..=50 references (`l:<logical id>`).
    #[schemars(length(min = 1, max = 50))]
    pub refs: Vec<String>,
    /// Validity instant, epoch seconds; defaults to now.
    pub as_of: Option<i64>,
    /// Include superseded versions (the most recent visible version is returned).
    pub include_superseded: Option<bool>,
    /// Include records in a non-active lifecycle state.
    pub include_inactive: Option<bool>,
    /// Response budget in estimated tokens (bytes / 4), at least 256.
    #[schemars(range(min = 256))]
    pub max_tokens: Option<u64>,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// Traversal direction.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DirectionArg {
    Out,
    In,
    Both,
}

/// `expand` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpandArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// 1..=10 seed references (`l:<logical id>`).
    #[schemars(length(min = 1, max = 10))]
    pub refs: Vec<String>,
    /// `out` (default), `in` or `both`.
    pub direction: Option<DirectionArg>,
    /// Hops, 0..=3; default 1.
    #[schemars(range(min = 0, max = 3))]
    pub max_depth: Option<u64>,
    /// Targets, 1..=50; default 20.
    #[schemars(range(min = 1, max = 50))]
    pub limit: Option<u64>,
    /// Only follow these edge kinds.
    pub edge_kinds: Option<Vec<String>>,
    /// Only return targets of these kinds.
    pub target_kinds: Option<Vec<String>>,
    /// Validity instant, epoch seconds; defaults to now.
    pub as_of: Option<i64>,
    /// Response budget in estimated tokens (bytes / 4), at least 256.
    #[schemars(range(min = 256))]
    pub max_tokens: Option<u64>,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

/// `feedback` arguments.
#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeedbackArgs {
    /// Store name; defaults to the session default store.
    pub store: Option<String>,
    /// `response_id` of a search result.
    pub response_id: String,
    /// References from that response that were used.
    pub used: Vec<String>,
    /// References from that response that were not used.
    pub unused: Option<Vec<String>>,
    /// Include the `hints` sidecar of suggested next calls; defaults to the
    /// operator's `hints` setting.
    // Accepted so every tool takes the same argument; this tool emits no hints.
    #[allow(dead_code)]
    pub hints: Option<bool>,
}

fn parse<T: DeserializeOwned>(arguments: Map<String, Value>) -> Result<T, ToolError> {
    serde_json::from_value(Value::Object(arguments))
        .map_err(|error| ToolError::bad_argument(format!("invalid arguments: {error}")))
}

// ------------------------------------------------------------------ helpers

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn resolve_store(ctx: &Ctx<'_>, store: Option<&str>) -> Result<String, ToolError> {
    match store {
        Some(store) => Ok(store.to_string()),
        None => ctx.session.default_store().ok_or_else(ToolError::no_store),
    }
}

fn max_tokens(ctx: &Ctx<'_>, given: Option<u64>) -> Result<u32, ToolError> {
    match given {
        None => Ok(ctx.shared.config.default_max_tokens),
        Some(value) if value >= MIN_MAX_TOKENS => Ok(u32::try_from(value).unwrap_or(u32::MAX)),
        Some(_) => Err(ToolError::bad_argument("max_tokens must be at least 256")),
    }
}

impl Ctx<'_> {
    /// The wait deadline, from arrival (`call_deadline_ms`).
    fn deadline(&self) -> Instant {
        Instant::now() + std::time::Duration::from_millis(self.shared.config.call_deadline_ms)
    }

    /// The bound on a started call, from its permit (`run_limit_ms`).
    fn run_limit(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.shared.config.run_limit_ms)
    }

    async fn acquire(
        &self,
        store: &str,
        kind: ToolKind,
        deadline: Instant,
    ) -> Result<StoreGuard, ToolError> {
        // Every call waits for an open in progress, and for an admission
        // permit, up to its deadline; only calls with a progress token are
        // sent progress notifications.
        let call = Admission {
            kind,
            wait_until: Some(deadline),
            progress: self.progress.as_ref(),
            cancel: Some(&self.cancel),
        };
        self.shared.registry.acquire(store, &call).await
    }

    async fn run<T, F>(&self, store: &str, kind: ToolKind, f: F) -> Result<(Answered, T), ToolError>
    where
        T: Send + 'static,
        F: FnOnce(&fathomdb_sdk::Engine) -> fathomdb_sdk::Result<T> + Send + 'static,
    {
        let guard = self.acquire(store, kind, self.deadline()).await?;
        bridge::call(
            guard,
            self.run_limit(),
            &self.cancel,
            self.progress.as_ref(),
            &self.shared.hooks,
            f,
        )
        .await
    }
}

fn envelope(
    answered_store: &str,
    epoch: u64,
    validity_instant: i64,
    degradation: Vec<Degradation>,
    max_tokens: u32,
) -> Envelope {
    Envelope {
        store: answered_store.to_string(),
        epoch,
        validity_instant,
        state: Unavailable::new(),
        degradation,
        budget: Budget { max_tokens, used_tokens: 0, cut: false },
    }
}

fn finish(output: &impl Serialize, text: String, max_tokens: u32, cut: bool) -> (Rendered, u64) {
    let value = serde_json::to_value(output).expect("tool outputs serialize");
    let (structured, whole) = shape::with_budget(value, &text, max_tokens, cut);
    (Rendered { structured, text }, whole)
}

fn plain(output: &impl Serialize, text: String) -> Rendered {
    Rendered { structured: serde_json::to_value(output).expect("tool outputs serialize"), text }
}

fn branch_str(branch: SoftFallbackBranch) -> &'static str {
    match branch {
        SoftFallbackBranch::Vector => "vector",
        SoftFallbackBranch::Text => "text",
        SoftFallbackBranch::TextEdge => "text_edge",
        SoftFallbackBranch::GraphArm => "graph_arm",
    }
}

enum ParsedRef {
    Logical(String),
    Unsupported,
    Malformed,
}

fn parse_ref(reference: &str) -> ParsedRef {
    match IdSpace::parse(reference) {
        Some(id) if id.space == IdSpaceKind::Logical => {
            if id.value.is_empty() || id.value.contains('\0') {
                ParsedRef::Malformed
            } else {
                ParsedRef::Logical(id.value)
            }
        }
        Some(_) => ParsedRef::Unsupported,
        None => ParsedRef::Malformed,
    }
}

fn bytes_over(whole_tokens: u64, max_tokens: u32) -> usize {
    let over = whole_tokens.saturating_sub(u64::from(max_tokens));
    usize::try_from(over.saturating_mul(4)).unwrap_or(usize::MAX)
}

// ------------------------------------------------------------------- tools

pub(crate) async fn list_stores(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let _: ListStoresArgs = parse(arguments)?;
    let default = ctx.session.default_store();
    let stores: Vec<StoreRow> = ctx
        .shared
        .registry
        .listing()
        .into_iter()
        .map(|row| StoreRow {
            default: default.as_deref() == Some(row.name.as_str()),
            readiness: match row.readiness {
                "opening" => Readiness::Opening,
                "reopening" => Readiness::Reopening,
                "ready" => Readiness::Ready,
                "draining" => Readiness::Draining,
                "failed" => Readiness::Failed,
                _ => Readiness::Closed,
            },
            open: row.open,
            name: row.name,
        })
        .collect();
    let mut text = format!("stores: {}", stores.len());
    for row in &stores {
        let readiness = serde_json::to_value(row.readiness).expect("serializes");
        text.push_str(&format!(
            "\n{} open={} readiness={}{}",
            row.name,
            row.open,
            readiness.as_str().unwrap_or("closed"),
            if row.default { " default" } else { "" }
        ));
    }
    Ok(plain(&ListStoresOutput { stores }, text))
}

pub(crate) async fn use_store(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: UseStoreArgs = parse(arguments)?;
    ctx.shared.registry.ensure_slot(&args.store)?;
    ctx.session.set_default_store(&args.store);
    let text = format!("default store: {}", args.store);
    Ok(plain(&UseStoreOutput { store: args.store }, text))
}

struct OrientRead {
    projections: Vec<ProjectionRow>,
    status: ProjectionStatusOut,
    readiness: EmbeddingReadinessOut,
    dense: DenseDisabled,
}

pub(crate) async fn orient(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: OrientArgs = parse(arguments)?;
    let store = resolve_store(ctx, args.store.as_deref())?;
    let validity_instant = now_secs();
    let (answered, read) = ctx
        .run(&store, ToolKind::Orient, |engine| {
            let mut projections: Vec<ProjectionRow> = read::projections(engine)?
                .into_iter()
                .map(|spec| ProjectionRow {
                    roles: spec.roles.iter().map(|role| role.as_str().to_string()).collect(),
                    fts: spec.fts.is_some(),
                    vector: spec.vector.is_some(),
                    name: spec.name,
                })
                .collect();
            projections.sort_by(|left, right| left.name.cmp(&right.name));
            let status = read::projection_status(engine)?;
            let readiness = read::embedding_readiness(engine)?;
            Ok(OrientRead {
                projections,
                status: ProjectionStatusOut {
                    runtime_embedder_available: status.runtime_embedder_available,
                    runtime_unavailability_reason: status
                        .runtime_unavailability_reason
                        .as_str()
                        .to_string(),
                    projections: status
                        .projections
                        .into_iter()
                        .map(|entry| ProjectionDenseRow {
                            name: entry.name,
                            dense_readiness: entry.dense_readiness.as_str().to_string(),
                        })
                        .collect(),
                    vector_unsupported_kinds: status.vector_unsupported_kinds,
                },
                readiness: EmbeddingReadinessOut {
                    state: readiness.state.as_str().to_string(),
                    usable_embedder: readiness.usable_embedder,
                    pending_count: readiness.pending_count,
                    affected_kinds: readiness.affected_kinds,
                },
                dense: DenseDisabled {
                    disabled: engine.dense_disabled(),
                    reason: engine.dense_disabled_reason(),
                },
            })
        })
        .await?;
    let max_tokens = ctx.shared.config.default_max_tokens;
    let output = OrientOutput {
        envelope: envelope(&answered.store, answered.epoch, validity_instant, vec![], max_tokens),
        projections: read.projections,
        projection_status: read.status,
        embedding_readiness: read.readiness,
        dense_disabled: read.dense,
        kinds: Unavailable::new(),
        counts: Unavailable::new(),
    };
    let mut text = format!(
        "orient store={} epoch={} validity_instant={}\nprojections: {}",
        answered.store,
        answered.epoch,
        validity_instant,
        output.projections.len()
    );
    for projection in &output.projections {
        text.push_str(&format!(
            "\n{} roles={} fts={} vector={}",
            projection.name,
            projection.roles.join(","),
            projection.fts,
            projection.vector
        ));
    }
    text.push_str(&format!(
        "\nembedding: state={} usable_embedder={} pending={}\ndense_disabled: {}\nkinds: unavailable\ncounts: unavailable",
        output.embedding_readiness.state,
        output.embedding_readiness.usable_embedder,
        output.embedding_readiness.pending_count,
        output.dense_disabled.disabled
    ));
    Ok(finish(&output, text, max_tokens, false).0)
}

struct SearchRun {
    hits: Vec<SearchHit>,
    explanation: Option<Explanation>,
    soft_fallback: Option<SoftFallbackBranch>,
    dense_disabled: bool,
    degradations: Vec<ArmDegradation>,
}

impl SearchRun {
    fn from_result(result: SearchResult, dense_disabled: bool) -> Self {
        Self {
            hits: result.results,
            explanation: result.explanation,
            soft_fallback: result.soft_fallback.map(|fallback| fallback.branch),
            dense_disabled,
            degradations: result.degradations,
        }
    }
}

/// The `degradation` list of a search answer, before canonical ordering.
fn search_degradations(
    mode: SearchMode,
    run: &SearchRun,
    embedder_usable: bool,
) -> Vec<Degradation> {
    let mut degradation = Vec::new();
    if mode == SearchMode::Hybrid {
        if let Some(branch) = run.soft_fallback {
            degradation.push(Degradation {
                arm: branch_str(branch).to_string(),
                reason: "soft_fallback".to_string(),
            });
        }
        if run.dense_disabled {
            degradation.push(Degradation {
                arm: "vector".to_string(),
                reason: "dense_disabled".to_string(),
            });
        }
        if !embedder_usable {
            degradation.push(Degradation {
                arm: "vector".to_string(),
                reason: "embedder_unavailable".to_string(),
            });
        }
        degradation.extend(run.degradations.iter().map(|entry| Degradation {
            arm: branch_str(entry.arm).to_string(),
            reason: entry.reason.as_str().to_string(),
        }));
    }
    degradation
}

fn search_filter(canonical: &CanonicalSearch) -> Option<SearchFilter> {
    let any = canonical.kind.is_some()
        || canonical.source_type.is_some()
        || canonical.status.is_some()
        || canonical.created_after.is_some()
        || !canonical.attributes.is_empty();
    if !any {
        return None;
    }
    let mut filter = SearchFilter::default();
    filter.kind = canonical.kind.clone();
    filter.source_type = canonical.source_type.clone();
    filter.status = canonical.status.clone();
    filter.created_after = canonical.created_after;
    filter.attributes = canonical.attributes.clone();
    Some(filter)
}

fn trace_out(explanation: &Explanation) -> TraceOut {
    let trace = &explanation.trace;
    TraceOut {
        query_chars: trace.query_chars,
        k: trace.k,
        rerank_depth: trace.rerank_depth,
        pool_n: trace.pool_n,
        alpha: trace.alpha,
        use_graph_arm: trace.use_graph_arm,
        recency: trace.recency,
        embedder_id: trace.embedder_id.clone(),
        ce_active: trace.ce_active,
        vector_hits: trace.vector_hits,
        text_hits: trace.text_hits,
        graph_hits: trace.graph_hits,
        dropped_edge_hits: trace.dropped_edge_hits,
    }
}

fn per_hit_out(reference: &str, explain: &PerHitExplain) -> PerHitOut {
    PerHitOut {
        reference: reference.to_string(),
        arm: branch_str(explain.arm).to_string(),
        vector_rank: explain.vector_rank,
        text_rank: explain.text_rank,
        graph_rank: explain.graph_rank,
        fused_score: explain.fused_score,
        ce_score: explain.ce_score,
        blended: explain.blended,
    }
}

fn render_search(output: &SearchOutput) -> String {
    let mode = match output.mode {
        ModeWord::Text => "text",
        ModeWord::Hybrid => "hybrid",
    };
    let mut text = format!(
        "search store={} epoch={} mode={} completeness={} results={} validity_instant={}\nresponse_id={} continuation={}\ndegradation={}",
        output.envelope.store,
        output.envelope.epoch,
        mode,
        output.completeness.as_str(),
        output.results.len(),
        output.envelope.validity_instant,
        output.response_id,
        output.continuation.as_deref().unwrap_or("none"),
        shape::render_degradation(&output.envelope.degradation),
    );
    let per_hit = match &output.explanation {
        Some(ExplanationOut::Detail { per_hit, .. }) => Some(per_hit),
        _ => None,
    };
    for (index, hit) in output.results.iter().enumerate() {
        text.push_str(&format!(
            "\n{}. {} kind={} score={} arm={} source={} snippet={}{}",
            index + 1,
            hit.reference,
            shape::quote(&hit.kind),
            hit.score,
            hit.arm,
            hit.source_id.as_deref().map_or_else(|| "none".to_string(), shape::quote),
            shape::quote(&hit.snippet),
            if hit.snippet_cut { " (cut)" } else { "" }
        ));
        if let Some(explain) = per_hit.and_then(|per_hit| per_hit.get(index)) {
            text.push_str(&format!(
                "\n   explain: vector_rank={:?} text_rank={:?} graph_rank={:?} fused={} blended={}",
                explain.vector_rank,
                explain.text_rank,
                explain.graph_rank,
                explain.fused_score,
                explain.blended
            ));
        }
    }
    if let Some(ExplanationOut::Unavailable { reason, .. }) = &output.explanation {
        text.push_str(&format!("\nexplanation: unavailable ({reason})"));
    }
    text
}

/// Everything the search budget loop reads: the page as cut to snippets, and
/// the request facts that the handles and envelope are derived from.
pub(crate) struct SearchShape<'a> {
    pub(crate) store: &'a str,
    pub(crate) epoch: u64,
    pub(crate) validity_instant: i64,
    pub(crate) degradation: &'a [Degradation],
    pub(crate) max_tokens: u32,
    pub(crate) canonical: &'a CanonicalSearch,
    pub(crate) canonical_json: &'a str,
    pub(crate) offset: usize,
    /// `(hit, write cursor, per-hit explanation)` in rank order.
    pub(crate) page: &'a [(Hit, u64, Option<PerHitOut>)],
    pub(crate) page_limit: usize,
    pub(crate) trace: Option<&'a TraceOut>,
    pub(crate) hosted: bool,
}

/// A shaped search page: the response, its handles, how many hits it kept,
/// and how many times the budget loop rendered it.
pub(crate) struct Shaped {
    pub(crate) rendered: Rendered,
    pub(crate) response_id: String,
    pub(crate) continuation: Option<String>,
    pub(crate) kept: usize,
    pub(crate) iterations: usize,
}

/// The search budget loop: render the page and drop trailing hits until the
/// whole response fits `max_tokens` or nothing is left. Pure and synchronous.
pub(crate) fn shape_search_page(input: &SearchShape<'_>) -> Shaped {
    let SearchShape {
        store,
        epoch,
        validity_instant,
        degradation,
        max_tokens,
        canonical,
        canonical_json,
        offset,
        page,
        page_limit,
        trace,
        hosted,
    } = *input;
    let mode = canonical.search_mode();
    let page_hits = page.len();
    let mut kept = page.len();
    let mut iterations = 0;
    loop {
        iterations += 1;
        let cut = kept < page.len();
        let continuation = shape::next_offset(offset, kept, page_hits, page_limit, MAX_RANKED)
            .map(|next| shape::handle('c', epoch, &[store, canonical_json, &next.to_string()]));
        let refs: Vec<&str> =
            page[..kept].iter().map(|(hit, _, _)| hit.reference.as_str()).collect();
        let response_id = shape::handle(
            'r',
            epoch,
            &[store, canonical_json, &offset.to_string(), &refs.join("\n")],
        );
        let explanation = canonical.explain.then(|| match (trace, mode) {
            (Some(trace), SearchMode::Hybrid) => ExplanationOut::Detail {
                trace: trace.clone(),
                per_hit: page[..kept]
                    .iter()
                    .filter_map(|(_, _, explain)| explain.clone())
                    .collect(),
            },
            _ => ExplanationOut::Unavailable {
                status: UnavailableWord::Unavailable,
                reason: if mode == SearchMode::Text { "text_mode" } else { "not_returned" }
                    .to_string(),
            },
        });
        let output = SearchOutput {
            envelope: envelope(store, epoch, validity_instant, degradation.to_vec(), max_tokens),
            mode: match mode {
                SearchMode::Text => ModeWord::Text,
                SearchMode::Hybrid => ModeWord::Hybrid,
            },
            response_id: response_id.clone(),
            results: page[..kept].iter().map(|(hit, _, _)| hit.clone()).collect(),
            completeness: shape::completeness(
                kept,
                page_hits,
                page_limit,
                cut,
                !degradation.is_empty(),
            ),
            verdict: UnavailableWord::Unavailable,
            continuation: continuation.clone(),
            continuation_exact: hosted.then_some(false),
            explanation,
        };
        let text = render_search(&output);
        let (rendered, whole) = finish(&output, text, max_tokens, cut);
        if whole <= u64::from(max_tokens) || kept == 0 {
            return Shaped { rendered, response_id, continuation, kept, iterations };
        }
        kept -= 1;
    }
}

pub(crate) async fn search(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: SearchArgs = parse(arguments)?;
    let store = resolve_store(ctx, args.store.as_deref())?;
    let max_tokens = max_tokens(ctx, args.max_tokens)?;
    let (canonical, offset, validity_instant, continued_epoch) = match &args.continuation {
        Some(handle) => {
            let only_paging = args.query.is_none()
                && args.mode.is_none()
                && args.kind.is_none()
                && args.source_type.is_none()
                && args.status.is_none()
                && args.created_after.is_none()
                && args.attributes.is_none()
                && args.as_of.is_none()
                && args.limit.is_none()
                && args.explain.is_none();
            if !only_paging {
                return Err(ToolError::bad_argument(
                    "a continuation call takes only store, continuation, max_tokens and hints",
                ));
            }
            ctx.shared.registry.ensure_slot(&store)?;
            let entry = ctx
                .session
                .continuation(&store, handle)
                .ok_or_else(|| ToolError::stale_handle("continuation"))?;
            (entry.args, entry.offset, entry.validity_instant, Some(entry.epoch))
        }
        None => {
            let query = args.query.clone().unwrap_or_default();
            if query.trim().is_empty() {
                return Err(ToolError::bad_argument("query must be a non-empty string"));
            }
            let mode = match args.mode {
                Some(ModeArg::Text) => SearchMode::Text,
                Some(ModeArg::Hybrid) => SearchMode::Hybrid,
                None => ctx.shared.config.search_mode,
            };
            let filtered = args.kind.is_some()
                || args.source_type.is_some()
                || args.status.is_some()
                || args.created_after.is_some()
                || args.attributes.is_some();
            if mode == SearchMode::Text && filtered {
                return Err(ToolError::bad_argument(
                    "kind, source_type, status, created_after and attributes need mode \"hybrid\"",
                ));
            }
            let limit = args.limit.unwrap_or(10);
            if !(1..=MAX_RANKED as u64).contains(&limit) {
                return Err(ToolError::bad_argument("limit must be in 1..=100"));
            }
            let canonical = CanonicalSearch {
                query,
                mode: mode.as_str(),
                kind: args.kind.clone(),
                source_type: args.source_type.clone(),
                status: args.status.clone(),
                created_after: args.created_after,
                attributes: args.attributes.clone().unwrap_or_default().into_iter().collect(),
                as_of: args.as_of,
                limit: limit as usize,
                explain: args.explain.unwrap_or(false),
            };
            (canonical, 0, args.as_of.unwrap_or_else(now_secs), None)
        }
    };

    let mode = canonical.search_mode();
    let kind = match mode {
        SearchMode::Text => ToolKind::SearchText,
        SearchMode::Hybrid => ToolKind::SearchHybrid,
    };
    let guard = ctx.acquire(&store, kind, ctx.deadline()).await?;
    if continued_epoch.is_some_and(|epoch| epoch != guard.epoch) {
        return Err(ToolError::stale_handle("continuation"));
    }
    let telemetry = guard.info.telemetry;
    let (run_limit, page_limit) = shape::page_window(offset, canonical.limit, MAX_RANKED);
    let view = ReadView { valid_as_of: Some(validity_instant), ..ReadView::default() };
    let request = canonical.clone();
    // While telemetry is on, hybrid searches run with explain so the query id
    // comes from this search's own correlation id, never the engine-wide
    // `last_telemetry_query_id` (MR-024.6).
    let explain = request.explain || telemetry;
    let started_limit = ctx.run_limit();
    let (answered, run) = bridge::call(
        guard,
        started_limit,
        &ctx.cancel,
        ctx.progress.as_ref(),
        &ctx.shared.hooks,
        move |engine| {
            let result = match mode {
                SearchMode::Text => engine.search_text_only(
                    &request.query,
                    TextSearchOptions { view, limit: run_limit },
                )?,
                SearchMode::Hybrid => engine.search(
                    &request.query,
                    SearchOptions {
                        filter: search_filter(&request).map(SearchFilterArg::Search),
                        explain,
                        view,
                        limit: run_limit,
                        ..SearchOptions::default()
                    },
                )?,
            };
            Ok(SearchRun::from_result(result, engine.dense_disabled()))
        },
    )
    .await?;

    let degradation = shape::canonical_degradation(search_degradations(
        mode,
        &run,
        answered.info.embedder_usable,
    ));
    let query_id = match (&run.explanation, telemetry && mode == SearchMode::Hybrid) {
        (Some(explanation), true) if explanation.correlation_id.starts_with('q') => {
            Some(explanation.correlation_id.clone())
        }
        _ => None,
    };
    let per_hit: Vec<Option<&PerHitExplain>> = (0..run.hits.len())
        .map(|index| {
            run.explanation.as_ref().and_then(|explanation| explanation.per_hit.get(index))
        })
        .collect();
    let snippet_cap = ctx.shared.config.snippet_chars as usize;
    let page: Vec<(Hit, u64, Option<PerHitOut>)> = run
        .hits
        .iter()
        .zip(per_hit)
        .skip(offset)
        .map(|(hit, explain)| {
            let reference = hit.id.to_prefixed();
            let (snippet, snippet_cut) = shape::snippet(&hit.body, snippet_cap);
            let per_hit = explain.map(|explain| per_hit_out(&reference, explain));
            (
                Hit {
                    reference,
                    kind: hit.kind.clone(),
                    snippet,
                    snippet_cut,
                    source_id: hit.source_id.clone(),
                    score: hit.score,
                    arm: branch_str(hit.branch).to_string(),
                    ce_score: hit.ce_score,
                },
                hit.write_cursor,
                per_hit,
            )
        })
        .collect();
    let canonical_json = serde_json::to_string(&canonical).expect("serializes");
    let trace = run.explanation.as_ref().map(trace_out);
    let shaped = shape_search_page(&SearchShape {
        store: &store,
        epoch: answered.epoch,
        validity_instant,
        degradation: &degradation,
        max_tokens,
        canonical: &canonical,
        canonical_json: &canonical_json,
        offset,
        page: &page,
        page_limit,
        trace: trace.as_ref(),
        hosted: answered.info.hosted,
    });
    let Shaped { rendered, response_id, continuation, kept, .. } = shaped;

    if let Some(handle) = continuation {
        ctx.session.put_continuation(
            &store,
            handle,
            Continuation {
                epoch: answered.epoch,
                args: canonical.clone(),
                offset: offset + kept,
                validity_instant,
            },
        );
    }
    ctx.session.put_response(
        &store,
        response_id,
        ResponseContext {
            epoch: answered.epoch,
            mode,
            query_id,
            refs: page[..kept]
                .iter()
                .map(|(hit, cursor, _)| (hit.reference.clone(), *cursor))
                .collect(),
        },
    );
    Ok(rendered)
}

fn render_fetch(output: &FetchOutput) -> String {
    let mut text = format!(
        "fetch store={} epoch={} items={} validity_instant={}",
        output.envelope.store,
        output.envelope.epoch,
        output.items.len(),
        output.envelope.validity_instant
    );
    for item in &output.items {
        match item {
            FetchItem::Record { reference, record } => text.push_str(&format!(
                "\n{} kind={} body={}{}",
                reference,
                shape::quote(&record.kind),
                shape::quote(&record.body),
                if record.body_cut { " (cut)" } else { "" }
            )),
            FetchItem::Error { reference, error } => text.push_str(&format!(
                "\n{} error={}/{}",
                shape::quote(reference),
                error.class_str(),
                error.code
            )),
        }
    }
    text
}

enum Slot {
    Record(String),
    Error(ToolError),
}

pub(crate) async fn fetch(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: FetchArgs = parse(arguments)?;
    let store = resolve_store(ctx, args.store.as_deref())?;
    let max_tokens = max_tokens(ctx, args.max_tokens)?;
    if args.refs.is_empty() || args.refs.len() > MAX_FETCH_REFS {
        return Err(ToolError::bad_argument("refs must hold 1..=50 references"));
    }
    let slots: Vec<Slot> = args
        .refs
        .iter()
        .map(|reference| match parse_ref(reference) {
            ParsedRef::Logical(id) => Slot::Record(id),
            ParsedRef::Unsupported => Slot::Error(ToolError::unsupported_ref()),
            ParsedRef::Malformed => Slot::Error(ToolError::malformed_ref()),
        })
        .collect();
    let ids: Vec<String> = slots
        .iter()
        .filter_map(|slot| match slot {
            Slot::Record(id) => Some(id.clone()),
            Slot::Error(_) => None,
        })
        .collect();
    let validity_instant = args.as_of.unwrap_or_else(now_secs);
    let view = ReadView {
        include_superseded: args.include_superseded.unwrap_or(false),
        include_inactive: args.include_inactive.unwrap_or(false),
        valid_as_of: Some(validity_instant),
        ..ReadView::default()
    };
    let (answered, records) = ctx
        .run(&store, ToolKind::Fetch, move |engine| {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            read::get_many(engine, &ids, Some(&view))
        })
        .await?;
    let mut records = records.into_iter();
    // (reference, kind, body, error) in request order.
    let mut resolved: Vec<(String, Option<String>, Option<ToolError>)> = Vec::new();
    let mut bodies: Vec<(String, bool)> = Vec::new();
    for (reference, slot) in args.refs.iter().zip(slots) {
        match slot {
            Slot::Error(error) => resolved.push((reference.clone(), None, Some(error))),
            Slot::Record(_) => match records.next().flatten() {
                Some(record) => {
                    resolved.push((reference.clone(), Some(record.kind), None));
                    bodies.push((record.body, false));
                }
                None => resolved.push((reference.clone(), None, Some(ToolError::not_found()))),
            },
        }
    }
    let build = |bodies: &[(String, bool)]| -> FetchOutput {
        let mut bodies = bodies.iter();
        let items = resolved
            .iter()
            .map(|(reference, kind, error)| match (kind, error) {
                (Some(kind), _) => {
                    let (body, body_cut) = bodies.next().cloned().unwrap_or_default();
                    FetchItem::Record {
                        reference: reference.clone(),
                        record: RecordOut { kind: kind.clone(), body, body_cut },
                    }
                }
                (None, error) => FetchItem::Error {
                    reference: reference.clone(),
                    error: error.clone().unwrap_or_else(ToolError::internal),
                },
            })
            .collect();
        FetchOutput {
            envelope: envelope(&store, answered.epoch, validity_instant, vec![], max_tokens),
            items,
        }
    };
    shape::cut_bodies(&mut bodies, |bodies| {
        let output = build(bodies);
        let cut = bodies.iter().any(|(_, cut)| *cut);
        bytes_over(finish(&output, render_fetch(&output), max_tokens, cut).1, max_tokens)
    });
    let output = build(&bodies);
    let cut = bodies.iter().any(|(_, cut)| *cut);
    let (rendered, whole) = finish(&output, render_fetch(&output), max_tokens, cut);
    if whole > u64::from(max_tokens) {
        return Err(ToolError::bad_argument(
            "the result cannot fit max_tokens even with empty bodies; fetch fewer refs or raise max_tokens",
        ));
    }
    Ok(rendered)
}

fn degradation_code(code: GraphExpansionDegradationCodeV1) -> &'static str {
    match code {
        GraphExpansionDegradationCodeV1::QuerySeedTextFallback => "query_seed_text_fallback",
        GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified => {
            "projection_legacy_unverified"
        }
        GraphExpansionDegradationCodeV1::ProjectionProcessing => "projection_processing",
        GraphExpansionDegradationCodeV1::ProjectionBlocked => "projection_blocked",
        GraphExpansionDegradationCodeV1::ProjectionDeferred => "projection_deferred",
        GraphExpansionDegradationCodeV1::ProjectionDegraded => "projection_degraded",
    }
}

fn render_expand(output: &ExpandOutput) -> String {
    let mut text = format!(
        "expand store={} epoch={} targets={} complete={} bound={} validity_instant={}\ndegradation={}",
        output.envelope.store,
        output.envelope.epoch,
        output.targets.len(),
        output.complete,
        output.bound.as_deref().unwrap_or("none"),
        output.envelope.validity_instant,
        shape::render_degradation(&output.envelope.degradation)
    );
    for target in &output.targets {
        text.push_str(&format!(
            "\n{} kind={} hops={} via={} body={}{}",
            target.reference,
            shape::quote(&target.kind),
            target.hop_count,
            shape::quote(&target.terminal_edge_kind),
            shape::quote(&target.body),
            if target.body_cut { " (cut)" } else { "" }
        ));
    }
    text
}

pub(crate) async fn expand(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: ExpandArgs = parse(arguments)?;
    let store = resolve_store(ctx, args.store.as_deref())?;
    let max_tokens = max_tokens(ctx, args.max_tokens)?;
    if args.refs.is_empty() || args.refs.len() > MAX_EXPAND_REFS {
        return Err(ToolError::bad_argument("refs must hold 1..=10 references"));
    }
    let mut ids = Vec::new();
    for reference in &args.refs {
        match parse_ref(reference) {
            ParsedRef::Logical(id) => ids.push(IdSpace::logical(id)),
            ParsedRef::Unsupported => return Err(ToolError::unsupported_ref()),
            ParsedRef::Malformed => return Err(ToolError::malformed_ref()),
        }
    }
    let max_depth = args.max_depth.unwrap_or(1);
    if max_depth > 3 {
        return Err(ToolError::bad_argument("max_depth must be in 0..=3"));
    }
    let limit = args.limit.unwrap_or(20);
    if !(1..=50).contains(&limit) {
        return Err(ToolError::bad_argument("limit must be in 1..=50"));
    }
    let direction = match args.direction.unwrap_or(DirectionArg::Out) {
        DirectionArg::Out => TraversalDirection::Outgoing,
        DirectionArg::In => TraversalDirection::Incoming,
        DirectionArg::Both => TraversalDirection::Both,
    };
    let validity_instant = args.as_of.unwrap_or_else(now_secs);
    let view = ReadView { valid_as_of: Some(validity_instant), ..ReadView::default() };
    let edge_kinds = args.edge_kinds.unwrap_or_default();
    let target_kinds = args.target_kinds.unwrap_or_default();
    let (answered, result) = ctx
        .run(&store, ToolKind::Expand, move |engine| {
            let request = GraphExpandRequestV1 {
                schema_version: 1,
                seed: GraphSeedV1::Explicit { schema_version: 1, logical_ids: ids },
                direction,
                edge_kinds,
                target_kinds,
                context: GraphReadContextV1::Current {
                    schema_version: 1,
                    context: ReadContextV1::new(view, SearchFilter::default())?,
                },
                max_depth: u32::try_from(max_depth).unwrap_or(3),
                result_limit: u32::try_from(limit).unwrap_or(50),
                max_work_units: EXPAND_WORK_UNITS,
                include_explanation: true,
                include_evidence: false,
            };
            graph::expand(engine, &request)
        })
        .await?;
    let degradation = shape::canonical_degradation(
        result
            .degradation_codes
            .iter()
            .map(|code| Degradation {
                arm: "graph".to_string(),
                reason: degradation_code(*code).to_string(),
            })
            .collect(),
    );
    let bound = if result.targets.len() as u64 >= limit {
        Some("result_limit".to_string())
    } else if !result.complete {
        Some("unspecified".to_string())
    } else {
        None
    };
    let mut bodies: Vec<(String, bool)> =
        result.targets.iter().map(|target| (target.body.clone(), false)).collect();
    let mut kept = result.targets.len();
    let build = |bodies: &[(String, bool)], kept: usize| -> ExpandOutput {
        ExpandOutput {
            envelope: envelope(
                &store,
                answered.epoch,
                validity_instant,
                degradation.clone(),
                max_tokens,
            ),
            targets: result.targets[..kept]
                .iter()
                .zip(bodies)
                .map(|(target, (body, body_cut))| Target {
                    reference: IdSpace::logical(target.logical_id.clone()).to_prefixed(),
                    kind: target.kind.clone(),
                    hop_count: target.origin.hop_count,
                    terminal_edge_kind: target.origin.terminal_edge_kind.clone(),
                    body: body.clone(),
                    body_cut: *body_cut,
                })
                .collect(),
            complete: result.complete,
            // Dropping targets to fit the budget is the cap the agent hit.
            bound: if kept < result.targets.len() {
                Some("budget".to_string())
            } else {
                bound.clone()
            },
        }
    };
    shape::cut_bodies(&mut bodies, |bodies| {
        let output = build(bodies, kept);
        let cut = bodies.iter().any(|(_, cut)| *cut);
        bytes_over(finish(&output, render_expand(&output), max_tokens, cut).1, max_tokens)
    });
    loop {
        let output = build(&bodies[..kept], kept);
        let cut = kept < result.targets.len() || bodies.iter().any(|(_, cut)| *cut);
        let (rendered, whole) = finish(&output, render_expand(&output), max_tokens, cut);
        if whole <= u64::from(max_tokens) || kept == 0 {
            return Ok(rendered);
        }
        kept -= 1;
    }
}

fn not_recorded(store: &str, epoch: u64, reason: NotRecordedReason, max_tokens: u32) -> Rendered {
    let output = FeedbackOutput {
        envelope: envelope(store, epoch, now_secs(), vec![], max_tokens),
        status: FeedbackStatus::NotRecorded,
        reason: Some(reason),
    };
    let reason_text = serde_json::to_value(reason).expect("serializes");
    let text = format!(
        "feedback store={store} status=not_recorded reason={}",
        reason_text.as_str().unwrap_or("")
    );
    finish(&output, text, max_tokens, false).0
}

pub(crate) async fn feedback(ctx: &Ctx<'_>, arguments: Map<String, Value>) -> Outcome {
    let args: FeedbackArgs = parse(arguments)?;
    let store = resolve_store(ctx, args.store.as_deref())?;
    ctx.shared.registry.ensure_slot(&store)?;
    let max_tokens = ctx.shared.config.default_max_tokens;
    let current_epoch = ctx.shared.registry.ready_epoch(&store);
    let entry = match ctx.session.response(&store, &args.response_id) {
        ResponseLookup::Found(entry) => entry,
        ResponseLookup::OtherStore => return Err(ToolError::wrong_store(&store)),
        ResponseLookup::Missing => {
            return Ok(not_recorded(
                &store,
                current_epoch.unwrap_or(0),
                NotRecordedReason::ResponseExpired,
                max_tokens,
            ))
        }
    };
    let unused = args.unused.clone().unwrap_or_default();
    let named: BTreeSet<&String> = args.used.iter().chain(unused.iter()).collect();
    if named.iter().any(|reference| !entry.refs.contains_key(*reference)) {
        return Err(ToolError::bad_argument(
            "used and unused may only name references from that response",
        ));
    }
    if current_epoch != Some(entry.epoch) {
        return Ok(not_recorded(
            &store,
            current_epoch.unwrap_or(entry.epoch),
            NotRecordedReason::StoreReopened,
            max_tokens,
        ));
    }
    if ctx.shared.config.telemetry_dir.is_none() {
        return Ok(not_recorded(
            &store,
            entry.epoch,
            NotRecordedReason::TelemetryDisabled,
            max_tokens,
        ));
    }
    if entry.mode == SearchMode::Text {
        return Ok(not_recorded(&store, entry.epoch, NotRecordedReason::TextMode, max_tokens));
    }
    let Some(query_id) = entry.query_id.clone() else {
        return Ok(not_recorded(
            &store,
            entry.epoch,
            NotRecordedReason::TelemetryDisabled,
            max_tokens,
        ));
    };
    let used: Vec<u64> = args.used.iter().map(|reference| entry.refs[reference]).collect();
    let unused: Vec<u64> = unused.iter().map(|reference| entry.refs[reference]).collect();
    // No waiting: a store that is no longer ready can only come back under a
    // new epoch, whose sink never saw this response.
    let guard = match ctx
        .shared
        .registry
        .acquire(
            &store,
            &Admission { kind: ToolKind::Feedback, wait_until: None, progress: None, cancel: None },
        )
        .await
    {
        Ok(guard) => guard,
        Err(error) if error.class_str() == "not_ready" => {
            return Ok(not_recorded(
                &store,
                entry.epoch,
                NotRecordedReason::StoreReopened,
                max_tokens,
            ))
        }
        Err(error) => return Err(error),
    };
    if guard.epoch != entry.epoch {
        return Ok(not_recorded(&store, guard.epoch, NotRecordedReason::StoreReopened, max_tokens));
    }
    let (answered, ()) =
        bridge::call(guard, ctx.run_limit(), &ctx.cancel, None, &ctx.shared.hooks, move |engine| {
            engine.record_feedback(&query_id, &used, &unused, "agent")
        })
        .await?;
    let output = FeedbackOutput {
        envelope: envelope(&store, answered.epoch, now_secs(), vec![], max_tokens),
        status: FeedbackStatus::Recorded,
        reason: None,
    };
    let text = format!("feedback store={store} status=recorded");
    Ok(finish(&output, text, max_tokens, false).0)
}

#[cfg(test)]
mod degradation_tests {
    use super::*;
    use fathomdb_sdk::{
        ArmDegradation, DegradationReason, Engine, InitialState, OpenOptions, PreparedWrite,
        SourceId,
    };

    fn entry(arm: &str, reason: &str) -> Degradation {
        Degradation { arm: arm.to_string(), reason: reason.to_string() }
    }

    /// A real hybrid `SearchResult` from a store opened through the SDK the
    /// adapter uses. The SDK cannot saturate the engine's embed queue (no
    /// caller-embedder route), so the engine's overloaded report is appended to
    /// this real value; the engine side of the report is tested in
    /// `fathomdb-engine/tests/ep03_embedder_overloaded.rs`.
    fn overloaded_hybrid_result() -> SearchResult {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = Engine::open(dir.path().join("mcp-ep03.sqlite"), OpenOptions::default())
            .expect("open through the SDK");
        engine
            .write(&[PreparedWrite::Node {
                kind: "note".to_string(),
                body: "mcpep03needle note".to_string(),
                source_id: SourceId::new("test:mcp-ep03").unwrap(),
                logical_id: Some("mcp-ep03".to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        let mut result = engine.search("mcpep03needle", SearchOptions::default()).unwrap();
        engine.close().unwrap();
        assert!(!result.results.is_empty(), "precondition: the text arm answered");
        result.degradations.push(ArmDegradation::new(
            SoftFallbackBranch::Vector,
            DegradationReason::EmbedderOverloaded,
        ));
        result
    }

    #[test]
    fn hybrid_search_reports_embedder_overloaded_degradation() {
        let run = SearchRun::from_result(overloaded_hybrid_result(), false);
        assert_eq!(
            search_degradations(SearchMode::Hybrid, &run, true),
            vec![entry("vector", "embedder_overloaded")]
        );
        assert_eq!(
            shape::canonical_degradation(search_degradations(SearchMode::Hybrid, &run, false)),
            vec![entry("vector", "embedder_overloaded"), entry("vector", "embedder_unavailable")],
            "the existing entries stay"
        );
    }
}

use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use fathomdb_engine::lifecycle::Subscriber;
use fathomdb_engine::{
    ActuationBatchV1, ActuationReceiptV1, ClosureLookupV1, ClosureStatusV1, ConsolidateAxis,
    ConsolidateReceipt, CounterSnapshot, DependencyDerivedLookupV1, DependencyListV1,
    DependencySourceLookupV1, DependencyTraceRequestV1, DependencyTraceResultV1, EmbedderChoice,
    Engine as CoreEngine, EngineConfig, EngineError, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, EvidenceSearchResultV1, ExciseReport, ExtractDocument,
    FrozenReadContextV1, GraphEvidenceResolveRequestV1, IngestWithExtractorReceipt, LifecycleState,
    OpenReport, PreparedWrite, ProjectionDelta, ProjectionSpec, ReadContextV1, ResolvedEvidenceV1,
    ResolvedGraphEvidenceV1, SearchExpandResult, SearchResult, SourceDependencyRegistrationV1,
    SourceDependencyV1, Subscription, WriteReceipt,
};

use crate::error::{Error, Result};
use crate::guard;
use crate::options::{
    FrozenSearchOptions, OpenOptions, ProjectedTextSearchOptions, SearchExpandOptions,
    SearchOptions, TextSearchOptions,
};

/// An open FathomDB database: the Rust peer of the Python and TypeScript
/// `Engine`. Namespace operations live in [`crate::read`], [`crate::graph`]
/// and [`crate::admin`].
///
/// Calls are synchronous and the handle is `Send + Sync`. `close` is
/// idempotent; after it, operations fail with `ErrorKind::Closing`.
pub struct Engine {
    core: CoreEngine,
    report: OpenReport,
    config: EngineConfig,
    subscriber: Mutex<SubscriberSlot>,
}

#[derive(Default)]
struct SubscriberSlot {
    closing: bool,
    current: Option<Subscription>,
}

impl Engine {
    /// Open (creating if absent) the database at `path`.
    ///
    /// # Errors
    /// `InvalidArgument` for an out-of-range `EngineConfig` field, the typed
    /// open errors (`DatabaseLocked`, `Corruption`, `Migration`, ...), and
    /// `Embedder` when `use_default_embedder` is set without the
    /// `default-embedder` feature.
    pub fn open<P: AsRef<Path>>(path: P, options: OpenOptions) -> Result<Engine> {
        let path = path.as_ref();
        guard::path(path)?;
        let choice = if options.use_default_embedder {
            EmbedderChoice::Default
        } else {
            EmbedderChoice::None
        };
        let opened = CoreEngine::open_with_choice_and_config(path, choice, options.config.clone())?;
        Ok(Engine {
            core: opened.engine,
            report: opened.report,
            config: options.config,
            subscriber: Mutex::new(SubscriberSlot::default()),
        })
    }

    pub(crate) fn core(&self) -> &CoreEngine {
        &self.core
    }

    /// The report produced when this database was opened.
    pub fn open_report(&self) -> &OpenReport {
        &self.report
    }

    /// The configuration requested at open.
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// Close the database. Idempotent; detaches any subscriber first.
    pub fn close(&self) -> Result<()> {
        {
            let mut slot = self.slot();
            slot.closing = true;
            slot.current = None;
        }
        Ok(self.core.close()?)
    }

    /// Wait up to `timeout_ms` for pending projection work.
    pub fn drain(&self, timeout_ms: u64) -> Result<()> {
        Ok(self.core.drain(timeout_ms)?)
    }

    /// Commit a batch of typed writes atomically.
    pub fn write(&self, batch: &[PreparedWrite]) -> Result<WriteReceipt> {
        batch.iter().try_for_each(guard::prepared_write)?;
        Ok(self.core.write(batch)?)
    }

    /// Apply one atomic actuation batch.
    pub fn actuate(&self, request: ActuationBatchV1) -> Result<ActuationReceiptV1> {
        Ok(self.core.actuate(request)?)
    }

    /// Register a derived artifact's source dependency.
    pub fn register_source_dependency(
        &self,
        request: SourceDependencyRegistrationV1,
    ) -> Result<SourceDependencyV1> {
        Ok(self.core.register_source_dependency(request)?)
    }

    /// List dependencies registered against one source revision.
    pub fn dependencies_for_source(
        &self,
        request: DependencySourceLookupV1,
    ) -> Result<DependencyListV1> {
        Ok(self.core.dependencies_for_source(request)?)
    }

    /// The dependency registered for one derived artifact, if any.
    pub fn dependency_for_derived(
        &self,
        request: DependencyDerivedLookupV1,
    ) -> Result<Option<SourceDependencyV1>> {
        Ok(self.core.dependency_for_derived(request)?)
    }

    /// Read a dependency closure's status, if registered.
    pub fn read_dependency_closure(
        &self,
        request: ClosureLookupV1,
    ) -> Result<Option<ClosureStatusV1>> {
        Ok(self.core.read_dependency_closure(request)?)
    }

    /// Trace a dependency graph under a frozen context.
    pub fn trace_dependency(
        &self,
        request: DependencyTraceRequestV1,
    ) -> Result<DependencyTraceResultV1> {
        Ok(self.core.trace_dependency(request)?)
    }

    /// Move a governed record to `to_state`.
    pub fn transition(
        &self,
        logical_id: &str,
        to_state: LifecycleState,
        reason: Option<&str>,
    ) -> Result<()> {
        guard::text(logical_id)?;
        guard::opt_text(reason)?;
        Ok(self.core.transition(logical_id, to_state, reason.map(str::to_string))?)
    }

    /// Physically remove a deleted governed record.
    pub fn purge(&self, logical_id: &str) -> Result<()> {
        guard::text(logical_id)?;
        Ok(self.core.purge(logical_id)?)
    }

    /// Erase every row written under `source_id`.
    pub fn erase_source(&self, source_id: &str) -> Result<ExciseReport> {
        guard::text(source_id)?;
        Ok(self.core.erase_source(source_id)?)
    }

    /// Declare projections and explicitly drop others.
    pub fn configure_projections(
        &self,
        specs: &[ProjectionSpec],
        drop: &[String],
    ) -> Result<ProjectionDelta> {
        specs.iter().try_for_each(guard::projection_spec)?;
        guard::texts(drop)?;
        Ok(self.core.configure_projections(specs, drop)?)
    }

    /// Ranked hybrid search.
    pub fn search(&self, query: &str, options: SearchOptions) -> Result<SearchResult> {
        let SearchOptions {
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            view,
            limit,
        } = options;
        guard::ranked_limit("limit", limit)?;
        let alpha = guard::alpha(alpha)?;
        guard::text(query)?;
        let filter = filter.map(|filter| filter.lower()).transpose()?;
        Ok(self.core.search_reranked_view_with_limit(
            query,
            filter,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n.unwrap_or(rerank_depth),
            explain,
            &view,
            limit,
        )?)
    }

    /// Lexical (FTS-only) search; never embeds the query.
    pub fn search_text_only(
        &self,
        query: &str,
        options: TextSearchOptions,
    ) -> Result<SearchResult> {
        guard::ranked_limit("limit", options.limit)?;
        guard::text(query)?;
        Ok(self.core.search_text_only_view_with_limit(query, &options.view, options.limit)?)
    }

    /// Lexical search over one declared property-FTS projection.
    pub fn search_projected_text(
        &self,
        query: &str,
        name: &str,
        options: ProjectedTextSearchOptions,
    ) -> Result<SearchResult> {
        guard::ranked_limit("limit", options.limit)?;
        guard::text(query)?;
        guard::text(name)?;
        if let Some(filter) = &options.filter {
            guard::search_filter(filter)?;
        }
        Ok(self.core.search_projected_text_with_limit(
            query,
            name,
            options.filter,
            &options.view,
            options.limit,
        )?)
    }

    /// Freeze a validity view and eligibility filter for repeatable reads.
    pub fn freeze_read_context(&self, context: &ReadContextV1) -> Result<FrozenReadContextV1> {
        guard::search_filter(&context.eligibility)?;
        Ok(self.core.freeze_read_context(context)?)
    }

    /// Ranked search under a frozen context; defaults as [`Engine::search`].
    pub fn search_frozen(
        &self,
        query: &str,
        context: &FrozenReadContextV1,
        options: FrozenSearchOptions,
    ) -> Result<SearchResult> {
        let FrozenSearchOptions { rerank_depth, use_graph_arm, alpha, pool_n, explain, limit } =
            options;
        self.core.validate_frozen_read_context_for_binding(context)?;
        guard::ranked_limit("limit", limit)?;
        let alpha = guard::alpha(alpha)?;
        guard::text(query)?;
        Ok(self.core.search_frozen(
            query,
            context,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n.unwrap_or(rerank_depth),
            explain,
            limit,
        )?)
    }

    /// Search then expand the hits' graph neighbourhood under a frozen context.
    pub fn search_expand_frozen(
        &self,
        query: &str,
        context: &FrozenReadContextV1,
        depth: u32,
        options: SearchExpandOptions,
    ) -> Result<SearchExpandResult> {
        self.core.validate_frozen_read_context_for_binding(context)?;
        guard::ranked_limit("search_limit", options.search_limit)?;
        guard::text(query)?;
        Ok(self.core.search_expand_frozen(query, context, depth, options.search_limit)?)
    }

    /// Frozen search attaching one evidence reference per hit.
    pub fn search_with_evidence(
        &self,
        request: &EvidenceSearchRequestV1,
    ) -> Result<EvidenceSearchResultV1> {
        guard::ranked_limit("limit", request.limit as usize)?;
        guard::text(&request.query)?;
        Ok(self.core.search_with_evidence(request)?)
    }

    /// Resolve an opaque evidence reference.
    pub fn resolve_evidence(
        &self,
        request: &EvidenceResolveRequestV1,
    ) -> Result<ResolvedEvidenceV1> {
        Ok(self.core.resolve_evidence(request)?)
    }

    /// Resolve an opaque graph evidence reference.
    pub fn resolve_graph_evidence(
        &self,
        request: &GraphEvidenceResolveRequestV1,
    ) -> Result<ResolvedGraphEvidenceV1> {
        Ok(self.core.resolve_graph_evidence(request)?)
    }

    /// Run the caller's extractor over `documents` and ingest its output.
    pub fn ingest_with_extractor(
        &self,
        cmd: &[&str],
        documents: &[ExtractDocument],
    ) -> Result<IngestWithExtractorReceipt> {
        Ok(self.core.ingest_with_extractor(cmd, documents)?)
    }

    /// Run the caller's consolidation provider over `axes`.
    pub fn consolidate_with_provider(
        &self,
        cmd: &[&str],
        axes: &[ConsolidateAxis],
    ) -> Result<ConsolidateReceipt> {
        Ok(self.core.consolidate_with_provider(cmd, axes)?)
    }

    /// Embed `text` with the engine's configured (mean-pooled) embedder.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        guard::text(text)?;
        Ok(self.core.embed_text(text)?)
    }

    /// Whether the dense arm is disabled for this open.
    pub fn dense_disabled(&self) -> bool {
        self.core.dense_disabled()
    }

    /// Why the dense arm is disabled, if it is.
    pub fn dense_disabled_reason(&self) -> Option<String> {
        self.core.dense_disabled_reason()
    }

    /// Query-time dense refusals since open.
    pub fn vector_equivalence_refusal_count(&self) -> u64 {
        self.core.vector_equivalence_refusal_count()
    }

    /// Enable opt-in local telemetry capture to a JSONL file.
    pub fn enable_telemetry(&self, sink_path: &str) -> Result<()> {
        guard::text(sink_path)?;
        Ok(self.core.enable_telemetry(sink_path)?)
    }

    /// The query id of the last telemetry-captured search.
    pub fn last_telemetry_query_id(&self) -> Option<String> {
        self.core.last_telemetry_query_id()
    }

    /// Record relevance feedback for a telemetry-captured query.
    pub fn record_feedback(
        &self,
        query_id: &str,
        relevant_ids: &[u64],
        irrelevant_ids: &[u64],
        label_source: &str,
    ) -> Result<()> {
        guard::text(query_id)?;
        guard::text(label_source)?;
        Ok(self.core.record_feedback(query_id, relevant_ids, irrelevant_ids, label_source)?)
    }

    /// Engine counters.
    pub fn counters(&self) -> CounterSnapshot {
        self.core.counters()
    }

    /// Turn per-statement profiling on or off.
    pub fn set_profiling(&self, enabled: bool) -> Result<()> {
        Ok(self.core.set_profiling(enabled)?)
    }

    /// Set the slow-statement threshold in milliseconds.
    pub fn set_slow_threshold_ms(&self, value: u64) -> Result<()> {
        Ok(self.core.set_slow_threshold_ms(value)?)
    }

    /// Attach `subscriber`, replacing any earlier one. Fails with `Closing`
    /// after `close`. `close` and drop detach it.
    pub fn attach_subscriber(&self, subscriber: Arc<dyn Subscriber>) -> Result<()> {
        let mut slot = self.slot();
        if slot.closing {
            return Err(Error::Engine(EngineError::Closing));
        }
        slot.current = Some(self.core.subscribe(subscriber));
        Ok(())
    }

    fn slot(&self) -> MutexGuard<'_, SubscriberSlot> {
        self.subscriber.lock().unwrap_or_else(|poison| poison.into_inner())
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.slot().current = None;
    }
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine").field("path", &self.core.path()).finish_non_exhaustive()
    }
}

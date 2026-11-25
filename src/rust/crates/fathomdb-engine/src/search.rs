mod bm25f;
mod fts_fast_path;
mod fusion_stage;
mod graph_arm;
mod projected_text;
mod vector_arm;

use crate::embed_dispatch::EmbedDispatcher;
use crate::errors::EngineError;
use crate::evidence::{self, EvidenceSearchResultV1};
use crate::filter::{SearchFilter, SnapshotFilterError};
use crate::frozen_read::{self, FrozenReadContextV1, FrozenReadError};
use crate::fusion::FusionHit;
use crate::graph_expand::{self, SearchExpandResult};
use crate::projection_generation::ProjectionRuntimeStateV1;
use crate::projection_runtime::PROJECTION_CURSOR_KEY;
use crate::search_types::{
    ArmDegradation, CapturedGraphOrigin, Explanation, GraphArmOptions, GraphFrontierStats,
    SearchResult, SoftFallback, MAX_SEARCH_RESULT_LIMIT,
};
use crate::snippet::{SnippetOptions, SnippetV1};
use crate::temporal::ReadView;
#[cfg(any(test, feature = "test-hooks"))]
use crate::test_hooks::evidence_linearization_hooks;
use crate::write_commit::load_next_cursor;
use fathomdb_embedder::RerankerDevicePolicyError;
use fathomdb_embedder_api::EmbedderIdentity;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub(crate) use bm25f::bm25f_search_inner;
pub(crate) use fts_fast_path::prepare_search_statement;
#[cfg(test)]
pub(crate) use fts_fast_path::retain_complete_rank_boundary_candidates;
#[cfg(feature = "test-hooks")]
pub(crate) use fts_fast_path::{
    append_json_witness_for_test, record_slice71_profile_statement_for_test,
    slice71_search_statement_trace,
};
pub(crate) use fusion_stage::{read_search_work_in_tx, NoEvidenceCapture};
#[cfg(any(test, feature = "test-hooks"))]
pub(crate) use graph_arm::explain_graph_arm_phase2_query_for_test;
pub(crate) use projected_text::read_projected_text_in_tx;

pub(crate) struct EvidenceCapture {
    frozen: FrozenReadContextV1,
    include_explanation: bool,
    graph_origins: HashMap<u64, CapturedGraphOrigin>,
    #[cfg(any(test, feature = "test-hooks"))]
    before_sidecar: Arc<evidence_linearization_hooks::OwnedHook>,
}

pub(crate) type ProjectedTextReaderResponse = Result<SearchResult, SearchReaderError>;

impl EvidenceCapture {
    pub(crate) fn new(
        frozen: FrozenReadContextV1,
        include_explanation: bool,
        #[cfg(any(test, feature = "test-hooks"))] before_sidecar: Arc<
            evidence_linearization_hooks::OwnedHook,
        >,
    ) -> Self {
        Self {
            frozen,
            include_explanation,
            graph_origins: HashMap::new(),
            #[cfg(any(test, feature = "test-hooks"))]
            before_sidecar,
        }
    }
}

/// Capability payload kept behind one pointer so adding optional search state
/// cannot inflate every request crossing the bounded reader channel.
pub(crate) struct SearchReaderWork {
    #[cfg(any(test, feature = "slice30-observer"))]
    slice30_owner: Option<crate::slice30_observer::OperationToken>,
    #[cfg(feature = "test-hooks")]
    pub(crate) d27_owner: Option<crate::embed_dispatch::d27_observation::Owner>,
    compiled: Option<fathomdb_query::CompiledQuery>,
    query_vector: Option<String>,
    query_vector_bin: Option<String>,
    result_limit: usize,
    candidate_limit: usize,
    direct_text_candidate_limit: Option<usize>,
    filter: Option<Box<SearchFilter>>,
    recency_enabled: bool,
    importance_enabled: bool,
    vector_stage_only: bool,
    raw_query: Box<str>,
    rerank_depth: usize,
    use_graph_arm: bool,
    alpha: f64,
    pool_n: usize,
    explain: bool,
    projection_runtime_state: ProjectionRuntimeStateV1,
    view: ReadView,
    frozen_binding: Option<Box<frozen_read::FrozenReadBinding>>,
    frozen_query_runtime: Option<Box<FrozenQueryRuntime>>,
    expand_depth: Option<u32>,
    snippet: Option<SnippetOptions>,
    graph_options: Option<Box<GraphArmOptions>>,
}

impl SearchReaderWork {
    /// Requests EP-17 snippets for the final hits.
    pub(crate) fn with_snippet(mut self, snippet: Option<SnippetOptions>) -> Self {
        self.snippet = snippet;
        self
    }

    /// Attaches opt-in graph-arm seeding and edge-traversal controls.
    pub(crate) fn with_graph_options(mut self, graph_options: Option<GraphArmOptions>) -> Self {
        self.graph_options = graph_options.filter(|opts| !opts.is_default()).map(Box::new);
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn hybrid(
        compiled: Option<fathomdb_query::CompiledQuery>,
        query_vector: Option<String>,
        query_vector_bin: Option<String>,
        result_limit: usize,
        candidate_limit: usize,
        filter: Option<SearchFilter>,
        recency_enabled: bool,
        importance_enabled: bool,
        vector_stage_only: bool,
        raw_query: &str,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        explain: bool,
        projection_runtime_state: ProjectionRuntimeStateV1,
        view: ReadView,
        frozen_binding: Option<frozen_read::FrozenReadBinding>,
        frozen_query_runtime: Option<Box<FrozenQueryRuntime>>,
        expand_depth: Option<u32>,
    ) -> Self {
        Self {
            #[cfg(any(test, feature = "slice30-observer"))]
            slice30_owner: crate::slice30_observer::capture_token(),
            #[cfg(feature = "test-hooks")]
            d27_owner: crate::embed_dispatch::d27_observation::current_owner(),
            compiled,
            query_vector,
            query_vector_bin,
            result_limit,
            candidate_limit,
            direct_text_candidate_limit: None,
            filter: filter.map(Box::new),
            recency_enabled,
            importance_enabled,
            vector_stage_only,
            raw_query: Box::from(raw_query),
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            explain,
            projection_runtime_state,
            view,
            frozen_binding: frozen_binding.map(Box::new),
            frozen_query_runtime,
            expand_depth,
            snippet: None,
            graph_options: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn evidence(
        result_limit: usize,
        candidate_limit: usize,
        filter: SearchFilter,
        recency_enabled: bool,
        importance_enabled: bool,
        vector_stage_only: bool,
        raw_query: &str,
        rerank_depth: usize,
        use_graph_arm: bool,
        alpha: f64,
        pool_n: usize,
        projection_runtime_state: ProjectionRuntimeStateV1,
        view: ReadView,
        frozen_binding: frozen_read::FrozenReadBinding,
        frozen_query_runtime: FrozenQueryRuntime,
    ) -> Self {
        Self::hybrid(
            None,
            None,
            None,
            result_limit,
            candidate_limit,
            Some(filter),
            recency_enabled,
            importance_enabled,
            vector_stage_only,
            raw_query,
            rerank_depth,
            use_graph_arm,
            alpha,
            pool_n,
            true,
            projection_runtime_state,
            view,
            Some(frozen_binding),
            Some(Box::new(frozen_query_runtime)),
            None,
        )
    }

    pub(crate) fn text_only(
        compiled: fathomdb_query::CompiledQuery,
        query: &str,
        result_limit: usize,
        candidate_limit: usize,
        view: ReadView,
        filter: Option<SearchFilter>,
    ) -> Self {
        Self {
            #[cfg(any(test, feature = "slice30-observer"))]
            slice30_owner: crate::slice30_observer::capture_token(),
            #[cfg(feature = "test-hooks")]
            d27_owner: crate::embed_dispatch::d27_observation::current_owner(),
            compiled: Some(compiled),
            query_vector: None,
            query_vector_bin: None,
            result_limit,
            candidate_limit,
            direct_text_candidate_limit: Some(MAX_SEARCH_RESULT_LIMIT),
            filter: filter.map(Box::new),
            recency_enabled: false,
            importance_enabled: false,
            vector_stage_only: false,
            raw_query: Box::from(query),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.3,
            pool_n: 0,
            explain: false,
            projection_runtime_state: ProjectionRuntimeStateV1::Absent,
            view,
            frozen_binding: None,
            frozen_query_runtime: None,
            expand_depth: None,
            snippet: None,
            graph_options: None,
        }
    }
}

pub(crate) struct FrozenQueryRuntime {
    embed_dispatch: Arc<EmbedDispatcher>,
    embedder_identity: EmbedderIdentity,
    observed_generation: Arc<AtomicU64>,
}

impl FrozenQueryRuntime {
    pub(crate) fn new(
        embed_dispatch: Arc<EmbedDispatcher>,
        embedder_identity: EmbedderIdentity,
        observed_generation: Arc<AtomicU64>,
    ) -> Self {
        Self { embed_dispatch, embedder_identity, observed_generation }
    }
}

pub(crate) trait SearchOriginCapture: Sized {
    type Output;

    fn requires_edge_origin(&self) -> bool {
        false
    }

    fn record_graph_origin(&mut self, _node_cursor: u64, _origin: CapturedGraphOrigin) {}

    #[allow(clippy::too_many_arguments)]
    fn finish(
        self,
        connection: &Connection,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<FusionHit>,
        graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        expanded: Option<SearchExpandResult>,
        degradations: Vec<ArmDegradation>,
        snippets: Option<Vec<SnippetV1>>,
    ) -> Result<Self::Output, SearchReaderError>;
}

impl SearchOriginCapture for EvidenceCapture {
    type Output = EvidenceSearchResultV1;

    fn requires_edge_origin(&self) -> bool {
        true
    }

    fn record_graph_origin(&mut self, node_cursor: u64, origin: CapturedGraphOrigin) {
        self.graph_origins.entry(node_cursor).or_insert(origin);
    }

    fn finish(
        self,
        connection: &Connection,
        cursor: u64,
        soft_fallback: Option<SoftFallback>,
        results: Vec<FusionHit>,
        _graph_stats: GraphFrontierStats,
        explanation: Option<Explanation>,
        _expanded: Option<SearchExpandResult>,
        degradations: Vec<ArmDegradation>,
        _snippets: Option<Vec<SnippetV1>>,
    ) -> Result<Self::Output, SearchReaderError> {
        #[cfg(any(test, feature = "test-hooks"))]
        self.before_sidecar.fire();
        let ownership: Vec<_> = results.iter().map(|hit| hit.ownership).collect();
        let search_result = SearchResult {
            projection_cursor: cursor,
            soft_fallback,
            results: results.into_iter().map(|hit| hit.hit).collect(),
            explanation,
            degradations,
            snippets: None,
            hints: None,
        };
        evidence::build_search_result(
            connection,
            &self.frozen,
            search_result,
            self.include_explanation,
            &self.graph_origins,
            &ownership,
        )
        .map_err(SearchReaderError::Evidence)
    }
}

pub(crate) enum SearchReaderError {
    Sqlite(rusqlite::Error),
    Evidence(EngineError),
    InvalidFilter(String),
    RerankerDevicePolicy(RerankerDevicePolicyError),
    FrozenRead(FrozenReadError),
    WriteValidation,
    InvalidArgument(String),
}

impl From<rusqlite::Error> for SearchReaderError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<SnapshotFilterError> for SearchReaderError {
    fn from(error: SnapshotFilterError) -> Self {
        match error {
            SnapshotFilterError::Sqlite(error) => Self::Sqlite(error),
            SnapshotFilterError::InvalidFilter(reason) => Self::InvalidFilter(reason),
        }
    }
}

impl From<graph_expand::SearchExpandHandlerError> for SearchReaderError {
    fn from(error: graph_expand::SearchExpandHandlerError) -> Self {
        match error {
            graph_expand::SearchExpandHandlerError::Sqlite(error) => Self::Sqlite(error),
            graph_expand::SearchExpandHandlerError::FrozenRead(error) => Self::FrozenRead(error),
            graph_expand::SearchExpandHandlerError::InvalidFilter(reason) => {
                Self::InvalidFilter(reason)
            }
        }
    }
}

fn load_projection_cursor_for_search(connection: &Connection) -> rusqlite::Result<u64> {
    connection
        .prepare_cached("SELECT value FROM _fathomdb_open_state WHERE key = ?1")?
        .query_row([PROJECTION_CURSOR_KEY], |row| row.get::<_, String>(0))
        .map(|value| value.parse::<u64>().unwrap_or(0))
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(0),
            _ => Err(err),
        })
}

fn next_write_cursor_for_search(connection: &Connection) -> u64 {
    load_next_cursor(connection)
}

fn open_historical_property_corpus() -> rusqlite::Result<Connection> {
    let corpus = Connection::open_in_memory()?;
    corpus.execute_batch(
        "PRAGMA temp_store=MEMORY;
        CREATE VIRTUAL TABLE history_property USING fts5(
        attr_value,attr_name UNINDEXED,write_cursor UNINDEXED,eligible UNINDEXED,
        tokenize='porter unicode61 remove_diacritics 2')",
    )?;
    Ok(corpus)
}

#[cfg(all(test, feature = "test-hooks"))]
mod historical_property_lifecycle_tests {
    use super::*;

    #[test]
    fn private_property_corpus_uses_memory_for_main_and_temporary_storage() {
        crate::configure_runtime_for_open().unwrap();
        let corpus = open_historical_property_corpus().unwrap();
        let temp_store: i64 = corpus.query_row("PRAGMA temp_store", [], |row| row.get(0)).unwrap();
        assert_eq!(temp_store, 2);
        let file: String = corpus
            .query_row("SELECT file FROM pragma_database_list WHERE name='main'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(file, "");
    }
}

#[cfg(all(test, feature = "slice50-measurement"))]
mod paired50_tests;

#[cfg(test)]
mod ranking_tests;

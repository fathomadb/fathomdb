//! Option structs standing in for the Python keyword / TypeScript options
//! arguments. Each `Default` equals the documented Python/TypeScript defaults.

use fathomdb_engine::{
    EngineConfig, Filter, Predicate, ReadView, SearchFilter, TraversalDirection,
};

use crate::error::Result;
use crate::guard;

/// `Engine::open` options: `config` and `use_default_embedder` (default `false`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenOptions {
    pub config: EngineConfig,
    /// Open with the pinned default embedder. Requires the `default-embedder`
    /// feature; without it the open fails with an `Embedder` error.
    pub use_default_embedder: bool,
}

/// Either filter form `search` accepts, as Python and TypeScript do.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchFilterArg {
    Search(SearchFilter),
    /// Lowered with `Filter::to_search_filter`; a `Json` term is `InvalidFilter`.
    Unified(Filter),
}

impl From<SearchFilter> for SearchFilterArg {
    fn from(filter: SearchFilter) -> Self {
        Self::Search(filter)
    }
}

impl From<Filter> for SearchFilterArg {
    fn from(filter: Filter) -> Self {
        Self::Unified(filter)
    }
}

impl SearchFilterArg {
    pub(crate) fn lower(self) -> Result<SearchFilter> {
        let filter = match self {
            Self::Search(filter) => filter,
            Self::Unified(filter) => {
                guard::filter(&filter)?;
                filter.to_search_filter()?
            }
        };
        guard::search_filter(&filter)?;
        Ok(filter)
    }
}

/// `Engine::search` options. `alpha` defaults to 0.3 and `pool_n` to
/// `rerank_depth`; `limit` must be in `1..=100`.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchOptions {
    pub filter: Option<SearchFilterArg>,
    pub rerank_depth: usize,
    pub use_graph_arm: bool,
    pub alpha: Option<f64>,
    pub pool_n: Option<usize>,
    pub explain: bool,
    pub view: ReadView,
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            filter: None,
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: None,
            pool_n: None,
            explain: false,
            view: ReadView::default(),
            limit: 10,
        }
    }
}

/// `Engine::search_text_only` options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextSearchOptions {
    pub view: ReadView,
    pub limit: usize,
}

impl Default for TextSearchOptions {
    fn default() -> Self {
        Self { view: ReadView::default(), limit: 10 }
    }
}

/// `Engine::search_projected_text` options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectedTextSearchOptions {
    pub filter: Option<SearchFilter>,
    pub view: ReadView,
    pub limit: usize,
}

impl Default for ProjectedTextSearchOptions {
    fn default() -> Self {
        Self { filter: None, view: ReadView::default(), limit: 10 }
    }
}

/// `Engine::search_frozen` options; defaults as [`SearchOptions`].
#[derive(Clone, Debug, PartialEq)]
pub struct FrozenSearchOptions {
    pub rerank_depth: usize,
    pub use_graph_arm: bool,
    pub alpha: Option<f64>,
    pub pool_n: Option<usize>,
    pub explain: bool,
    pub limit: usize,
}

impl Default for FrozenSearchOptions {
    fn default() -> Self {
        Self {
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: None,
            pool_n: None,
            explain: false,
            limit: 10,
        }
    }
}

/// `graph::search_expand` / `Engine::search_expand_frozen` options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchExpandOptions {
    pub search_limit: usize,
}

impl Default for SearchExpandOptions {
    fn default() -> Self {
        Self { search_limit: 10 }
    }
}

/// `read::list` options. `predicates` and `filter` are mutually exclusive.
#[derive(Clone, Debug, PartialEq)]
pub struct ListOptions {
    pub predicates: Vec<Predicate>,
    pub filter: Option<Filter>,
    pub limit: usize,
    pub view: Option<ReadView>,
}

impl Default for ListOptions {
    fn default() -> Self {
        Self { predicates: Vec::new(), filter: None, limit: 100, view: None }
    }
}

/// `graph::neighbors` options; `direction` defaults to `Both`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NeighborsOptions {
    pub direction: TraversalDirection,
    pub view: Option<ReadView>,
}

impl Default for NeighborsOptions {
    fn default() -> Self {
        Self { direction: TraversalDirection::Both, view: None }
    }
}

/// [`crate::rerank`] options: `alpha` defaults to 0.3, `pool_n` to `rerank_depth`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RerankOptions {
    pub alpha: Option<f64>,
    pub pool_n: Option<usize>,
}

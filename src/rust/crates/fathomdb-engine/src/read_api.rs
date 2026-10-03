use crate::dependency_closure;
use crate::errors::EngineError;
use crate::filter::{Filter, Predicate, PREDICATE_PATH_ALLOWLIST};
use crate::frozen_read::FrozenReadContextV1;
use crate::pagination::{self, PageRequestV1, PageV1};
use crate::read::{NodeRecord, OpStoreRow, OperationalStateRecordV1, PageReaderError};
use crate::reader_pool::ReaderRequest;
use crate::temporal::{current_epoch_seconds, ReadView};
use crate::Engine;
use rusqlite::{params, OptionalExtension};
use std::sync::mpsc::{self, Receiver};

impl Engine {
    /// 0.8.16 Slice 5 / F9 (R-F9-1) — read back the `importance` scalar for the
    /// `canonical_nodes` row identified by `write_cursor`. `None` = SQL `NULL` =
    /// never assigned (graceful-absent). The reciprocal read for
    /// [`Engine::write_node_importance`].
    pub fn node_importance(&self, write_cursor: u64) -> Result<Option<f64>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let eligibility =
            dependency_closure::read_eligibility_sql("canonical_nodes", false, false, false, 2);
        connection
            .query_row(
                &format!(
                    "SELECT importance FROM canonical_nodes \
                     WHERE write_cursor = ?1{eligibility} LIMIT 1"
                ),
                params![write_cursor, current_epoch_seconds()],
                |r| r.get::<_, Option<f64>>(0),
            )
            .optional()
            .map(Option::flatten)
            .map_err(|_| EngineError::Storage)
    }

    /// Test-only matched-shape canonical query without frozen-token or cursor work.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn read_canonical_page_baseline_for_test(
        &self,
        kind: &str,
        context: &FrozenReadContextV1,
        limit: usize,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::canonical_page_baseline(
                kind.to_string(),
                context.clone(),
                limit,
                respond,
            ))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    /// Slice 30 (G2) — `read.get`: active-only point lookup by `logical_id`.
    /// Delegates to [`Engine::read_get_many`]; returns the single slot. A
    /// missing/superseded id is `None` (a normal absence, not an error). Reads
    /// ride the ReaderWorkerPool DEFERRED-tx path (never the writer lock).
    pub fn read_get(
        &self,
        logical_id: &str,
        view: &ReadView,
    ) -> Result<Option<NodeRecord>, EngineError> {
        let ids = [logical_id.to_string()];
        let rows = self.read_get_many(&ids, view)?;
        Ok(rows.into_iter().next().flatten())
    }

    /// Slice 30 (G2) — `read.get_many`: active-only point lookup over many
    /// `logical_id`s. Returns one slot per requested id in REQUEST ORDER, `None`
    /// where no active row carries that id (partial, never all-or-nothing).
    pub fn read_get_many(
        &self,
        logical_ids: &[String],
        view: &ReadView,
    ) -> Result<Vec<Option<NodeRecord>>, EngineError> {
        self.ensure_open()?;
        if logical_ids.is_empty() {
            return Ok(Vec::new());
        }
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::get_by_id(logical_ids.to_vec(), *view, response_tx);
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Slice 30 (G3) — `read.collection`: paginated op-store read-back over
    /// `operational_mutations` for `collection`, `ORDER BY id`. `limit` is
    /// MANDATORY (clamped to the ~1M cap); `after_id` is the exclusive cursor.
    /// Reads ride the ReaderWorkerPool DEFERRED-tx path.
    pub fn read_collection(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.read_collection_dispatch(collection, after_id, limit)
    }

    /// Slice 30 (G3) — `read.mutations`: the mutation-log-oriented alias surface
    /// over the SAME op-store read-back as [`Engine::read_collection`].
    pub fn read_mutations(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.read_collection_dispatch(collection, after_id, limit)
    }

    fn read_collection_dispatch(
        &self,
        collection: &str,
        after_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<OpStoreRow>, EngineError> {
        self.ensure_open()?;
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request =
            ReaderRequest::read_collection(collection.to_string(), after_id, limit, response_tx);
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// Slice 35 (G4) — `read.list`: list active `canonical_nodes` of a given
    /// `kind`, optionally filtered by a closed [`Predicate`] set, up to `limit`
    /// rows. Returns `Vec<NodeRecord>` (active only; `superseded_at IS NULL`).
    ///
    /// Multiple predicates are combined as AND (D-F5). An empty predicate slice
    /// returns all active nodes of the given kind up to `limit` (unfiltered path).
    /// Compilation target: `json_extract(body, '$.field') <op> ?` with bound
    /// parameters (injection-safe per D-F4). See `dev/adr/ADR-0.8.0-filter-grammar.md`.
    ///
    /// Path validation happens at [`Predicate`] construction time; `read_list`
    /// revalidates as defense-in-depth (enum variants are `pub`, so direct
    /// struct-literal construction could bypass the constructors).
    pub fn read_list(
        &self,
        kind: &str,
        predicates: &[Predicate],
        limit: usize,
        view: &ReadView,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        // Defense-in-depth: revalidate paths even if the caller bypassed the
        // validated constructors by constructing enum variants directly.
        for pred in predicates {
            let path = Predicate::path(pred);
            if !PREDICATE_PATH_ALLOWLIST.contains(&path) {
                return Err(EngineError::InvalidFilter {
                    reason: format!("path '{path}' is not in the predicate path allowlist"),
                });
            }
        }
        let (response_tx, response_rx) = mpsc::sync_channel(1);
        let request = ReaderRequest::read_list(
            kind.to_string(),
            predicates.to_vec(),
            limit,
            *view,
            response_tx,
        );
        if self.reader_pool.dispatch(request).is_err() {
            return Err(EngineError::Closing);
        }
        match response_rx.recv().map_err(|_| EngineError::Storage)? {
            Ok(rows) => Ok(rows),
            Err(err) => {
                self.emit_sqlite_internal_error(&err);
                Err(EngineError::Storage)
            }
        }
    }

    /// 0.8.11 Slice 40 (#17) — unified-`Filter` entry point for the
    /// canonical_nodes `read.list` backend. Accepts the **full** [`FilterTerm`]
    /// set (D3): `Json` runs the shipped allowlisted `json_extract` path;
    /// `Status`/`CreatedAfter` lower to allowlisted json-paths; `Kind`/`SourceType`
    /// **constant-fold** against the partition `kind` (a guaranteed-empty fold
    /// returns an empty `Vec` without touching SQL). Dispatches to the same
    /// [`Engine::read_list`] machinery the shipped `Predicate` surface uses, so
    /// every inherited invariant (`superseded_at IS NULL`, `json_valid(body)`,
    /// the `canonical_nodes(kind)` index, parameterized binds) is preserved.
    pub fn read_list_filter(
        &self,
        kind: &str,
        filter: &Filter,
        limit: usize,
        view: &ReadView,
    ) -> Result<Vec<NodeRecord>, EngineError> {
        self.ensure_open()?;
        match filter.lower_for_read_list(kind)? {
            None => Ok(Vec::new()),
            Some(preds) => self.read_list(kind, &preds, limit, view),
        }
    }

    /// Read one stable keyset page of canonical logical nodes.
    ///
    /// The authenticated frozen context supplies validity and eligibility.
    /// Continuations are database-, selector-, context-, and limit-bound and
    /// refuse after relevant state drift.
    pub fn read_canonical_page(
        &self,
        kind: &str,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<PageV1<NodeRecord>, EngineError> {
        self.ensure_open()?;
        pagination::validate_request(page)?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::canonical_page(
                kind.to_string(),
                context.clone(),
                page.clone(),
                respond,
            ))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    /// Read the current value of one governed `latest_state` record.
    ///
    /// Supplying a frozen context binds this point read to the same authority
    /// used by a page walk. A context is optional for an ordinary current read.
    pub fn read_operational_state(
        &self,
        collection: &str,
        record_key: &str,
        context: Option<&FrozenReadContextV1>,
    ) -> Result<Option<OperationalStateRecordV1>, EngineError> {
        self.ensure_open()?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::operational_state(
                collection.to_string(),
                record_key.to_string(),
                context.cloned(),
                respond,
            ))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    /// Read one stable keyset page of a governed `latest_state` collection.
    pub fn read_operational_state_page(
        &self,
        collection: &str,
        context: &FrozenReadContextV1,
        page: &PageRequestV1,
    ) -> Result<PageV1<OperationalStateRecordV1>, EngineError> {
        self.ensure_open()?;
        pagination::validate_request(page)?;
        let (respond, receive) = mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(ReaderRequest::operational_state_page(
                collection.to_string(),
                context.clone(),
                page.clone(),
                respond,
            ))
            .map_err(|_| EngineError::Closing)?;
        self.receive_page_result(receive)
    }

    pub(crate) fn receive_page_result<T>(
        &self,
        receive: Receiver<Result<T, PageReaderError>>,
    ) -> Result<T, EngineError> {
        match receive.recv().map_err(|_| EngineError::Storage)? {
            Ok(value) => Ok(value),
            Err(PageReaderError::Engine(error)) => Err(error),
            Err(PageReaderError::Sqlite(error)) => {
                self.emit_sqlite_internal_error(&error);
                Err(EngineError::Storage)
            }
        }
    }
}

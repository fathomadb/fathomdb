use crate::admission::{admit, fingerprint, readonly, Fingerprint};
use crate::failure::Failure;
use crate::policy::Policy;
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, ScopePolicy};
use rusqlite::Connection;
use std::cell::OnceCell;
use std::path::{Path, PathBuf};

pub(crate) struct ReadConnection {
    connection: Connection,
    queued_erasure_cursors: OnceCell<Result<Vec<i64>, &'static str>>,
    pub(crate) started_epoch_ms: u128,
    pub(crate) at_epoch_s: i64,
}
fn queued_scope_error(error: rusqlite::Error) -> &'static str {
    if error.sqlite_error_code() == Some(rusqlite::ErrorCode::OperationInterrupted) {
        "budget_exhausted"
    } else {
        "unknown_classification"
    }
}
impl ReadConnection {
    pub(crate) fn queued_erasure_contains(&self, cursor: i64) -> Result<bool, &'static str> {
        let cursors =
            self.queued_erasure_cursors.get_or_init(|| self.load_queued_erasure_cursors());
        match cursors {
            Ok(cursors) => Ok(cursors.binary_search(&cursor).is_ok()),
            Err(error) => Err(*error),
        }
    }
    fn load_queued_erasure_cursors(&self) -> Result<Vec<i64>, &'static str> {
        // Queued erasure admission bounds: at most 16 queued scopes of 16,384 cursors,
        // within 8 MiB of total request metadata. The cache dies with this snapshot.
        const MAX_CURSORS: usize = 16 * 16_384;
        let (requests, bytes): (i64, i64) = self.connection.query_row(
            "SELECT count(*),COALESCE(sum(octet_length(payload)),0) FROM _fathomdb_erasure_requests WHERE phase='queued'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).map_err(queued_scope_error)?;
        if requests > 16 || bytes > 8 * 1024 * 1024 {
            return Err("unknown_classification");
        }
        if requests == 0 {
            return Ok(Vec::new());
        }
        let invalid: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_erasure_requests WHERE phase='queued' AND (json_extract(payload,'$.version') IS NOT 1 OR json_type(payload,'$.scope.cursors') IS NOT 'array' OR json_array_length(payload,'$.scope.cursors')>16384 OR json_extract(payload,'$.status.logical_state') IS NOT 'unchanged_by_this_request'))",
            [], |row| row.get(0),
        ).map_err(queued_scope_error)?;
        if invalid {
            return Err("unknown_classification");
        }
        let mut statement = self.connection.prepare(
            "SELECT cursor.type,cursor.value FROM _fathomdb_erasure_requests request JOIN json_each(request.payload,'$.scope.cursors') cursor WHERE request.phase='queued' LIMIT ?1",
        ).map_err(queued_scope_error)?;
        let mut rows = statement.query([MAX_CURSORS as i64 + 1]).map_err(queued_scope_error)?;
        let mut cursors = Vec::new();
        while let Some(row) = rows.next().map_err(queued_scope_error)? {
            if cursors.len() == MAX_CURSORS
                || row.get::<_, String>(0).map_err(queued_scope_error)? != "integer"
            {
                return Err("unknown_classification");
            }
            let cursor: i64 = row.get(1).map_err(queued_scope_error)?;
            if cursor <= 0 {
                return Err("unknown_classification");
            }
            cursors.push(cursor);
        }
        cursors.sort_unstable();
        cursors.dedup();
        Ok(cursors)
    }
}
#[cfg(test)]
#[path = "../tests/integration/queued_scope.rs"]
mod queued_scope_tests;

impl std::ops::Deref for ReadConnection {
    type Target = Connection;
    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
pub(crate) fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
fn valid_engine_scope(scope: &str) -> bool {
    if scope.is_empty() {
        return true;
    }
    scope != "*"
        && scope.len() <= 128
        && !scope.bytes().any(|b| b == 0x1e)
        && !scope.chars().any(char::is_control)
        && scope.trim() == scope
}
pub(crate) struct Session {
    pub(crate) engine: Option<Engine>,
    pub(crate) engine_scope: String,
    open_default_embedder: bool,
    pub(crate) cached_cpu: bool,
    pub(crate) private_root: Option<PathBuf>,
    pub(crate) source_cache: Option<PathBuf>,
    pub(crate) source_cache_hint: Option<PathBuf>,
    pub(crate) path: PathBuf,
    pub(crate) fingerprint: Fingerprint,
    pub(crate) lock_fingerprint: Fingerprint,
    pub(crate) database_id: String,
    pub(crate) policy: Policy,
}
impl Session {
    #[cfg(test)]
    pub(crate) fn attach(path: &Path, policy: Policy) -> Result<Self, Failure> {
        Self::attach_mode(path, policy, false)
    }
    pub(crate) fn attach_mode(
        path: &Path,
        policy: Policy,
        cached_cpu: bool,
    ) -> Result<Self, Failure> {
        let admitted = admit(path)?;
        if fingerprint(&admitted.path)? != admitted.fingerprint {
            return Err(Failure::preflight("expired"));
        }
        let opened = Engine::open_with_choice_and_config(
            &admitted.path,
            if cached_cpu { EmbedderChoice::Default } else { EmbedderChoice::None },
            EngineConfig {
                scheduler_runtime_threads: Some(1),
                embedder_pool_size: Some(1),
                embedder_call_timeout_ms: Some(25_000),
                scope_policy: ScopePolicy::Unscoped,
                ..EngineConfig::default()
            },
        )
        .map_err(|_| Failure::startup())?;
        let c = readonly(&admitted.path, false).map_err(|_| Failure::startup())?;
        let database_id = c
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_database_id'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| Failure::startup())?;
        let lock_fingerprint = fingerprint(Path::new(&format!("{}.lock", admitted.path.display())))
            .map_err(|_| Failure::startup())?;
        Ok(Self {
            engine: Some(opened.engine),
            engine_scope: String::new(),
            open_default_embedder: cached_cpu,
            cached_cpu,
            private_root: None,
            source_cache: None,
            source_cache_hint: None,
            path: admitted.path,
            fingerprint: admitted.fingerprint,
            lock_fingerprint,
            database_id,
            policy,
        })
    }
    pub(crate) fn engine(&self) -> Result<&Engine, &'static str> {
        self.engine.as_ref().ok_or("attachment_expired")
    }
    pub(crate) fn ensure_engine_scope(&mut self, target_scope: &str) -> Result<(), Failure> {
        if self.engine_scope == target_scope && self.engine.is_some() {
            return Ok(());
        }
        if !valid_engine_scope(target_scope) {
            return Err(Failure::read("invalid_range"));
        }
        let lock_path = PathBuf::from(format!("{}.lock", self.path.display()));
        if fingerprint(&self.path).map_err(|_| Failure::read("attachment_expired"))?
            != self.fingerprint
            || fingerprint(&lock_path).map_err(|_| Failure::read("attachment_expired"))?
                != self.lock_fingerprint
        {
            return Err(Failure::read("attachment_expired"));
        }
        if let Some(previous) = self.engine.take() {
            previous.close().map_err(|_| Failure::read("attachment_expired"))?;
        }
        let scope_policy = if target_scope.is_empty() {
            ScopePolicy::Unscoped
        } else {
            ScopePolicy::read_only(vec![target_scope.to_string()])
        };
        let opened = Engine::open_with_choice_and_config(
            &self.path,
            if self.open_default_embedder { EmbedderChoice::Default } else { EmbedderChoice::None },
            EngineConfig {
                scheduler_runtime_threads: Some(1),
                embedder_pool_size: Some(1),
                embedder_call_timeout_ms: Some(25_000),
                scope_policy,
                ..EngineConfig::default()
            },
        )
        .map_err(|_| Failure::read("attachment_expired"))?;
        let current_fp =
            fingerprint(&self.path).map_err(|_| Failure::read("attachment_expired"))?;
        let current_lock_fp =
            fingerprint(&lock_path).map_err(|_| Failure::read("attachment_expired"))?;
        if current_fp != self.fingerprint || current_lock_fp != self.lock_fingerprint {
            let _ = opened.engine.close();
            return Err(Failure::read("attachment_expired"));
        }
        self.engine = Some(opened.engine);
        self.engine_scope = target_scope.to_string();
        Ok(())
    }
    pub(crate) fn connection(&self) -> Result<ReadConnection, &'static str> {
        let started_epoch_ms = epoch_ms();
        if fingerprint(&self.path).map_err(|_| "attachment_expired")? != self.fingerprint
            || fingerprint(Path::new(&format!("{}.lock", self.path.display())))
                .map_err(|_| "attachment_expired")?
                != self.lock_fingerprint
        {
            return Err("attachment_expired");
        }
        let c = readonly(&self.path, false)?;
        c.execute_batch("BEGIN").map_err(|_| "storage")?;
        let version: u32 = c
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(|_| "attachment_expired")?;
        let id: String = c
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_database_id'",
                [],
                |r| r.get(0),
            )
            .map_err(|_| "attachment_expired")?;
        let coalescing: i64 = c
            .query_row(
                "SELECT coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| "attachment_expired")?;
        if version != fathomdb_schema::SCHEMA_VERSION || id != self.database_id || coalescing != 0 {
            return Err("attachment_expired");
        }
        Ok(ReadConnection {
            connection: c,
            queued_erasure_cursors: OnceCell::new(),
            started_epoch_ms,
            at_epoch_s: (started_epoch_ms / 1000) as i64,
        })
    }
    pub(crate) fn close(mut self) -> Result<(), &'static str> {
        match self.engine.take() {
            Some(engine) => engine.close().map_err(|_| "close_failed"),
            None => Ok(()),
        }
    }
}

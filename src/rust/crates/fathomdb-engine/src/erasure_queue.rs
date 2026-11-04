use super::*;
use serde::{Deserialize, Serialize};

/// Exact selector whose committed cut is frozen on queue acceptance.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PhysicalErasureSelector {
    /// All retained canonical roots belonging to this governed source.
    SourceAllAtCut { source_id: String },
    /// All retained versions of a deleted logical node and its touching edges.
    DeletedLogicalAllAtCut { logical_id: String },
}
impl std::fmt::Debug for PhysicalErasureSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("PhysicalErasureSelector(<redacted>)")
    }
}
/// Logical effect of this request, independently of preexisting visibility.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasureLogicalState {
    UnchangedByThisRequest,
    Removed,
}
/// Durable progress of one keyed request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasurePhase {
    Queued,
    Executing,
    Sanitizing,
    Finalizing,
    Complete,
    WithdrawnBeforeRemoval,
}
/// Inspectable expected blocker, never a success or deadline estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasureBlocker {
    ResourceEnvelopeChanged,
    OriginalSinkUnavailable,
    WalCheckpoint,
    OwnedReaders,
}
/// Identity-free status under the Engine's existing access authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErasureStatus {
    pub phase: ErasurePhase,
    pub logical_state: ErasureLogicalState,
    pub blocker: Option<ErasureBlocker>,
    pub service_state: ErasureServiceState,
    pub target_count: u64,
}
/// Queue acceptance never means removal; heavy admission requires a handle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum QueueResult {
    Accepted { ticket: String, logical_state: ErasureLogicalState },
    NeedsMaintenanceAdmission,
}
/// Withdrawal cannot cancel activated physical obligations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawResult {
    WithdrawnBeforeRemoval,
    AlreadyActivated,
    Unknown,
}
/// Actual physical completion or retained, inspectable unfinished work.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum CleanupResult {
    Complete { receipt: ErasureStatus },
    Incomplete { status: ErasureStatus },
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scope {
    pub(crate) cursors: Vec<i64>,
    sources: Vec<String>,
    identities: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    version: u32,
    pub(crate) ticket: String,
    key: String,
    pub(crate) selector: Option<PhysicalErasureSelector>,
    cut: u64,
    scope: Scope,
    pub(crate) originals: Vec<String>,
    pub(crate) status: ErasureStatus,
}

fn error(reason: MaintenanceError) -> EngineError {
    EngineError::Maintenance(reason)
}
fn valid_hex(value: &str, len: usize) -> bool {
    value.len() == len && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn validate_key(key: &str) -> Result<(), EngineError> {
    if valid_hex(key, 32) {
        Ok(())
    } else {
        Err(error(MaintenanceError::InvalidKey))
    }
}
fn validate_ticket(ticket: &str) -> Result<(), EngineError> {
    if ticket.len() == 65
        && ticket.as_bytes()[32] == b':'
        && valid_hex(&ticket[..32], 32)
        && valid_hex(&ticket[33..], 32)
    {
        Ok(())
    } else {
        Err(error(MaintenanceError::InvalidTicket))
    }
}
fn validate_selector(selector: &PhysicalErasureSelector) -> Result<(), EngineError> {
    match selector {
        PhysicalErasureSelector::SourceAllAtCut { source_id } => {
            SourceId::new(source_id)?;
        }
        PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id } => {
            if logical_id.is_empty() || logical_id.contains(':') || logical_id.len() > 1024 {
                return Err(EngineError::WriteValidation);
            }
        }
    }
    Ok(())
}
fn hash(domain: &str, value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"fathomdb.queued-scope.v1\0");
    digest.update(domain.as_bytes());
    digest.update([0]);
    digest.update(value.as_bytes());
    identity::hex_encode(&digest.finalize())
}
pub(crate) fn load(c: &Connection, ticket: &str) -> Result<Option<Request>, EngineError> {
    let raw:Option<(String,String,String,String)>=c.query_row("SELECT ticket,idempotency_key,phase,payload FROM _fathomdb_erasure_requests WHERE ticket=?1",[ticket],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|_|EngineError::Storage)?;
    raw.map(|(ticket, key, phase, raw)| decode_row(c, ticket, key, phase, raw)).transpose()
}

fn decode(raw: &str) -> Result<Request, EngineError> {
    let r: Request = serde_json::from_str(raw).map_err(|_| EngineError::Storage)?;
    let queued = r.status.phase == ErasurePhase::Queued;
    let terminal =
        matches!(r.status.phase, ErasurePhase::Complete | ErasurePhase::WithdrawnBeforeRemoval);
    if r.version != 1
        || validate_ticket(&r.ticket).is_err()
        || validate_key(&r.key).is_err()
        || (queued
            && (r.selector.is_none()
                || r.status.logical_state != ErasureLogicalState::UnchangedByThisRequest))
        || (terminal
            && (r.selector.is_some()
                || r.scope != Scope::default()
                || r.cut != 0
                || !r.originals.is_empty()))
        || r.scope.cursors.len() > 16384
        || !r.scope.cursors.windows(2).all(|v| v[0] < v[1])
        || r.scope
            .sources
            .iter()
            .chain(&r.scope.identities)
            .chain(&r.originals)
            .any(|v| !valid_hex(v, 64))
        || r.scope.cursors.iter().any(|v| *v <= 0)
    {
        return Err(EngineError::Storage);
    }
    Ok(r)
}
pub(crate) fn store(c: &Connection, r: &Request) -> Result<(), EngineError> {
    let raw = serde_json::to_string(r).map_err(|_| EngineError::Storage)?;
    c.execute("INSERT INTO _fathomdb_erasure_requests(ticket,idempotency_key,payload) VALUES(?1,?2,?3) ON CONFLICT(ticket) DO UPDATE SET payload=excluded.payload",params![r.ticket,r.key,raw]).map_err(|_|EngineError::Storage)?;
    Ok(())
}
fn decode_row(
    c: &Connection,
    ticket: String,
    key: String,
    phase: String,
    raw: String,
) -> Result<Request, EngineError> {
    let request = decode(&raw)?;
    let nonce: String = c
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_erasure_store'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let expected = serde_json::to_value(request.status.phase).map_err(|_| EngineError::Storage)?;
    if ticket != request.ticket
        || key != request.key
        || expected.as_str() != Some(phase.as_str())
        || request.ticket.split_once(':').map(|v| v.0) != Some(nonce.as_str())
    {
        return Err(EngineError::Storage);
    }
    Ok(request)
}
fn requests(c: &Connection) -> Result<Vec<Request>, EngineError> {
    let mut statement=c.prepare("SELECT ticket,idempotency_key,phase,payload FROM _fathomdb_erasure_requests ORDER BY ticket").map_err(|_|EngineError::Storage)?;
    let rows = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(|_| EngineError::Storage)?;
    rows.map(|row| {
        let (ticket, key, phase, raw) = row.map_err(|_| EngineError::Storage)?;
        decode_row(c, ticket, key, phase, raw)
    })
    .collect()
}

pub(crate) fn validate(c: &Connection) -> Result<(), EngineError> {
    let rows = requests(c)?;
    if rows.len() > 4113
        || rows.iter().filter(|r| r.status.phase == ErasurePhase::Queued).count() > 16
    {
        return Err(EngineError::Storage);
    }
    let mut active = rows.iter().filter(|r| {
        matches!(
            r.status.phase,
            ErasurePhase::Executing | ErasurePhase::Sanitizing | ErasurePhase::Finalizing
        )
    });
    let executing = active.next();
    if active.next().is_some() {
        return Err(EngineError::Storage);
    }
    let root = physical_erasure::root(c)?;
    match (root.as_ref().and_then(|r| r.ticket.as_deref()), executing) {
        (Some(ticket), Some(request)) if request.ticket == ticket => {}
        (None, None) => {}
        _ => return Err(EngineError::Storage),
    }
    if root.is_some()
        && crate::erasure_maintenance::read_policy(c)? == PhysicalErasurePolicy::ExplicitMaintenance
        && root.as_ref().and_then(|r| r.ticket.as_ref()).is_none()
    {
        return Err(EngineError::Storage);
    }
    Ok(())
}
fn queued(c: &Connection) -> Result<Vec<Request>, EngineError> {
    let mut statement=c.prepare("SELECT ticket,idempotency_key,phase,payload FROM _fathomdb_erasure_requests WHERE phase='queued' ORDER BY ticket").map_err(|_|EngineError::Storage)?;
    let rows = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .map_err(|_| EngineError::Storage)?;
    rows.map(|row| {
        let (ticket, key, phase, raw) = row.map_err(|_| EngineError::Storage)?;
        decode_row(c, ticket, key, phase, raw)
    })
    .collect()
}

fn by_key(
    c: &Connection,
    key: &str,
    selector: &PhysicalErasureSelector,
) -> Result<Option<QueueResult>, EngineError> {
    let ticket: Option<String> = c
        .query_row(
            "SELECT ticket FROM _fathomdb_erasure_requests WHERE idempotency_key=?1",
            [key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some(ticket) = ticket else { return Ok(None) };
    let r = load(c, &ticket)?.ok_or(EngineError::Storage)?;
    if matches!(r.status.phase, ErasurePhase::Complete | ErasurePhase::WithdrawnBeforeRemoval) {
        return Err(error(MaintenanceError::KeyRetired));
    }
    if r.selector.as_ref() != Some(selector) {
        return Err(error(MaintenanceError::IdempotencyConflict));
    }
    Ok(Some(QueueResult::Accepted { ticket, logical_state: r.status.logical_state }))
}

pub(crate) fn preview(
    c: &Connection,
    selector: &PhysicalErasureSelector,
) -> Result<Scope, EngineError> {
    let (argument, node_predicate, edge_predicate) = match selector {
        PhysicalErasureSelector::SourceAllAtCut { source_id } => {
            (source_id.as_str(), "source_id=?1", "source_id=?1")
        }
        PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id } => {
            let state:Option<String>=c.query_row("SELECT state FROM canonical_nodes WHERE logical_id=?1 AND superseded_at IS NULL",[logical_id],|r|r.get(0)).optional().map_err(|_|EngineError::Storage)?;
            if let Some(raw) = state {
                if raw != "deleted" {
                    let from_state =
                        LifecycleState::from_str_opt(&raw).ok_or(EngineError::Storage)?;
                    return Err(EngineError::IllegalTransition {
                        from_state,
                        to_state: LifecycleState::Purged,
                        legal: from_state.legal_next_states(),
                    });
                }
            }
            (logical_id.as_str(), "logical_id=?1", "from_id=?1 OR to_id=?1")
        }
    };
    let mut scope = Scope::default();
    let mut node_cursors = Vec::new();
    for (table, predicate) in
        [("canonical_nodes", node_predicate), ("canonical_edges", edge_predicate)]
    {
        let mut statement = c
            .prepare(&format!("SELECT write_cursor FROM {table} WHERE {predicate}"))
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([argument], |r| r.get::<_, i64>(0))
            .map_err(|_| EngineError::Storage)?;
        for cursor in rows {
            let cursor = cursor.map_err(|_| EngineError::Storage)?;
            scope.cursors.push(cursor);
            if table == "canonical_nodes" {
                node_cursors.push(cursor)
            }
        }
    }
    let revisions = node_cursors
        .iter()
        .map(|v| dependency_closure::source_revision_for_cursor(c, *v))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    for (_, dependents) in dependency_closure::physical_dependents_for_sources(c, &revisions)? {
        scope.cursors.extend(dependents.into_iter().map(|(_, cursor)| cursor));
    }
    scope.cursors.sort_unstable();
    scope.cursors.dedup();
    if scope.cursors.len() > 16384 {
        return Err(error(MaintenanceError::UnsupportedScope));
    }
    let cursors = serde_json::to_string(&scope.cursors).map_err(|_| EngineError::Storage)?;
    for table in ["canonical_nodes", "canonical_edges"] {
        let mut statement=c.prepare(&format!("SELECT source_id,logical_id,body FROM {table} WHERE write_cursor IN (SELECT value FROM json_each(?1))")).map_err(|_|EngineError::Storage)?;
        let rows = statement
            .query_map([&cursors], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (source, logical, body) = row.map_err(|_| EngineError::Storage)?;
            if let Some(source) = source {
                scope.sources.push(hash("source", &source));
            }
            scope.identities.push(hash(
                "identity",
                &derive_stable_id(logical.as_deref(), body.as_deref().unwrap_or("")).to_prefixed(),
            ));
        }
    }
    match selector {
        PhysicalErasureSelector::SourceAllAtCut { source_id } => {
            scope.sources.push(hash("source", source_id))
        }
        PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id } => {
            scope.identities.push(hash("identity", &format!("l:{logical_id}")))
        }
    }
    scope.sources.sort_unstable();
    scope.sources.dedup();
    scope.identities.sort_unstable();
    scope.identities.dedup();
    Ok(scope)
}
fn overlaps(a: &Scope, b: &Scope) -> bool {
    a.cursors.iter().any(|v| b.cursors.contains(v))
        || a.sources.iter().any(|v| b.sources.contains(v))
        || a.identities.iter().any(|v| b.identities.contains(v))
}
fn check_identity(
    scope: &Scope,
    source: Option<&str>,
    logical: Option<&str>,
    body: &str,
) -> Result<(), EngineError> {
    if source.is_some_and(|s| scope.sources.contains(&hash("source", s)))
        || scope
            .identities
            .contains(&hash("identity", &derive_stable_id(logical, body).to_prefixed()))
    {
        return Err(error(MaintenanceError::QueuedScopeFenced));
    }
    Ok(())
}
pub(crate) fn guard_plain(c: &Connection, batch: &[PreparedWrite]) -> Result<(), EngineError> {
    let queued = queued(c)?;
    if queued.is_empty() {
        return Ok(());
    }
    if !write_commit::canonical_batch_has_no_custom_triggers(c).map_err(|_| EngineError::Storage)? {
        return Err(error(MaintenanceError::QueuedScopeFenced));
    }
    for request in queued {
        for write in batch {
            let (table, source, logical, body, from, to, kind) = match write {
                PreparedWrite::Node { source_id, logical_id, body, .. } => (
                    "canonical_nodes",
                    source_id.as_str(),
                    logical_id.as_deref(),
                    body.as_str(),
                    None,
                    None,
                    None,
                ),
                PreparedWrite::Edge { source_id, logical_id, body, from, to, kind, .. } => (
                    "canonical_edges",
                    source_id.as_str(),
                    logical_id.as_deref(),
                    body.as_deref().unwrap_or(""),
                    Some(from.as_str()),
                    Some(to.as_str()),
                    Some(kind.as_str()),
                ),
                _ => return Err(error(MaintenanceError::QueuedScopeFenced)),
            };
            check_identity(&request.scope, Some(source), logical, body)?;
            for endpoint in [from, to].into_iter().flatten() {
                if request.scope.identities.contains(&hash("identity", &format!("l:{endpoint}"))) {
                    return Err(error(MaintenanceError::QueuedScopeFenced));
                }
            }
            let endpoints = if table == "canonical_edges" { "from_id,to_id" } else { "NULL,NULL" };
            let predicate = if table == "canonical_edges" {
                "logical_id=?1 OR (from_id=?2 AND to_id=?3 AND kind=?4)"
            } else {
                "logical_id=?1"
            };
            let mut statement = c
                .prepare(&format!(
                    "SELECT source_id,logical_id,body,{endpoints} FROM {table} WHERE {predicate}"
                ))
                .map_err(|_| EngineError::Storage)?;
            let row = |r: &rusqlite::Row<'_>| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            };
            let rows = if table == "canonical_edges" {
                statement.query_map(params![logical, from, to, kind], row)
            } else {
                statement.query_map(params![logical], row)
            }
            .map_err(|_| EngineError::Storage)?;
            for row in rows {
                let (source, logical, body, from, to) = row.map_err(|_| EngineError::Storage)?;
                check_identity(
                    &request.scope,
                    source.as_deref(),
                    logical.as_deref(),
                    body.as_deref().unwrap_or(""),
                )?;
                for endpoint in [from, to].into_iter().flatten() {
                    if request
                        .scope
                        .identities
                        .contains(&hash("identity", &format!("l:{endpoint}")))
                    {
                        return Err(error(MaintenanceError::QueuedScopeFenced));
                    }
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn guard_admin(c: &Connection) -> Result<(), EngineError> {
    if !queued(c)?.is_empty() {
        return Err(error(MaintenanceError::QueuedScopeFenced));
    }
    Ok(())
}

impl Engine {
    /// Queue durable intent without deleting or hiding data. First-time normal
    /// admission refuses heavy discovery; acquire a window for that preflight.
    pub fn queue_physical_erasure(
        &self,
        selector: PhysicalErasureSelector,
        key: &str,
    ) -> Result<QueueResult, EngineError> {
        let _operation = self.ensure_open()?;
        validate_key(key)?;
        validate_selector(&selector)?;
        if self.physical_policy != PhysicalErasurePolicy::ExplicitMaintenance {
            return Err(error(MaintenanceError::PolicyMismatch));
        }
        let guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let c = guard.as_ref().ok_or(EngineError::Closing)?;
        if let Some(result) = by_key(c, key, &selector)? {
            return Ok(result);
        }
        Ok(QueueResult::NeedsMaintenanceAdmission)
    }
    /// Read only a store-bound ticket. Unknown/wrong-store well-formed tickets
    /// return None; no selector or store-wide listing is disclosed.
    pub fn physical_erasure_status(
        &self,
        ticket: &str,
    ) -> Result<Option<ErasureStatus>, EngineError> {
        let _operation = self.ensure_open()?;
        validate_ticket(ticket)?;
        let guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let c = guard.as_ref().ok_or(EngineError::Closing)?;
        Ok(load(c, ticket)?.map(|r| r.status))
    }
    /// Withdraw only a queued request; no physical-success claim is produced.
    pub fn withdraw_queued_erasure(&self, ticket: &str) -> Result<WithdrawResult, EngineError> {
        let _operation = self.ensure_open()?;
        validate_ticket(ticket)?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let c = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        let Some(mut request) = load(&tx, ticket)? else { return Ok(WithdrawResult::Unknown) };
        if request.status.phase == ErasurePhase::WithdrawnBeforeRemoval {
            return Ok(WithdrawResult::WithdrawnBeforeRemoval);
        }
        if request.status.phase != ErasurePhase::Queued {
            return Ok(WithdrawResult::AlreadyActivated);
        }
        request.selector = None;
        request.scope = Scope::default();
        request.cut = 0;
        request.originals.clear();
        request.status.phase = ErasurePhase::WithdrawnBeforeRemoval;
        store(&tx, &request)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(WithdrawResult::WithdrawnBeforeRemoval)
    }
}
impl ErasureMaintenance {
    /// Admit exact immutable intent using heavy resource/closure discovery in
    /// this owned window; acceptance still performs no destructive work.
    pub fn queue_physical_erasure(
        &self,
        selector: PhysicalErasureSelector,
        key: &str,
    ) -> Result<QueueResult, EngineError> {
        let _serial =
            self.operation.try_lock().map_err(|_| error(MaintenanceError::MaintenanceBusy))?;
        self.engine.maintenance_admission.with_permit(self.token, || {
            let _operation = self.engine.ensure_open()?;
            validate_key(key)?;
            validate_selector(&selector)?;
            let mut guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            let c = guard.as_mut().ok_or(EngineError::Closing)?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| EngineError::Storage)?;
            if let Some(result) = by_key(&tx, key, &selector)? {
                return Ok(result);
            }
            dependency_closure::guard_no_pending_physical(&tx)?;
            physical_erasure::envelope(&tx)?;
            let rows = requests(&tx)?;
            if rows.iter().filter(|r| r.status.phase == ErasurePhase::Queued).count() >= 16
                || rows.len() >= 4096
            {
                return Err(error(MaintenanceError::Capacity));
            }
            let scope = preview(&tx, &selector)?;
            if rows
                .iter()
                .filter(|r| r.status.phase == ErasurePhase::Queued)
                .any(|r| overlaps(&r.scope, &scope))
            {
                return Err(error(MaintenanceError::ScopeConflict));
            }
            let store_id: String = tx
                .query_row(
                    "SELECT value FROM _fathomdb_open_state WHERE key='_fathomdb_erasure_store'",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            let nonce: String = tx
                .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))
                .map_err(|_| EngineError::Storage)?;
            let ticket = format!("{store_id}:{nonce}");
            let request = Request {
                version: 1,
                ticket: ticket.clone(),
                key: key.into(),
                selector: Some(selector),
                cut: self.engine.next_cursor.load(Ordering::SeqCst),
                status: ErasureStatus {
                    phase: ErasurePhase::Queued,
                    logical_state: ErasureLogicalState::UnchangedByThisRequest,
                    blocker: None,
                    service_state: ErasureServiceState::NormalWithQueuedRestrictions,
                    target_count: scope.cursors.len() as u64,
                },
                scope,
                originals: telemetry::registered_sinks(&tx)?,
            };
            let size: u64 = tx
                .query_row(
                    "SELECT COALESCE(SUM(length(payload)),0) FROM _fathomdb_erasure_requests",
                    [],
                    |r| r.get(0),
                )
                .map_err(|_| EngineError::Storage)?;
            if size + serde_json::to_vec(&request).map_err(|_| EngineError::Storage)?.len() as u64
                > 8 << 20
            {
                return Err(error(MaintenanceError::Capacity));
            }
            store(&tx, &request)?;
            physical_erasure::faultpoint("before_queue_commit")?;
            tx.commit().map_err(|_| EngineError::Storage)?;
            physical_erasure::faultpoint("after_queue_commit")?;
            Ok(QueueResult::Accepted {
                ticket,
                logical_state: ErasureLogicalState::UnchangedByThisRequest,
            })
        })
    }
}

thread_local! { static ACTIVATION: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) }; }
fn with_activation<T>(
    ticket: &str,
    operation: impl FnOnce() -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            ACTIVATION.with(|v| *v.borrow_mut() = self.0.take());
        }
    }
    let _restore = Restore(ACTIVATION.with(|v| v.borrow_mut().replace(ticket.into())));
    operation()
}
pub(crate) fn activate_in(
    c: &Connection,
    verb: &str,
    argument: &str,
) -> Result<Option<String>, EngineError> {
    let ticket = ACTIVATION.with(|v| v.borrow().clone());
    let Some(ticket) = ticket else { return Ok(None) };
    let mut request = load(c, &ticket)?.ok_or(EngineError::Storage)?;
    let matches = match request.selector.as_ref() {
        Some(PhysicalErasureSelector::SourceAllAtCut { source_id }) => {
            verb == "erase_source" && source_id == argument
        }
        Some(PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id }) => {
            verb == "purge" && logical_id == argument
        }
        _ => false,
    };
    if request.status.phase != ErasurePhase::Queued || !matches {
        return Err(EngineError::Storage);
    }
    request.status.phase = ErasurePhase::Executing;
    request.status.logical_state = ErasureLogicalState::Removed;
    request.status.service_state = ErasureServiceState::ReadsAllowedWritesFenced;
    store(c, &request)?;
    Ok(Some(ticket))
}
pub(crate) fn scrub_in(c: &Connection, ticket: &str) -> Result<(), EngineError> {
    let mut request = load(c, ticket)?.ok_or(EngineError::Storage)?;
    if request.status.logical_state != ErasureLogicalState::Removed {
        return Err(EngineError::Storage);
    }
    request.selector = None;
    request.cut = 0;
    request.scope = Scope::default();
    request.originals.clear();
    request.status.phase = ErasurePhase::Finalizing;
    store(c, &request)
}
pub(crate) fn complete_in(c: &Connection, ticket: &str) -> Result<(), EngineError> {
    let mut request = load(c, ticket)?.ok_or(EngineError::Storage)?;
    if request.status.phase != ErasurePhase::Finalizing
        || request.selector.is_some()
        || request.scope != Scope::default()
    {
        return Err(EngineError::Storage);
    }
    request.status.phase = ErasurePhase::Complete;
    request.status.blocker = None;
    request.status.service_state = ErasureServiceState::NormalWithQueuedRestrictions;
    store(c, &request)
}

impl ErasureMaintenance {
    /// Authenticate an existing recorded original sink without registry writes,
    /// path creation, capture enablement, or substitution of another sink.
    pub fn reattach_original_erasure_sink(
        &self,
        ticket: &str,
        path: &str,
    ) -> Result<(), EngineError> {
        let _serial =
            self.operation.try_lock().map_err(|_| error(MaintenanceError::MaintenanceBusy))?;
        self.engine.maintenance_admission.with_permit(self.token, || {
            let _operation = self.engine.ensure_open()?;
            validate_ticket(ticket)?;
            let path = std::fs::canonicalize(path)
                .map_err(|_| error(MaintenanceError::OriginalSinkMismatch))?;
            if !path.is_file() {
                return Err(error(MaintenanceError::OriginalSinkMismatch));
            }
            let fingerprint = telemetry::sink_fingerprint(&path)?;
            let guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            let c = guard.as_ref().ok_or(EngineError::Closing)?;
            let mut request =
                load(c, ticket)?.ok_or_else(|| error(MaintenanceError::UnknownTicket))?;
            if request.status.phase == ErasurePhase::Queued {
                request.originals.extend(telemetry::registered_sinks(c)?);
                request.originals.sort_unstable();
                request.originals.dedup();
                physical_erasure::transaction(c, || store(c, &request))?;
            }
            if !request.originals.contains(&fingerprint)
                || !telemetry::registered_sinks(c)?.contains(&fingerprint)
            {
                return Err(error(MaintenanceError::OriginalSinkMismatch));
            }
            self.original_sinks.lock().map_err(|_| EngineError::Storage)?.insert(fingerprint, path);
            Ok(())
        })
    }

    /// Complete exactly one accepted ticket through the existing physical proof,
    /// original-sink and WAL authority. Errors after commit retain owed work;
    /// callers inspect status rather than infer rollback from an error.
    pub fn complete_physical_erasure(&self, ticket: &str) -> Result<CleanupResult, EngineError> {
        let _serial =
            self.operation.try_lock().map_err(|_| error(MaintenanceError::MaintenanceBusy))?;
        self.engine.maintenance_admission.with_permit(self.token, || {
            let _operation = self.engine.ensure_open()?;
            validate_ticket(ticket)?;
            let _service = self
                .engine
                .physical_service_gate
                .try_lock()
                .map_err(|_| error(MaintenanceError::MaintenanceBusy))?;
            let result = self.complete_ticket_inner(ticket);
            let guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            let c = guard.as_ref().ok_or(EngineError::Closing)?;
            let active = physical_erasure::active(c)?;
            self.engine.explicit_debt.store(active, Ordering::Release);
            match result {
                Ok(()) => {
                    let mut status = load(c, ticket)?.ok_or(EngineError::Storage)?.status;
                    status.service_state = ErasureServiceState::MaintenanceOwned;
                    Ok(if status.phase == ErasurePhase::Complete {
                        CleanupResult::Complete { receipt: status }
                    } else {
                        CleanupResult::Incomplete { status }
                    })
                }
                Err(EngineError::ErasureIncomplete { stage, .. }) => {
                    let mut request = load(c, ticket)?.ok_or(EngineError::Storage)?;
                    request.status.blocker = Some(match stage.as_str() {
                        "telemetry_redaction" => ErasureBlocker::OriginalSinkUnavailable,
                        _ => ErasureBlocker::WalCheckpoint,
                    });
                    physical_erasure::transaction(c, || store(c, &request))?;
                    let mut status = request.status;
                    status.service_state = ErasureServiceState::MaintenanceOwned;
                    Ok(CleanupResult::Incomplete { status })
                }
                Err(error) => Err(error),
            }
        })
    }

    fn complete_ticket_inner(&self, ticket: &str) -> Result<(), EngineError> {
        let mut request = {
            let guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            load(guard.as_ref().ok_or(EngineError::Closing)?, ticket)?
                .ok_or_else(|| error(MaintenanceError::UnknownTicket))?
        };
        if request.status.phase == ErasurePhase::Complete {
            return Ok(());
        }
        if request.status.phase == ErasurePhase::WithdrawnBeforeRemoval {
            return Err(error(MaintenanceError::KeyRetired));
        }
        let mut sinks = self.original_sinks.lock().map_err(|_| EngineError::Storage)?.clone();
        if let Some(sink) = self.engine.telemetry.lock().map_err(|_| EngineError::Storage)?.as_ref()
        {
            let path = sink.path().to_owned();
            sinks.insert(telemetry::sink_fingerprint(&path)?, path);
        }
        {
            let guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            let c = guard.as_ref().ok_or(EngineError::Closing)?;
            let originals = if request.status.phase == ErasurePhase::Queued {
                request.originals.extend(telemetry::registered_sinks(c)?);
                request.originals.sort_unstable();
                request.originals.dedup();
                match physical_erasure::envelope(c) {
                    Ok(()) => {}
                    Err(EngineError::ErasureIncomplete { .. }) => {
                        request.status.blocker = Some(ErasureBlocker::ResourceEnvelopeChanged);
                        physical_erasure::transaction(c, || store(c, &request))?;
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                }
                let selector = request.selector.as_ref().ok_or(EngineError::Storage)?;
                if preview(c, selector)? != request.scope {
                    return Err(EngineError::Storage);
                }
                physical_erasure::transaction(c, || store(c, &request))?;
                request.originals.clone()
            } else {
                erasure::pending_original_fingerprints(c)?
            };
            if originals.iter().any(|fingerprint| {
                sinks.get(fingerprint).is_none_or(|path| {
                    !path.is_file()
                        || telemetry::sink_fingerprint(path).ok().as_ref() != Some(fingerprint)
                })
            }) {
                request.status.blocker = Some(ErasureBlocker::OriginalSinkUnavailable);
                physical_erasure::transaction(c, || store(c, &request))?;
                return Ok(());
            }
            if request.status.phase != ErasurePhase::Queued
                && physical_erasure::root(c)?.and_then(|d| d.ticket).as_deref() != Some(ticket)
            {
                return Err(EngineError::Storage);
            }
        }
        if request.status.phase == ErasurePhase::Queued {
            with_activation(ticket, || {
                match request.selector.as_ref().ok_or(EngineError::Storage)? {
                    PhysicalErasureSelector::SourceAllAtCut { source_id } => {
                        self.engine.excise_source_inner("erase_source", source_id).map(|_| ())
                    }
                    PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id } => {
                        self.engine.purge_inner(logical_id).map(|_| ())
                    }
                }
            })?;
        }
        let ids = {
            let guard = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
            physical_erasure::pending_ids(guard.as_ref().ok_or(EngineError::Closing)?)?
        };
        self.engine.finish_physical_dependency_closures_with_sinks(
            "erase_source",
            &ids,
            Some(&sinks),
        )
    }
}

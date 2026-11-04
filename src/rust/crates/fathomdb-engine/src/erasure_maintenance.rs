use super::*;
use serde::{Deserialize, Serialize};

const POLICY_KEY: &str = "_fathomdb_physical_erasure_policy";
const STORE_KEY: &str = "_fathomdb_erasure_store";

/// Persisted scheduling policy for physical erasure. Selection does not imply
/// logical revocation, physical completion, or a completion deadline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalErasurePolicy {
    /// Preserve synchronous erasure and the idle/reopen continuation worker.
    #[default]
    Autonomous,
    /// Execute physical work only in an explicitly acquired exclusive window.
    ExplicitMaintenance,
}

impl PhysicalErasurePolicy {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::Autonomous => "autonomous",
            Self::ExplicitMaintenance => "explicit_maintenance",
        }
    }
}

pub(crate) fn read_policy(c: &Connection) -> Result<PhysicalErasurePolicy, EngineError> {
    let raw: String = c
        .query_row("SELECT value FROM _fathomdb_open_state WHERE key=?1", [POLICY_KEY], |r| {
            r.get(0)
        })
        .map_err(|_| EngineError::Storage)?;
    let policy = match raw.as_str() {
        "autonomous" => PhysicalErasurePolicy::Autonomous,
        "explicit_maintenance" => PhysicalErasurePolicy::ExplicitMaintenance,
        _ => return Err(EngineError::Storage),
    };
    let store: String = c
        .query_row("SELECT value FROM _fathomdb_open_state WHERE key=?1", [STORE_KEY], |r| r.get(0))
        .map_err(|_| EngineError::Storage)?;
    if store.len() != 32 || !store.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(EngineError::Storage);
    }
    Ok(policy)
}

pub(crate) fn initialize_policy(
    c: &Connection,
    policy: PhysicalErasurePolicy,
) -> Result<(), EngineError> {
    physical_erasure::transaction(c, || {
        c.execute(
            "INSERT INTO _fathomdb_open_state(key,value) VALUES(?1,?2)",
            params![POLICY_KEY, policy.wire()],
        )
        .map_err(|_| EngineError::Storage)?;
        c.execute(
            "INSERT INTO _fathomdb_open_state(key,value) VALUES(?1,lower(hex(randomblob(16))))",
            [STORE_KEY],
        )
        .map_err(|_| EngineError::Storage)?;
        Ok(())
    })
}

pub(crate) fn validate_policy_before_open(
    path: &Path,
    requested: Option<PhysicalErasurePolicy>,
) -> Result<(), EngineOpenError> {
    if !path.exists() || std::fs::metadata(path).is_ok_and(|m| m.len() == 0) {
        return Ok(());
    }
    let c = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| policy_open_error())?;
    let effective = read_policy(&c).map_err(|_| policy_open_error())?;
    if requested.is_some_and(|p| p != effective) {
        return Err(EngineOpenError::EngineConfiguration(
            EngineConfigurationError::PhysicalErasurePolicyMismatch,
        ));
    }
    Ok(())
}

pub(crate) fn policy_open_error() -> EngineOpenError {
    EngineOpenError::Corruption(CorruptionDetail {
        kind: CorruptionKind::SchemaInconsistent,
        stage: OpenStage::SchemaProbe,
        locator: CorruptionLocator::OpaqueSqliteError {
            sqlite_extended_code: rusqlite::ffi::SQLITE_CORRUPT,
        },
        recovery_hint: RecoveryHint {
            code: "E_CORRUPT_ERASURE_POLICY",
            doc_anchor: "design/explicit-maintenance-erasure/contract.md",
        },
    })
}

impl Engine {
    /// Return the validated persisted policy honored by this open incarnation.
    pub fn physical_erasure_policy(&self) -> PhysicalErasurePolicy {
        self.physical_policy
    }
}

/// Identity-free maintenance refusal reason shared by language bindings.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceError {
    /// Invalid opaque idempotency-key syntax.
    InvalidKey,
    /// Invalid store-bound ticket syntax.
    InvalidTicket,
    /// A retained key is bound to a different selector.
    IdempotencyConflict,
    /// A terminal key cannot be reused for another request.
    KeyRetired,
    /// Queue or receipt metadata capacity is exhausted.
    Capacity,
    /// The selected closure cannot fit the supported resource limits.
    UnsupportedScope,
    /// The selected immutable closure overlaps queued intent.
    ScopeConflict,
    /// Mutation cannot prove disjointness from queued scope.
    QueuedScopeFenced,
    /// Exclusive ownership is currently acquiring or held.
    MaintenanceBusy,
    /// Direct destructive erasure is unavailable in the explicit profile.
    MaintenanceRequired,
    /// Acquisition was requested from an already admitted operation/callback.
    SelfPinnedReader,
    /// The handle no longer owns this Engine incarnation.
    StaleHandle,
    /// This API is unavailable under the persisted policy.
    PolicyMismatch,
}

impl Display for MaintenanceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "physical erasure maintenance: {self:?}")
    }
}
impl std::error::Error for MaintenanceError {}

/// Service availability after releasing ephemeral maintenance ownership.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasureServiceState {
    /// Ordinary operations are available subject to queued scope guards.
    NormalWithQueuedRestrictions,
    /// Exclusive maintenance owns ordinary admission.
    MaintenanceOwned,
    /// Logical removal committed and physical work still fences mutations.
    ReadsAllowedWritesFenced,
    /// This Engine has closed or is unavailable.
    ClosedOrUnavailable,
}

/// Exclusive, non-cloneable ownership of one Engine's maintenance window.
/// Drop releases only ephemeral ownership, never durable erasure obligations.
pub struct ErasureMaintenance {
    pub(crate) engine: Arc<Engine>,
    pub(crate) token: u64,
    pub(crate) operation: Mutex<()>,
    released: bool,
}

impl ErasureMaintenance {
    /// Release this window. Durable work, if any, remains fenced for explicit
    /// retry. Repeating close is safe; a closed Engine reports unavailable.
    pub fn close(&mut self) -> Result<ErasureServiceState, EngineError> {
        self.release();
        if self.engine.closed.load(Ordering::Acquire) {
            return Ok(ErasureServiceState::ClosedOrUnavailable);
        }
        let connection = self.engine.connection.lock().map_err(|_| EngineError::Storage)?;
        let c = connection.as_ref().ok_or(EngineError::Closing)?;
        Ok(if physical_erasure::active(c)? {
            ErasureServiceState::ReadsAllowedWritesFenced
        } else {
            ErasureServiceState::NormalWithQueuedRestrictions
        })
    }

    fn release(&mut self) {
        if !self.released {
            if self.engine.maintenance_admission.valid(self.token).is_ok() {
                let _ = physical_erasure::faultpoint("before_maintenance_thaw");
                self.engine.projection_runtime.set_frozen(false);
                self.engine.projection_runtime.notify_new_work();
                self.engine.maintenance_admission.release(self.token);
            }
            self.released = true;
        }
    }
}
impl Drop for ErasureMaintenance {
    fn drop(&mut self) {
        self.release();
    }
}

impl Engine {
    /// Acquire an exclusive maintenance window. `acquisition_wait_ms` bounds
    /// only cooperative admission/drain; no active SQL deadline is promised.
    /// New ordinary operations refuse promptly while acquisition or ownership
    /// is active. Callback/self-pinned acquisition refuses without waiting.
    pub fn begin_erasure_maintenance(
        self: &Arc<Self>,
        acquisition_wait_ms: u64,
    ) -> Result<ErasureMaintenance, EngineError> {
        if self.closed.load(Ordering::Acquire) {
            return Err(EngineError::Closing);
        }
        if self.physical_policy != PhysicalErasurePolicy::ExplicitMaintenance {
            return Err(EngineError::Maintenance(MaintenanceError::PolicyMismatch));
        }
        let started = Instant::now();
        let timeout = Duration::from_millis(acquisition_wait_ms);
        let token = self.maintenance_admission.acquire(timeout)?;
        self.projection_runtime.set_frozen(true);
        let remaining =
            timeout.saturating_sub(started.elapsed()).as_millis().min(u64::MAX as u128) as u64;
        if !self.projection_runtime.wait_for_workers_idle(remaining)
            || self.maintenance_admission.valid(token).is_err()
        {
            self.projection_runtime.set_frozen(false);
            self.projection_runtime.notify_new_work();
            self.maintenance_admission.release(token);
            return Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy));
        }
        Ok(ErasureMaintenance {
            engine: Arc::clone(self),
            token,
            operation: Mutex::new(()),
            released: false,
        })
    }

    pub(crate) fn require_autonomous_erasure(&self) -> Result<(), EngineError> {
        if self.physical_policy == PhysicalErasurePolicy::ExplicitMaintenance {
            return Err(EngineError::Maintenance(MaintenanceError::MaintenanceRequired));
        }
        Ok(())
    }
}

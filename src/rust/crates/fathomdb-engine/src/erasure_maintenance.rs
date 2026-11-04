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

use super::*;
use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::Instant;

const MAX_OPERATIONS: usize = 128;
const MAX_AFFECTED_REVISIONS: usize = 256;
const MAX_PENDING_CURSORS: usize = 128;
const MAX_SOURCE_REFS: usize = 1024;
const MAX_SOURCE_REFS_PER_OPERATION: usize = 8;

/// One operation in a bounded caller-decided actuation batch.
#[derive(Clone, Debug, PartialEq)]
pub enum ActuationOperationV1 {
    /// Store one complete canonical-source node revision.
    PutCanonicalNode(ProvenancedNodeV1),
    /// Store one complete derived-semantic node revision.
    PutDerivedNode(ProvenancedNodeV1),
    /// Register one immutable source-to-derived dependency.
    RegisterSourceDependency(SourceDependencyRegistrationV1),
    /// Apply one revision-pinned lifecycle transition.
    TransitionLifecycle(LifecycleActuationV1),
}

/// Revision-pinned lifecycle change in an actuation batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleActuationV1 {
    /// Bare logical ID after normalization.
    pub logical_id: String,
    /// Immutable revision the caller observed as current.
    pub expected_current_revision_id: ArtifactRevisionId,
    /// Target state (`active` or `deleted`).
    pub to_state: LifecycleState,
    /// Optional advisory reason for a delete-family transition.
    pub reason: Option<String>,
}

impl LifecycleActuationV1 {
    /// Validate and normalize a revision-pinned lifecycle operation.
    ///
    /// # Errors
    ///
    /// Returns `logical_id_invalid` for empty, delimiter-bearing, or non-logical
    /// addresses and `lifecycle_target_invalid` for non-transition targets.
    pub fn new(
        logical_id: impl Into<String>,
        expected_current_revision_id: ArtifactRevisionId,
        to_state: LifecycleState,
        reason: Option<String>,
    ) -> Result<Self, ActuationError> {
        let supplied = logical_id.into();
        let logical_id = supplied.strip_prefix("l:").unwrap_or(&supplied).to_string();
        if logical_id.is_empty()
            || logical_id.contains('\x1e')
            || supplied.starts_with("h:")
            || supplied.starts_with("p:")
        {
            return Err(ActuationError::new(ActuationErrorReason::LogicalIdInvalid, "/logicalId"));
        }
        if !matches!(to_state, LifecycleState::Active | LifecycleState::Deleted) {
            return Err(ActuationError::new(
                ActuationErrorReason::LifecycleTargetInvalid,
                "/toState",
            ));
        }
        Ok(Self { logical_id, expected_current_revision_id, to_state, reason })
    }
}

/// Closed schema-version-1 actuation request.
#[derive(Clone, Debug, PartialEq)]
pub struct ActuationBatchV1 {
    /// Closed schema discriminator.
    pub schema_version: u32,
    /// Caller-owned idempotency key.
    pub operation_id: String,
    /// Optional caller policy/version identity.
    pub decision_policy_id: Option<String>,
    /// Optional compare-and-set global write boundary.
    pub expected_write_boundary: Option<u64>,
    /// Ordered operations, bounded to 1–128.
    pub operations: Vec<ActuationOperationV1>,
}

impl ActuationBatchV1 {
    /// Construct a bounded actuation request.
    ///
    /// # Errors
    ///
    /// Returns a typed ID or operation-count error before any Engine call.
    pub fn new(
        operation_id: impl Into<String>,
        operations: Vec<ActuationOperationV1>,
    ) -> Result<Self, ActuationError> {
        let operation_id = operation_id.into();
        if !valid_caller_identity(&operation_id) {
            return Err(ActuationError::new(
                ActuationErrorReason::OperationIdInvalid,
                "/operationId",
            ));
        }
        if !(1..=MAX_OPERATIONS).contains(&operations.len()) {
            return Err(ActuationError::new(
                ActuationErrorReason::OperationCountInvalid,
                "/operations",
            ));
        }
        Ok(Self {
            schema_version: 1,
            operation_id,
            decision_policy_id: None,
            expected_write_boundary: None,
            operations,
        })
    }

    /// Attach a caller policy/version identity.
    ///
    /// # Errors
    ///
    /// Returns `decision_policy_id_invalid` for an ID outside the caller grammar.
    pub fn with_decision_policy_id(
        mut self,
        decision_policy_id: impl Into<String>,
    ) -> Result<Self, ActuationError> {
        let value = decision_policy_id.into();
        if !valid_caller_identity(&value) {
            return Err(ActuationError::new(
                ActuationErrorReason::DecisionPolicyIdInvalid,
                "/decisionPolicyId",
            ));
        }
        self.decision_policy_id = Some(value);
        Ok(self)
    }

    /// Attach a compare-and-set global write boundary.
    #[must_use]
    pub fn with_expected_write_boundary(mut self, boundary: u64) -> Self {
        self.expected_write_boundary = Some(boundary);
        self
    }
}

/// Terminal actuation outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActuationOutcomeV1 {
    /// Every operation committed atomically.
    Committed,
    /// Reserved for Slice 30 dependency closure.
    CommittedClosurePending,
    /// The whole request was refused without domain mutation.
    Refused,
}

impl ActuationOutcomeV1 {
    fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "committed",
            Self::CommittedClosurePending => "committed_closure_pending",
            Self::Refused => "refused",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "committed" => Some(Self::Committed),
            "committed_closure_pending" => Some(Self::CommittedClosurePending),
            "refused" => Some(Self::Refused),
            _ => None,
        }
    }
}

/// Closed terminal refusal reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActuationRefusalReasonV1 {
    ExpectedWriteBoundaryMismatch,
    WriteRefused,
    ProvenanceRoleMismatch,
    ReferenceUnavailable,
    DependencyRefused,
    LifecycleRefused,
    DependencyClosureRequired,
    WriteCursorExhausted,
    DependencyGenerationExhausted,
}

impl ActuationRefusalReasonV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExpectedWriteBoundaryMismatch => "expected_write_boundary_mismatch",
            Self::WriteRefused => "write_refused",
            Self::ProvenanceRoleMismatch => "provenance_role_mismatch",
            Self::ReferenceUnavailable => "reference_unavailable",
            Self::DependencyRefused => "dependency_refused",
            Self::LifecycleRefused => "lifecycle_refused",
            Self::DependencyClosureRequired => "dependency_closure_required",
            Self::WriteCursorExhausted => "write_cursor_exhausted",
            Self::DependencyGenerationExhausted => "dependency_generation_exhausted",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "expected_write_boundary_mismatch" => Some(Self::ExpectedWriteBoundaryMismatch),
            "write_refused" => Some(Self::WriteRefused),
            "provenance_role_mismatch" => Some(Self::ProvenanceRoleMismatch),
            "reference_unavailable" => Some(Self::ReferenceUnavailable),
            "dependency_refused" => Some(Self::DependencyRefused),
            "lifecycle_refused" => Some(Self::LifecycleRefused),
            "dependency_closure_required" => Some(Self::DependencyClosureRequired),
            "write_cursor_exhausted" => Some(Self::WriteCursorExhausted),
            "dependency_generation_exhausted" => Some(Self::DependencyGenerationExhausted),
            _ => None,
        }
    }
}

/// Compact terminal receipt for one actuation operation ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActuationReceiptV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub request_sha256: String,
    pub outcome: ActuationOutcomeV1,
    pub refused_operation_index: Option<usize>,
    pub refused_field_path: Option<String>,
    pub reason_codes: Vec<ActuationRefusalReasonV1>,
    pub affected_revision_ids: Vec<String>,
    pub resulting_write_boundary: Option<u64>,
    pub resulting_dependency_generation: Option<u64>,
    pub pending_projection_write_cursors: Vec<u64>,
    pub closure_operation_ids: Vec<String>,
}

/// Closed request/idempotency failure reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActuationErrorReason {
    UnsupportedSchemaVersion,
    UnknownField,
    UnknownOperationVariant,
    FieldMissing,
    FieldTypeInvalid,
    OperationIdInvalid,
    DecisionPolicyIdInvalid,
    OperationCountInvalid,
    LogicalIdInvalid,
    RevisionIdInvalid,
    LifecycleTargetInvalid,
    NestedRequestInvalid,
    OperationIdConflict,
    OperationIdErased,
}

impl ActuationErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::UnknownOperationVariant => "unknown_operation_variant",
            Self::FieldMissing => "field_missing",
            Self::FieldTypeInvalid => "field_type_invalid",
            Self::OperationIdInvalid => "operation_id_invalid",
            Self::DecisionPolicyIdInvalid => "decision_policy_id_invalid",
            Self::OperationCountInvalid => "operation_count_invalid",
            Self::LogicalIdInvalid => "logical_id_invalid",
            Self::RevisionIdInvalid => "revision_id_invalid",
            Self::LifecycleTargetInvalid => "lifecycle_target_invalid",
            Self::NestedRequestInvalid => "nested_request_invalid",
            Self::OperationIdConflict => "operation_id_conflict",
            Self::OperationIdErased => "operation_id_erased",
        }
    }
}

/// Typed actuation request/idempotency error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActuationError {
    pub reason: ActuationErrorReason,
    pub field_path: String,
}

impl ActuationError {
    pub(crate) fn new(reason: ActuationErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for ActuationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl Error for ActuationError {}

impl From<ActuationError> for EngineError {
    fn from(error: ActuationError) -> Self {
        Self::Actuation(error)
    }
}

#[derive(Clone)]
struct Refusal {
    reason: ActuationRefusalReasonV1,
    index: Option<usize>,
    path: Option<String>,
}

fn encode_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_be_bytes());
}

fn encode_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn encode_i64(hasher: &mut Sha256, value: i64) {
    hasher.update(value.to_be_bytes());
}

fn encode_string(hasher: &mut Sha256, value: &str) {
    encode_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn encode_optional<T>(hasher: &mut Sha256, value: Option<T>, encode: impl Fn(&mut Sha256, T)) {
    match value {
        Some(value) => {
            hasher.update([1]);
            encode(hasher, value);
        }
        None => hasher.update([0]),
    }
}

fn request_digest(request: &ActuationBatchV1) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"fathomdb.actuation.v1\0");
    hasher.update([0x01]);
    encode_u32(&mut hasher, request.schema_version);
    hasher.update([0x02]);
    encode_string(&mut hasher, &request.operation_id);
    hasher.update([0x03]);
    encode_optional(&mut hasher, request.decision_policy_id.as_deref(), encode_string);
    hasher.update([0x04]);
    encode_optional(&mut hasher, request.expected_write_boundary, encode_u64);
    hasher.update([0x05]);
    encode_u32(&mut hasher, request.operations.len() as u32);
    for operation in &request.operations {
        encode_operation(&mut hasher, operation);
    }
    hex_encode(&hasher.finalize())
}

fn validate_request(request: &ActuationBatchV1) -> Result<(), ActuationError> {
    if request.schema_version != 1 {
        return Err(ActuationError::new(
            ActuationErrorReason::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    if !valid_caller_identity(&request.operation_id) {
        return Err(ActuationError::new(ActuationErrorReason::OperationIdInvalid, "/operationId"));
    }
    if request.decision_policy_id.as_deref().is_some_and(|value| !valid_caller_identity(value)) {
        return Err(ActuationError::new(
            ActuationErrorReason::DecisionPolicyIdInvalid,
            "/decisionPolicyId",
        ));
    }
    if !(1..=MAX_OPERATIONS).contains(&request.operations.len()) {
        return Err(ActuationError::new(
            ActuationErrorReason::OperationCountInvalid,
            "/operations",
        ));
    }
    for (index, operation) in request.operations.iter().enumerate() {
        let ActuationOperationV1::TransitionLifecycle(lifecycle) = operation else {
            continue;
        };
        if lifecycle.logical_id.is_empty()
            || lifecycle.logical_id.contains('\x1e')
            || lifecycle.logical_id.starts_with("l:")
            || lifecycle.logical_id.starts_with("h:")
            || lifecycle.logical_id.starts_with("p:")
        {
            return Err(ActuationError::new(
                ActuationErrorReason::LogicalIdInvalid,
                format!("/operations/{index}/logicalId"),
            ));
        }
        if !matches!(lifecycle.to_state, LifecycleState::Active | LifecycleState::Deleted) {
            return Err(ActuationError::new(
                ActuationErrorReason::LifecycleTargetInvalid,
                format!("/operations/{index}/toState"),
            ));
        }
    }
    Ok(())
}

fn encode_operation(hasher: &mut Sha256, operation: &ActuationOperationV1) {
    match operation {
        ActuationOperationV1::PutCanonicalNode(node) => {
            hasher.update([0x10]);
            encode_node(hasher, node);
        }
        ActuationOperationV1::PutDerivedNode(node) => {
            hasher.update([0x11]);
            encode_node(hasher, node);
        }
        ActuationOperationV1::RegisterSourceDependency(dependency) => {
            hasher.update([0x12, 0x40]);
            encode_u32(hasher, dependency.schema_version);
            hasher.update([0x41]);
            encode_string(hasher, dependency.dependency_id.as_str());
            hasher.update([0x42]);
            encode_string(hasher, dependency.source_revision_id.as_str());
            hasher.update([0x43]);
            encode_string(hasher, dependency.derived_revision_id.as_str());
        }
        ActuationOperationV1::TransitionLifecycle(lifecycle) => {
            hasher.update([0x13, 0x50]);
            encode_string(hasher, &lifecycle.logical_id);
            hasher.update([0x51]);
            encode_string(hasher, lifecycle.expected_current_revision_id.as_str());
            hasher.update([0x52]);
            hasher.update([match lifecycle.to_state {
                LifecycleState::Active => 0,
                LifecycleState::Deleted => 1,
                _ => unreachable!("constructor excludes other lifecycle targets"),
            }]);
            hasher.update([0x53]);
            encode_optional(hasher, lifecycle.reason.as_deref(), encode_string);
        }
    }
}

fn encode_node(hasher: &mut Sha256, node: &ProvenancedNodeV1) {
    hasher.update([0x20]);
    encode_string(hasher, &node.kind);
    hasher.update([0x21]);
    encode_string(hasher, &node.body);
    hasher.update([0x22]);
    encode_string(hasher, node.source_id.as_str());
    hasher.update([0x23]);
    encode_optional(hasher, node.logical_id.as_deref(), encode_string);
    hasher.update([0x24, if node.state == InitialState::Pending { 0 } else { 1 }]);
    hasher.update([0x25]);
    encode_optional(hasher, node.reason.as_deref(), encode_string);
    hasher.update([0x26]);
    encode_optional(hasher, node.valid_from, encode_i64);
    hasher.update([0x27]);
    encode_optional(hasher, node.valid_until, encode_i64);
    hasher.update([0x28, 0x30]);
    encode_u32(hasher, node.provenance.schema_version);
    hasher.update([0x31]);
    hasher.update([match node.provenance.role {
        ProvenanceRole::Canonical => 0,
        ProvenanceRole::Derived => 1,
    }]);
    hasher.update([0x32]);
    encode_string(hasher, node.provenance.artifact_revision_id.as_str());
    hasher.update([0x33]);
    encode_string(hasher, node.provenance.source_version_id.as_str());
    hasher.update([0x34]);
    encode_optional(
        hasher,
        node.provenance.source_revision_id.as_ref().map(SourceRevisionId::as_str),
        encode_string,
    );
    hasher.update([0x35]);
    encode_optional(hasher, node.provenance.locator.as_ref(), |hasher, locator| match locator {
        SourceLocator::WholeBody => hasher.update([0]),
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            hasher.update([1]);
            encode_u64(hasher, *start_inclusive);
            encode_u64(hasher, *end_exclusive);
        }
    });
    hasher.update([0x36]);
    encode_optional(
        hasher,
        node.provenance.canonical_source_hash.as_ref().map(CanonicalHash::digest_hex),
        encode_string,
    );
}

fn receipt_for_refusal(
    request: &ActuationBatchV1,
    digest: String,
    refusal: Refusal,
) -> ActuationReceiptV1 {
    ActuationReceiptV1 {
        schema_version: 1,
        operation_id: request.operation_id.clone(),
        request_sha256: digest,
        outcome: ActuationOutcomeV1::Refused,
        refused_operation_index: refusal.index,
        refused_field_path: refusal.path,
        reason_codes: vec![refusal.reason],
        affected_revision_ids: Vec::new(),
        resulting_write_boundary: None,
        resulting_dependency_generation: None,
        pending_projection_write_cursors: Vec::new(),
        closure_operation_ids: Vec::new(),
    }
}

fn load_receipt(
    connection: &Connection,
    operation_id: &str,
    digest: &str,
    request: Option<&ActuationBatchV1>,
) -> Result<Option<ActuationReceiptV1>, EngineError> {
    type Row = (
        i64,
        Option<String>,
        Option<i64>,
        String,
        Option<i64>,
        Option<String>,
        String,
        String,
        Option<i64>,
        Option<i64>,
        String,
        String,
    );
    let row: Option<Row> = connection
        .query_row(
            "SELECT schema_version,request_sha256,operations_count,outcome,\
                    refused_operation_index,refused_field_path,reason_codes_json,\
                    affected_revision_ids_json,resulting_write_boundary,\
                    resulting_dependency_generation,pending_projection_write_cursors_json,\
                    closure_operation_ids_json FROM _fathomdb_actuation_receipts \
             WHERE operation_id=?1",
            [operation_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                ))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((
        schema,
        stored_digest,
        operations_count,
        outcome,
        refused_index,
        refused_path,
        reasons_json,
        affected_json,
        boundary,
        generation,
        pending_json,
        closure_json,
    )) = row
    else {
        return Ok(None);
    };
    if outcome == "erased" {
        let erased_is_canonical = schema == 1
            && stored_digest.is_none()
            && operations_count.is_none()
            && refused_index.is_none()
            && refused_path.is_none()
            && reasons_json == "[]"
            && affected_json == "[]"
            && boundary.is_none()
            && generation.is_none()
            && pending_json == "[]"
            && closure_json == "[]"
            && validate_source_refs(connection, operation_id, 0)? == 0;
        if !erased_is_canonical {
            return Err(EngineError::Storage);
        }
        return Err(
            ActuationError::new(ActuationErrorReason::OperationIdErased, "/operationId").into()
        );
    }
    let Some(stored_digest) = stored_digest else {
        return Err(EngineError::Storage);
    };
    if stored_digest.len() != 64
        || stored_digest
            .bytes()
            .any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(EngineError::Storage);
    }
    if stored_digest != digest {
        return Err(
            ActuationError::new(ActuationErrorReason::OperationIdConflict, "/operationId").into()
        );
    }
    if schema != 1 || !(1..=MAX_OPERATIONS as i64).contains(&operations_count.unwrap_or(0)) {
        return Err(EngineError::Storage);
    }
    let operations_count =
        usize::try_from(operations_count.unwrap_or(0)).map_err(|_| EngineError::Storage)?;
    if request.is_some_and(|value| value.operations.len() != operations_count) {
        return Err(EngineError::Storage);
    }
    let outcome = ActuationOutcomeV1::parse(&outcome).ok_or(EngineError::Storage)?;
    let reason_strings: Vec<String> =
        serde_json::from_str(&reasons_json).map_err(|_| EngineError::Storage)?;
    if serde_json::to_string(&reason_strings).map_err(|_| EngineError::Storage)? != reasons_json {
        return Err(EngineError::Storage);
    }
    let reason_codes = reason_strings
        .iter()
        .map(|value| ActuationRefusalReasonV1::parse(value).ok_or(EngineError::Storage))
        .collect::<Result<Vec<_>, _>>()?;
    let affected_revision_ids: Vec<String> =
        serde_json::from_str(&affected_json).map_err(|_| EngineError::Storage)?;
    if serde_json::to_string(&affected_revision_ids).map_err(|_| EngineError::Storage)?
        != affected_json
    {
        return Err(EngineError::Storage);
    }
    if affected_revision_ids.len() > MAX_AFFECTED_REVISIONS
        || affected_revision_ids.len() > operations_count.saturating_mul(2)
        || affected_revision_ids.iter().any(|id| !stored_artifact_revision_id_is_valid(id))
        || affected_revision_ids.iter().collect::<BTreeSet<_>>().len()
            != affected_revision_ids.len()
    {
        return Err(EngineError::Storage);
    }
    let pending_strings: Vec<String> =
        serde_json::from_str(&pending_json).map_err(|_| EngineError::Storage)?;
    if serde_json::to_string(&pending_strings).map_err(|_| EngineError::Storage)? != pending_json
        || pending_strings.len() > MAX_PENDING_CURSORS
        || pending_strings.len() > operations_count
    {
        return Err(EngineError::Storage);
    }
    let pending_projection_write_cursors = pending_strings
        .iter()
        .map(|value| {
            let parsed = value.parse::<u64>().map_err(|_| EngineError::Storage)?;
            if parsed == 0 || parsed.to_string() != *value {
                return Err(EngineError::Storage);
            }
            Ok(parsed)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if pending_projection_write_cursors.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(EngineError::Storage);
    }
    let closure_operation_ids: Vec<String> =
        serde_json::from_str(&closure_json).map_err(|_| EngineError::Storage)?;
    if serde_json::to_string(&closure_operation_ids).map_err(|_| EngineError::Storage)?
        != closure_json
        || !closure_operation_ids.is_empty()
    {
        return Err(EngineError::Storage);
    }
    let refused_index = refused_index
        .map(|value| usize::try_from(value).map_err(|_| EngineError::Storage))
        .transpose()?;
    let resulting_write_boundary =
        boundary.map(|value| u64::try_from(value).map_err(|_| EngineError::Storage)).transpose()?;
    let resulting_dependency_generation = generation
        .map(|value| u64::try_from(value).map_err(|_| EngineError::Storage))
        .transpose()?;
    match outcome {
        ActuationOutcomeV1::Committed => {
            if refused_index.is_some()
                || refused_path.is_some()
                || !reason_codes.is_empty()
                || resulting_write_boundary.is_none()
                || resulting_dependency_generation == Some(0)
            {
                return Err(EngineError::Storage);
            }
            let boundary = resulting_write_boundary.ok_or(EngineError::Storage)?;
            if pending_projection_write_cursors.iter().any(|cursor| *cursor > boundary) {
                return Err(EngineError::Storage);
            }
            for cursor in &pending_projection_write_cursors {
                let cursor = i64::try_from(*cursor).map_err(|_| EngineError::Storage)?;
                let revision_id: Option<String> = connection
                    .query_row(
                        "SELECT revision_id FROM _fathomdb_artifact_revisions \
                         WHERE write_cursor=?1",
                        [cursor],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|_| EngineError::Storage)?;
                let Some(revision_id) = revision_id else {
                    return Err(EngineError::Storage);
                };
                if !affected_revision_ids.contains(&revision_id) {
                    return Err(EngineError::Storage);
                }
                if let Some(request) = request {
                    let created_by_request = request.operations.iter().any(|operation| {
                        matches!(
                            operation,
                            ActuationOperationV1::PutCanonicalNode(node)
                                | ActuationOperationV1::PutDerivedNode(node)
                                if node.provenance.artifact_revision_id.as_str() == revision_id
                        )
                    });
                    if !created_by_request {
                        return Err(EngineError::Storage);
                    }
                }
            }
        }
        ActuationOutcomeV1::CommittedClosurePending => return Err(EngineError::Storage),
        ActuationOutcomeV1::Refused => {
            if reason_codes.len() != 1
                || !affected_revision_ids.is_empty()
                || resulting_write_boundary.is_some()
                || resulting_dependency_generation.is_some()
                || !pending_projection_write_cursors.is_empty()
            {
                return Err(EngineError::Storage);
            }
            validate_refusal_shape(
                reason_codes[0],
                refused_index,
                refused_path.as_deref(),
                operations_count,
            )?;
        }
    }
    validate_source_refs(
        connection,
        operation_id,
        operations_count.saturating_mul(MAX_SOURCE_REFS_PER_OPERATION),
    )?;
    Ok(Some(ActuationReceiptV1 {
        schema_version: 1,
        operation_id: operation_id.to_string(),
        request_sha256: stored_digest,
        outcome,
        refused_operation_index: refused_index,
        refused_field_path: refused_path,
        reason_codes,
        affected_revision_ids,
        resulting_write_boundary,
        resulting_dependency_generation,
        pending_projection_write_cursors,
        closure_operation_ids,
    }))
}

fn validate_refusal_shape(
    reason: ActuationRefusalReasonV1,
    index: Option<usize>,
    path: Option<&str>,
    operations_count: usize,
) -> Result<(), EngineError> {
    if reason == ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch {
        return if index.is_none() && path == Some("/expectedWriteBoundary") {
            Ok(())
        } else {
            Err(EngineError::Storage)
        };
    }
    let Some(index) = index.filter(|value| *value < operations_count) else {
        return Err(EngineError::Storage);
    };
    let operation_path = |suffix: &str| format!("/operations/{index}{suffix}");
    let matches_one = |suffixes: &[&str]| {
        path.is_some_and(|value| suffixes.iter().any(|suffix| value == operation_path(suffix)))
    };
    let valid = match reason {
        ActuationRefusalReasonV1::WriteRefused => matches_one(&[
            "/record",
            "/record/provenance/sourceVersionId",
            "/record/provenance/sourceRevisionId",
            "/record/provenance/sourceLocator",
            "/record/provenance/canonicalSourceHash",
        ]),
        ActuationRefusalReasonV1::WriteCursorExhausted => matches_one(&["/record"]),
        ActuationRefusalReasonV1::ProvenanceRoleMismatch => {
            matches_one(&["/record/provenance/role"])
        }
        ActuationRefusalReasonV1::ReferenceUnavailable => matches_one(&[
            "/record/provenance/sourceRevisionId",
            "/dependency/sourceRevisionId",
            "/dependency/derivedRevisionId",
        ]),
        ActuationRefusalReasonV1::DependencyRefused
        | ActuationRefusalReasonV1::DependencyGenerationExhausted => matches_one(&["/dependency"]),
        ActuationRefusalReasonV1::LifecycleRefused => {
            matches_one(&["/expectedCurrentRevisionId", "/toState"])
        }
        ActuationRefusalReasonV1::DependencyClosureRequired => matches_one(&[""]),
        ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch => unreachable!(),
    };
    valid.then_some(()).ok_or(EngineError::Storage)
}

fn validate_source_refs(
    connection: &Connection,
    operation_id: &str,
    max_refs: usize,
) -> Result<usize, EngineError> {
    let limit = i64::try_from(max_refs.saturating_add(1)).map_err(|_| EngineError::Storage)?;
    let mut statement = connection
        .prepare(
            "SELECT schema_version,ref_kind,ref_value \
             FROM _fathomdb_actuation_receipt_source_refs \
             WHERE operation_id=?1 ORDER BY ref_kind,ref_value LIMIT ?2",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(params![operation_id, limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    if rows.len() > max_refs || rows.len() > MAX_SOURCE_REFS {
        return Err(EngineError::Storage);
    }
    for (schema, kind, value) in &rows {
        let valid = match kind.as_str() {
            "source_id" => {
                SourceId::new(value.clone()).is_ok()
                    || value.starts_with(SourceId::ENGINE_PREFIX)
                    || value == SourceId::LEGACY_PRE_0_8_20
            }
            "source_revision_id" | "artifact_revision_id" => {
                stored_artifact_revision_id_is_valid(value)
            }
            _ => false,
        };
        if *schema != 1 || !valid {
            return Err(EngineError::Storage);
        }
    }
    Ok(rows.len())
}

fn store_receipt(
    connection: &Connection,
    receipt: &ActuationReceiptV1,
    operations: usize,
) -> Result<(), EngineError> {
    let operations = i64::try_from(operations).map_err(|_| EngineError::Storage)?;
    let refused_operation_index = receipt
        .refused_operation_index
        .map(|value| i64::try_from(value).map_err(|_| EngineError::Storage))
        .transpose()?;
    let resulting_write_boundary = receipt
        .resulting_write_boundary
        .map(|value| i64::try_from(value).map_err(|_| EngineError::Storage))
        .transpose()?;
    let resulting_dependency_generation = receipt
        .resulting_dependency_generation
        .map(|value| i64::try_from(value).map_err(|_| EngineError::Storage))
        .transpose()?;
    let reasons = receipt.reason_codes.iter().map(|reason| reason.as_str()).collect::<Vec<_>>();
    let pending =
        receipt.pending_projection_write_cursors.iter().map(u64::to_string).collect::<Vec<_>>();
    connection
        .execute(
            "INSERT INTO _fathomdb_actuation_receipts(\
               operation_id,schema_version,request_sha256,operations_count,outcome,\
               refused_operation_index,refused_field_path,reason_codes_json,\
               affected_revision_ids_json,resulting_write_boundary,\
               resulting_dependency_generation,pending_projection_write_cursors_json,\
               closure_operation_ids_json\
             ) VALUES(?1,1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                receipt.operation_id,
                receipt.request_sha256,
                operations,
                receipt.outcome.as_str(),
                refused_operation_index,
                receipt.refused_field_path,
                serde_json::to_string(&reasons).map_err(|_| EngineError::Storage)?,
                serde_json::to_string(&receipt.affected_revision_ids)
                    .map_err(|_| EngineError::Storage)?,
                resulting_write_boundary,
                resulting_dependency_generation,
                serde_json::to_string(&pending).map_err(|_| EngineError::Storage)?,
                serde_json::to_string(&receipt.closure_operation_ids)
                    .map_err(|_| EngineError::Storage)?,
            ],
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(())
}

fn collect_source_refs(request: &ActuationBatchV1) -> BTreeSet<(&'static str, String)> {
    let mut refs = BTreeSet::new();
    for operation in &request.operations {
        match operation {
            ActuationOperationV1::PutCanonicalNode(node)
            | ActuationOperationV1::PutDerivedNode(node) => {
                refs.insert(("source_id", node.source_id.as_str().to_string()));
                refs.insert((
                    "artifact_revision_id",
                    node.provenance.artifact_revision_id.as_str().to_string(),
                ));
                if let Some(source) = &node.provenance.source_revision_id {
                    refs.insert(("source_revision_id", source.as_str().to_string()));
                }
            }
            ActuationOperationV1::RegisterSourceDependency(dependency) => {
                refs.insert((
                    "source_revision_id",
                    dependency.source_revision_id.as_str().to_string(),
                ));
                refs.insert((
                    "artifact_revision_id",
                    dependency.derived_revision_id.as_str().to_string(),
                ));
            }
            ActuationOperationV1::TransitionLifecycle(lifecycle) => {
                refs.insert((
                    "artifact_revision_id",
                    lifecycle.expected_current_revision_id.as_str().to_string(),
                ));
            }
        }
    }
    refs
}

fn enrich_resolved_refs(
    connection: &Connection,
    request: &ActuationBatchV1,
    refs: &mut BTreeSet<(&'static str, String)>,
) -> Result<(), EngineError> {
    for operation in &request.operations {
        let logical_id = match operation {
            ActuationOperationV1::PutCanonicalNode(node)
            | ActuationOperationV1::PutDerivedNode(node) => node.logical_id.as_deref(),
            ActuationOperationV1::TransitionLifecycle(lifecycle) => {
                Some(lifecycle.logical_id.as_str())
            }
            ActuationOperationV1::RegisterSourceDependency(_) => None,
        };
        let Some(logical_id) = logical_id else {
            continue;
        };
        let Some((revision, _, role)) = current_revision_for_logical(connection, logical_id)?
        else {
            continue;
        };
        refs.insert(("artifact_revision_id", revision.clone()));
        if role == "canonical_source" {
            refs.insert(("source_revision_id", revision.clone()));
        }
        let source_id: Option<String> = connection
            .query_row(
                "SELECT source_id FROM _fathomdb_source_links WHERE artifact_revision_id=?1",
                [&revision],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if let Some(source_id) = source_id {
            refs.insert(("source_id", source_id));
        }
    }
    Ok(())
}

fn store_source_refs(
    connection: &Connection,
    operation_id: &str,
    refs: &BTreeSet<(&'static str, String)>,
    operations_count: usize,
) -> Result<(), EngineError> {
    if refs.len() > MAX_SOURCE_REFS
        || refs.len() > operations_count.saturating_mul(MAX_SOURCE_REFS_PER_OPERATION)
    {
        return Err(EngineError::Storage);
    }
    for (kind, value) in refs {
        connection
            .execute(
                "INSERT INTO _fathomdb_actuation_receipt_source_refs(\
                   operation_id,schema_version,ref_kind,ref_value\
                 ) VALUES(?1,1,?2,?3)",
                params![operation_id, kind, value],
            )
            .map_err(|_| EngineError::Storage)?;
    }
    Ok(())
}

pub(super) fn redact_actuation_receipts_for_refs(
    connection: &Connection,
    refs: &BTreeSet<(String, String)>,
) -> Result<(), EngineError> {
    for (kind, value) in refs {
        let mut after = String::new();
        loop {
            let operation_ids = {
                let mut statement = connection
                    .prepare(
                        "SELECT operation_id FROM _fathomdb_actuation_receipt_source_refs \
                         WHERE ref_kind=?1 AND ref_value=?2 AND operation_id>?3 \
                         ORDER BY operation_id LIMIT 64",
                    )
                    .map_err(|_| EngineError::Storage)?;
                let rows = statement
                    .query_map(params![kind, value, after], |row| row.get::<_, String>(0))
                    .map_err(|_| EngineError::Storage)?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|_| EngineError::Storage)?;
                rows
            };
            let Some(last) = operation_ids.last().cloned() else {
                break;
            };
            for operation_id in operation_ids {
                let digest: String = connection
                    .query_row(
                        "SELECT request_sha256 FROM _fathomdb_actuation_receipts \
                         WHERE operation_id=?1 AND outcome!='erased'",
                        [&operation_id],
                        |row| row.get(0),
                    )
                    .map_err(|_| EngineError::Storage)?;
                let _ = load_receipt(connection, &operation_id, &digest, None)?
                    .ok_or(EngineError::Storage)?;
                connection
                    .execute(
                        "DELETE FROM _fathomdb_actuation_receipt_source_refs \
                         WHERE operation_id=?1",
                        [&operation_id],
                    )
                    .map_err(|_| EngineError::Storage)?;
                connection
                    .execute(
                        "UPDATE _fathomdb_actuation_receipts SET \
                           request_sha256=NULL,operations_count=NULL,outcome='erased', \
                           refused_operation_index=NULL,refused_field_path=NULL, \
                           reason_codes_json='[]',affected_revision_ids_json='[]', \
                           resulting_write_boundary=NULL,resulting_dependency_generation=NULL, \
                           pending_projection_write_cursors_json='[]',closure_operation_ids_json='[]' \
                         WHERE operation_id=?1",
                        [&operation_id],
                    )
                    .map_err(|_| EngineError::Storage)?;
            }
            after = last;
        }
    }
    Ok(())
}

fn current_revision_for_logical(
    connection: &Connection,
    logical_id: &str,
) -> Result<Option<(String, LifecycleState, String)>, EngineError> {
    let row: Option<(String, String, String)> = connection
        .query_row(
            "SELECT ar.revision_id,n.state,ar.artifact_role \
             FROM canonical_nodes n JOIN _fathomdb_artifact_revisions ar \
               ON ar.artifact_class='node' AND ar.write_cursor=n.write_cursor \
             WHERE n.logical_id=?1 AND n.superseded_at IS NULL",
            [logical_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    row.map(|(revision, state, role)| {
        Ok((revision, LifecycleState::from_str_opt(&state).ok_or(EngineError::Storage)?, role))
    })
    .transpose()
}

fn closure_refusal_for_losses(
    connection: &Connection,
    losses: &[(usize, String)],
) -> Result<Option<Refusal>, EngineError> {
    for (index, revision) in losses {
        let has_dependency: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_dependencies d \
                 JOIN _fathomdb_source_links l \
                   ON l.artifact_revision_id=d.derived_revision_id \
                 WHERE l.source_revision_id=?1)",
                [revision],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        if has_dependency {
            return Ok(Some(Refusal {
                reason: ActuationRefusalReasonV1::DependencyClosureRequired,
                index: Some(*index),
                path: Some(format!("/operations/{index}")),
            }));
        }
    }
    Ok(None)
}

fn simulation_cursor_base(
    connection: &Connection,
    put_count: u64,
    preferred: u64,
) -> Result<u64, EngineError> {
    if preferred.checked_add(put_count).is_some_and(|value| value <= i64::MAX as u64) {
        return Ok(preferred);
    }
    let ceiling = i64::MAX
        .checked_sub(i64::try_from(put_count).map_err(|_| EngineError::Storage)?)
        .ok_or(EngineError::Storage)?;
    let highest: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(write_cursor),0) FROM (\
               SELECT write_cursor FROM canonical_nodes WHERE write_cursor <= ?1 \
               UNION ALL SELECT write_cursor FROM canonical_edges WHERE write_cursor <= ?1 \
               UNION ALL SELECT write_cursor FROM operational_mutations WHERE write_cursor <= ?1 \
               UNION ALL SELECT write_cursor FROM operational_state WHERE write_cursor <= ?1\
             )",
            [ceiling],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    u64::try_from(highest).map_err(|_| EngineError::Storage)
}

fn simulate_request(
    connection: &Connection,
    request: &ActuationBatchV1,
    base_cursor: u64,
    provenance_row_cap: u64,
) -> Result<Option<Refusal>, EngineError> {
    connection
        .execute_batch("SAVEPOINT fathomdb_actuation_validation")
        .map_err(|_| EngineError::Storage)?;
    let outcome = (|| {
        let mut next_cursor = base_cursor;
        let mut losses = Vec::new();
        for (index, operation) in request.operations.iter().enumerate() {
            match operation {
                ActuationOperationV1::PutCanonicalNode(node)
                | ActuationOperationV1::PutDerivedNode(node) => {
                    let expected_role =
                        matches!(operation, ActuationOperationV1::PutCanonicalNode(_));
                    if matches!(node.provenance.role, ProvenanceRole::Canonical) != expected_role {
                        return Ok(Some(Refusal {
                            reason: ActuationRefusalReasonV1::ProvenanceRoleMismatch,
                            index: Some(index),
                            path: Some(format!("/operations/{index}/record/provenance/role")),
                        }));
                    }
                    if let Some(logical_id) = &node.logical_id {
                        if let Some((revision, _, role)) =
                            current_revision_for_logical(connection, logical_id)?
                        {
                            if role == "canonical_source" {
                                losses.push((index, revision));
                            }
                        }
                    }
                    let write = PreparedWrite::ProvenancedNode(node.clone());
                    let plan = match validate_write(connection, &write) {
                        Ok(plan) => plan,
                        Err(error) => return map_domain_error(error, index, None).map(Some),
                    };
                    match apply_batch_in_transaction(
                        connection,
                        &[write],
                        &[plan],
                        next_cursor,
                        provenance_row_cap,
                        &[],
                    ) {
                        Ok(_) => next_cursor = next_cursor.saturating_add(1),
                        Err(CommitBatchError::Provenance(error)) => {
                            return map_domain_error(EngineError::Provenance(error), index, None)
                                .map(Some);
                        }
                        Err(CommitBatchError::Sql(_)) => return Err(EngineError::Storage),
                    }
                }
                ActuationOperationV1::RegisterSourceDependency(dependency) => {
                    let prospective = DependencyProspectiveState::default();
                    let validated = match validate_source_dependency_registration(
                        connection,
                        dependency.clone(),
                        &prospective,
                    ) {
                        Ok(validated) => validated,
                        Err(error) => {
                            let field = missing_dependency_field(connection, dependency).ok();
                            return map_domain_error(error, index, field).map(Some);
                        }
                    };
                    apply_validated_source_dependency(connection, &validated, 1)?;
                }
                ActuationOperationV1::TransitionLifecycle(lifecycle) => {
                    if lifecycle.to_state == LifecycleState::Deleted {
                        if let Some((revision, _, role)) =
                            current_revision_for_logical(connection, &lifecycle.logical_id)?
                        {
                            if role == "canonical_source" {
                                losses.push((index, revision));
                            }
                        }
                    }
                    if let Err(error) = apply_lifecycle(connection, lifecycle, index) {
                        return match error {
                            RefusalOrInfrastructure::Refusal(refusal) => Ok(Some(refusal)),
                            RefusalOrInfrastructure::Infrastructure(error) => Err(error),
                        };
                    }
                }
            }
        }
        closure_refusal_for_losses(connection, &losses)
    })();
    connection
        .execute_batch(
            "ROLLBACK TO fathomdb_actuation_validation; RELEASE fathomdb_actuation_validation",
        )
        .map_err(|_| EngineError::Storage)?;
    outcome
}

fn map_domain_error(
    error: EngineError,
    index: usize,
    missing_dependency_field: Option<&str>,
) -> Result<Refusal, EngineError> {
    let (reason, suffix) = match error {
        EngineError::WriteValidation | EngineError::SchemaValidation => {
            (ActuationRefusalReasonV1::WriteRefused, "/record")
        }
        EngineError::Provenance(ref provenance)
            if provenance.reason == ProvenanceErrorReason::RoleInvalid =>
        {
            (ActuationRefusalReasonV1::ProvenanceRoleMismatch, "/record/provenance/role")
        }
        EngineError::Provenance(ref provenance)
            if provenance.reason == ProvenanceErrorReason::SourceRevisionMissing =>
        {
            (ActuationRefusalReasonV1::ReferenceUnavailable, "/record/provenance/sourceRevisionId")
        }
        EngineError::Provenance(provenance) => {
            let suffix = format!("/record{}", provenance.field_path);
            return Ok(Refusal {
                reason: ActuationRefusalReasonV1::WriteRefused,
                index: Some(index),
                path: Some(format!("/operations/{index}{suffix}")),
            });
        }
        EngineError::Dependency(ref dependency)
            if dependency.reason == DependencyErrorReason::DependencyGenerationExhausted =>
        {
            (ActuationRefusalReasonV1::DependencyGenerationExhausted, "/dependency")
        }
        EngineError::Dependency(ref dependency)
            if dependency.reason == DependencyErrorReason::DependencyReferenceMissing =>
        {
            let field = missing_dependency_field.unwrap_or("derivedRevisionId");
            let suffix = format!("/dependency/{field}");
            return Ok(Refusal {
                reason: ActuationRefusalReasonV1::ReferenceUnavailable,
                index: Some(index),
                path: Some(format!("/operations/{index}{suffix}")),
            });
        }
        EngineError::Dependency(dependency) => {
            let suffix = if dependency.field_path.is_empty() {
                "/dependency".to_string()
            } else {
                format!("/dependency{}", dependency.field_path)
            };
            return Ok(Refusal {
                reason: ActuationRefusalReasonV1::DependencyRefused,
                index: Some(index),
                path: Some(format!("/operations/{index}{suffix}")),
            });
        }
        EngineError::IllegalTransition { .. } => {
            (ActuationRefusalReasonV1::LifecycleRefused, "/toState")
        }
        other => return Err(other),
    };
    Ok(Refusal { reason, index: Some(index), path: Some(format!("/operations/{index}{suffix}")) })
}

fn missing_dependency_field(
    connection: &Connection,
    dependency: &SourceDependencyRegistrationV1,
) -> Result<&'static str, EngineError> {
    let derived_exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions WHERE revision_id=?1)",
            [dependency.derived_revision_id.as_str()],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(if derived_exists { "sourceRevisionId" } else { "derivedRevisionId" })
}

fn apply_lifecycle(
    connection: &Connection,
    operation: &LifecycleActuationV1,
    index: usize,
) -> Result<String, RefusalOrInfrastructure> {
    let current = current_revision_for_logical(connection, &operation.logical_id)
        .map_err(RefusalOrInfrastructure::Infrastructure)?;
    let Some((revision, from_state, _)) = current else {
        return Err(RefusalOrInfrastructure::Refusal(Refusal {
            reason: ActuationRefusalReasonV1::LifecycleRefused,
            index: Some(index),
            path: Some(format!("/operations/{index}/expectedCurrentRevisionId")),
        }));
    };
    if revision != operation.expected_current_revision_id.as_str() {
        return Err(RefusalOrInfrastructure::Refusal(Refusal {
            reason: ActuationRefusalReasonV1::LifecycleRefused,
            index: Some(index),
            path: Some(format!("/operations/{index}/expectedCurrentRevisionId")),
        }));
    }
    if !is_legal_transition_move(from_state, operation.to_state) {
        return Err(RefusalOrInfrastructure::Refusal(Refusal {
            reason: ActuationRefusalReasonV1::LifecycleRefused,
            index: Some(index),
            path: Some(format!("/operations/{index}/toState")),
        }));
    }
    let (write_cursor, body): (i64, String) = connection
        .query_row(
            "SELECT write_cursor,body FROM canonical_nodes \
             WHERE logical_id=?1 AND superseded_at IS NULL",
            [&operation.logical_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| RefusalOrInfrastructure::Infrastructure(EngineError::Storage))?;
    if operation.to_state == LifecycleState::Active {
        validate_nested_projection_sources_for_body(connection, &body)
            .map_err(RefusalOrInfrastructure::Infrastructure)?;
    }
    let new_reason =
        if operation.to_state == LifecycleState::Active { None } else { operation.reason.clone() };
    connection
        .execute(
            "UPDATE canonical_nodes SET state=?1, reason=?2 \
             WHERE logical_id=?3 AND superseded_at IS NULL",
            params![operation.to_state.as_str(), new_reason, operation.logical_id],
        )
        .map_err(|_| RefusalOrInfrastructure::Infrastructure(EngineError::Storage))?;
    purge_row_projections_for_cursor_in(
        connection,
        write_cursor,
        &[ProjectionClass::Attribute, ProjectionClass::PropertyFts],
    )
    .map_err(|_| RefusalOrInfrastructure::Infrastructure(EngineError::Storage))?;
    if operation.to_state == LifecycleState::Active {
        project_node_attributes(connection, write_cursor, &body)
            .map_err(|_| RefusalOrInfrastructure::Infrastructure(EngineError::Storage))?;
        refresh_vector_attr_values_for_row(connection, write_cursor, &body)
            .map_err(|_| RefusalOrInfrastructure::Infrastructure(EngineError::Storage))?;
    }
    Ok(revision)
}

enum RefusalOrInfrastructure {
    Refusal(Refusal),
    Infrastructure(EngineError),
}

enum ActuationAttempt {
    New(ActuationReceiptV1),
    Replay(ActuationReceiptV1),
}

impl From<EngineError> for RefusalOrInfrastructure {
    fn from(error: EngineError) -> Self {
        Self::Infrastructure(error)
    }
}

impl Engine {
    /// Atomically apply a bounded, caller-decided semantic actuation batch.
    ///
    /// The Engine validates structure and lifecycle invariants but does not make
    /// semantic judgments. Exact operation-ID replay returns the stored terminal
    /// receipt without repeating domain work.
    ///
    /// # Errors
    ///
    /// Returns `Actuation` for an operation-ID conflict or erased tombstone,
    /// `Storage`/`Closing` for infrastructure failure, and a terminal refused
    /// receipt for admitted domain failure.
    pub fn actuate(&self, request: ActuationBatchV1) -> Result<ActuationReceiptV1, EngineError> {
        self.ensure_open()?;
        validate_request(&request)?;
        let digest = request_digest(&request);
        let initial = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            load_receipt(connection, &request.operation_id, &digest, Some(&request))
        };
        match initial {
            Ok(Some(receipt)) => return Ok(receipt),
            Ok(None) => {}
            Err(error) => {
                self.emit_event(lifecycle::Phase::Started, lifecycle::EventCategory::Writer, None);
                let code = error.stable_code();
                self.counters.record_error(code);
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Writer,
                    Some(code),
                );
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Error,
                    Some(code),
                );
                return Err(error);
            }
        }

        #[cfg(debug_assertions)]
        {
            let delay = self.actuation_after_initial_lookup_delay_ms.load(Ordering::SeqCst);
            if delay > 0 {
                std::thread::sleep(std::time::Duration::from_millis(delay));
            }
        }
        let mut admitted_started = None;
        let result = self.actuate_new(&request, &digest, &mut admitted_started);
        match result {
            Ok(ActuationAttempt::Replay(receipt)) => Ok(receipt),
            Ok(ActuationAttempt::New(receipt)) => {
                self.detect_slow(
                    admitted_started.expect("new actuation has an admission timestamp"),
                    lifecycle::EventCategory::Writer,
                );
                let rows = if receipt.outcome == ActuationOutcomeV1::Committed {
                    request
                        .operations
                        .iter()
                        .filter(|operation| {
                            matches!(
                                operation,
                                ActuationOperationV1::PutCanonicalNode(_)
                                    | ActuationOperationV1::PutDerivedNode(_)
                            )
                        })
                        .count() as u64
                } else {
                    0
                };
                self.counters.record_write(rows);
                self.emit_event(lifecycle::Phase::Finished, lifecycle::EventCategory::Writer, None);
                Ok(receipt)
            }
            Err(error) => {
                if let Some(started) = admitted_started {
                    self.detect_slow(started, lifecycle::EventCategory::Writer);
                } else {
                    self.emit_event(
                        lifecycle::Phase::Started,
                        lifecycle::EventCategory::Writer,
                        None,
                    );
                }
                let code = error.stable_code();
                self.counters.record_error(code);
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Writer,
                    Some(code),
                );
                self.emit_event(
                    lifecycle::Phase::Failed,
                    lifecycle::EventCategory::Error,
                    Some(code),
                );
                Err(error)
            }
        }
    }

    fn actuate_new(
        &self,
        request: &ActuationBatchV1,
        digest: &str,
        admitted_started: &mut Option<Instant>,
    ) -> Result<ActuationAttempt, EngineError> {
        if request
            .operations
            .iter()
            .any(|operation| matches!(operation, ActuationOperationV1::TransitionLifecycle(_)))
        {
            self.drain_for_non_embedding_mutation()?;
        }
        let mut refs = collect_source_refs(request);
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        if let Some(receipt) = load_receipt(&tx, &request.operation_id, digest, Some(request))? {
            tx.commit().map_err(|_| EngineError::Storage)?;
            return Ok(ActuationAttempt::Replay(receipt));
        }
        self.emit_event(lifecycle::Phase::Started, lifecycle::EventCategory::Writer, None);
        *admitted_started = Some(Instant::now());
        enrich_resolved_refs(&tx, request, &mut refs)?;

        let base_cursor = load_next_cursor(&tx);
        if request.expected_write_boundary.is_some_and(|expected| expected != base_cursor) {
            let receipt = receipt_for_refusal(
                request,
                digest.to_string(),
                Refusal {
                    reason: ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch,
                    index: None,
                    path: Some("/expectedWriteBoundary".into()),
                },
            );
            store_receipt(&tx, &receipt, request.operations.len())?;
            store_source_refs(&tx, &request.operation_id, &refs, request.operations.len())?;
            #[cfg(debug_assertions)]
            if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
                return Err(EngineError::Storage);
            }
            tx.commit().map_err(|_| EngineError::Storage)?;
            return Ok(ActuationAttempt::New(receipt));
        }
        let put_count = request
            .operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation,
                    ActuationOperationV1::PutCanonicalNode(_)
                        | ActuationOperationV1::PutDerivedNode(_)
                )
            })
            .count() as u64;
        let cursor_available =
            base_cursor.checked_add(put_count).is_some_and(|value| value <= i64::MAX as u64);
        let validation_base = simulation_cursor_base(&tx, put_count, base_cursor)?;
        if let Some(refusal) = simulate_request(
            &tx,
            request,
            validation_base,
            self.provenance_row_cap.load(Ordering::Relaxed),
        )? {
            let receipt = receipt_for_refusal(request, digest.to_string(), refusal);
            store_receipt(&tx, &receipt, request.operations.len())?;
            store_source_refs(&tx, &request.operation_id, &refs, request.operations.len())?;
            #[cfg(debug_assertions)]
            if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
                return Err(EngineError::Storage);
            }
            tx.commit().map_err(|_| EngineError::Storage)?;
            return Ok(ActuationAttempt::New(receipt));
        }
        if !cursor_available {
            let index = request
                .operations
                .iter()
                .position(|operation| {
                    matches!(
                        operation,
                        ActuationOperationV1::PutCanonicalNode(_)
                            | ActuationOperationV1::PutDerivedNode(_)
                    )
                })
                .unwrap_or(0);
            let receipt = receipt_for_refusal(
                request,
                digest.to_string(),
                Refusal {
                    reason: ActuationRefusalReasonV1::WriteCursorExhausted,
                    index: Some(index),
                    path: Some(format!("/operations/{index}/record")),
                },
            );
            store_receipt(&tx, &receipt, request.operations.len())?;
            store_source_refs(&tx, &request.operation_id, &refs, request.operations.len())?;
            #[cfg(debug_assertions)]
            if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
                return Err(EngineError::Storage);
            }
            tx.commit().map_err(|_| EngineError::Storage)?;
            return Ok(ActuationAttempt::New(receipt));
        }
        let batch_writes = request
            .operations
            .iter()
            .filter_map(|operation| match operation {
                ActuationOperationV1::PutCanonicalNode(node)
                | ActuationOperationV1::PutDerivedNode(node) => {
                    Some(PreparedWrite::ProvenancedNode(node.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let vector_kinds_to_enrol =
            self.batch_vector_kinds_needing_enrolment(&tx, &batch_writes)?;

        tx.execute_batch("SAVEPOINT fathomdb_actuation_domain")
            .map_err(|_| EngineError::Storage)?;
        let mut next_cursor = base_cursor;
        let mut dependency_generation = None;
        let mut affected = Vec::new();
        let mut affected_set = BTreeSet::new();
        let mut pending = Vec::new();
        let mut refusal = None;
        let mut vector_enrolment_applied = false;
        let mut unstranded = false;

        for (index, operation) in request.operations.iter().enumerate() {
            let outcome: Result<(), RefusalOrInfrastructure> = (|| match operation {
                ActuationOperationV1::PutCanonicalNode(node)
                | ActuationOperationV1::PutDerivedNode(node) => {
                    let expected_role =
                        matches!(operation, ActuationOperationV1::PutCanonicalNode(_));
                    let role_matches =
                        matches!(node.provenance.role, ProvenanceRole::Canonical) == expected_role;
                    if !role_matches {
                        Err(RefusalOrInfrastructure::Refusal(Refusal {
                            reason: ActuationRefusalReasonV1::ProvenanceRoleMismatch,
                            index: Some(index),
                            path: Some(format!("/operations/{index}/record/provenance/role")),
                        }))
                    } else {
                        let previous = node
                            .logical_id
                            .as_deref()
                            .map(|logical| current_revision_for_logical(&tx, logical))
                            .transpose()
                            .map_err(RefusalOrInfrastructure::Infrastructure)?
                            .flatten()
                            .map(|(revision, _, _)| revision);
                        let write = PreparedWrite::ProvenancedNode(node.clone());
                        let plan = validate_write(&tx, &write).map_err(|error| {
                            map_domain_error(error, index, None)
                                .map(RefusalOrInfrastructure::Refusal)
                                .unwrap_or_else(RefusalOrInfrastructure::Infrastructure)
                        })?;
                        let enrol = if vector_enrolment_applied {
                            &[]
                        } else {
                            vector_enrolment_applied = true;
                            vector_kinds_to_enrol.as_slice()
                        };
                        match apply_batch_in_transaction(
                            &tx,
                            &[write],
                            &[plan],
                            next_cursor,
                            self.provenance_row_cap.load(Ordering::Relaxed),
                            enrol,
                        ) {
                            Ok((_, repaired)) => {
                                unstranded |= repaired;
                                next_cursor += 1;
                                let new_revision =
                                    node.provenance.artifact_revision_id.as_str().to_string();
                                if affected_set.insert(new_revision.clone()) {
                                    affected.push(new_revision);
                                }
                                if let Some(previous) = previous {
                                    if affected_set.insert(previous.clone()) {
                                        affected.push(previous);
                                    }
                                }
                                let terminal: bool = tx
                                    .query_row(
                                        "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_terminal \
                                         WHERE write_cursor=?1)",
                                        [next_cursor],
                                        |row| row.get(0),
                                    )
                                    .map_err(|_| {
                                        RefusalOrInfrastructure::Infrastructure(EngineError::Storage)
                                    })?;
                                if !terminal {
                                    pending.push(next_cursor);
                                }
                                Ok(())
                            }
                            Err(CommitBatchError::Provenance(error)) => {
                                Err(RefusalOrInfrastructure::Refusal(map_domain_error(
                                    EngineError::Provenance(error),
                                    index,
                                    None,
                                )?))
                            }
                            Err(CommitBatchError::Sql(_)) => {
                                Err(RefusalOrInfrastructure::Infrastructure(EngineError::Storage))
                            }
                        }
                    }
                }
                ActuationOperationV1::RegisterSourceDependency(dependency) => {
                    let prospective = DependencyProspectiveState::default();
                    let validated = validate_source_dependency_registration(
                        &tx,
                        dependency.clone(),
                        &prospective,
                    )
                    .map_err(|error| {
                        let field = missing_dependency_field(&tx, dependency).ok();
                        map_domain_error(error, index, field)
                            .map(RefusalOrInfrastructure::Refusal)
                            .unwrap_or_else(RefusalOrInfrastructure::Infrastructure)
                    })?;
                    match validated {
                        ValidatedSourceDependencyRegistration::Replay(_)
                        | ValidatedSourceDependencyRegistration::ProspectiveReplay => Ok(()),
                        ValidatedSourceDependencyRegistration::Insert(_) => {
                            let generation = match dependency_generation {
                                Some(generation) => generation,
                                None => match reserve_dependency_generation(&tx) {
                                    Ok(generation) => {
                                        dependency_generation = Some(generation);
                                        generation
                                    }
                                    Err(error) => {
                                        return Err(match map_domain_error(error, index, None) {
                                            Ok(mapped) => RefusalOrInfrastructure::Refusal(mapped),
                                            Err(error) => {
                                                RefusalOrInfrastructure::Infrastructure(error)
                                            }
                                        });
                                    }
                                },
                            };
                            apply_validated_source_dependency(&tx, &validated, generation)
                                .map_err(RefusalOrInfrastructure::Infrastructure)?;
                            Ok(())
                        }
                    }
                }
                ActuationOperationV1::TransitionLifecycle(lifecycle) => {
                    match apply_lifecycle(&tx, lifecycle, index) {
                        Ok(revision) => {
                            if affected_set.insert(revision.clone()) {
                                affected.push(revision);
                            }
                            Ok(())
                        }
                        Err(error) => Err(error),
                    }
                }
            })();
            if let Err(error) = outcome {
                match error {
                    RefusalOrInfrastructure::Refusal(value) => {
                        refusal = Some(value);
                        break;
                    }
                    RefusalOrInfrastructure::Infrastructure(error) => return Err(error),
                }
            }
            #[cfg(debug_assertions)]
            if self.actuation_failure_after_operation.load(Ordering::SeqCst) == index {
                self.actuation_failure_after_operation.store(usize::MAX, Ordering::SeqCst);
                return Err(EngineError::Storage);
            }
        }

        if let Some(refusal) = refusal {
            tx.execute_batch(
                "ROLLBACK TO fathomdb_actuation_domain; RELEASE fathomdb_actuation_domain",
            )
            .map_err(|_| EngineError::Storage)?;
            let receipt = receipt_for_refusal(request, digest.to_string(), refusal);
            store_receipt(&tx, &receipt, request.operations.len())?;
            store_source_refs(&tx, &request.operation_id, &refs, request.operations.len())?;
            #[cfg(debug_assertions)]
            if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
                return Err(EngineError::Storage);
            }
            tx.commit().map_err(|_| EngineError::Storage)?;
            return Ok(ActuationAttempt::New(receipt));
        }
        if let Some(generation) = dependency_generation {
            store_dependency_generation(&tx, generation)?;
        }
        tx.execute_batch("RELEASE fathomdb_actuation_domain").map_err(|_| EngineError::Storage)?;
        if affected.len() > MAX_AFFECTED_REVISIONS {
            return Err(EngineError::Storage);
        }
        for revision in &affected {
            refs.insert(("artifact_revision_id", revision.clone()));
        }
        let receipt = ActuationReceiptV1 {
            schema_version: 1,
            operation_id: request.operation_id.clone(),
            request_sha256: digest.to_string(),
            outcome: ActuationOutcomeV1::Committed,
            refused_operation_index: None,
            refused_field_path: None,
            reason_codes: Vec::new(),
            affected_revision_ids: affected,
            resulting_write_boundary: Some(next_cursor),
            resulting_dependency_generation: dependency_generation,
            pending_projection_write_cursors: pending,
            closure_operation_ids: Vec::new(),
        };
        store_receipt(&tx, &receipt, request.operations.len())?;
        store_source_refs(&tx, &request.operation_id, &refs, request.operations.len())?;
        #[cfg(debug_assertions)]
        if self.force_next_commit_failure.swap(false, Ordering::SeqCst) {
            return Err(EngineError::Storage);
        }
        tx.commit().map_err(|_| EngineError::Storage)?;
        self.next_cursor.store(next_cursor, Ordering::SeqCst);
        if !receipt.pending_projection_write_cursors.is_empty() || unstranded {
            self.projection_runtime.notify_new_work();
        }
        Ok(ActuationAttempt::New(receipt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn canonical(revision: &str) -> ProvenancedNodeV1 {
        ProvenancedNodeV1 {
            kind: "doc".into(),
            body: "body".into(),
            source_id: SourceId::new("source-a").unwrap(),
            logical_id: Some(revision.into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new(revision).unwrap(),
                SourceVersionId::new(format!("version-{revision}")).unwrap(),
            ),
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn request_digest_is_deterministic_and_operation_order_sensitive(
            left in "[a-z][a-z0-9]{0,15}",
            right in "[a-z][a-z0-9]{0,15}",
        ) {
            prop_assume!(left != right);
            let forward = ActuationBatchV1::new(
                "property-operation",
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical(&left)),
                    ActuationOperationV1::PutCanonicalNode(canonical(&right)),
                ],
            ).unwrap();
            let reverse = ActuationBatchV1::new(
                "property-operation",
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical(&right)),
                    ActuationOperationV1::PutCanonicalNode(canonical(&left)),
                ],
            ).unwrap();

            prop_assert_eq!(request_digest(&forward), request_digest(&forward));
            prop_assert_ne!(request_digest(&forward), request_digest(&reverse));
        }

        #[test]
        fn persisted_receipt_round_trip_and_replay_are_equivalent(
            suffix in "[a-z][a-z0-9]{0,15}",
            body in ".{0,64}",
        ) {
            let dir = tempfile::TempDir::new().unwrap();
            let path = dir.path().join(format!("property{}", fathomdb_schema::SQLITE_SUFFIX));
            let revision = format!("revision-{suffix}");
            let mut node = canonical(&revision);
            node.body = body;
            let request = ActuationBatchV1::new(
                format!("operation-{suffix}"),
                vec![ActuationOperationV1::PutCanonicalNode(node)],
            ).unwrap();
            let first = {
                let opened = Engine::open(&path).unwrap();
                opened.engine.actuate(request.clone()).unwrap()
            };
            let reopened = Engine::open(&path).unwrap();
            prop_assert_eq!(reopened.engine.actuate(request).unwrap(), first);
        }

        #[test]
        fn committed_receipt_collection_formulas_hold_for_bounded_batches(
            count in 1_usize..16,
            suffix in "[a-z][a-z0-9]{0,8}",
        ) {
            let dir = tempfile::TempDir::new().unwrap();
            let path = dir.path().join(format!("collections{}", fathomdb_schema::SQLITE_SUFFIX));
            let revisions = (0..count)
                .map(|index| format!("revision-{suffix}-{index}"))
                .collect::<Vec<_>>();
            let operations = revisions
                .iter()
                .map(|revision| ActuationOperationV1::PutCanonicalNode(canonical(revision)))
                .collect();
            let request = ActuationBatchV1::new(
                format!("collections-{suffix}"),
                operations,
            ).unwrap();
            let opened = Engine::open(&path).unwrap();
            let receipt = opened.engine.actuate(request.clone()).unwrap();
            prop_assert_eq!(&receipt.affected_revision_ids, &revisions);
            prop_assert_eq!(receipt.resulting_write_boundary, Some(count as u64));
            prop_assert!(receipt.pending_projection_write_cursors.len() <= count);
            prop_assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
        }
    }

    #[test]
    fn refusal_path_grammar_is_exact_and_complete() {
        let admitted = [
            (ActuationRefusalReasonV1::WriteRefused, "/operations/0/record"),
            (
                ActuationRefusalReasonV1::WriteRefused,
                "/operations/0/record/provenance/sourceVersionId",
            ),
            (
                ActuationRefusalReasonV1::WriteRefused,
                "/operations/0/record/provenance/sourceRevisionId",
            ),
            (
                ActuationRefusalReasonV1::WriteRefused,
                "/operations/0/record/provenance/sourceLocator",
            ),
            (
                ActuationRefusalReasonV1::WriteRefused,
                "/operations/0/record/provenance/canonicalSourceHash",
            ),
            (ActuationRefusalReasonV1::WriteCursorExhausted, "/operations/0/record"),
            (
                ActuationRefusalReasonV1::ProvenanceRoleMismatch,
                "/operations/0/record/provenance/role",
            ),
            (
                ActuationRefusalReasonV1::ReferenceUnavailable,
                "/operations/0/record/provenance/sourceRevisionId",
            ),
            (
                ActuationRefusalReasonV1::ReferenceUnavailable,
                "/operations/0/dependency/sourceRevisionId",
            ),
            (
                ActuationRefusalReasonV1::ReferenceUnavailable,
                "/operations/0/dependency/derivedRevisionId",
            ),
            (ActuationRefusalReasonV1::DependencyRefused, "/operations/0/dependency"),
            (ActuationRefusalReasonV1::DependencyGenerationExhausted, "/operations/0/dependency"),
            (ActuationRefusalReasonV1::LifecycleRefused, "/operations/0/expectedCurrentRevisionId"),
            (ActuationRefusalReasonV1::LifecycleRefused, "/operations/0/toState"),
            (ActuationRefusalReasonV1::DependencyClosureRequired, "/operations/0"),
        ];
        for (reason, path) in admitted {
            assert_eq!(validate_refusal_shape(reason, Some(0), Some(path), 1), Ok(()));
        }
        assert_eq!(
            validate_refusal_shape(
                ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch,
                None,
                Some("/expectedWriteBoundary"),
                1,
            ),
            Ok(())
        );

        let rejected = [
            (ActuationRefusalReasonV1::WriteRefused, "/operations/0/record/provenance"),
            (
                ActuationRefusalReasonV1::ReferenceUnavailable,
                "/operations/0/unowned/sourceRevisionId",
            ),
            (ActuationRefusalReasonV1::LifecycleRefused, "/operations/0/logicalId"),
            (ActuationRefusalReasonV1::DependencyRefused, "/operations/0/dependency/id"),
        ];
        for (reason, path) in rejected {
            assert!(matches!(
                validate_refusal_shape(reason, Some(0), Some(path), 1),
                Err(EngineError::Storage)
            ));
        }
    }

    #[test]
    fn lifecycle_digest_encodes_reason_exactly_once() {
        fn push_string(bytes: &mut Vec<u8>, value: &str) {
            bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }

        let lifecycle = LifecycleActuationV1::new(
            "subject",
            ArtifactRevisionId::new("revision-r1").unwrap(),
            LifecycleState::Deleted,
            Some("expired".into()),
        )
        .unwrap();
        let request = ActuationBatchV1::new(
            "digest-operation",
            vec![ActuationOperationV1::TransitionLifecycle(lifecycle)],
        )
        .unwrap();

        let mut bytes = b"fathomdb.actuation.v1\0".to_vec();
        bytes.push(0x01);
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.push(0x02);
        push_string(&mut bytes, "digest-operation");
        bytes.extend_from_slice(&[0x03, 0x00, 0x04, 0x00, 0x05]);
        bytes.extend_from_slice(&1_u32.to_be_bytes());
        bytes.extend_from_slice(&[0x13, 0x50]);
        push_string(&mut bytes, "subject");
        bytes.push(0x51);
        push_string(&mut bytes, "revision-r1");
        bytes.extend_from_slice(&[0x52, 0x01, 0x53, 0x01]);
        push_string(&mut bytes, "expired");
        let expected = hex_encode(&Sha256::digest(bytes));

        assert_eq!(request_digest(&request), expected);
    }
}

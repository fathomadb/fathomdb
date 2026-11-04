"""Typed physical-erasure intent and status; acceptance is never deletion."""
from __future__ import annotations

from typing import Literal, TypedDict

PhysicalErasurePolicy = Literal["autonomous", "explicit_maintenance"]
ErasureLogicalState = Literal["unchanged_by_this_request", "removed"]
ErasureServiceState = Literal["normal_with_queued_restrictions", "maintenance_owned", "reads_allowed_writes_fenced", "closed_or_unavailable"]
ErasurePhase = Literal["queued", "executing", "sanitizing", "finalizing", "complete", "withdrawn_before_removal"]
ErasureBlocker = Literal["resource_envelope_changed", "original_sink_unavailable", "wal_checkpoint", "owned_readers"]
WithdrawResult = Literal["withdrawn_before_removal", "already_activated", "unknown"]


class SourceAllAtCut(TypedDict):
    """All retained roots of one source at acceptance's committed cut."""
    kind: Literal["source_all_at_cut"]
    source_id: str


class DeletedLogicalAllAtCut(TypedDict):
    """All retained versions and touching edges of a deleted logical node."""
    kind: Literal["deleted_logical_all_at_cut"]
    logical_id: str


PhysicalErasureSelector = SourceAllAtCut | DeletedLogicalAllAtCut


class ErasureStatus(TypedDict):
    """Identity-free keyed observation; counts imply no completion deadline."""
    phase: ErasurePhase
    logical_state: ErasureLogicalState
    blocker: ErasureBlocker | None
    service_state: ErasureServiceState
    target_count: int


class AcceptedQueue(TypedDict):
    """Durable intent; reads retain their existing eligibility."""
    outcome: Literal["accepted"]
    ticket: str
    logical_state: ErasureLogicalState


class NeedsMaintenanceAdmission(TypedDict):
    """No acceptance; perform heavy preflight inside an exclusive window."""
    outcome: Literal["needs_maintenance_admission"]


QueueResult = AcceptedQueue | NeedsMaintenanceAdmission


class CompleteCleanup(TypedDict):
    """All registered physical obligations discharged before this receipt."""
    outcome: Literal["complete"]
    receipt: ErasureStatus


class IncompleteCleanup(TypedDict):
    """Retained owed work; committed logical removal cannot be rolled back."""
    outcome: Literal["incomplete"]
    status: ErasureStatus


CleanupResult = CompleteCleanup | IncompleteCleanup

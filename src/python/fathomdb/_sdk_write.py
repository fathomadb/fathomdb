"""Private write operations for the Python Engine facade."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal, cast

from fathomdb._fathomdb import ConsolidateReceipt, EraseReport, IngestWithExtractorReceipt
from fathomdb._fathomdb import erase_source as _native_erase_source
from fathomdb._fathomdb import purge as _native_purge
from fathomdb._fathomdb import transition as _native_transition
from fathomdb.types import (
    ActuationBatchV1,
    ActuationReceiptV1,
    ClosureLookupV1,
    ClosureProofV1,
    ClosureStatusV1,
    DependencyDerivedLookupV1,
    DependencyListV1,
    DependencySourceLookupV1,
    SourceDependencyRegistrationV1,
    SourceDependencyV1,
    WriteReceipt,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


def write(self: Engine, batch: list[Any] | None = None) -> WriteReceipt:
    receipt = self._native.write(batch or [])
    return WriteReceipt(
        cursor=receipt.cursor,
        row_cursors=tuple(receipt.row_cursors),
        dangling_edge_endpoints=receipt.dangling_edge_endpoints,
    )


def register_source_dependency(
    self: Engine, request: SourceDependencyRegistrationV1
) -> SourceDependencyV1:
    value = self._native.register_source_dependency(request)
    return SourceDependencyV1(
        schema_version=value.schema_version,
        dependency_id=value.dependency_id,
        source_revision_id=value.source_revision_id,
        derived_revision_id=value.derived_revision_id,
        registered_dependency_generation=value.registered_dependency_generation,
    )


def actuate(self: Engine, request: ActuationBatchV1) -> ActuationReceiptV1:
    value = self._native.actuate(request)
    return ActuationReceiptV1(
        schema_version=value.schema_version,
        operation_id=value.operation_id,
        request_sha256=value.request_sha256,
        outcome=cast(
            Literal["committed", "committed_closure_pending", "refused"],
            value.outcome,
        ),
        refused_operation_index=value.refused_operation_index,
        refused_field_path=value.refused_field_path,
        reason_codes=tuple(value.reason_codes),
        affected_revision_ids=tuple(value.affected_revision_ids),
        resulting_write_boundary=value.resulting_write_boundary,
        resulting_dependency_generation=value.resulting_dependency_generation,
        pending_projection_write_cursors=tuple(value.pending_projection_write_cursors),
        projection_generation_id=value.projection_generation_id,
        closure_operation_ids=tuple(value.closure_operation_ids),
    )


def dependencies_for_source(self: Engine, request: DependencySourceLookupV1) -> DependencyListV1:
    value = self._native.dependencies_for_source(request)
    return DependencyListV1(
        schema_version=value.schema_version,
        items=tuple(
            SourceDependencyV1(
                schema_version=item.schema_version,
                dependency_id=item.dependency_id,
                source_revision_id=item.source_revision_id,
                derived_revision_id=item.derived_revision_id,
                registered_dependency_generation=item.registered_dependency_generation,
            )
            for item in value.items
        ),
    )


def dependency_for_derived(
    self: Engine, request: DependencyDerivedLookupV1
) -> SourceDependencyV1 | None:
    value = self._native.dependency_for_derived(request)
    if value is None:
        return None
    return SourceDependencyV1(
        schema_version=value.schema_version,
        dependency_id=value.dependency_id,
        source_revision_id=value.source_revision_id,
        derived_revision_id=value.derived_revision_id,
        registered_dependency_generation=value.registered_dependency_generation,
    )


def read_dependency_closure(self: Engine, request: ClosureLookupV1) -> ClosureStatusV1 | None:
    value = self._native.read_dependency_closure(request)
    if value is None:
        return None
    root = (
        {"type": "source_revision", "source_revision_id": value.source_revision_id}
        if value.root_type == "source_revision"
        else {"type": "source_bucket", "source_id": value.source_id}
    )
    proof = value.proof
    return ClosureStatusV1(
        schema_version=value.schema_version,
        closure_operation_id=value.closure_operation_id,
        root=root,
        cause=value.cause,
        phase=value.phase,
        effective_at_epoch_s=value.effective_at_epoch_s,
        admitted_write_boundary=value.admitted_write_boundary,
        admitted_dependency_generation=value.admitted_dependency_generation,
        affected_count=value.affected_count,
        blocker_code=value.blocker_code,
        proof=None
        if proof is None
        else ClosureProofV1(
            schema_version=proof.schema_version,
            proof_write_boundary=proof.proof_write_boundary,
            current_active_dependent_nodes=proof.current_active_dependent_nodes,
            current_derived_edges=proof.current_derived_edges,
            view_eligible_dependents=proof.view_eligible_dependents,
            ownerless_projection_rows=proof.ownerless_projection_rows,
            post_admission_registrations=proof.post_admission_registrations,
            remaining_dependency_rows=proof.remaining_dependency_rows,
            remaining_canonical_rows=proof.remaining_canonical_rows,
            remaining_projection_rows=proof.remaining_projection_rows,
            remaining_receipt_reference_rows=proof.remaining_receipt_reference_rows,
        ),
    )


def transition(self: Engine, logical_id: str, to_state: str, reason: str | None = None) -> None:
    _native_transition(self._native, logical_id, to_state, reason)


def purge(self: Engine, logical_id: str) -> None:
    _native_purge(self._native, logical_id)


def erase_source(self: Engine, source_id: str) -> EraseReport:
    return _native_erase_source(self._native, source_id)


def ingest_with_extractor(
    self: Engine,
    cmd: list[str],
    documents: list[dict[str, str]],
) -> IngestWithExtractorReceipt:

    return self._native.ingest_with_extractor(cmd, documents)


def consolidate_with_provider(
    self: Engine,
    cmd: list[str],
    axes: list[dict[str, str]],
) -> ConsolidateReceipt:

    return self._native.consolidate_with_provider(cmd, axes)

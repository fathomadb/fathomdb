"""TC-140 — the native stub exposes runtime observability and typed errors."""

from fathomdb import _fathomdb
from fathomdb._fathomdb import (
    CudaAllocatorReport,
    CudaDeviceInfo,
    Engine,
    ErasureIncompleteError,
    IllegalTransitionError,
    NotLifecycleAddressableError,
    OpenReport,
    ProjectionDestructiveError,
    VectorEquivalenceMismatchError,
)


def test_native_stub_exposes_runtime_members() -> None:
    """Keep the hand-maintained native stub aligned with its public runtime API."""

    def verify(engine: Engine, report: OpenReport) -> None:
        dense_disabled: bool = report.dense_disabled
        dense_disabled_reason: str | None = report.dense_disabled_reason
        device_resolution = report.embedder_device_resolution
        # 0.8.23 Slice 80.6 (D-80.6-6) — the witness is part of the native
        # runtime surface, so the hand-maintained stub must declare it.
        gpu_allocation_witness = report.embedder_gpu_allocation_witness
        engine_dense_disabled: bool = engine.dense_disabled()
        engine_dense_disabled_reason: str | None = engine.dense_disabled_reason()
        refusal_count: int = engine.vector_equivalence_refusal_count()
        mismatch: VectorEquivalenceMismatchError = VectorEquivalenceMismatchError(
            "mismatch", reason="probe"
        )
        transition: IllegalTransitionError = IllegalTransitionError(
            "illegal transition",
            from_state="active",
            to_state="deleted",
            legal=["deleted"],
        )
        addressable: NotLifecycleAddressableError = NotLifecycleAddressableError(
            "not lifecycle addressable", id_space="h"
        )
        incomplete: ErasureIncompleteError = ErasureIncompleteError(
            "checkpoint blocked", stage="checkpoint", detail="busy"
        )
        destructive: ProjectionDestructiveError = ProjectionDestructiveError(
            "destructive projection", name="facts", delta="roles"
        )

        _ = (
            dense_disabled,
            dense_disabled_reason,
            device_resolution,
            gpu_allocation_witness,
            engine_dense_disabled,
            engine_dense_disabled_reason,
            refusal_count,
            mismatch,
            transition,
            addressable,
            incomplete,
            destructive,
        )

    # `verify` is intentionally type-checked, not invoked: it needs no database.
    assert callable(verify)


def test_native_stub_declares_the_cuda_allocator_report() -> None:
    """0.8.28 Slice 30 (AC30-06): the native CUDA facts carry the allocator report."""

    def verify(info: CudaDeviceInfo) -> None:
        report: CudaAllocatorReport | None = info.cuda_allocator
        if report is not None:
            path: str | None = report.path
            reason: str = report.reason
            pool_max_size_bytes: int | None = report.pool_max_size_bytes
            release_threshold: str | None = report.release_threshold
            module_load_init: str = report.module_load_init
            _ = (path, reason, pool_max_size_bytes, release_threshold, module_load_init)

    assert callable(verify)
    assert _fathomdb.CudaAllocatorReport.__name__ == "CudaAllocatorReport"

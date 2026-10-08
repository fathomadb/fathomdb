"""Private open operations for the Python Engine facade."""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, Literal, NoReturn, cast

from fathomdb.config import EngineConfig
from fathomdb.errors import (
    FrozenReadError,
)
from fathomdb.types import (
    CudaAllocatorReport,
    CudaDeviceInfo,
    CudaVisibleDevice,
    DeviceResolution,
    EffectiveEmbedDevice,
    EmbedderIdentity,
    FrozenReadContextV1,
    GpuAllocationWitness,
    MigrationStepReport,
    OpenReport,
    ReadContextV1,
    ReadView,
    SearchFilter,
)

if TYPE_CHECKING:
    from fathomdb.engine import Engine


_KWARG_FIELDS = {
    "embedder_pool_size",
    "scheduler_runtime_threads",
    "provenance_row_cap",
    "embedder_call_timeout_ms",
    "slow_threshold_ms",
}


_ENGINE_CONFIG_RANGES = {
    "embedder_pool_size": (1, 64),
    "scheduler_runtime_threads": (1, 64),
    "provenance_row_cap": (0, 2**53 - 1),
    "embedder_call_timeout_ms": (1, 2**32 - 1),
    "slow_threshold_ms": (0, 2**53 - 1),
}


def _native_engine_config(config: EngineConfig) -> dict[str, int | None]:
    if not isinstance(config, EngineConfig):
        raise TypeError("config must be an EngineConfig")
    native: dict[str, int | None] = {}
    for name, (minimum, maximum) in _ENGINE_CONFIG_RANGES.items():
        value = getattr(config, name)
        if value is not None:
            if isinstance(value, bool) or not isinstance(value, int):
                raise TypeError(f"{name} must be an integer")
            if not minimum <= value <= maximum:
                raise ValueError(f"{name} must be in {minimum}..={maximum}")
        native[name] = value
    return native


def _frozen_trace_error(reason: str, path: str) -> NoReturn:
    raise FrozenReadError(f"{reason} at {path}", reason=reason, field_path=path)


def _validate_frozen_trace_keys(value: Any, allowed: set[str], path: str) -> None:
    unknown = sorted(set(vars(value)) - allowed)
    if unknown:
        escaped = unknown[0].replace("~", "~0").replace("/", "~1")
        _frozen_trace_error("context_invalid", f"{path}/{escaped}")


def _validate_frozen_trace_context(value: Any) -> FrozenReadContextV1:
    if not isinstance(value, FrozenReadContextV1):
        _frozen_trace_error("context_invalid", "/context")
    _validate_frozen_trace_keys(
        value,
        {"schema_version", "effective_valid_at", "context", "token"},
        "/context",
    )
    if (
        not isinstance(value.schema_version, int)
        or isinstance(value.schema_version, bool)
        or value.schema_version != 1
    ):
        _frozen_trace_error("unsupported_schema_version", "/context/schemaVersion")
    if (
        not isinstance(value.effective_valid_at, int)
        or isinstance(value.effective_valid_at, bool)
        or not -(2**63) <= value.effective_valid_at < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/effectiveValidAt")
    if not isinstance(value.token, str):
        _frozen_trace_error("token_malformed", "/context/token")
    if len(value.token.encode("utf-8")) > 1024:
        _frozen_trace_error("token_too_large", "/context/token")
    context = value.context
    if not isinstance(context, ReadContextV1):
        _frozen_trace_error("context_invalid", "/context/context")
    _validate_frozen_trace_keys(
        context,
        {"schema_version", "view", "eligibility"},
        "/context/context",
    )
    if (
        not isinstance(context.schema_version, int)
        or isinstance(context.schema_version, bool)
        or context.schema_version != 1
    ):
        _frozen_trace_error("unsupported_schema_version", "/context/context/schemaVersion")
    view = context.view
    if not isinstance(view, ReadView):
        _frozen_trace_error("context_invalid", "/context/context/view")
    _validate_frozen_trace_keys(
        view,
        {
            "include_superseded",
            "include_inactive",
            "include_out_of_window",
            "valid_as_of",
        },
        "/context/context/view",
    )
    for name, path in (
        ("include_superseded", "includeSuperseded"),
        ("include_inactive", "includeInactive"),
        ("include_out_of_window", "includeOutOfWindow"),
    ):
        if not isinstance(getattr(view, name, None), bool):
            _frozen_trace_error("context_invalid", f"/context/context/view/{path}")
    if view.valid_as_of is not None and (
        not isinstance(view.valid_as_of, int)
        or isinstance(view.valid_as_of, bool)
        or not -(2**63) <= view.valid_as_of < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/context/view/validAsOf")
    eligibility = context.eligibility
    if not isinstance(eligibility, SearchFilter):
        _frozen_trace_error("context_invalid", "/context/context/eligibility")
    _validate_frozen_trace_keys(
        eligibility,
        {"source_type", "kind", "created_after", "status", "attributes"},
        "/context/context/eligibility",
    )
    for name, path in (
        ("source_type", "sourceType"),
        ("kind", "kind"),
        ("status", "status"),
    ):
        candidate = getattr(eligibility, name, None)
        if candidate is not None and not isinstance(candidate, str):
            _frozen_trace_error("context_invalid", f"/context/context/eligibility/{path}")
    if eligibility.created_after is not None and (
        not isinstance(eligibility.created_after, int)
        or isinstance(eligibility.created_after, bool)
        or not -(2**63) <= eligibility.created_after < 2**63
    ):
        _frozen_trace_error("context_invalid", "/context/context/eligibility/createdAfter")
    attributes = eligibility.attributes
    if not isinstance(attributes, (list, tuple)):
        _frozen_trace_error("context_invalid", "/context/context/eligibility/attributes")
    if len(attributes) > 64:
        _frozen_trace_error("context_invalid", "/context/context/eligibility/attributes")
    for index, pair in enumerate(attributes):
        if (
            not isinstance(pair, (list, tuple))
            or len(pair) != 2
            or not all(isinstance(item, str) for item in pair)
        ):
            _frozen_trace_error(
                "context_invalid",
                f"/context/context/eligibility/attributes/{index}",
            )
    return value


def _map_gpu_allocation_witness(native: Any) -> GpuAllocationWitness:
    """Map the native GPU allocation witness field-for-field.

    Deliberately exhaustive: R80-13 requires the record stay re-derivable, so
    nothing here summarizes, rounds, or drops a number.
    """

    return GpuAllocationWitness(
        schema=native.schema,
        sole_gpu_consumer_precondition=native.sole_gpu_consumer_precondition,
        device_ordinal_requested=native.device_ordinal_requested,
        device_ordinal_actual=native.device_ordinal_actual,
        device_uuid=native.device_uuid,
        device_name=native.device_name,
        compute_capability=native.compute_capability,
        free_before_bytes=native.free_before_bytes,
        free_after_bytes=native.free_after_bytes,
        total_bytes=native.total_bytes,
        delta_bytes=native.delta_bytes,
        delta_floor_bytes=native.delta_floor_bytes,
        control_allocation_request_bytes=native.control_allocation_request_bytes,
        control_block_count=native.control_block_count,
        control_free_before_bytes=native.control_free_before_bytes,
        control_free_after_bytes=native.control_free_after_bytes,
        control_delta_bytes=native.control_delta_bytes,
        embedded_vector_dim=native.embedded_vector_dim,
    )


def _map_cuda_device_info(native: Any) -> CudaDeviceInfo:
    allocator = native.cuda_allocator
    return CudaDeviceInfo(
        ordinal=native.ordinal,
        uuid=native.uuid,
        name=native.name,
        driver_version=native.driver_version,
        compute_capability=native.compute_capability,
        cuda_toolkit_version=native.cuda_toolkit_version,
        cuda_allocator=(
            None
            if allocator is None
            else CudaAllocatorReport(
                path=allocator.path,
                reason=allocator.reason,
                pool_max_size_bytes=allocator.pool_max_size_bytes,
                release_threshold=allocator.release_threshold,
                module_load_init=allocator.module_load_init,
            )
        ),
    )


def _map_open_report(native: Any) -> OpenReport:
    """Map one native open-time snapshot into the public Python contract."""

    device_resolution = native.embedder_device_resolution
    reranker_device_resolution = native.reranker_device_resolution
    gpu_allocation_witness = native.embedder_gpu_allocation_witness
    return OpenReport(
        schema_version_before=native.schema_version_before,
        schema_version_after=native.schema_version_after,
        migration_steps=[
            MigrationStepReport(
                step_id=step.step_id,
                duration_ms=step.duration_ms,
                failed=step.failed,
            )
            for step in native.migration_steps
        ],
        embedder_warmup_ms=native.embedder_warmup_ms,
        query_backend=native.query_backend,
        default_embedder=EmbedderIdentity(
            name=native.default_embedder.name,
            revision=native.default_embedder.revision,
            dimension=native.default_embedder.dimension,
        ),
        embedder_download_ms=native.embedder_download_ms,
        embedder_events=list(native.embedder_events),
        embedder_mean_centering_required=native.embedder_mean_centering_required,
        embedder_mean_vec_pinned=native.embedder_mean_vec_pinned,
        dense_disabled=native.dense_disabled,
        dense_disabled_reason=native.dense_disabled_reason,
        embedder_device_resolution=(
            None
            if device_resolution is None
            else DeviceResolution(
                requested_policy=device_resolution.requested_policy,
                cuda_compiled=device_resolution.cuda_compiled,
                effective_device=EffectiveEmbedDevice(
                    kind=cast(Literal["cpu", "cuda"], device_resolution.effective_device.kind),
                    cuda_device=(
                        None
                        if device_resolution.effective_device.cuda_device is None
                        else _map_cuda_device_info(device_resolution.effective_device.cuda_device)
                    ),
                ),
                visible_cuda_devices=tuple(
                    CudaVisibleDevice(
                        visible_ordinal=device.visible_ordinal,
                        uuid=device.uuid,
                        name=device.name,
                        compute_capability=device.compute_capability,
                    )
                    for device in device_resolution.visible_cuda_devices
                ),
                selected_cuda_uuid=device_resolution.selected_cuda_uuid,
                reason=device_resolution.reason,
            )
        ),
        reranker_device_resolution=(
            None
            if reranker_device_resolution is None
            else DeviceResolution(
                requested_policy=reranker_device_resolution.requested_policy,
                cuda_compiled=reranker_device_resolution.cuda_compiled,
                effective_device=EffectiveEmbedDevice(
                    kind=cast(
                        Literal["cpu", "cuda"],
                        reranker_device_resolution.effective_device.kind,
                    ),
                    cuda_device=(
                        None
                        if reranker_device_resolution.effective_device.cuda_device is None
                        else _map_cuda_device_info(
                            reranker_device_resolution.effective_device.cuda_device
                        )
                    ),
                ),
                visible_cuda_devices=tuple(
                    CudaVisibleDevice(
                        visible_ordinal=device.visible_ordinal,
                        uuid=device.uuid,
                        name=device.name,
                        compute_capability=device.compute_capability,
                    )
                    for device in reranker_device_resolution.visible_cuda_devices
                ),
                selected_cuda_uuid=reranker_device_resolution.selected_cuda_uuid,
                reason=reranker_device_resolution.reason,
            )
        ),
        embedder_gpu_allocation_witness=(
            None
            if gpu_allocation_witness is None
            else _map_gpu_allocation_witness(gpu_allocation_witness)
        ),
    )


def open_report(self: Engine) -> OpenReport:

    return _map_open_report(self._native.open_report())

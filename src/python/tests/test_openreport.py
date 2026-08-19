"""AC-068c — `engine.open_report()` surfaces the native `OpenReport`.

Shape D (locked HITL 2026-05-24): the report is exposed as an
engine-attached accessor, not a return-shape change on `Engine.open`.
`engine.open_report()` is a snapshot captured at open time; repeat calls
return identical data.

Spec refs:
- `dev/design/engine.md` § "`Engine.open` success result" — locked
  field subset (`schema_version_before`, `schema_version_after`,
  `migration_steps`, `embedder_warmup_ms`).
- Native struct: `src/rust/crates/fathomdb-engine/src/lib.rs:541-548`
  carries two additional fields (`query_backend`, `default_embedder`).
- `dev/interfaces/python.md` Engine-attached instrumentation list.
"""

from __future__ import annotations

from types import SimpleNamespace

from fathomdb import Engine
from fathomdb.engine import _map_open_report


def test_open_report_returns_native_fields(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        report = engine.open_report()

        assert isinstance(report.schema_version_before, int)
        assert isinstance(report.schema_version_after, int)
        assert report.schema_version_after >= report.schema_version_before
        assert isinstance(report.migration_steps, list)
        assert isinstance(report.embedder_warmup_ms, int)
        assert report.embedder_warmup_ms >= 0
        assert isinstance(report.query_backend, str)
        assert report.query_backend

        identity = report.default_embedder
        assert isinstance(identity.name, str) and identity.name
        assert isinstance(identity.revision, str) and identity.revision
        assert isinstance(identity.dimension, int) and identity.dimension > 0
    finally:
        engine.close()


def test_open_report_is_idempotent(db_path: str) -> None:
    engine = Engine.open(db_path)
    try:
        first = engine.open_report()
        second = engine.open_report()

        assert first.schema_version_before == second.schema_version_before
        assert first.schema_version_after == second.schema_version_after
        assert first.embedder_warmup_ms == second.embedder_warmup_ms
        assert first.query_backend == second.query_backend
        assert first.default_embedder.name == second.default_embedder.name
        assert first.default_embedder.revision == second.default_embedder.revision
        assert first.default_embedder.dimension == second.default_embedder.dimension

        assert len(first.migration_steps) == len(second.migration_steps)
        for a, b in zip(first.migration_steps, second.migration_steps):
            assert a.step_id == b.step_id
            assert a.duration_ms == b.duration_ms
            assert a.failed == b.failed
    finally:
        engine.close()


def test_open_report_exposes_absent_device_resolution_without_an_embedder(db_path: str) -> None:
    """No configured embedder has no runtime-device selection to report."""

    engine = Engine.open(db_path)
    try:
        report = engine.open_report()
        assert report.embedder_device_resolution is None
    finally:
        engine.close()


def test_open_report_maps_present_auto_cpu_device_resolution() -> None:
    """A present native resolution retains requested, effective, and reason facts."""

    native = SimpleNamespace(
        schema_version_before=1,
        schema_version_after=1,
        migration_steps=[],
        embedder_warmup_ms=0,
        query_backend="sqlite",
        default_embedder=SimpleNamespace(name="test", revision="test", dimension=384),
        embedder_download_ms=None,
        embedder_events=[],
        embedder_mean_centering_required=False,
        embedder_mean_vec_pinned=False,
        dense_disabled=False,
        dense_disabled_reason=None,
        embedder_device_resolution=SimpleNamespace(
            requested_policy="auto",
            cuda_compiled=True,
            effective_device=SimpleNamespace(kind="cpu", cuda_device=None),
            visible_cuda_devices=(
                SimpleNamespace(
                    visible_ordinal=0,
                    uuid="GPU-first",
                    name="RTX 3090",
                    compute_capability="8.6",
                ),
            ),
            selected_cuda_uuid=None,
            reason="cuda_probe_failed",
        ),
        # Pre-existing drift repaired in place: Slice 71 added
        # `reranker_device_resolution` to `_map_open_report` but never to this
        # fixture, so the test was already failing with `AttributeError` on
        # this branch before Slice 80.6 touched it.
        reranker_device_resolution=None,
        embedder_gpu_allocation_witness=None,
    )

    resolution = _map_open_report(native).embedder_device_resolution

    assert resolution is not None
    assert resolution.requested_policy == "auto"
    assert resolution.cuda_compiled is True
    assert resolution.effective_device.kind == "cpu"
    assert resolution.effective_device.cuda_device is None
    assert resolution.visible_cuda_devices[0].uuid == "GPU-first"
    assert resolution.selected_cuda_uuid is None
    assert resolution.reason == "cuda_probe_failed"


def test_open_report_exposes_absent_gpu_allocation_witness(db_path: str) -> None:
    """0.8.23 Slice 80.6 (D-80.6-6) — an ordinary open measured no witness.

    ``None`` here means *no witness was taken*, never "a witness measured
    nothing": a zero or below-floor allocation delta is a typed failure inside
    the witness and fails the open (R80-12), so a zero-valued record cannot
    reach this attribute.
    """

    engine = Engine.open(db_path)
    try:
        assert engine.open_report().embedder_gpu_allocation_witness is None
    finally:
        engine.close()


def test_open_report_maps_a_present_gpu_allocation_witness() -> None:
    """R80-13 — every number the verdict used survives the mapping.

    The point of the field is that a reader re-derives the verdict rather than
    trusting it, so the assertions below recompute the delta, the floor
    comparison, and the control-allocation check from the mapped record alone.
    """

    witness = SimpleNamespace(
        schema="fathomdb.tegra-gpu-allocation-witness/v1",
        sole_gpu_consumer_precondition="the witness run must be the sole GPU consumer",
        device_ordinal_requested=0,
        device_ordinal_actual=0,
        device_uuid="GPU-11111111-2222-3333-4444-555555555555",
        device_name="Orin",
        compute_capability="8.7",
        free_before_bytes=40_000_000_000,
        free_after_bytes=39_856_635_904,
        total_bytes=65_000_000_000,
        delta_bytes=143_364_096,
        delta_floor_bytes=67_108_864,
        control_allocation_request_bytes=1_073_741_824,
        control_block_count=8,
        control_free_before_bytes=42_000_000_000,
        control_free_after_bytes=40_800_000_000,
        control_delta_bytes=1_200_000_000,
        embedded_vector_dim=384,
    )
    native = SimpleNamespace(
        schema_version_before=1,
        schema_version_after=1,
        migration_steps=[],
        embedder_warmup_ms=0,
        query_backend="sqlite",
        default_embedder=SimpleNamespace(name="test", revision="test", dimension=384),
        embedder_download_ms=None,
        embedder_events=[],
        embedder_mean_centering_required=False,
        embedder_mean_vec_pinned=False,
        dense_disabled=False,
        dense_disabled_reason=None,
        embedder_device_resolution=None,
        reranker_device_resolution=None,
        embedder_gpu_allocation_witness=witness,
    )

    mapped = _map_open_report(native).embedder_gpu_allocation_witness

    assert mapped is not None
    assert mapped.schema == "fathomdb.tegra-gpu-allocation-witness/v1"
    assert "sole GPU consumer" in mapped.sole_gpu_consumer_precondition
    assert mapped.device_ordinal_requested == 0
    assert mapped.device_ordinal_actual == 0
    assert mapped.device_uuid == "GPU-11111111-2222-3333-4444-555555555555"
    assert mapped.device_name == "Orin"
    assert mapped.compute_capability == "8.7"
    assert mapped.free_before_bytes == 40_000_000_000
    assert mapped.free_after_bytes == 39_856_635_904
    assert mapped.total_bytes == 65_000_000_000
    assert mapped.delta_bytes == 143_364_096
    assert mapped.delta_floor_bytes == 67_108_864
    assert mapped.control_allocation_request_bytes == 1_073_741_824
    assert mapped.control_block_count == 8
    assert mapped.control_free_before_bytes == 42_000_000_000
    assert mapped.control_free_after_bytes == 40_800_000_000
    assert mapped.control_delta_bytes == 1_200_000_000
    assert mapped.embedded_vector_dim == 384

    # The verdict is re-derivable from the record alone.
    assert mapped.free_before_bytes - mapped.free_after_bytes == mapped.delta_bytes
    assert mapped.delta_bytes >= mapped.delta_floor_bytes
    assert (
        mapped.control_free_before_bytes - mapped.control_free_after_bytes
        >= mapped.control_allocation_request_bytes
    )


def test_open_signature_returns_engine_handle(db_path: str) -> None:
    """Shape D guarantee — `Engine.open` still returns just the engine."""

    engine = Engine.open(db_path)
    try:
        assert isinstance(engine, Engine)
    finally:
        engine.close()

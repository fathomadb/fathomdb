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


def test_open_signature_returns_engine_handle(db_path: str) -> None:
    """Shape D guarantee — `Engine.open` still returns just the engine."""

    engine = Engine.open(db_path)
    try:
        assert isinstance(engine, Engine)
    finally:
        engine.close()

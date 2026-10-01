"""Python configuration must reach the native open boundary without coercion."""

from __future__ import annotations

from dataclasses import FrozenInstanceError
from pathlib import Path
import sqlite3
from typing import Any, cast

import pytest

from fathomdb import Engine, EngineConfig
from fathomdb import engine as engine_module


def test_open_forwards_all_requested_values_and_keeps_frozen_snapshot(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    calls: list[tuple[str, bool, dict[str, int | None]]] = []

    class Native:
        @staticmethod
        def open(path: str, *, use_default_embedder: bool, config: dict[str, int | None]) -> object:
            calls.append((path, use_default_embedder, config))
            return object()

    monkeypatch.setattr(engine_module, "_NativeEngine", Native)
    requested = EngineConfig(
        scheduler_runtime_threads=4,
        embedder_pool_size=3,
        embedder_call_timeout_ms=2_000,
        provenance_row_cap=0,
        slow_threshold_ms=0,
    )
    opened = Engine.open(str(tmp_path / "configured.fathom"), config=requested)
    assert calls == [(str(tmp_path / "configured.fathom"), False, vars(requested))]
    assert opened.config == requested
    with pytest.raises(FrozenInstanceError):
        opened.config.slow_threshold_ms = 99  # type: ignore[misc]


@pytest.mark.parametrize(
    ("name", "value", "error_type"),
    [
        ("scheduler_runtime_threads", True, TypeError),
        ("scheduler_runtime_threads", 0, ValueError),
        ("scheduler_runtime_threads", 65, ValueError),
        ("embedder_pool_size", False, TypeError),
        ("embedder_pool_size", 65, ValueError),
        ("embedder_call_timeout_ms", -1, ValueError),
        ("embedder_call_timeout_ms", 2**32, ValueError),
        ("provenance_row_cap", 2**53, ValueError),
        ("slow_threshold_ms", 2**53, ValueError),
        ("slow_threshold_ms", 2**200, ValueError),
        ("slow_threshold_ms", 1.5, TypeError),
    ],
)
def test_invalid_request_fails_before_native_open(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
    name: str,
    value: object,
    error_type: type[Exception],
) -> None:
    class Native:
        @staticmethod
        def open(*_args: object, **_kwargs: object) -> object:
            raise AssertionError("invalid config reached native open")

    monkeypatch.setattr(engine_module, "_NativeEngine", Native)
    with pytest.raises(error_type):
        Engine.open(str(tmp_path / "invalid.fathom"), **cast(Any, {name: value}))
    assert not (tmp_path / "invalid.fathom").exists()


def test_explicit_zero_remains_distinct_from_omission(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    requests: list[dict[str, int | None]] = []

    class Native:
        @staticmethod
        def open(_path: str, *, use_default_embedder: bool, config: dict[str, int | None]) -> object:
            assert not use_default_embedder
            requests.append(config)
            return object()

    monkeypatch.setattr(engine_module, "_NativeEngine", Native)
    Engine.open(str(tmp_path / "omitted.fathom"))
    Engine.open(str(tmp_path / "zero.fathom"), provenance_row_cap=0, slow_threshold_ms=0)
    assert requests[0]["provenance_row_cap"] is None
    assert requests[1]["provenance_row_cap"] == 0
    assert requests[1]["slow_threshold_ms"] == 0


@pytest.mark.parametrize("workers", [1, 2, 4, 64])
def test_installed_scheduler_worker_inventory(tmp_path: Path, workers: int) -> None:
    task_dir = Path("/proc/self/task")
    if not task_dir.is_dir():
        pytest.skip("Linux task inventory is required for exact worker attribution")
    before = len(list(task_dir.iterdir()))
    opened = Engine.open(
        str(tmp_path / f"workers-{workers}.sqlite"), scheduler_runtime_threads=workers
    )
    try:
        assert len(list(task_dir.iterdir())) == before + 9 + workers
    finally:
        opened.close()
    assert len(list(task_dir.iterdir())) == before


def test_installed_independent_scheduler_inventories(tmp_path: Path) -> None:
    task_dir = Path("/proc/self/task")
    if not task_dir.is_dir():
        pytest.skip("Linux task inventory is required for exact worker attribution")
    before = len(list(task_dir.iterdir()))
    first = Engine.open(str(tmp_path / "first.sqlite"), scheduler_runtime_threads=1)
    try:
        second = Engine.open(str(tmp_path / "second.sqlite"), scheduler_runtime_threads=4)
        try:
            assert len(list(task_dir.iterdir())) == before + (9 + 1) + (9 + 4)
        finally:
            second.close()
        assert len(list(task_dir.iterdir())) == before + 9 + 1
    finally:
        first.close()
    assert len(list(task_dir.iterdir())) == before


@pytest.mark.parametrize(("cap", "remaining"), [(0, 3), (1, 1)])
def test_installed_provenance_cap_and_explicit_zero(
    tmp_path: Path, cap: int, remaining: int
) -> None:
    path = tmp_path / f"provenance-{cap}.sqlite"
    opened = Engine.open(str(path), provenance_row_cap=cap)
    try:
        opened.write(
            [
                {
                    "admin_schema": {
                        "name": "events",
                        "kind": "append_only_log",
                        "schema_json": '{"type":"object"}',
                        "retention_json": "{}",
                    }
                }
            ]
        )
        for index in range(3):
            opened.write(
                [
                    {
                        "op_store": {
                            "collection": "events",
                            "record_key": str(index),
                            "body": '{"value":1}',
                        }
                    }
                ]
            )
    finally:
        opened.close()
    with sqlite3.connect(path) as connection:
        count = connection.execute(
            "SELECT COUNT(*) FROM operational_mutations WHERE collection_name = 'events'"
        ).fetchone()
    assert count == (remaining,)


def test_installed_slow_setter_preserves_requested_snapshot(tmp_path: Path) -> None:
    requested = EngineConfig(slow_threshold_ms=0)
    opened = Engine.open(str(tmp_path / "slow.sqlite"), config=requested)
    try:
        opened.set_slow_threshold_ms(value=500)
        assert opened.config is requested
        assert opened.config.slow_threshold_ms == 0
    finally:
        opened.close()


def test_installed_open_forms_are_exclusive_and_equivalent(tmp_path: Path) -> None:
    requested = EngineConfig(scheduler_runtime_threads=2, provenance_row_cap=0)
    with pytest.raises(ValueError):
        Engine.open(str(tmp_path / "mixed.sqlite"), config=requested, embedder_pool_size=2)
    assert not (tmp_path / "mixed.sqlite").exists()

    configured = Engine.open(str(tmp_path / "object.sqlite"), config=requested)
    keywords = Engine.open(
        str(tmp_path / "keywords.sqlite"),
        scheduler_runtime_threads=2,
        provenance_row_cap=0,
    )
    try:
        assert configured.config == keywords.config == requested
    finally:
        configured.close()
        keywords.close()


@pytest.mark.parametrize(
    ("config", "error_type"),
    [
        ({"scheduler_runtime_threads": True}, TypeError),
        ({"scheduler_runtime_threads": -1}, ValueError),
        ({"embedder_pool_size": 65}, ValueError),
        ({"embedder_call_timeout_ms": 2**32}, ValueError),
        ({"provenance_row_cap": 2**53}, ValueError),
        ({"provenance_row_cap": 2**200}, ValueError),
        ({"slow_threshold_ms": 1.5}, TypeError),
        ({"unknown": 1}, TypeError),
    ],
)
def test_native_open_validates_before_filesystem(
    tmp_path: Path, config: dict[str, object], error_type: type[Exception]
) -> None:
    path = tmp_path / "invalid-native.sqlite"
    with pytest.raises(error_type):
        engine_module._NativeEngine.open(str(path), config=cast(Any, config))
    assert not path.exists()

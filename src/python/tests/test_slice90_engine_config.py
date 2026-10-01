"""Python configuration must reach the native open boundary without coercion."""

from __future__ import annotations

from dataclasses import FrozenInstanceError
from pathlib import Path

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
        Engine.open(str(tmp_path / "invalid.fathom"), **{name: value})
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

"""0.8.28 Slice 31 section 15: the CUDA pool settings are open settings.

`cuda_pool`, `cuda_pool_max_size` and `cuda_pool_release_threshold` are
validated before the native open, forwarded to it, and read back through
`Engine.config`. They are fixed once per process, so the conflict tests run
in a child interpreter.
"""

from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import textwrap
from typing import Any, cast

import pytest

from fathomdb import Engine, EngineConfig
from fathomdb import engine as engine_module

MIB = 1 << 20


@pytest.mark.parametrize(
    ("name", "value", "error_type"),
    [
        ("cuda_pool", "AUTO", ValueError),
        ("cuda_pool", "enabled", ValueError),
        ("cuda_pool", 1, ValueError),
        ("cuda_pool_max_size", 32 * MIB - 1, ValueError),
        ("cuda_pool_max_size", 2**64, ValueError),
        ("cuda_pool_max_size", "64M", TypeError),
        ("cuda_pool_max_size", True, TypeError),
        ("cuda_pool_release_threshold", 0, ValueError),
        ("cuda_pool_release_threshold", "MAX", ValueError),
    ],
)
def test_an_invalid_pool_setting_fails_before_the_native_open(
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


def test_the_pool_settings_reach_the_native_open_and_read_back(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    calls: list[dict[str, object]] = []

    class Native:
        @staticmethod
        def open(_path: str, *, use_default_embedder: bool, config: dict[str, object]) -> object:
            assert not use_default_embedder
            calls.append(config)
            return object()

    monkeypatch.setattr(engine_module, "_NativeEngine", Native)
    requested = EngineConfig(
        cuda_pool="off", cuda_pool_max_size=64 * MIB, cuda_pool_release_threshold="max"
    )
    opened = Engine.open(str(tmp_path / "pool.fathom"), config=requested)
    assert calls[0]["cuda_pool"] == "off"
    assert calls[0]["cuda_pool_max_size"] == 64 * MIB
    assert calls[0]["cuda_pool_release_threshold"] == "max"
    assert opened.config == requested
    Engine.open(str(tmp_path / "omitted.fathom"))
    assert calls[1]["cuda_pool"] is None
    assert calls[1]["cuda_pool_max_size"] is None
    assert calls[1]["cuda_pool_release_threshold"] is None


def _run_child(source: str, tmp_path: Path) -> subprocess.CompletedProcess[str]:
    env = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("FATHOMDB_POOL_")
    }
    return subprocess.run(
        [sys.executable, "-c", textwrap.dedent(source), str(tmp_path)],
        capture_output=True,
        text=True,
        env=env,
        timeout=120,
        check=False,
    )


def test_installed_a_conflicting_second_open_raises_invalid_argument(tmp_path: Path) -> None:
    result = _run_child(
        """
        import sys
        from pathlib import Path
        from fathomdb import Engine
        from fathomdb.errors import InvalidArgumentError

        root = Path(sys.argv[1])
        first = Engine.open(str(root / "first.sqlite"), cuda_pool="on")
        assert first.config.cuda_pool == "on"
        try:
            Engine.open(str(root / "second.sqlite"), cuda_pool="off")
        except InvalidArgumentError as error:
            print("conflict:", error)
        else:
            raise SystemExit("a different cuda_pool must conflict")
        same = Engine.open(str(root / "same.sqlite"), cuda_pool="on")
        omitted = Engine.open(str(root / "omitted.sqlite"))
        for engine in (first, same, omitted):
            engine.close()
        print("done")
        """,
        tmp_path,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert (
        "cuda_pool=off conflicts with cuda_pool=on already in force in this process "
        "(from sdk); open with the same value or omit it"
    ) in result.stdout, result.stdout
    assert "done" in result.stdout


def test_installed_a_different_size_conflicts_naming_the_setting(tmp_path: Path) -> None:
    result = _run_child(
        """
        import sys
        from pathlib import Path
        from fathomdb import Engine
        from fathomdb.errors import InvalidArgumentError

        root = Path(sys.argv[1])
        first = Engine.open(str(root / "first.sqlite"), cuda_pool_max_size=64 << 20)
        try:
            Engine.open(str(root / "second.sqlite"), cuda_pool_release_threshold="max")
        except InvalidArgumentError as error:
            print("conflict:", error)
        first.close()
        """,
        tmp_path,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert (
        "cuda_pool_release_threshold=max conflicts with cuda_pool_release_threshold=0 "
        "already in force in this process (from default)"
    ) in result.stdout, result.stdout

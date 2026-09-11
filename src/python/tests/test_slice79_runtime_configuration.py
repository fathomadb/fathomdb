"""Focused installed/runtime tests for Slice 79's startup control."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def _run(script: str, tmp_path: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["SLICE79_DB"] = str(tmp_path / "runtime.fathom")
    return subprocess.run(
        [sys.executable, "-c", script],
        check=False,
        capture_output=True,
        text=True,
        env=env,
        timeout=60,
    )


def test_performance_configuration_precedes_real_engine_open(tmp_path: Path) -> None:
    result = _run(
        """
import os
from fathomdb import Engine, admin
configured = admin.configure_runtime(sqlite_mode="performance")
assert configured.sqlite_mode == "performance"
engine = Engine.open(os.environ["SLICE79_DB"])
engine.close()
""",
        tmp_path,
    )
    assert result.returncode == 0, result.stderr


def test_diagnostics_repeat_and_conflict_are_typed(tmp_path: Path) -> None:
    result = _run(
        """
from fathomdb import admin
from fathomdb.errors import RuntimeConfigurationError
assert admin.configure_runtime(sqlite_mode="diagnostics").sqlite_mode == "diagnostics"
assert admin.configure_runtime(sqlite_mode="diagnostics").sqlite_mode == "diagnostics"
try:
    admin.configure_runtime(sqlite_mode="performance")
except RuntimeConfigurationError as error:
    assert error.reason == "conflict"
    assert error.requested_mode == "performance"
    assert error.effective_mode == "diagnostics"
else:
    raise AssertionError("expected RuntimeConfigurationError")
""",
        tmp_path,
    )
    assert result.returncode == 0, result.stderr


def test_invalid_mode_is_rejected_before_native_mutation(tmp_path: Path) -> None:
    result = _run(
        """
from fathomdb import admin
try:
    admin.configure_runtime(sqlite_mode="fast")
except ValueError:
    pass
else:
    raise AssertionError("expected ValueError")
assert admin.configure_runtime(sqlite_mode="performance").sqlite_mode == "performance"
""",
        tmp_path,
    )
    assert result.returncode == 0, result.stderr

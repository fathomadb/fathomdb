"""Contention blocks must bind an installed wheel to a source role."""

from __future__ import annotations

from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_contention_block as block  # noqa: E402


def test_role_identity_fails_closed_on_wheel_or_source_drift() -> None:
    protocol = {
        "status": "FROZEN_S02_PYTHON_PAIRED",
        "candidate": {"source_sha": "a" * 40, "wheel_sha256": "b" * 64},
    }
    block.validate_identity(protocol, "candidate", "a" * 40, "b" * 64)
    with pytest.raises(ValueError, match="wheel"):
        block.validate_identity(protocol, "candidate", "a" * 40, "c" * 64)
    with pytest.raises(ValueError, match="source"):
        block.validate_identity(protocol, "candidate", "d" * 40, "b" * 64)


def test_python_launcher_keeps_venv_symlink(tmp_path: Path) -> None:
    launcher = tmp_path / "python"
    launcher.symlink_to(sys.executable)
    assert block.launcher_path(launcher) == launcher.absolute()
    assert block.launcher_path(launcher) != launcher.resolve()

"""Ensure release wheels cannot carry stale Python bytecode from a checkout."""

from pathlib import Path
import sys
from zipfile import ZipFile

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_python_wheel_contents import validate_archive  # noqa: E402


def test_rejects_cached_bytecode(tmp_path: Path) -> None:
    wheel = tmp_path / "fathomdb.whl"
    with ZipFile(wheel, "w") as archive:
        archive.writestr("fathomdb/__init__.py", "")
        archive.writestr("fathomdb/__pycache__/__init__.cpython-312.pyc", b"stale")

    with pytest.raises(ValueError, match="stale Python bytecode"):
        validate_archive(wheel)


def test_accepts_clean_package(tmp_path: Path) -> None:
    wheel = tmp_path / "fathomdb.whl"
    with ZipFile(wheel, "w") as archive:
        archive.writestr("fathomdb/__init__.py", "")
        archive.writestr("fathomdb/_fathomdb.abi3.so", b"native")

    assert validate_archive(wheel) == 2

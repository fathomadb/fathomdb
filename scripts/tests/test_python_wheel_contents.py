"""Ensure release wheels carry only expected clean files for SDK and CLI distributions."""

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


def test_cli_wheel_accepts_unix_and_windows_fathomdb_binary(tmp_path: Path) -> None:
    for binary in ("fathomdb", "fathomdb.exe"):
        wheel = tmp_path / f"fathomdb_cli-{binary}.whl"
        with ZipFile(wheel, "w") as archive:
            archive.writestr(f"fathomdb_cli-0.8.30.data/scripts/{binary}", b"ELF")
            archive.writestr("fathomdb_cli-0.8.30.dist-info/METADATA", "Name: fathomdb-cli\n")
            archive.writestr("fathomdb_cli-0.8.30.dist-info/licenses/LICENSE", "MIT License\n")
        assert validate_archive(wheel, kind="cli") == 3


def test_cli_wheel_rejects_missing_fathomdb_binary(tmp_path: Path) -> None:
    wheel = tmp_path / "fathomdb_cli_missing.whl"
    with ZipFile(wheel, "w") as archive:
        archive.writestr("fathomdb_cli-0.8.30.dist-info/METADATA", "Name: fathomdb-cli\n")

    with pytest.raises(ValueError, match="missing required CLI binary"):
        validate_archive(wheel, kind="cli")


def test_cli_wheel_rejects_perf_regression_check_binary(tmp_path: Path) -> None:
    for bad_bin in ("perf-regression-check", "perf-regression-check.exe"):
        wheel = tmp_path / f"fathomdb_cli_{bad_bin}.whl"
        with ZipFile(wheel, "w") as archive:
            archive.writestr("fathomdb_cli-0.8.30.data/scripts/fathomdb", b"ELF")
            archive.writestr(f"fathomdb_cli-0.8.30.data/scripts/{bad_bin}", b"ELF")

        with pytest.raises(ValueError, match="forbidden binary"):
            validate_archive(wheel, kind="cli")


def test_cli_wheel_rejects_native_python_extension_module(tmp_path: Path) -> None:
    for ext in ("fathomdb/_fathomdb.abi3.so", "fathomdb/_fathomdb.pyd"):
        wheel = tmp_path / f"fathomdb_cli_{Path(ext).name}.whl"
        with ZipFile(wheel, "w") as archive:
            archive.writestr("fathomdb_cli-0.8.30.data/scripts/fathomdb", b"ELF")
            archive.writestr(ext, b"native")

        with pytest.raises(ValueError, match="forbidden native Python extension"):
            validate_archive(wheel, kind="cli")

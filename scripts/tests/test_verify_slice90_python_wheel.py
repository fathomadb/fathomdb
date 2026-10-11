"""The wheel proof must refuse a checkout with unrelated tracked changes."""

from pathlib import Path
import runpy
import shutil
import subprocess
import sys

import pytest


PROOF_SCRIPT = Path(__file__).resolve().parents[1] / "verify-slice90-python-wheel.sh"
FEATURE_HELPER = Path(__file__).resolve().parents[1] / "slice90_wheel_features.py"


def test_unrelated_tracked_change_is_rejected_before_build(tmp_path: Path) -> None:
    repo = tmp_path / "candidate"
    (repo / "scripts").mkdir(parents=True)
    shutil.copy2(PROOF_SCRIPT, repo / "scripts" / PROOF_SCRIPT.name)
    (repo / "README.md").write_text("committed\n", encoding="utf-8")
    subprocess.run(["git", "init", "-q", str(repo)], check=True)
    subprocess.run(["git", "-C", str(repo), "add", "scripts", "README.md"], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(repo),
            "-c",
            "user.name=Slice Test",
            "-c",
            "user.email=slice-test@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
        check=True,
    )
    commit = subprocess.check_output(["git", "-C", str(repo), "rev-parse", "HEAD"], text=True).strip()
    (repo / "README.md").write_text("uncommitted\n", encoding="utf-8")

    result = subprocess.run(
        ["bash", str(repo / "scripts" / PROOF_SCRIPT.name), commit],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 1
    assert "checkout has uncommitted tracked files" in result.stderr


def test_features_parse_without_tomllib_or_tomli(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pyproject = tmp_path / "pyproject.toml"
    pyproject.write_text(
        '[tool.maturin]\nfeatures = ["pyo3/extension-module", "default-embedder"]\n',
        encoding="utf-8",
    )
    read_features = runpy.run_path(str(FEATURE_HELPER))["read_maturin_features"]
    monkeypatch.setitem(sys.modules, "tomllib", None)
    monkeypatch.setitem(sys.modules, "tomli", None)
    assert read_features(pyproject) == ("pyo3/extension-module", "default-embedder")

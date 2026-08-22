"""Contract tests for isolated per-test FathomDB setup."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import fathomdb_test_setup


def test_prepare_creates_one_configured_database_and_doctor_evidence(tmp_path):
    calls: list[tuple[list[str], dict[str, str]]] = []

    def doctor(command, *, env):
        calls.append((command, env))
        return '{"status":"ready"}'

    opened: list[Path] = []

    def open_database(path: Path) -> dict[str, str]:
        opened.append(path)
        path.touch()
        return {"query_backend": "sqlite"}

    prepared = fathomdb_test_setup.prepare_test_database(
        tmp_path, test_id="locomo-gpu-cell-01", embed_device="cuda:0", rerank_device="cuda:0",
        doctor_runner=doctor, database_opener=open_database,
    )

    assert prepared.database_path == tmp_path / "locomo-gpu-cell-01" / "fathomdb.sqlite"
    assert opened == [prepared.database_path]
    config = json.loads(prepared.config_path.read_text())
    assert config == {"schema_version": "fathomdb.test-setup.v1", "embed_device": "cuda:0", "rerank_device": "cuda:0"}
    assert [call[0][1:3] for call in calls] == [["doctor", "gpu"], ["doctor", "reranker-gpu"], ["doctor", "check-integrity"]]
    assert all(call[1]["FATHOMDB_EMBED_DEVICE"] == "cuda:0" for call in calls)
    assert prepared.doctor_path.is_file()


def test_prepare_refuses_to_reuse_a_test_database(tmp_path):
    (tmp_path / "same").mkdir()

    with pytest.raises(FileExistsError, match="already exists"):
        fathomdb_test_setup.prepare_test_database(tmp_path, test_id="same")

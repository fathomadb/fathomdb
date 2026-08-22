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

    def open_database(path: Path, use_default_embedder: bool) -> dict[str, str]:
        opened.append(path)
        path.touch()
        assert use_default_embedder is True
        return {"query_backend": "sqlite"}

    prepared = fathomdb_test_setup.prepare_test_database(
        tmp_path, test_id="locomo-gpu-cell-01", embed_device="cuda:0", rerank_device="cuda:0", embedder="default", warm_cache=True,
        doctor_runner=doctor, database_opener=open_database,
    )

    assert prepared.database_path == tmp_path / "locomo-gpu-cell-01" / "fathomdb.sqlite"
    assert opened == [prepared.database_path]
    config = json.loads(prepared.config_path.read_text())
    assert config == {
        "schema_version": "fathomdb.test-setup.v1",
        "embed_device": "cuda:0",
        "embedder": "default",
        "rerank_device": "cuda:0",
        "cross_encoder": "enabled",
    }
    assert [call[0][1:3] for call in calls] == [["doctor", "gpu"], ["doctor", "reranker-gpu"], ["doctor", "warm-cache"], ["doctor", "check-integrity"]]
    assert all(call[1]["FATHOMDB_EMBED_DEVICE"] == "cuda:0" for call in calls)
    assert prepared.doctor_path.is_file()
    assert prepared.database_path.parent.stat().st_mode & 0o777 == 0o700
    assert prepared.config_path.stat().st_mode & 0o777 == 0o600
    assert prepared.doctor_path.stat().st_mode & 0o777 == 0o600
    assert prepared.database_path.stat().st_mode & 0o777 == 0o600


def test_prepare_records_disabled_cross_encoder_without_probing_it(tmp_path):
    calls: list[list[str]] = []

    def doctor(command, *, env):
        calls.append(command)
        return '{"status":"ready"}'

    def open_database(path: Path, _use_default_embedder: bool) -> dict[str, str]:
        path.touch()
        return {"query_backend": "sqlite"}

    prepared = fathomdb_test_setup.prepare_test_database(
        tmp_path,
        test_id="tc5-gpu",
        embed_device="cuda:0",
        rerank_device="cpu",
        embedder="default",
        warm_cache=True,
        check_reranker=False,
        doctor_runner=doctor,
        database_opener=open_database,
    )

    assert not any(command[1:3] == ["doctor", "reranker-gpu"] for command in calls)
    evidence = json.loads(prepared.doctor_path.read_text())
    assert evidence["reranker_gpu"] == {
        "status": "not_applicable",
        "reason": "cross_encoder_disabled",
    }
    assert json.loads(prepared.config_path.read_text())["cross_encoder"] == "disabled"
    assert json.loads(prepared.config_path.read_text())["rerank_device"] is None


def test_prepare_refuses_to_reuse_a_test_database(tmp_path):
    (tmp_path / "same").mkdir()

    with pytest.raises(FileExistsError, match="already exists"):
        fathomdb_test_setup.prepare_test_database(tmp_path, test_id="same")


def test_gpu_or_default_embedder_requires_an_explicit_cache_warm_preflight(tmp_path):
    with pytest.raises(ValueError, match="warm_cache"):
        fathomdb_test_setup.prepare_test_database(tmp_path, test_id="gpu", embed_device="cuda:0", embedder="default")

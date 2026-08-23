"""Contract tests for TEMPORAL-01's synthetic TRACE validity cell."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import pytest

from experiments import temporal_01_trace_validity as trace_validity
from experiments.fathomdb_test_setup import PreparedDatabase


CONFIG = Path("experiments/configs/temporal-01/trace-validity.v1.json")


def test_committed_config_is_fts_only_and_exercises_half_open_boundaries():
    config = trace_validity.load_config(CONFIG)

    assert config.program_track == "TEMPORAL-01"
    assert config.profile_id == "a0_turn_fts"
    assert config.embedder == "none"
    assert config.embed_device == "cpu"
    assert config.reranker == "none"
    assert config.rerank_device == "cpu"
    assert len(config.records) == 4
    assert len(config.probes) == 8
    assert config.probes[0].valid_as_of == 1000
    assert config.probes[0].expected_ids == ("trace-always", "trace-prior")
    assert config.probes[2].valid_as_of == 2000
    assert config.probes[2].expected_ids == ("trace-always",)


def test_config_rejects_unknown_fields():
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    document["unapproved"] = True

    with pytest.raises(trace_validity.TemporalTraceValidityError, match="keys drifted"):
        trace_validity.resolve_config(document)


def test_dry_run_validates_without_creating_an_artifact_root(tmp_path):
    report = trace_validity.dry_run(CONFIG, artifact_root=tmp_path / "new-root")

    assert report == {
        "schema_version": "temporal-01-trace-validity-dry-run.v1",
        "program_track": "TEMPORAL-01",
        "state": "ready",
        "record_count": 4,
        "probe_count": 8,
        "new_database_count": 1,
        "device": {"embed": "cpu", "rerank": "cpu", "gpu": None},
    }
    assert not (tmp_path / "new-root").exists()


@dataclass
class _Hit:
    id: str


@dataclass
class _SearchResult:
    results: list[_Hit]


class _Engine:
    def __init__(self) -> None:
        self.writes: list[dict[str, object]] = []
        self.closed = False

    def write(self, records: list[dict[str, object]]) -> None:
        self.writes.extend(records)

    def drain(self, *, timeout_s: int) -> None:
        assert timeout_s == 30

    def search_text_only(self, _query: str, *, view: object, limit: int) -> _SearchResult:
        instant = view.valid_as_of  # type: ignore[attr-defined]
        ids = {
            1000: ["trace-prior", "trace-always"],
            1500: ["trace-prior", "trace-always"],
            2000: ["trace-always"],
            2500: ["trace-always"],
            3000: ["trace-middle", "trace-always"],
            4000: ["trace-always"],
            5000: ["trace-late", "trace-always"],
            6000: ["trace-always"],
        }[instant]
        assert limit == 10
        return _SearchResult([_Hit(value) for value in ids])

    def close(self) -> None:
        self.closed = True


def test_run_uses_a_fresh_fts_database_and_writes_safe_receipt(tmp_path):
    artifact_root = tmp_path / "artifacts"
    base_dir = tmp_path / "registry"
    prepared = PreparedDatabase(
        database_path=artifact_root / "cell" / "fathomdb.sqlite",
        config_path=artifact_root / "cell" / "fathomdb-config.v1.json",
        doctor_path=artifact_root / "cell" / "fathomdb-doctor.v1.json",
    )
    calls: list[dict[str, object]] = []
    engine = _Engine()

    def prepare(root: Path, **kwargs: object) -> PreparedDatabase:
        calls.append({"root": root, **kwargs})
        prepared.database_path.parent.mkdir(parents=True)
        prepared.config_path.write_text("{}", encoding="utf-8")
        prepared.doctor_path.write_text("{}", encoding="utf-8")
        return prepared

    run_id, run_dir, metrics = trace_validity.run(
        CONFIG,
        artifact_root=artifact_root,
        base_dir=base_dir,
        prepare=prepare,
        open_engine=lambda _path: engine,
    )

    assert calls == [{
        "root": artifact_root,
        "test_id": "temporal-01-trace-validity",
        "embed_device": "cpu",
        "rerank_device": "cpu",
        "embedder": "none",
        "warm_cache": False,
        "check_reranker": False,
        "fathomdb_bin": "fathomdb",
    }]
    assert len(engine.writes) == 4
    assert engine.closed is True
    assert metrics["exact_probe_count"] == 8
    assert metrics["unexpected_hit_count"] == 0
    assert metrics["missing_expected_hit_count"] == 0
    record = json.loads((run_dir / "record.json").read_text(encoding="utf-8"))
    assert record["run_id"] == run_id
    assert record["cost_usd"] == 0.0
    assert record["metrics"]["exact_probe_count"] == 8
    assert str(artifact_root) not in (run_dir / "record.json").read_text(encoding="utf-8")
    assert run_id in (base_dir / "index.jsonl").read_text(encoding="utf-8")


def test_run_fails_closed_if_a_probe_returns_a_wrong_hit(tmp_path):
    config = trace_validity.load_config(CONFIG)

    with pytest.raises(trace_validity.TemporalTraceValidityError, match="validity mismatch"):
        trace_validity.evaluate_probes(
            config,
            search=lambda _query, _view, _limit: ["trace-prior"],
        )

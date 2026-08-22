"""Contract tests for the SCALE-02 A0 efficiency-envelope runner."""

from __future__ import annotations

import json
from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

import pytest

from experiments import scale_02


CONFIG = Path("experiments/configs/scale-02/a0-envelope.v1.json")


def test_committed_config_freezes_approved_a0_ladder_and_policy():
    config = scale_02.load_config(CONFIG)

    assert config.program_track == "SCALE-02"
    assert config.profile_id == "a0_turn_fts"
    assert config.top_k == 10
    assert config.embed_device == "cpu"
    assert config.rerank_device == "cpu"
    assert config.sizes == (10000, 17272, 25000, 40000, 50000)
    assert config.real_primary_count == 17272
    assert config.repetitions == 5
    assert config.cold_query_count == 100
    assert config.steady_query_count == 1000
    assert config.approval_state == "approved"
    assert config.approval_by == "HITL"
    assert config.policy["state"] == "approved"
    assert (
        config.fathomdb_bin_sha256
        == "6b40882fe3a99ed06750ff1e2062a914bf683f7d1c41298a6642bf501e2a36be"
    )
    assert (
        config.python_extension_sha256
        == "a622fe87161630d5bfb64fad1dc4eb3883971e1a46e21d411ceffc6a3624ee27"
    )


def test_config_rejects_unknown_fields():
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    document["unexpected"] = True

    with pytest.raises(scale_02.Scale02Error, match="keys drifted"):
        scale_02.resolve_config(document)


def test_hitl_can_approve_the_frozen_policy_without_changing_measurements():
    document = json.loads(CONFIG.read_text(encoding="utf-8"))
    document["approval"] = {
        "state": "approved",
        "approved_by": "HITL",
        "approved_at": "2026-08-22T12:00:00-05:00",
    }
    document["advisory_policy"]["state"] = "approved"

    config = scale_02.resolve_config(document)

    assert config.approval_state == "approved"
    assert config.policy["state"] == "approved"


def test_fixture_extension_is_deterministic_and_disclosed():
    real = tuple(
        scale_02.SourceDocument(
            document_id=f"real-{index}",
            text=f"body {index}",
            content_sha256=f"{index:064x}",
        )
        for index in range(3)
    )

    rows = scale_02.build_rows(real, 7, seed="0x10")
    repeated = scale_02.build_rows(real, 7, seed="0x10")
    alternate = scale_02.build_rows(real, 7, seed="0x11")

    assert rows == repeated
    assert [row.origin for row in rows] == ["real"] * 3 + ["derived_fixture"] * 4
    assert len({row.logical_id for row in rows}) == 7
    assert all("scale02-derived-row" in row.body for row in rows[3:])
    assert rows[3].body != alternate[3].body


def test_dry_run_checks_inputs_without_creating_database(tmp_path, monkeypatch):
    config = scale_02.load_config(CONFIG)
    python = tmp_path / "python"
    fathomdb_bin = tmp_path / "fathomdb"
    python.touch()
    fathomdb_bin.touch()
    monkeypatch.setattr(
        scale_02,
        "load_config",
        lambda _path: replace(config, python=python, fathomdb_bin=fathomdb_bin),
    )
    monkeypatch.setattr(
        scale_02,
        "_validate_runtime",
        lambda _config: {
            "release": "0.8.23",
            "cli_sha256": "2" * 64,
            "python_extension_sha256": "3" * 64,
        },
    )
    monkeypatch.setattr(
        scale_02,
        "load_fixture",
        lambda _config: scale_02.Fixture(
            documents=tuple(
                scale_02.SourceDocument(
                    document_id=f"doc-{index}",
                    text="body",
                    content_sha256=f"{index:064x}",
                )
                for index in range(17272)
            ),
            queries=tuple(f"query {index}" for index in range(100)),
            fixture_digest="1" * 64,
        ),
    )
    output = tmp_path / "must-not-exist"

    report = scale_02.dry_run(CONFIG, output_root=output)

    assert report["schema_version"] == "scale-02-dry-run.v1"
    assert report["state"] == "ready"
    assert report["cell_count"] == 25
    assert report["largest_point"] == {
        "canonical_records": 50000,
        "real_records": 17272,
        "derived_fixture_records": 32728,
    }
    assert report["new_database_count"] == 25
    assert not output.exists()


def test_live_execution_refuses_pending_hitl_before_database_setup(
    tmp_path, monkeypatch
):
    called = False

    def forbidden_prepare(*_args, **_kwargs):
        nonlocal called
        called = True
        raise AssertionError("database setup must not run")

    config = scale_02.load_config(CONFIG)
    pending = replace(
        config,
        approval_state="pending_hitl",
        approval_by=None,
        approval_at=None,
        policy={**config.policy, "state": "proposal_pending_hitl"},
    )
    monkeypatch.setattr(scale_02, "load_config", lambda _path: pending)
    monkeypatch.setattr(scale_02, "prepare_test_database", forbidden_prepare)

    with pytest.raises(scale_02.Scale02Error, match="HITL approval"):
        scale_02.run_point(CONFIG, 10000, output_root=tmp_path)

    assert called is False


def test_aggregate_requires_all_repetitions_and_keeps_cache_states_separate():
    config = replace(
        scale_02.load_config(CONFIG),
        cold_query_count=3,
        steady_query_count=3,
        mutation_count=2,
    )
    repetition = {
        "schema_version": "scale-02-repetition.v1",
        "point": 10000,
        "repetition": 1,
        "real_records": 10000,
        "derived_fixture_records": 0,
        "open_ms": 4.0,
        "ingest_ack_ms": 100.0,
        "ready_ms": 110.0,
        "cold_query_ms": [2.0, 3.0, 4.0],
        "steady_query_ms": [1.0, 2.0, 3.0],
        "mutation_to_ready_ms": [1.5, 2.5],
        "errors": 0,
        "timeouts": 0,
        "database_bytes": 4096,
        "derived_index_bytes": 1024,
        "peak_rss_bytes": 2048,
        "host_memory_bytes": 8192,
        "peak_host_cpu_fraction": 0.25,
        "peak_host_memory_fraction": 0.5,
        "process_cpu_seconds": 1.0,
        "effective_cpu_cores": 0.5,
        "device": {"embed": "cpu", "rerank": "cpu", "gpu": None},
        "doctor_sha256": "2" * 64,
    }

    with pytest.raises(scale_02.Scale02Error, match="five repetitions"):
        scale_02.aggregate_point(config, 10000, [repetition])

    repetitions = [dict(repetition, repetition=index + 1) for index in range(5)]
    summary = scale_02.aggregate_point(config, 10000, repetitions)

    assert summary["schema_version"] == "scale-02-point-summary.v1"
    assert summary["cache_states"]["cold"]["latency_ms"]["p95"] == 4.0
    assert summary["cache_states"]["steady"]["latency_ms"]["p95"] == 3.0
    assert summary["cache_states"]["cold"] is not summary["cache_states"]["steady"]
    assert summary["claim_boundary"] == "advisory_a0_efficiency_only"
    assert summary["provenance"]["doctor_sha256s"] == ["2" * 64] * 5
    assert summary["operations"]["acknowledgement_to_ready_ms"]["p50"] == 10.0
    assert summary["operations"]["ingest_records_per_second"]["p50"] == 100000.0
    assert "body" not in json.dumps(summary)

    invalid = [dict(item) for item in repetitions]
    invalid[0]["ingest_ack_ms"] = 0.0
    with pytest.raises(scale_02.Scale02Error, match="positive ingest timing"):
        scale_02.aggregate_point(config, 10000, invalid)


def test_repetition_uses_fresh_cpu_setup_and_counts_warmup_errors(tmp_path):
    config = replace(
        scale_02.load_config(CONFIG),
        sizes=(3,),
        real_primary_count=3,
        cold_query_count=1,
        warmup_query_count=1,
        steady_query_count=2,
        mutation_count=1,
        write_batch_size=2,
    )
    fixture = scale_02.Fixture(
        documents=tuple(
            scale_02.SourceDocument(f"doc-{index}", f"body {index}", f"{index:064x}")
            for index in range(3)
        ),
        queries=("query one", "query two"),
        fixture_digest="1" * 64,
    )
    prepare_calls = []

    def prepare(root, **kwargs):
        prepare_calls.append((root, kwargs))
        directory = root / kwargs["test_id"]
        directory.mkdir(parents=True)
        database = directory / "fathomdb.sqlite"
        doctor = directory / "doctor.json"
        config_path = directory / "config.json"
        database.touch()
        doctor.write_text("{}", encoding="utf-8")
        config_path.write_text("{}", encoding="utf-8")
        return scale_02.PreparedDatabase(database, config_path, doctor)

    engines = []

    class Engine:
        calls = 0
        closed = False

        def __init__(self):
            engines.append(self)

        def write(self, _rows):
            return None

        def drain(self, **_kwargs):
            return None

        def search_text_only(self, query, **_kwargs):
            self.calls += 1
            if self.calls == 2:
                raise RuntimeError("warmup failure")
            return SimpleNamespace(
                results=[SimpleNamespace(body=query)]
                if query.startswith("scale02mutationtoken")
                else []
            )

        def close(self):
            self.closed = True
            return None

    def engine_factory(_path):
        return Engine()

    result = scale_02._execute_repetition(
        config,
        fixture,
        scale_02.build_rows(fixture.documents, 3, seed=config.growth_seed),
        point_root=tmp_path,
        point=3,
        repetition=1,
        engine_factory=engine_factory,
        prepare=prepare,
    )

    assert prepare_calls[0][1]["embed_device"] == "cpu"
    assert prepare_calls[0][1]["embedder"] == "none"
    assert prepare_calls[0][1]["check_reranker"] is False
    assert result["errors"] == 1
    assert len(result["cold_query_ms"]) == 1
    assert len(result["steady_query_ms"]) == 2
    assert len(engines) == 2
    assert all(engine.closed for engine in engines)
    assert "peak_host_cpu_fraction" in result
    assert "effective_cpu_cores" in result


def test_run_point_rejects_repository_artifact_root_before_loading_fixture(
    monkeypatch,
):
    config = scale_02.load_config(CONFIG)
    approved = replace(
        config,
        approval_state="approved",
        approval_by="HITL",
        approval_at="2026-08-22T12:00:00-05:00",
        policy={**config.policy, "state": "approved"},
    )
    monkeypatch.setattr(scale_02, "load_config", lambda _path: approved)
    monkeypatch.setattr(
        scale_02,
        "load_fixture",
        lambda _config: (_ for _ in ()).throw(AssertionError("must not load fixture")),
    )

    with pytest.raises(scale_02.Scale02Error, match="outside the repository"):
        scale_02.run_point(CONFIG, 10000, output_root=scale_02.REPO_ROOT / "bad")


def test_run_point_refuses_to_skip_an_unpassed_ladder_point(tmp_path, monkeypatch):
    config = scale_02.load_config(CONFIG)
    approved = replace(
        config,
        approval_state="approved",
        approval_by="HITL",
        approval_at="2026-08-22T12:00:00-05:00",
        policy={**config.policy, "state": "approved"},
        sizes=(3, 4),
    )
    monkeypatch.setattr(scale_02, "load_config", lambda _path: approved)
    monkeypatch.setattr(
        scale_02,
        "load_fixture",
        lambda _config: (_ for _ in ()).throw(AssertionError("must not load fixture")),
    )

    with pytest.raises(scale_02.Scale02Error, match="prior ladder point 3"):
        scale_02.run_point(
            CONFIG,
            4,
            output_root=tmp_path / "artifacts",
            record_base_dir=tmp_path / "registry",
        )


def test_post_boundary_baseline_accepts_observed_limit_but_not_blocked_point(
    tmp_path,
):
    config = replace(scale_02.load_config(CONFIG), sizes=(3, 4))
    registry = tmp_path / "registry" / "runs"
    observed = registry / "observed"
    observed.mkdir(parents=True)
    (observed / "record.json").write_text(
        json.dumps(
            {
                "experiment": "scale-02-a0-3",
                "verdict": "advisory_limit_observed",
                "metrics": {
                    "point": 3,
                    "advisory": {"eligibility": "fail"},
                },
            }
        ),
        encoding="utf-8",
    )

    scale_02._require_prior_points(
        config,
        4,
        tmp_path / "registry",
        allow_advisory_limit_observed=True,
    )

    (observed / "record.json").write_text(
        json.dumps(
            {
                "experiment": "scale-02-a0-3",
                "verdict": "blocked_execution",
                "metrics": {"point": 3},
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(scale_02.Scale02Error, match="prior ladder point 3"):
        scale_02._require_prior_points(
            config,
            4,
            tmp_path / "registry",
            allow_advisory_limit_observed=True,
        )


def test_scale_adjusted_advisory_uses_linear_budget_without_rewriting_original():
    summary = {
        "cache_states": {"steady": {"latency_ms": {"p50": 23.9}}},
        "advisory": {
            "eligibility": "fail",
            "criteria": {
                "complete_repetitions": True,
                "errors": True,
                "timeouts": True,
                "steady_fts_p50": False,
                "steady_fts_p99": True,
                "mutation_to_ready_p99": True,
                "peak_rss_fraction": True,
            },
        },
    }

    adjusted = scale_02._scale_adjusted_advisory(summary, 25_000)

    assert adjusted["steady_fts_p50_ms"] == 25.0
    assert adjusted["eligibility"] == "pass"
    assert adjusted["original_fixed_policy_eligibility"] == "fail"
    assert summary["advisory"]["criteria"]["steady_fts_p50"] is False


def test_authorized_execution_failure_writes_blocked_receipt(tmp_path, monkeypatch):
    config = scale_02.load_config(CONFIG)
    approved = replace(
        config,
        approval_state="approved",
        approval_by="HITL",
        approval_at="2026-08-22T12:00:00-05:00",
        policy={**config.policy, "state": "approved"},
        sizes=(3,),
    )
    fixture = scale_02.Fixture(
        (scale_02.SourceDocument("doc-0", "body", "1" * 64),),
        ("body",),
        "1" * 64,
    )
    monkeypatch.setattr(scale_02, "load_config", lambda _path: approved)
    monkeypatch.setattr(scale_02, "_validate_runtime", lambda _config: {})
    monkeypatch.setattr(scale_02, "load_fixture", lambda _config: fixture)
    monkeypatch.setattr(
        scale_02,
        "_execute_repetition",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(RuntimeError("engine failed")),
    )
    registry = tmp_path / "registry"

    with pytest.raises(scale_02.Scale02Error, match="engine failed"):
        scale_02.run_point(
            CONFIG,
            3,
            output_root=tmp_path / "artifacts",
            record_base_dir=registry,
        )

    rows = (registry / "index.jsonl").read_text(encoding="utf-8").splitlines()
    assert len(rows) == 1
    index_row = json.loads(rows[0])
    record = json.loads(
        (registry / "runs" / index_row["run_id"] / "record.json").read_text()
    )
    assert index_row["verdict"] == "blocked_execution"
    assert record["metrics"]["point"] == 3
    assert record["metrics"]["completed_repetitions"] == 0
    assert record["metrics"]["error_type"] == "RuntimeError"


def test_run_point_writes_standard_receipt_and_generated_index(tmp_path, monkeypatch):
    config = scale_02.load_config(CONFIG)
    resolved = json.loads(json.dumps(config.resolved))
    resolved["approval"] = {
        "state": "approved",
        "approved_by": "HITL",
        "approved_at": "2026-08-22T12:00:00-05:00",
    }
    resolved["advisory_policy"]["state"] = "approved"
    approved = replace(
        config,
        approval_state="approved",
        approval_by="HITL",
        approval_at="2026-08-22T12:00:00-05:00",
        policy={
            **config.policy,
            "state": "approved",
            "required_repetitions": 1,
        },
        resolved=resolved,
        sizes=(3,),
        real_primary_count=3,
        repetitions=1,
        cold_query_count=1,
        warmup_query_count=1,
        steady_query_count=2,
        mutation_count=1,
        write_batch_size=2,
    )
    fixture = scale_02.Fixture(
        documents=tuple(
            scale_02.SourceDocument(f"doc-{index}", f"body {index}", f"{index:064x}")
            for index in range(3)
        ),
        queries=("body",),
        fixture_digest="1" * 64,
    )
    monkeypatch.setattr(scale_02, "load_config", lambda _path: approved)
    monkeypatch.setattr(scale_02, "load_fixture", lambda _config: fixture)

    def prepare(root, **kwargs):
        directory = root / kwargs["test_id"]
        directory.mkdir(parents=True)
        database = directory / "fathomdb.sqlite"
        doctor = directory / "doctor.json"
        config_path = directory / "config.json"
        database.touch()
        doctor.write_text("{}", encoding="utf-8")
        config_path.write_text("{}", encoding="utf-8")
        return scale_02.PreparedDatabase(database, config_path, doctor)

    class Engine:
        def write(self, _rows):
            return None

        def drain(self, **_kwargs):
            return None

        def search_text_only(self, query, **_kwargs):
            return SimpleNamespace(
                results=[SimpleNamespace(body=query)]
                if query.startswith("scale02mutationtoken")
                else []
            )

        def close(self):
            return None

    registry = tmp_path / "registry"
    result = scale_02.run_point(
        CONFIG,
        3,
        output_root=tmp_path / "artifacts",
        engine_factory=lambda _path: Engine(),
        prepare=prepare,
        record_base_dir=registry,
    )

    record = json.loads(
        (registry / "runs" / result["run_id"] / "record.json").read_text()
    )
    index_row = json.loads((registry / "index.jsonl").read_text())
    assert record["schema_version"] == "experiments.record.v1"
    assert record["metrics"]["schema_version"] == "scale-02-point-summary.v1"
    assert index_row["schema_version"] == "experiments.index-row.v1"
    assert (registry / "INDEX.md").is_file()

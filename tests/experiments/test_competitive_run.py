"""Contract tests for competitive run definitions and safe result receipts."""

from __future__ import annotations

import json
import hashlib
from datetime import UTC, datetime
from pathlib import Path

import pytest

from experiments import _lib, competitive_run


def definition() -> dict:
    return {
        "schema_version": "competitive.run.v1",
        "case_id": "fathomdb-ir-c-a0",
        "scoreboard": "retrieval",
        "product": {"name": "FathomDB", "version": "0.8.27", "deployment": "embedded"},
        "evidence": {"kind": "local", "reference_url": None},
        "workload": {
            "name": "IR-C",
            "corpus_sha256": "a" * 64,
            "queries_sha256": "b" * 64,
            "items": 10506,
            "queries": 4597,
            "search_mode": "fts",
            "top_k": 10,
            "filter": "none",
        },
        "measurement": {
            "boundary": "public Python call through materialized hits",
            "cache_state": "steady",
            "concurrency": 1,
            "repetitions": 3,
            "seed": 7,
        },
        "target_hardware": {
            "cpu": "reference CPU",
            "cores": 8,
            "ram_gib": 32,
            "storage": "NVMe SSD",
            "gpu": "none",
        },
        "settings": {"reader_threads": 1},
    }


def result(config: dict) -> dict:
    return {
        "schema_version": "competitive.result.v1",
        "case_id": config["case_id"],
        "config_sha256": _lib.config_sha256(config),
        "status": "complete",
        "observed": {
            "product_version": "0.8.27",
            "cpu": "observed CPU",
            "cores": 8,
            "ram_gib": 32,
            "storage": "NVMe SSD",
            "gpu": "none",
            "settings": {"reader_threads": 1},
            "workload": dict(config["workload"]),
            "measurement": dict(config["measurement"]),
            "queries_completed": 4597 * 3,
            "errors": 0,
        },
        "metrics": {"recall_at_10": 0.697451, "ndcg_at_10": 0.5},
        "source": {
            "url": None,
            "locator": "external/results/summary.json",
            "sha256": "c" * 64,
        },
        "comparability": "candidate_match",
        "reason": None,
    }


def test_definition_rejects_unknown_settings_and_missing_boundary():
    config = definition()
    assert competitive_run.validate_definition(config) == config
    config["measurement"]["typo"] = 1
    with pytest.raises(ValueError, match="unknown"):
        competitive_run.validate_definition(config)
    del config["measurement"]["typo"]
    del config["measurement"]["boundary"]
    with pytest.raises(ValueError, match="missing"):
        competitive_run.validate_definition(config)


def test_committed_example_definition_is_valid():
    path = (
        Path(__file__).resolve().parents[2]
        / "experiments/configs/competitive/run.example.json"
    )
    config = json.loads(path.read_text(encoding="utf-8"))
    assert (
        competitive_run.validate_definition(config)["case_id"]
        == "example-local-retrieval"
    )
    result_path = path.with_name("result.example.json")
    output = json.loads(result_path.read_text(encoding="utf-8"))
    assert competitive_run.validate_result(config, output)["status"] == "blocked"


def test_cli_can_validate_run_definition_before_result_exists(tmp_path):
    path = tmp_path / "run.json"
    path.write_text(json.dumps(definition()), encoding="utf-8")
    assert competitive_run.main(["validate", str(path)]) == 0


def test_result_rejects_drift_and_wrong_metric_family():
    config = definition()
    output = result(config)
    assert competitive_run.validate_result(config, output) == output
    output["observed"]["settings"]["reader_threads"] = 2
    with pytest.raises(ValueError, match="settings drift"):
        competitive_run.validate_result(config, output)
    output = result(config)
    output["metrics"] = {"p95_ms": 12.0}
    with pytest.raises(ValueError, match="metric"):
        competitive_run.validate_result(config, output)
    output = result(config)
    output["observed"]["measurement"]["concurrency"] = 2
    with pytest.raises(ValueError, match="measurement drift"):
        competitive_run.validate_result(config, output)
    output = result(config)
    del output["source"]["locator"]
    with pytest.raises(ValueError, match="missing"):
        competitive_run.validate_result(config, output)
    output = result(config)
    output["observed"]["queries_completed"] = config["workload"]["queries"]
    with pytest.raises(ValueError, match="all queries"):
        competitive_run.validate_result(config, output)


def test_blocked_result_can_report_no_observation():
    config = definition()
    output = result(config)
    output.update(status="blocked", observed=None, metrics={}, reason="adapter_failed")
    assert competitive_run.validate_result(config, output) == output


@pytest.mark.parametrize(
    "settings",
    [
        {"api_key": "sk-private-value"},
        {"auth_token": "private-value"},
        {"apiKey": "private-value"},
        {"APIKey": "private-value"},
        {"accessToken": "private-value"},
        {"clientSecret": "private-value"},
        {"adapter": {"predictions": ["raw answer"]}},
        {"safe_name": "Bearer private-value"},
    ],
)
def test_definition_rejects_sensitive_or_raw_settings(settings):
    config = definition()
    config["settings"] = settings
    with pytest.raises(ValueError, match="settings"):
        competitive_run.validate_definition(config)


def test_definition_accepts_content_free_tuning_identifiers():
    config = definition()
    config["settings"] = {"embedding_model": "all-MiniLM-L6-v2", "max_tokens": 256}
    assert competitive_run.validate_definition(config) == config


def test_local_complete_receipt_requires_runner_evidence(tmp_path):
    with pytest.raises(ValueError, match="execution witness"):
        competitive_run.record_run(
            definition(),
            result(definition()),
            ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
            base_dir=tmp_path,
            code={},
            env={},
        )
    assert not (tmp_path / "index.jsonl").exists()


def _execution(config: dict, output: dict, root: Path) -> dict:
    result_path = root / "result.json"
    result_path.write_text(json.dumps(output), encoding="utf-8")
    return {
        "schema_version": "competitive.execution.v1",
        "config_sha256": _lib.config_sha256(config),
        "source_sha256": output["source"]["sha256"],
        "status": output["status"],
        "result_locator": str(result_path),
        "result_sha256": hashlib.sha256(result_path.read_bytes()).hexdigest(),
    }


def test_execution_witness_requires_matching_result_artifact(tmp_path):
    config = definition()
    output = result(config)
    witness = _execution(config, output, tmp_path)
    (tmp_path / "result.json").write_text("{}", encoding="utf-8")
    with pytest.raises(ValueError, match="execution result artifact"):
        competitive_run.record_run(
            config,
            output,
            ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
            base_dir=tmp_path / "receipts",
            code={},
            env={},
            execution=witness,
        )


def test_record_run_writes_existing_envelope_and_renders_family(tmp_path):
    config = definition()
    output = result(config)
    run_id, run_dir = competitive_run.record_run(
        config,
        output,
        ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
        base_dir=tmp_path,
        code={
            "git_sha": "abc",
            "dirty": False,
            "branch": "main",
            "baseline_commit": None,
        },
        env={"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        execution=_execution(config, output, tmp_path),
    )
    record = json.loads((run_dir / "record.json").read_text())
    assert record["schema_version"] == "experiments.record.v1"
    assert record["config"]["resolved"] == config
    assert record["metrics"]["competitive_result"] == output
    assert len((tmp_path / "index.jsonl").read_text().splitlines()) == 1
    assert run_id in competitive_run.render_scoreboards(tmp_path / "runs")


def test_record_run_refuses_same_minute_collision_without_changing_receipt(tmp_path):
    config = definition()
    output = result(config)
    kwargs = {
        "base_dir": tmp_path,
        "code": {
            "git_sha": "abc",
            "dirty": False,
            "branch": "main",
            "baseline_commit": None,
        },
        "env": {"python": "3.12", "lockfile_sha256": None, "gpu": None, "key_deps": {}},
        "execution": _execution(config, output, tmp_path),
    }
    _, run_dir = competitive_run.record_run(
        config,
        output,
        ts=datetime(2026, 10, 12, 12, 0, 1, tzinfo=UTC),
        **kwargs,
    )
    first_receipt = (run_dir / "record.json").read_bytes()
    first_index = (tmp_path / "index.jsonl").read_bytes()
    second = result(config)
    second["metrics"]["recall_at_10"] = 0.9
    second_root = tmp_path / "second"
    second_root.mkdir()
    kwargs["execution"] = _execution(config, second, second_root)
    with pytest.raises(ValueError, match="already exists"):
        competitive_run.record_run(
            config,
            second,
            ts=datetime(2026, 10, 12, 12, 0, 59, tzinfo=UTC),
            **kwargs,
        )
    assert (run_dir / "record.json").read_bytes() == first_receipt
    assert (tmp_path / "index.jsonl").read_bytes() == first_index


def test_published_reference_is_labeled_and_never_claims_matched_workload():
    config = definition()
    config["case_id"] = "reference-ir-c"
    config["product"] = {
        "name": "ExampleDB",
        "version": "1.0",
        "deployment": "embedded",
    }
    config["evidence"] = {
        "kind": "published",
        "reference_url": "https://example.org/results",
    }
    config["workload"]["corpus_sha256"] = None
    config["workload"]["queries_sha256"] = None
    output = result(config)
    output["observed"]["product_version"] = "1.0"
    output["source"]["url"] = "https://example.org/results"
    output["comparability"] = "context_only"
    assert competitive_run.validate_result(config, output) == output
    output["comparability"] = "candidate_match"
    with pytest.raises(ValueError, match="published"):
        competitive_run.validate_result(config, output)

"""Successor product receipts keep external execution detail out of the repo."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path

import pytest
from hypothesis import given, strategies as st

from experiments import competitive_product, competitive_run, competitive_runner


ROOT = Path(__file__).resolve().parents[2]


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _fixture(tmp_path: Path, monkeypatch) -> Path:
    monkeypatch.setenv("PYTHONPATH", str(ROOT))
    secret = tmp_path / "secret-ACME-api-key-sk-private"
    secret.mkdir()
    adapter = secret / "fixture.py"
    adapter.write_text(
        """import hashlib
import json
from experiments import _lib

def run(definition, inputs, artifact_root):
    summary = artifact_root / 'source-secret-customer.json'
    summary.write_text(json.dumps({'private': 'sk-secret-answer'}))
    return {
        'schema_version': 'competitive.result.v1',
        'case_id': definition['case_id'],
        'config_sha256': _lib.config_sha256(definition),
        'status': 'complete',
        'observed': {
            'product_version': definition['product']['version'],
            **definition['target_hardware'],
            'settings': definition['settings'],
            'workload': definition['workload'],
            'measurement': definition['measurement'],
            'queries_completed': definition['workload']['queries'] * definition['measurement']['repetitions'],
            'errors': 0,
        },
        'metrics': {'recall_at_10': 0.7},
        'source': {'url': None, 'locator': str(summary), 'sha256': hashlib.sha256(summary.read_bytes()).hexdigest()},
        'comparability': 'candidate_match',
        'reason': None,
    }
""",
        encoding="utf-8",
    )
    inputs = {}
    for name in ("corpus", "queries", "gold"):
        path = secret / f"{name}-secret.json"
        path.write_text("[]", encoding="utf-8")
        inputs[name] = {"path": str(path), "sha256": _sha(path)}
    run = json.loads(
        (ROOT / "experiments/configs/competitive/run.example.json").read_text()
    )
    run["workload"]["corpus_sha256"] = inputs["corpus"]["sha256"]
    run["workload"]["queries_sha256"] = inputs["queries"]["sha256"]
    plan = {
        "schema_version": "competitive.experiment.v2",
        "program_track": None,
        "run": run,
        "adapter": {
            "protocol": "competitive.adapter.v1",
            "id": "fixture",
            "path": str(adapter),
            "sha256": _sha(adapter),
            "timeout_s": 30,
        },
        "inputs": inputs,
        "output": {
            "schema_version": "competitive.result.v2",
            "metrics": ["recall_at_10"],
        },
    }
    path = secret / "experiment.json"
    path.write_text(json.dumps(plan), encoding="utf-8")
    return path


def test_successor_receipt_is_path_free_and_hash_linked(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    receipts = tmp_path / "committed"
    external = tmp_path / "external-secret-root"
    run_id, run_dir = competitive_product.run_experiment(
        plan, external, receipts, ts=datetime(2026, 10, 12, 12, tzinfo=UTC)
    )
    record = json.loads((run_dir / "record.json").read_text())
    safe = (
        "".join(
            (run_dir / name).read_text()
            for name in ("record.json", "config.resolved.yaml", "metrics.json")
        )
        + (receipts / "index.jsonl").read_text()
    )
    assert "secret-ACME" not in safe
    assert "sk-secret-answer" not in safe
    assert "external-secret-root" not in safe
    assert record["config"]["resolved"]["schema_version"] == "competitive.run.v2"
    assert (
        record["metrics"]["competitive_result"]["schema_version"]
        == "competitive.result.v2"
    )
    row = competitive_product.validate_receipt(record, external)
    assert row["run_id"] == run_id
    assert row["axis"] == "product"
    assert row["family"] == "retrieval"
    assert row["evidence_status"] == "verified_local"
    assert row["source_evidence"]["sha256"] == _sha(
        external / "source-secret-customer.json"
    )
    assert row["metrics"] == {"recall_at_10": 0.7}
    assert (
        competitive_product.validate_receipt(record)["evidence_status"]
        == "hash_linked_local"
    )
    assert (
        competitive_product.validate_receipt(record, external)["status"] == "complete"
    )
    rendered = competitive_run.render_scoreboards(receipts / "runs")
    assert run_id in rendered
    assert "hash_linked_local" in rendered


def test_successor_rejects_tampered_links(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    external = tmp_path / "external"
    _, run_dir = competitive_product.run_experiment(
        plan,
        external,
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    record["metrics"]["competitive_execution"]["result_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="result.*hash|hash.*result"):
        competitive_product.validate_receipt(record, external)
    record = json.loads((run_dir / "record.json").read_text())
    (external / "result.json").write_text("{}", encoding="utf-8")
    with pytest.raises(ValueError, match="result.*hash|hash.*result"):
        competitive_product.validate_receipt(record, external)


def test_safe_projection_rejects_embedded_private_paths(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    full = json.loads(plan.read_text())
    full["run"]["measurement"]["boundary"] = "SDK call at /home/customer/private-data"
    with pytest.raises(ValueError, match="path|secret"):
        competitive_product.safe_projection(
            {
                **full,
                "schema_version": competitive_product.EXECUTION_CONFIG_SCHEMA,
                "plan_sha256": "a" * 64,
            }
        )
    full["run"]["measurement"]["boundary"] = "SDK call"
    full["run"]["settings"]["model_uri"] = "file:///home/customer/private-model"
    with pytest.raises(ValueError, match="path|secret"):
        competitive_product.safe_projection(
            {
                **full,
                "schema_version": competitive_product.EXECUTION_CONFIG_SCHEMA,
                "plan_sha256": "a" * 64,
            }
        )


def test_successor_rejects_unsafe_record_envelope(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    _, run_dir = competitive_product.run_experiment(
        plan,
        tmp_path / "external",
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    record["read"] = "/private/secret-api-key"
    with pytest.raises(ValueError, match="path|secret"):
        competitive_product.validate_receipt(record)


def test_renderer_keeps_invalid_successor_visible(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    _, run_dir = competitive_product.run_experiment(
        plan,
        tmp_path / "external",
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    record["metrics"]["competitive_execution"]["safe_result_sha256"] = "0" * 64
    (run_dir / "record.json").write_text(json.dumps(record))
    rendered = competitive_run.render_scoreboards(tmp_path / "committed/runs")
    assert "invalid" in rendered


def test_unsafe_envelope_is_rejected_before_receipt_write(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    monkeypatch.setattr(
        competitive_product._lib,
        "git_info",
        lambda: {"branch": "feature/secret-customer"},
    )
    receipts = tmp_path / "committed"
    with pytest.raises(ValueError, match="path|secret"):
        competitive_product.run_experiment(
            plan,
            tmp_path / "external",
            receipts,
            ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
        )
    assert not (receipts / "index.jsonl").exists()


def test_external_config_mutation_is_rejected_before_receipt_write(
    tmp_path, monkeypatch
):
    plan = _fixture(tmp_path, monkeypatch)
    config = json.loads(plan.read_text())
    adapter = Path(config["adapter"]["path"])
    adapter.write_text(
        adapter.read_text().replace(
            "    return {",
            "    (artifact_root / 'execution-config.json').write_text('{}')\n    return {",
        ),
        encoding="utf-8",
    )
    config["adapter"]["sha256"] = _sha(adapter)
    plan.write_text(json.dumps(config))
    receipts = tmp_path / "committed"
    with pytest.raises(ValueError, match="hash|drift"):
        competitive_product.run_experiment(
            plan,
            tmp_path / "external",
            receipts,
            ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
        )
    assert not (receipts / "index.jsonl").exists()


def test_malformed_successor_envelope_renders_invalid(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    _, run_dir = competitive_product.run_experiment(
        plan,
        tmp_path / "external",
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    del record["title"]
    (run_dir / "record.json").write_text(json.dumps(record))
    assert "invalid" in competitive_run.render_scoreboards(tmp_path / "committed/runs")


@pytest.mark.parametrize(
    "damage", ["config_list", "scoreboard_list", "metrics_list", "truncated", "unsupported_schema", "missing_schema"]
)
def test_malformed_product_receipt_remains_visible(tmp_path, monkeypatch, damage):
    plan = _fixture(tmp_path, monkeypatch)
    _, run_dir = competitive_product.run_experiment(
        plan,
        tmp_path / "external",
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    path = run_dir / "record.json"
    record = json.loads(path.read_text())
    if damage == "config_list":
        record["config"]["resolved"] = []
    elif damage == "scoreboard_list":
        record["config"]["resolved"]["scoreboard"] = []
    elif damage == "metrics_list":
        record["metrics"] = []
    elif damage == "unsupported_schema":
        record["config"]["resolved"]["schema_version"] = "competitive.run.v99"
    elif damage == "missing_schema":
        del record["config"]["resolved"]["schema_version"]
    else:
        path.write_text("{", encoding="utf-8")
    if damage != "truncated":
        path.write_text(json.dumps(record), encoding="utf-8")
    rendered = competitive_run.render_scoreboards(tmp_path / "committed/runs")
    assert "Invalid evidence" in rendered
    assert "invalid" in rendered


@given(
    st.text(alphabet="abcdefghijklmnopqrstuvwxyz0123456789", min_size=1, max_size=24)
)
def test_safe_projection_is_invariant_to_external_locator_changes(tag):
    run = json.loads(
        (ROOT / "experiments/configs/competitive/run.example.json").read_text()
    )
    full = {
        "schema_version": competitive_product.EXECUTION_CONFIG_SCHEMA,
        "program_track": None,
        "plan_sha256": "a" * 64,
        "run": run,
        "adapter": {
            "protocol": competitive_runner.ADAPTER_PROTOCOL,
            "id": "fixture",
            "path": "/outside/adapter.py",
            "sha256": "c" * 64,
            "timeout_s": 30,
        },
        "inputs": {
            "corpus": {"path": "/outside/corpus.json", "sha256": "a" * 64},
            "queries": {"path": "/outside/queries.json", "sha256": "b" * 64},
            "gold": {"path": "/outside/gold.json", "sha256": "d" * 64},
        },
        "output": {
            "schema_version": competitive_product.RESULT_SCHEMA,
            "metrics": ["recall_at_10"],
        },
    }
    original = competitive_product.safe_projection(full)
    full["adapter"]["path"] = f"/outside/{tag}/adapter.py"
    for binding in full["inputs"].values():
        binding["path"] = f"/outside/{tag}/{Path(binding['path']).name}"
    assert competitive_product.safe_projection(full) == original


def test_v1_is_legacy_path_bearing(tmp_path, monkeypatch):
    plan = _fixture(tmp_path, monkeypatch)
    legacy_plan = json.loads(plan.read_text())
    legacy_plan["schema_version"] = "competitive.experiment.v1"
    legacy_plan["output"]["schema_version"] = "competitive.result.v1"
    plan.write_text(json.dumps(legacy_plan), encoding="utf-8")
    from experiments import competitive_runner

    _, run_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "external",
        tmp_path / "committed",
        ts=datetime(2026, 10, 12, 12, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    row = competitive_product.validate_receipt(record)
    assert row["evidence_status"] == "legacy_path_bearing"
    assert row["comparability"] == "context_only"
    assert "secret-ACME" in (run_dir / "record.json").read_text()
    rendered = competitive_run.render_scoreboards(tmp_path / "committed/runs")
    assert "fixture" in rendered
    assert "legacy_path_bearing" in rendered
    assert "candidate_match" not in rendered

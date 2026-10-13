"""End-to-end contract tests for the versioned competitive runner."""

from __future__ import annotations

import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path

import pytest

from experiments import competitive_run, competitive_runner


ROOT = Path(__file__).resolve().parents[2]


def test_committed_experiment_example_has_the_versioned_runner_shape():
    example = ROOT / "experiments/configs/competitive/experiment.example.json"
    plan, digest = competitive_runner.load_plan(example)
    assert plan["schema_version"] == "competitive.experiment.v1"
    assert plan["adapter"]["id"] == "retrieval"
    assert len(digest) == 64


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _adapter(
    path: Path,
    *,
    bad_source: bool = False,
    wrong_metric: bool = False,
    mutate_input: bool = False,
) -> None:
    path.write_text(
        """from pathlib import Path
import hashlib
import json
from experiments import _lib

def run(definition, inputs, artifact_root):
    assert set(inputs) == {'corpus', 'queries', 'gold'}
"""
        + ("    inputs['corpus'].write_text('[1]')\n" if mutate_input else "")
        + """    summary = Path(artifact_root) / 'summary.json'
    summary.write_text(json.dumps({'count': definition['workload']['queries']}))
    digest = hashlib.sha256(summary.read_bytes()).hexdigest()
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
        'metrics': {"""
        + ("'ndcg_at_10'" if wrong_metric else "'recall_at_10'")
        + """: 0.7},
        'source': {'url': None, 'locator': str(summary), 'sha256': """
        + ("'0' * 64" if bad_source else "digest")
        + """},
        'comparability': 'candidate_match',
        'reason': None,
    }
""",
        encoding="utf-8",
    )


def _plan(
    tmp_path: Path,
    monkeypatch,
    *,
    bad_source: bool = False,
    wrong_metric: bool = False,
    mutate_input: bool = False,
) -> Path:
    adapter_dir = tmp_path / "adapter"
    adapter_dir.mkdir()
    adapter = adapter_dir / "cell.py"
    _adapter(
        adapter,
        bad_source=bad_source,
        wrong_metric=wrong_metric,
        mutate_input=mutate_input,
    )
    monkeypatch.setenv("PYTHONPATH", str(ROOT))
    corpus = tmp_path / "corpus.json"
    queries = tmp_path / "queries.json"
    gold = tmp_path / "gold.json"
    corpus.write_text("[]", encoding="utf-8")
    queries.write_text("[]", encoding="utf-8")
    gold.write_text("[]", encoding="utf-8")
    definition = json.loads(
        (ROOT / "experiments/configs/competitive/run.example.json").read_text()
    )
    definition["workload"]["corpus_sha256"] = _sha(corpus)
    definition["workload"]["queries_sha256"] = _sha(queries)
    plan = {
        "schema_version": "competitive.experiment.v1",
        "program_track": None,
        "run": definition,
        "adapter": {
            "protocol": "competitive.adapter.v1",
            "id": "fixture-retrieval",
            "path": str(adapter),
            "sha256": _sha(adapter),
            "timeout_s": 30,
        },
        "inputs": {
            "corpus": {"path": str(corpus), "sha256": _sha(corpus)},
            "queries": {"path": str(queries), "sha256": _sha(queries)},
            "gold": {"path": str(gold), "sha256": _sha(gold)},
        },
        "output": {
            "schema_version": "competitive.result.v1",
            "metrics": ["recall_at_10"],
        },
    }
    path = tmp_path / "experiment.json"
    path.write_text(json.dumps(plan), encoding="utf-8")
    return path


def test_preflight_checks_all_inputs_without_creating_output(tmp_path, monkeypatch):
    plan = _plan(tmp_path, monkeypatch)
    output = tmp_path / "output"
    resolved = competitive_runner.preflight(plan, output)
    assert (
        resolved["adapter"]["sha256"]
        == json.loads(plan.read_text())["adapter"]["sha256"]
    )
    assert resolved["inputs"]["corpus"]["sha256"] == _sha(tmp_path / "corpus.json")
    assert not output.exists()
    (tmp_path / "queries.json").write_text("[1]", encoding="utf-8")
    with pytest.raises(ValueError, match="queries.*digest"):
        competitive_runner.preflight(plan, output)


def test_preflight_reads_adapter_without_importing_it(tmp_path, monkeypatch):
    path = _plan(tmp_path, monkeypatch)
    plan = json.loads(path.read_text())
    adapter = Path(plan["adapter"]["path"])
    marker = tmp_path / "imported"
    adapter.write_text(
        f"from pathlib import Path\nPath({str(marker)!r}).write_text('yes')\n"
        + adapter.read_text()
    )
    plan["adapter"]["sha256"] = _sha(adapter)
    path.write_text(json.dumps(plan), encoding="utf-8")
    competitive_runner.preflight(path, tmp_path / "output")
    assert not marker.exists()


def test_cli_preflight_and_run_use_the_same_config(tmp_path, monkeypatch, capsys):
    plan = _plan(tmp_path, monkeypatch)
    output = tmp_path / "output"
    assert (
        competitive_runner.main(["preflight", str(plan), "--output-root", str(output)])
        == 0
    )
    assert (
        json.loads(capsys.readouterr().out)["schema_version"]
        == "competitive.experiment.v1"
    )
    assert not output.exists()
    receipts = tmp_path / "receipts"
    assert (
        competitive_runner.main(
            [
                "run",
                str(plan),
                "--output-root",
                str(output),
                "--receipts-root",
                str(receipts),
            ]
        )
        == 0
    )
    run_id = capsys.readouterr().out.strip()
    assert (receipts / "runs" / run_id / "record.json").is_file()


def test_runner_rejects_adapter_protocol_drift(tmp_path, monkeypatch):
    path = _plan(tmp_path, monkeypatch)
    plan = json.loads(path.read_text())
    plan["adapter"]["protocol"] = "competitive.adapter.v2"
    path.write_text(json.dumps(plan), encoding="utf-8")
    with pytest.raises(ValueError, match="adapter protocol"):
        competitive_runner.load_plan(path)


def test_runner_rejects_output_metric_contract_drift(tmp_path, monkeypatch):
    path = _plan(tmp_path, monkeypatch)
    plan = json.loads(path.read_text())
    plan["output"]["metrics"] = ["p95_ms"]
    path.write_text(json.dumps(plan), encoding="utf-8")
    with pytest.raises(ValueError, match="output metric"):
        competitive_runner.load_plan(path)


def test_run_repeats_same_definition_and_records_each_attempt(tmp_path, monkeypatch):
    plan = _plan(tmp_path, monkeypatch)
    receipts = tmp_path / "receipts"
    first_id, first_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "output-1",
        receipts,
        ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
    )
    second_id, second_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "output-2",
        receipts,
        ts=datetime(2026, 10, 12, 12, 0, 30, tzinfo=UTC),
    )
    assert first_id != second_id
    first = json.loads((first_dir / "record.json").read_text())
    second = json.loads((second_dir / "record.json").read_text())
    assert first["config"]["sha256"] == second["config"]["sha256"]
    assert first["metrics"]["competitive_result"]["status"] == "complete"
    witness = json.loads((tmp_path / "output-1" / "execution.json").read_text())
    assert first["metrics"]["competitive_execution"] == witness
    assert witness["schema_version"] == "competitive.execution.v1"
    assert witness["config_sha256"] == first["config"]["sha256"]
    assert witness["result_sha256"] == _sha(tmp_path / "output-1" / "result.json")
    assert witness["inputs"]["gold"]["sha256"] == _sha(tmp_path / "gold.json")
    assert (tmp_path / "output-1" / "stdout.log").exists()
    assert (tmp_path / "output-1" / "stderr.log").exists()
    assert len((receipts / "index.jsonl").read_text().splitlines()) == 2
    scoreboard = competitive_run.render_scoreboards(receipts / "runs")
    assert first["config"]["sha256"][:12] in scoreboard
    assert witness["plan_sha256"][:12] in scoreboard
    assert witness["adapter"]["sha256"][:12] in scoreboard
    assert "fixture-retrieval" in scoreboard


def test_bad_adapter_source_becomes_blocked_receipt(tmp_path, monkeypatch):
    plan = _plan(tmp_path, monkeypatch, bad_source=True)
    receipts = tmp_path / "receipts"
    _, run_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "output",
        receipts,
        ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    assert record["verdict"] == "blocked"
    assert record["metrics"]["competitive_result"]["metrics"] == {}
    assert record["metrics"]["competitive_result"]["reason"] == "adapter_source_invalid"
    assert len((receipts / "index.jsonl").read_text().splitlines()) == 1


def test_cli_signals_blocked_attempt_after_recording_it(tmp_path, monkeypatch, capsys):
    plan = _plan(tmp_path, monkeypatch, bad_source=True)
    output = tmp_path / "output"
    receipts = tmp_path / "receipts"
    assert (
        competitive_runner.main(
            [
                "run",
                str(plan),
                "--output-root",
                str(output),
                "--receipts-root",
                str(receipts),
            ]
        )
        == 1
    )
    run_id = capsys.readouterr().out.strip()
    assert (receipts / "runs" / run_id / "record.json").is_file()


def test_wrong_aggregate_metric_becomes_blocked_receipt(tmp_path, monkeypatch):
    plan = _plan(tmp_path, monkeypatch, wrong_metric=True)
    _, run_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "output",
        tmp_path / "receipts",
        ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    assert record["metrics"]["competitive_result"]["status"] == "blocked"
    assert record["metrics"]["competitive_result"]["reason"] == "adapter_result_invalid"


def test_input_changed_during_run_becomes_blocked_receipt(tmp_path, monkeypatch):
    plan = _plan(tmp_path, monkeypatch, mutate_input=True)
    _, run_dir = competitive_runner.run_experiment(
        plan,
        tmp_path / "output",
        tmp_path / "receipts",
        ts=datetime(2026, 10, 12, 12, 0, tzinfo=UTC),
    )
    record = json.loads((run_dir / "record.json").read_text())
    assert record["metrics"]["competitive_result"]["status"] == "blocked"
    assert record["metrics"]["competitive_result"]["reason"] == "input_mutated"

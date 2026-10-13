"""Contracts for the versioned cross-SDK benchmark coordinator."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path

import pytest

from experiments import sdk_answer, sdk_benchmark, sdk_scoreboard


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _fixture(tmp_path: Path) -> Path:
    fixture = tmp_path / "fixture.json"
    fixture.write_text(
        json.dumps(
            {
                "schema_version": "sdk.fixture.v1",
                "corpus": [{"id": "doc-1", "body": "amber otter"}],
                "queries": [{"id": "q1", "text": "amber"}],
                "gold": {"q1": ["doc-1"]},
            }
        ),
        encoding="utf-8",
    )
    return fixture


def _plan(tmp_path: Path) -> Path:
    fixture = _fixture(tmp_path)
    plan = {
        "schema_version": "sdk.experiment.v1",
        "program_track": None,
        "suite_id": "synthetic-fts-v1",
        "fixtures": {"manifest": {"path": str(fixture), "sha256": _digest(fixture)}},
        "arms": [
            {
                "sdk": "python",
                "artifact": {"path": str(fixture), "sha256": _digest(fixture)},
            }
        ],
        "cases": [
            {
                "id": "fts-top10",
                "family": "retrieval",
                "operation_id": "engine.search_text_only",
                "operation_version": 1,
                "required": True,
                "arguments": {
                    "query_set": "all",
                    "limit": 10,
                    "view": {"valid_as_of": 1700000000},
                },
                "setup": "canonical-nodes-v1",
                "boundary": "sdk_call_to_materialized_result",
            }
        ],
        "sampling": {
            "warmup_per_query": 0,
            "repetitions": 1,
            "seed": 7,
            "cache_state": "steady_after_declared_warmup",
            "concurrency": 1,
            "order": "balanced",
        },
        "output": {"result_schema": "sdk.result.v1", "metrics": ["recall_at_10"]},
    }
    path = tmp_path / "sdk-experiment.json"
    path.write_text(json.dumps(plan), encoding="utf-8")
    return path


def test_closed_plan_and_temporal_instant(tmp_path: Path):
    path = _plan(tmp_path)
    plan, digest = sdk_benchmark.load_plan(path)
    assert plan["schema_version"] == "sdk.experiment.v1"
    assert digest == _digest(path)

    altered = json.loads(path.read_text(encoding="utf-8"))
    altered["cases"][0]["arguments"]["view"] = {}
    path.write_text(json.dumps(altered), encoding="utf-8")
    with pytest.raises(ValueError, match="valid_as_of"):
        sdk_benchmark.load_plan(path)

    altered["cases"][0]["arguments"]["view"] = {"valid_as_of": 1700000000}
    altered["surprise"] = "ignored?"
    path.write_text(json.dumps(altered), encoding="utf-8")
    with pytest.raises(ValueError, match="unknown|fields"):
        sdk_benchmark.load_plan(path)


def test_preflight_detects_fixture_drift_without_creating_output(tmp_path: Path):
    path = _plan(tmp_path)
    output = tmp_path / "output"
    sdk_benchmark.preflight(path, output)
    assert not output.exists()
    fixture = tmp_path / "fixture.json"
    fixture.write_text("{}", encoding="utf-8")
    with pytest.raises(ValueError, match="digest"):
        sdk_benchmark.preflight(path, output)


@pytest.mark.parametrize(
    ("operation_id", "arguments"),
    [
        ("read.get", {"view": {"valid_as_of": 1700000000}}),
        ("graph.neighbors", {"depth": 1, "view": {"valid_as_of": 1700000000}}),
    ],
)
def test_registered_read_and_graph_shapes(
    tmp_path: Path, operation_id: str, arguments: dict
):
    path = _plan(tmp_path)
    plan = json.loads(path.read_text())
    plan["cases"][0]["operation_id"] = operation_id
    plan["cases"][0]["arguments"] = arguments
    path.write_text(json.dumps(plan))
    loaded, _ = sdk_benchmark.load_plan(path)
    assert loaded["cases"][0]["operation_id"] == operation_id


def test_rejects_cold_warmup_and_answer_without_pipeline(tmp_path: Path):
    path = _plan(tmp_path)
    plan = json.loads(path.read_text())
    plan["sampling"]["cache_state"] = "first_pass_after_populated_reopen"
    plan["sampling"]["warmup_per_query"] = 1
    path.write_text(json.dumps(plan))
    with pytest.raises(ValueError, match="warmup"):
        sdk_benchmark.load_plan(path)

    plan["sampling"]["warmup_per_query"] = 0
    plan["cases"][0]["family"] = "answer"
    plan["output"]["metrics"] = ["answer_accuracy"]
    path.write_text(json.dumps(plan))
    with pytest.raises(ValueError, match="answer"):
        sdk_benchmark.load_plan(path)


def test_aggregate_rejects_duplicate_or_inconsistent_query_samples(tmp_path: Path):
    fixture = json.loads(_fixture(tmp_path).read_text())
    case = json.loads(_plan(tmp_path).read_text())["cases"][0]
    sample = {"query_id": "q1", "elapsed_ns": 100, "ids": ["l:doc-1"]}
    with pytest.raises(ValueError, match="duplicate|repetition|inconsistent"):
        sdk_benchmark._aggregate(case, [sample, {**sample, "ids": []}], fixture)


def test_latency_requires_exact_oracle_before_reporting_speed(tmp_path: Path):
    fixture = json.loads(_fixture(tmp_path).read_text())
    case = json.loads(_plan(tmp_path).read_text())["cases"][0]
    case["family"] = "latency_capacity"
    with pytest.raises(ValueError, match="oracle"):
        sdk_benchmark._check_oracle(case, fixture, {"q1": ["l:doc-1"]})
    fixture["oracles"] = {"read.get": {"q1": ["l:doc-1"]}}
    case["operation_id"] = "read.get"
    with pytest.raises(ValueError, match="oracle"):
        sdk_benchmark._check_oracle(case, fixture, {"q1": []})
    assert sdk_benchmark._check_oracle(case, fixture, {"q1": ["l:doc-1"]}) == "verified"


def test_unverified_retrieval_does_not_publish_timing(tmp_path: Path):
    fixture = json.loads(_fixture(tmp_path).read_text())
    case = json.loads(_plan(tmp_path).read_text())["cases"][0]
    metrics, ordered = sdk_benchmark._aggregate(
        case, [{"query_id": "q1", "elapsed_ns": 100, "ids": []}], fixture
    )
    assert sdk_benchmark._check_oracle(case, fixture, ordered) == "unverified"
    assert "p50_ms" not in sdk_benchmark._publishable_metrics(
        case, metrics, "unverified"
    )


def test_undeclared_fathomdb_override_is_rejected(monkeypatch):
    monkeypatch.setenv("FATHOMDB_PERF_READER_PRAGMAS", "cache_size=1")
    with pytest.raises(ValueError, match="FATHOMDB_PERF_READER_PRAGMAS"):
        sdk_benchmark._check_treatment_environment()


def test_node_runtime_override_is_rejected(monkeypatch):
    monkeypatch.setenv("NODE_OPTIONS", "--jitless")
    with pytest.raises(ValueError, match="NODE_OPTIONS"):
        sdk_benchmark._check_treatment_environment()


def test_sdk_wrapper_digest_changes_with_public_source(tmp_path: Path):
    package = tmp_path / "package"
    package.mkdir()
    source = package / "index.js"
    source.write_text("export const answer = 1")
    original = sdk_benchmark._wrapper_digest(package, {".js"})
    source.write_text("export const answer = 2")
    assert sdk_benchmark._wrapper_digest(package, {".js"}) != original


def test_python_wrapper_digest_tracks_loaded_package(tmp_path: Path):
    package = tmp_path / "fathomdb"
    package.mkdir()
    (package / "__init__.py").write_text("version = 1")
    native = package / "_fathomdb.abi3.so"
    native.write_bytes(b"binary")
    original = sdk_benchmark.wrapper_sha256("python", native_path=native)
    (package / "__init__.py").write_text("version = 2")
    assert sdk_benchmark.wrapper_sha256("python", native_path=native) != original


def test_observed_bootstrap_identity_uses_loaded_python_package():
    from fathomdb import _fathomdb

    observed = sdk_benchmark._bootstrap_identity()
    assert observed["native_sha256"] == _digest(Path(_fathomdb.__file__))
    assert observed["wrapper_sha256"] == sdk_benchmark.wrapper_sha256(
        "python", native_path=_fathomdb.__file__
    )


def test_crashed_driver_descendants_are_killed(tmp_path: Path, monkeypatch):
    class CrashedProcess:
        pid = 424242
        returncode = 1

        def communicate(self, timeout=None):
            return "", "failed"

    signals = []
    monkeypatch.setattr(sdk_benchmark, "_verified_binding", lambda *_: tmp_path / "x")
    monkeypatch.setattr(sdk_benchmark, "_driver_command", lambda *_: ["driver"])
    monkeypatch.setattr(
        sdk_benchmark.subprocess, "Popen", lambda *_a, **_k: CrashedProcess()
    )
    monkeypatch.setattr(
        sdk_benchmark.os, "killpg", lambda pid, sig: signals.append(sig)
    )
    with pytest.raises(ValueError, match="failed"):
        sdk_benchmark._invoke_driver(
            {"sdk": "rust", **{name: {} for name in ("runtime", "driver", "native")}},
            {},
            tmp_path,
            1,
        )
    assert sdk_benchmark.signal.SIGKILL in signals


def test_setup_failure_still_writes_blocked_cell_receipt(tmp_path: Path, monkeypatch):
    path = _plan(tmp_path)
    plan, plan_sha = sdk_benchmark.load_plan(path)
    fixture = json.loads(_fixture(tmp_path).read_text())
    arm = {
        "sdk": "python",
        "engine_revision": "local-synthetic-0.8.27",
        "sdk_version": "0.8.27",
        "dependency_closure_sha256": "0" * 64,
        "native": {"sha256": "1" * 64},
        "driver": {"sha256": "2" * 64},
    }
    monkeypatch.setattr(
        sdk_benchmark,
        "_invoke_driver",
        lambda *_args: {
            "status": "complete",
            "operations": [
                {"operation_id": "engine.search_text_only", "operation_version": 1}
            ],
        },
    )
    monkeypatch.setattr(
        sdk_benchmark,
        "_prepare_repetition",
        lambda *_args: (_ for _ in ()).throw(RuntimeError("doctor failed")),
    )
    output = tmp_path / "output"
    output.mkdir()
    result = sdk_benchmark._cell(
        plan=plan,
        plan_sha=plan_sha,
        case=plan["cases"][0],
        arm_binding={"artifact": {"sha256": "3" * 64}},
        arm=arm,
        fixture=fixture,
        output=output,
        receipts_root=tmp_path / "receipts",
        suite_attempt_id="test-nonce",
        ts=datetime(2026, 10, 13, tzinfo=UTC),
    )
    assert result["status"] == "blocked"
    assert len((tmp_path / "receipts/index.jsonl").read_text().splitlines()) == 1


def test_pinned_artifacts_are_rechecked_after_measurement(tmp_path: Path):
    artifact = _fixture(tmp_path)
    binding = {"path": str(artifact), "sha256": _digest(artifact)}
    plan = {"fixtures": {"manifest": binding}}
    arm_binding = {"artifact": binding}
    arm = {name: binding for name in ("runtime", "driver", "native", "bootstrap_cli")}
    arm["sdk"] = "rust"
    arm["wrapper_sha256"] = binding["sha256"]
    sdk_benchmark._verify_pinned(plan, arm_binding, arm)
    artifact.write_text("changed during execution", encoding="utf-8")
    with pytest.raises(ValueError, match="digest drift"):
        sdk_benchmark._verify_pinned(plan, arm_binding, arm)


def test_validation_resolves_arm_manifest_without_creating_output(tmp_path: Path):
    path = _plan(tmp_path)
    output = tmp_path / "never-created"
    with pytest.raises(ValueError, match="arm|fields"):
        sdk_benchmark.validate_experiment(path, output)
    assert not output.exists()


def test_retrieval_metrics_obey_rank_cutoff(tmp_path: Path):
    fixture = json.loads(_fixture(tmp_path).read_text())
    case = json.loads(_plan(tmp_path).read_text())["cases"][0]
    case["arguments"]["limit"] = 20
    ids = [f"l:irrelevant-{index}" for index in range(10)] + ["l:doc-1"]
    metrics, _ = sdk_benchmark._aggregate(
        case, [{"query_id": "q1", "elapsed_ns": 100, "ids": ids}], fixture
    )
    assert metrics["recall_at_10"] == 0.0
    assert metrics["ndcg_at_10"] == 0.0
    assert metrics["mrr"] == pytest.approx(1 / 11)


def test_one_suite_can_declare_latency_and_retrieval_metrics(tmp_path: Path):
    path = _plan(tmp_path)
    plan = json.loads(path.read_text())
    latency_case = {
        **plan["cases"][0],
        "id": "fts-latency",
        "family": "latency_capacity",
    }
    plan["cases"].append(latency_case)
    plan["output"]["metrics"] = ["recall_at_10", "p50_ms"]
    path.write_text(json.dumps(plan))
    resolved, _ = sdk_benchmark.load_plan(path)
    assert len(resolved["cases"]) == 2


def test_seeded_balanced_arm_order_interleaves_repetitions(tmp_path: Path, monkeypatch):
    path = _plan(tmp_path)
    plan, plan_sha = sdk_benchmark.load_plan(path)
    plan["arms"] = [
        {"sdk": sdk, "artifact": {"sha256": "3" * 64}}
        for sdk in ("python", "typescript")
    ]
    plan["sampling"]["repetitions"] = 3
    plan["cases"].append(
        {**plan["cases"][0], "id": "fts-latency", "family": "latency_capacity"}
    )
    arms = {
        sdk: {"sdk": sdk, "native": {"sha256": "1" * 64}, "sdk_version": "test"}
        for sdk in ("python", "typescript")
    }
    monkeypatch.setattr(
        sdk_benchmark,
        "validate_experiment",
        lambda *_: {"plan": plan, "plan_sha256": plan_sha},
    )
    monkeypatch.setattr(sdk_benchmark, "_load_arm", lambda sdk, *_: arms[sdk])
    monkeypatch.setattr(sdk_benchmark, "_verify_pinned", lambda *_: None)
    monkeypatch.setattr(
        sdk_benchmark,
        "_prepare_repetition",
        lambda root, arm, fixture, rep: (root / "db", {}),
    )
    executions = []
    persisted = []
    persisted_results = []

    def invoke(arm, request, root, timeout):
        if request["mode"] == "capabilities":
            return {
                "status": "complete",
                "operations": [
                    {"operation_id": case["operation_id"], "operation_version": 1}
                    for case in plan["cases"]
                    if case["family"] != "answer"
                ],
            }
        executions.append((root.parent.name, arm["sdk"], root.name))
        return {
            "status": "complete",
            "operation_id": request["operation_id"],
            "operation_version": 1,
            "identity": {"native_sha256": "1" * 64, "sdk_version": "test"},
            "effective_options": request["arguments"],
            "samples": [{"query_id": "q1", "elapsed_ns": 100, "ids": ["l:doc-1"]}],
        }

    def persist(**kwargs):
        persisted.append((kwargs["case"]["id"], kwargs["arm"]["sdk"]))
        persisted_results.append(kwargs["result"])
        return {
            "case_id": kwargs["case"]["id"],
            "sdk": kwargs["arm"]["sdk"],
            "status": "complete",
        }

    monkeypatch.setattr(sdk_benchmark, "_invoke_driver", invoke)
    monkeypatch.setattr(sdk_benchmark, "_persist_cell", persist)
    sdk_benchmark.run_experiment(path, tmp_path / "out-1", tmp_path / "receipts")
    first = executions[:]
    assert [case.rsplit("-", 1)[0] for case, _, _ in first[:12]] == [
        "fts-top10"
    ] * 6 + ["fts-latency"] * 6
    assert len(first) == 12
    for offset in (0, 6):
        rounds = [first[offset + index : offset + index + 2] for index in (0, 2, 4)]
        assert all(
            {item[1] for item in round_} == {"python", "typescript"}
            for round_ in rounds
        )
        assert rounds[0][0][1] != rounds[1][0][1]
        assert rounds[0][0][1] == rounds[2][0][1]
    assert sorted(persisted) == sorted(
        (case["id"], sdk) for case in plan["cases"] for sdk in arms
    )
    for result in persisted_results:
        if result["family"] == "retrieval":
            evidence = (
                tmp_path
                / "out-1"
                / f"{result['case_id']}-{result['sdk']}"
                / "evidence.json"
            )
            assert result["evidence_sha256"] == _digest(evidence)
    executions.clear()
    sdk_benchmark.run_experiment(path, tmp_path / "out-2", tmp_path / "receipts")
    assert executions == first


def test_answer_rejects_retrieval_evidence_changed_after_result(
    tmp_path: Path, monkeypatch
):
    plan_path = _plan(tmp_path)
    plan, plan_sha = sdk_benchmark.load_plan(plan_path)
    gold_path = tmp_path / "gold.json"
    gold_path.write_text("{}")
    plan["fixtures"]["answer_gold"] = {
        "path": str(gold_path),
        "sha256": _digest(gold_path),
    }
    output = tmp_path / "output"
    upstream = output / "fts-top10-python"
    upstream.mkdir(parents=True)
    evidence = upstream / "evidence.json"
    evidence.write_text('{"questions": []}')
    (upstream / "result.json").write_text(
        json.dumps({"evidence_sha256": _digest(evidence)})
    )
    evidence.write_text('{"questions": [{"id": "q1"}]}')
    monkeypatch.setattr(sdk_benchmark, "_verify_pinned", lambda *_: None)
    monkeypatch.setattr(
        sdk_answer,
        "run_answer_case",
        lambda **_: pytest.fail("tampered evidence reached scorer"),
    )
    captured = []
    monkeypatch.setattr(
        sdk_benchmark,
        "_persist_cell",
        lambda **kwargs: captured.append(kwargs["result"]) or kwargs["result"],
    )
    result = sdk_benchmark._answer_cell(
        plan=plan,
        plan_sha=plan_sha,
        case={
            "id": "answer-fts",
            "family": "answer",
            "operation_id": "answer_pipeline.first_sentence_v1",
            "operation_version": 1,
            "arguments": {"answer_config": {"upstream": {"case_id": "fts-top10"}}},
        },
        arm_binding=plan["arms"][0],
        arm={"sdk": "python"},
        output=output,
        receipts_root=tmp_path / "receipts",
        suite_attempt_id="attempt",
        ts=datetime(2026, 10, 13, tzinfo=UTC),
    )
    assert result["status"] == "blocked"
    assert result["reason"] == "ValueError"
    assert captured == [result]


def test_answer_plan_rejects_unimplemented_repetitions(tmp_path: Path, monkeypatch):
    path = _plan(tmp_path)
    plan = json.loads(path.read_text())
    plan["fixtures"]["answer_gold"] = plan["fixtures"]["manifest"]
    plan["cases"].append(
        {
            "id": "answer-fts",
            "family": "answer",
            "operation_id": "answer_pipeline.first_sentence_v1",
            "operation_version": 1,
            "required": True,
            "arguments": {
                "answer_config": {
                    "upstream": {"case_id": "fts-top10"},
                    "gold_sha256": plan["fixtures"]["answer_gold"]["sha256"],
                }
            },
            "setup": "bound-retrieval-v1",
            "boundary": "answer_and_judge",
        }
    )
    plan["sampling"]["repetitions"] = 2
    plan["output"]["metrics"].append("answer_accuracy")
    path.write_text(json.dumps(plan))
    monkeypatch.setattr(sdk_answer, "validate_answer_config", lambda *_: "0" * 64)
    with pytest.raises(ValueError, match="answer.*repetition"):
        sdk_benchmark.load_plan(path)


def test_real_python_fts_rerun_has_distinct_safe_receipts(tmp_path: Path):
    from fathomdb import _fathomdb

    root = Path(__file__).resolve().parents[2]
    cli = root / "target/debug/fathomdb"
    driver = root / "experiments/sdk_drivers/python_driver.py"
    if not cli.is_file():
        pytest.skip("local FathomDB CLI is required")
    assert driver.is_file()
    native = Path(_fathomdb.__file__)
    runtime = Path(sys.executable).resolve()
    arm_manifest = {
        "schema_version": "sdk.arm.v1",
        "sdk": "python",
        "sdk_version": "0.8.27",
        "engine_revision": "local-synthetic-0.8.27",
        "dependency_closure_sha256": "0" * 64,
        "wrapper_sha256": sdk_benchmark.wrapper_sha256("python"),
        "features": [],
        "device": "cpu",
        "runtime": {"path": str(runtime), "sha256": _digest(runtime)},
        "driver": {"path": str(driver), "sha256": _digest(driver)},
        "native": {"path": str(native), "sha256": _digest(native)},
        "bootstrap_cli": {"path": str(cli), "sha256": _digest(cli)},
    }
    arm_path = tmp_path / "arm.json"
    arm_path.write_text(json.dumps(arm_manifest), encoding="utf-8")
    plan_path = _plan(tmp_path)
    plan = json.loads(plan_path.read_text(encoding="utf-8"))
    plan["arms"][0]["artifact"] = {"path": str(arm_path), "sha256": _digest(arm_path)}
    plan_path.write_text(json.dumps(plan), encoding="utf-8")

    ts = datetime(2026, 10, 13, 12, 0, tzinfo=UTC)
    receipts = tmp_path / "receipts"
    first = sdk_benchmark.run_experiment(plan_path, tmp_path / "out-1", receipts, ts=ts)
    second = sdk_benchmark.run_experiment(
        plan_path, tmp_path / "out-2", receipts, ts=ts
    )
    assert first["suite_attempt_id"] != second["suite_attempt_id"]
    assert first["cells"][0]["status"] == second["cells"][0]["status"] == "complete"
    first_id, second_id = first["cells"][0]["run_id"], second["cells"][0]["run_id"]
    assert first_id != second_id
    records = [
        json.loads((receipts / "runs" / item / "record.json").read_text())
        for item in (first_id, second_id)
    ]
    assert records[0]["config"]["sha256"] == records[1]["config"]["sha256"]
    assert len((receipts / "index.jsonl").read_text().splitlines()) == 2
    for record in records:
        safe_bytes = json.dumps(record)
        assert str(tmp_path) not in safe_bytes
        assert "amber otter" not in safe_bytes
    evidence = json.loads(
        (tmp_path / "out-1" / "fts-top10-python" / "result.json").read_text()
    )
    assert evidence["ordered_ids"] == {"q1": ["l:doc-1"]}


def test_real_three_sdk_fts_suite_has_equal_ordered_ids(tmp_path: Path):
    from fathomdb import _fathomdb

    root = Path(__file__).resolve().parents[2]
    cli = root / "target/debug/fathomdb"
    ts_native = root / "src/ts/fathomdb.linux-x64-gnu.node"
    rust_binary = (
        root / "experiments/sdk_drivers/rust/target/debug/fathomdb-sdk-benchmark-driver"
    )
    node_text = os.environ.get("FATHOMDB_BENCH_NODE") or shutil.which("node")
    if not node_text:
        pytest.skip("Node 25 runtime is unavailable")
    node = Path(node_text).resolve()
    version = subprocess.run(
        [str(node), "--version"], capture_output=True, text=True, check=True
    ).stdout
    if not version.startswith("v25.") or any(
        not path.is_file() for path in (cli, ts_native, rust_binary)
    ):
        pytest.skip("three locally built SDK artifacts and Node 25 are required")

    driver_by_sdk = {
        "python": root / "experiments/sdk_drivers/python_driver.py",
        "typescript": root / "experiments/sdk_drivers/typescript_driver.mjs",
        "rust": rust_binary,
    }
    native_by_sdk = {
        "python": Path(_fathomdb.__file__),
        "typescript": ts_native,
        "rust": rust_binary,
    }
    runtime_by_sdk = {
        "python": Path(sys.executable).resolve(),
        "typescript": node,
        "rust": rust_binary,
    }
    plan_path = _plan(tmp_path)
    plan = json.loads(plan_path.read_text())
    plan["arms"] = []
    for sdk in ("python", "typescript", "rust"):
        arm = {
            "schema_version": "sdk.arm.v1",
            "sdk": sdk,
            "sdk_version": "0.8.27",
            "engine_revision": "local-synthetic-0.8.27",
            "dependency_closure_sha256": "0" * 64,
            "wrapper_sha256": sdk_benchmark.wrapper_sha256(
                sdk, _digest(native_by_sdk[sdk])
            ),
            "features": [],
            "device": "cpu",
            "runtime": {
                "path": str(runtime_by_sdk[sdk]),
                "sha256": _digest(runtime_by_sdk[sdk]),
            },
            "driver": {
                "path": str(driver_by_sdk[sdk]),
                "sha256": _digest(driver_by_sdk[sdk]),
            },
            "native": {
                "path": str(native_by_sdk[sdk]),
                "sha256": _digest(native_by_sdk[sdk]),
            },
            "bootstrap_cli": {"path": str(cli), "sha256": _digest(cli)},
        }
        arm_path = tmp_path / f"{sdk}-arm.json"
        arm_path.write_text(json.dumps(arm))
        plan["arms"].append(
            {
                "sdk": sdk,
                "artifact": {"path": str(arm_path), "sha256": _digest(arm_path)},
            }
        )
    plan_path.write_text(json.dumps(plan))
    manifest = sdk_benchmark.run_experiment(
        plan_path, tmp_path / "suite", tmp_path / "receipts"
    )
    assert [cell["status"] for cell in manifest["cells"]] == ["complete"] * 3
    ordered = [
        json.loads(
            (tmp_path / "suite" / f"fts-top10-{sdk}" / "result.json").read_text()
        )["ordered_ids"]
        for sdk in ("python", "typescript", "rust")
    ]
    assert ordered == [{"q1": ["l:doc-1"]}] * 3
    report = sdk_scoreboard.build_scoreboard(
        plan_path, tmp_path / "suite/suite-manifest.json", tmp_path / "receipts"
    )
    assert report["suite_status"] == "complete"
    assert len(report["families"]["retrieval"]["sdk"]) == 3


def test_answer_case_binds_same_attempt_retrieval_and_scores(tmp_path: Path):
    from fathomdb import _fathomdb

    root = Path(__file__).resolve().parents[2]
    cli = root / "target/debug/fathomdb"
    if not cli.is_file():
        pytest.skip("local CLI required")
    gold = {
        "schema_version": "sdk.answer-gold.v1",
        "questions": [
            {
                "id": "q1",
                "question": "What phrase?",
                "answer": "amber otter",
                "source_ids": ["sdk:doc-1"],
                "labels": ["factoid"],
            }
        ],
    }
    gold_path = tmp_path / "answer-gold.json"
    gold_path.write_text(json.dumps(gold))
    plan_path = _plan(tmp_path)
    plan = json.loads(plan_path.read_text())
    plan["fixtures"]["answer_gold"] = {
        "path": str(gold_path),
        "sha256": _digest(gold_path),
    }
    answer_config = {
        "schema_version": "sdk.answer-config.v1",
        "upstream": {"case_id": "fts-top10", "arm_relation": "same_sdk"},
        "evidence_transform": "ordered_snippets_v1",
        "answerer": "first_sentence_v1",
        "prompt": sdk_answer.PROMPT,
        "context": {"max_hits": 1, "max_chars": 100},
        "decoding": {"temperature": 0, "seed": 0, "max_output_chars": 100},
        "judge": "exact_token_f1_grounding_v1",
        "gold_sha256": _digest(gold_path),
        "question_order": ["q1"],
        "cost_ceiling_usd": 0,
    }
    plan["cases"].append(
        {
            "id": "answer-fts",
            "family": "answer",
            "operation_id": "answer_pipeline.first_sentence_v1",
            "operation_version": 1,
            "required": True,
            "arguments": {"answer_config": answer_config},
            "setup": "bound-retrieval-v1",
            "boundary": "answer_and_judge",
        }
    )
    plan["output"]["metrics"] = ["recall_at_10", "answer_accuracy"]
    driver = root / "experiments/sdk_drivers/python_driver.py"
    native = Path(_fathomdb.__file__)
    runtime = Path(sys.executable).resolve()
    arm = {
        "schema_version": "sdk.arm.v1",
        "sdk": "python",
        "sdk_version": "0.8.27",
        "engine_revision": "local-synthetic-0.8.27",
        "dependency_closure_sha256": "0" * 64,
        "wrapper_sha256": sdk_benchmark.wrapper_sha256("python"),
        "features": [],
        "device": "cpu",
        "runtime": {"path": str(runtime), "sha256": _digest(runtime)},
        "driver": {"path": str(driver), "sha256": _digest(driver)},
        "native": {"path": str(native), "sha256": _digest(native)},
        "bootstrap_cli": {"path": str(cli), "sha256": _digest(cli)},
    }
    arm_path = tmp_path / "python-arm.json"
    arm_path.write_text(json.dumps(arm))
    plan["arms"][0]["artifact"] = {"path": str(arm_path), "sha256": _digest(arm_path)}
    plan_path.write_text(json.dumps(plan))

    first = sdk_benchmark.run_experiment(
        plan_path, tmp_path / "one", tmp_path / "receipts"
    )
    second = sdk_benchmark.run_experiment(
        plan_path, tmp_path / "two", tmp_path / "receipts"
    )
    assert [cell["status"] for cell in first["cells"]] == ["complete", "complete"]
    assert [cell["status"] for cell in second["cells"]] == ["complete", "complete"]
    first_answer = json.loads(
        (tmp_path / "one/answer-fts-python/result.json").read_text()
    )
    second_answer = json.loads(
        (tmp_path / "two/answer-fts-python/result.json").read_text()
    )
    assert first_answer["metrics"]["answer_accuracy"] == 1.0
    assert first_answer["upstream"]["suite_attempt_id"] == first["suite_attempt_id"]
    assert (
        first_answer["upstream"]["retrieval_result_sha256"]
        != second_answer["upstream"]["retrieval_result_sha256"]
    )
    answer_record = json.loads(
        (
            tmp_path / "receipts/runs" / first["cells"][1]["run_id"] / "record.json"
        ).read_text()
    )
    assert "question_order" not in json.dumps(answer_record)
    assert "What phrase?" not in json.dumps(answer_record)
    report = sdk_scoreboard.build_scoreboard(
        plan_path, tmp_path / "one/suite-manifest.json", tmp_path / "receipts"
    )
    assert report["families"]["answer"]["sdk"][0]["status"] == "complete"
    evidence_path = tmp_path / "one/fts-top10-python/evidence.json"
    evidence_path.write_text(
        evidence_path.read_text().replace("amber otter", "changed evidence")
    )
    tampered = sdk_scoreboard.build_scoreboard(
        plan_path, tmp_path / "one/suite-manifest.json", tmp_path / "receipts"
    )
    assert tampered["families"]["retrieval"]["sdk"][0]["status"] == "invalid"
    assert tampered["families"]["answer"]["sdk"][0]["status"] == "invalid"

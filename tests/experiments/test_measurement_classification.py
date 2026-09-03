"""Contract tests for executable experiment measurement classification."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import pytest

from experiments import _lib
from experiments import measurement_classification as mc


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")


def _authority() -> dict:
    return {
        "schema_version": "measurement.plan.v2",
        "plan_id": "native-search-v1",
        "measurement_roots": [
            {
                "source_artifact_id": "metrics",
                "json_pointers": ["/retrieval"],
            }
        ],
        "metric_exclusions": [],
        "components": [
            {
                "id": "engine",
                "name": "FathomDB Engine.search",
                "kind": "fathomdb_engine_search",
            },
            {
                "id": "metric",
                "name": "deterministic retrieval metric",
                "kind": "deterministic_metric",
            },
        ],
        "call_paths": [
            {
                "id": "search",
                "operation": "Engine.search",
                "source_artifact_ids": ["implementation"],
            }
        ],
        "comparisons": [],
        "metric_bindings": [
            {
                "id": "recall",
                "source_artifact_id": "metrics",
                "json_pointer": "/retrieval/recall_at_3",
                "value_type": "number",
                "allowed_values": [],
                "layer": "data_plane",
                "contributing_component_ids": ["engine", "metric"],
                "execution_witness_ids": ["search-call"],
            }
        ],
        "claims": [
            {
                "id": "retrieval-quality",
                "layer": "data_plane",
                "metric_ids": ["recall"],
            }
        ],
    }


def _complete_document(root: Path) -> tuple[dict, dict]:
    metrics_path = root / "metrics.json"
    implementation_path = root / "runner.py"
    _write_json(metrics_path, {"retrieval": {"recall_at_3": 1.0}})
    implementation_path.write_text("engine.search(query)\n", encoding="utf-8")
    authority = _authority()
    document = {
        "schema_version": mc.SCHEMA_VERSION,
        "classifier_version": "1",
        "classification_id": "",
        "run_id": "native-run",
        "outcome": "complete",
        "blocked_reason": None,
        "measurement_plan_id": "native-search-v1",
        "source_artifacts": [
            {
                "id": "metrics",
                "locator_kind": "repository_path",
                "locator": "metrics.json",
                "role": "metrics_payload",
                "sha256": _sha(metrics_path),
                "measurement_root_json_pointers": ["/retrieval"],
            },
            {
                "id": "implementation",
                "locator_kind": "repository_path",
                "locator": "runner.py",
                "role": "implementation",
                "sha256": _sha(implementation_path),
                "measurement_root_json_pointers": [],
            },
        ],
        "components": copy.deepcopy(authority["components"]),
        "call_paths": copy.deepcopy(authority["call_paths"]),
        "execution_witnesses": [
            {
                "id": "search-call",
                "arm_id": None,
                "call_path_id": "search",
                "component_id": "engine",
                "engine_search_state": "executed",
                "call_count": 1,
                "count_semantics": "exact",
                "evidence_kind": "instrumented_call",
                "source_artifact_ids": ["implementation"],
            }
        ],
        "metrics": copy.deepcopy(authority["metric_bindings"]),
        "metric_exclusions": [],
        "comparisons": [],
        "claims": copy.deepcopy(authority["claims"]),
        "migration": {
            "kind": "native",
            "manifest_path": None,
            "manifest_entry_sha256": None,
            "measurement_plan_id": "native-search-v1",
            "measurement_plan_sha256": mc.canonical_sha256(authority),
        },
    }
    document["classification_id"] = mc.classification_id(document)
    return document, authority


def test_valid_complete_classification_is_source_bound_and_plan_complete(tmp_path):
    document, authority = _complete_document(tmp_path)

    mc.validate_classification(document, repository_root=tmp_path, authority=authority)


@pytest.mark.parametrize(
    ("mutation", "message"),
    [
        (lambda row: row.update({"surprise": True}), "unknown keys"),
        (
            lambda row: row["source_artifacts"][1].update(
                {"measurement_root_json_pointers": ["/"]}
            ),
            "evidence-only",
        ),
        (lambda row: row["metrics"].clear(), "measurement plan"),
        (
            lambda row: row["metrics"][0].update({"layer": "end_to_end"}),
            "layer",
        ),
        (
            lambda row: row["execution_witnesses"][0].update(
                {"call_count": 0}
            ),
            "executed",
        ),
    ],
)
def test_complete_classification_rejects_closed_contract_violations(
    tmp_path, mutation, message
):
    document, authority = _complete_document(tmp_path)
    mutation(document)
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match=message):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_source_bound_plan_prevents_hiding_an_answerer(tmp_path):
    document, authority = _complete_document(tmp_path)
    answerer = {"id": "answer", "name": "answer model", "kind": "answer_generator"}
    authority["components"].append(answerer)
    authority["metric_bindings"][0]["contributing_component_ids"].append("answer")
    authority["metric_bindings"][0]["layer"] = "end_to_end"
    authority["claims"][0]["layer"] = "end_to_end"
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="measurement plan"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_enum_measurement_and_unknown_historical_witness_are_valid(tmp_path):
    document, authority = _complete_document(tmp_path)
    answerer = {"id": "answer", "name": "answer model", "kind": "answer_generator"}
    document["components"].append(answerer)
    authority["components"].append(copy.deepcopy(answerer))
    _write_json(tmp_path / "metrics.json", {"lifecycle": {"erasure": "pass"}})
    document["source_artifacts"][0]["sha256"] = _sha(tmp_path / "metrics.json")
    document["source_artifacts"][0]["measurement_root_json_pointers"] = [
        "/lifecycle"
    ]
    authority["measurement_roots"][0]["json_pointers"] = ["/lifecycle"]
    metric = document["metrics"][0]
    metric.update(
        {
            "json_pointer": "/lifecycle/erasure",
            "value_type": "enum",
            "allowed_values": ["pass", "fail"],
            "layer": "end_to_end",
            "contributing_component_ids": ["engine", "metric", "answer"],
            "execution_witness_ids": ["unknown-call"],
        }
    )
    document["execution_witnesses"] = [
        {
            "id": "unknown-call",
            "arm_id": None,
            "call_path_id": "search",
            "component_id": "engine",
            "engine_search_state": "unknown_historical",
            "call_count": None,
            "count_semantics": "unknown",
            "evidence_kind": "immutable_receipt",
            "source_artifact_ids": ["implementation"],
        }
    ]
    authority["metric_bindings"] = copy.deepcopy(document["metrics"])
    authority["metric_bindings"][0]["execution_witness_ids"] = ["unknown-call"]
    document["claims"][0]["layer"] = "end_to_end"
    authority["claims"][0]["layer"] = "end_to_end"
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)

    mc.validate_classification(document, repository_root=tmp_path, authority=authority)


def test_comparison_sets_are_derived_not_narrated(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["components"].append(
        {"id": "planner", "name": "query planner", "kind": "query_planner"}
    )
    authority["components"] = copy.deepcopy(document["components"])
    document["execution_witnesses"][0]["arm_id"] = "control"
    treatment_witness = copy.deepcopy(document["execution_witnesses"][0])
    treatment_witness.update({"id": "treatment-call", "arm_id": "treatment"})
    document["execution_witnesses"].append(treatment_witness)
    comparison = {
        "id": "arms",
        "arms": [
            {
                "id": "control",
                "component_ids": ["engine", "metric"],
                "execution_witness_ids": ["search-call"],
            },
            {
                "id": "treatment",
                "component_ids": ["engine", "metric", "planner"],
                "execution_witness_ids": ["treatment-call"],
            },
        ],
        "shared_component_ids": ["engine", "metric"],
        "differing_component_ids": ["planner"],
    }
    document["comparisons"] = [comparison]
    authority["comparisons"] = [copy.deepcopy(comparison)]
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)
    mc.validate_classification(document, repository_root=tmp_path, authority=authority)

    document["comparisons"][0]["differing_component_ids"] = []
    authority["comparisons"][0]["differing_component_ids"] = []
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)
    with pytest.raises(mc.ClassificationError, match="differing"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_blocked_classification_satisfies_presence_but_not_success(tmp_path):
    document, authority = _complete_document(tmp_path)
    document.update(
        {
            "outcome": "blocked",
            "blocked_reason": {
                "code": "engine_search_failed",
                "stage": "search",
                "message": "typed refusal",
                "detail": {"kind": "unavailable"},
            },
            "execution_witnesses": [],
            "metrics": [],
            "claims": [],
        }
    )
    document["classification_id"] = mc.classification_id(document)

    validated = mc.validate_classification(
        document, repository_root=tmp_path, authority=authority
    )
    assert validated["outcome"] == "blocked"
    assert mc.is_successful_evidence(validated) is False


def test_blocked_classification_rejects_unknown_reason(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["outcome"] = "blocked"
    document["blocked_reason"] = {
        "code": "something_happened",
        "stage": "search",
        "message": "not typed",
        "detail": {},
    }
    document["execution_witnesses"] = []
    document["metrics"] = []
    document["claims"] = []
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="blocked_reason"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_sidecar_write_is_idempotent_and_rejects_content_collision(tmp_path):
    document, authority = _complete_document(tmp_path)
    run_dir = tmp_path / "runs" / document["run_id"]

    first = mc.write_classification(
        run_dir, document, repository_root=tmp_path, authority=authority
    )
    second = mc.write_classification(
        run_dir, document, repository_root=tmp_path, authority=authority
    )
    assert first == second

    changed = copy.deepcopy(document)
    changed["classifier_version"] = "2"
    changed["classification_id"] = mc.classification_id(changed)
    with pytest.raises(mc.ClassificationError, match="conflict"):
        mc.write_classification(
            run_dir, changed, repository_root=tmp_path, authority=authority
        )


def test_index_prefix_detects_rewrite_and_post_cutover_missing_sidecar(tmp_path):
    experiments = tmp_path / "experiments"
    experiments.mkdir()
    index = experiments / "index.jsonl"
    historical = b'{"run_id":"historical","experiment":"GLOBAL-01"}\n'
    index.write_bytes(historical)
    policy = {
        "path": "experiments/index.jsonl",
        "prefix_bytes": len(historical),
        "prefix_lines": 1,
        "prefix_sha256": hashlib.sha256(historical).hexdigest(),
    }
    mc.validate_index_prefix(index, policy)

    index.write_bytes(historical.replace(b"historical", b"historicaL"))
    with pytest.raises(mc.ClassificationError, match="prefix"):
        mc.validate_index_prefix(index, policy)

    index.write_bytes(historical + b'{"run_id":"new-run","experiment":"demo"}\n')
    with pytest.raises(mc.ClassificationError, match="classification_missing"):
        mc.validate_post_cutover_presence(
            experiments_dir=experiments,
            index_path=index,
            prefix_lines=1,
        )


def test_classification_id_covers_the_full_body(tmp_path):
    document, _ = _complete_document(tmp_path)
    original = mc.classification_id(document)
    document["claims"][0]["layer"] = "end_to_end"
    assert mc.classification_id(document) != original


def test_engine_metric_requires_an_execution_witness(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["metrics"][0]["execution_witness_ids"] = []
    authority["metric_bindings"][0]["execution_witness_ids"] = []
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="execution witness"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_successful_data_plane_metric_rejects_unknown_historical_witness(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["execution_witnesses"][0].update(
        {
            "engine_search_state": "unknown_historical",
            "call_count": None,
            "count_semantics": "unknown",
            "evidence_kind": "immutable_receipt",
        }
    )
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="unknown_historical"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_plan_bound_roots_reject_hidden_numeric_metric(tmp_path):
    document, authority = _complete_document(tmp_path)
    metrics_path = tmp_path / "metrics.json"
    _write_json(
        metrics_path,
        {"retrieval": {"recall_at_3": 1.0}, "quality_score": 0.25},
    )
    document["source_artifacts"][0]["sha256"] = _sha(metrics_path)
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="unclassified"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_plan_rejects_overlapping_measurement_roots(tmp_path):
    document, authority = _complete_document(tmp_path)
    authority["measurement_roots"][0]["json_pointers"].append(
        "/retrieval/recall_at_3"
    )
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="overlap"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_source_hash_mismatch_rejects(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["source_artifacts"][0]["sha256"] = "0" * 64
    document["classification_id"] = mc.classification_id(document)

    with pytest.raises(mc.ClassificationError, match="SHA-256"):
        mc.validate_classification(
            document, repository_root=tmp_path, authority=authority
        )


def test_native_blocker_preserves_specific_failure_category():
    blocker = mc._native_blocker(  # noqa: SLF001 - contract-level typed outcome
        mc.ClassificationError("native expected hit is missing")
    )
    assert blocker["code"] == "expected_hit_missing"


def test_portable_repository_gate_loads_post_cutover_plan_and_sidecar(tmp_path):
    experiments = tmp_path / "experiments"
    plan = _authority()
    plan_path = experiments / "configs" / "native.measurement-plan.v1.json"
    _write_json(plan_path, plan)
    config = {
        "program_track": "GLOBAL-01",
        "measurement_plan": {
            "path": "experiments/configs/native.measurement-plan.v1.json",
            "sha256": mc.canonical_sha256(plan),
            "plan_id": plan["plan_id"],
        },
    }
    run_id, run_dir = _lib.write_record(
        "post-cutover",
        ts=__import__("datetime").datetime(
            2026, 9, 3, 18, 0, tzinfo=__import__("datetime").timezone.utc
        ),
        config_obj=config,
        metrics={"retrieval": {"recall_at_3": 1.0}},
        verdict="pass",
        read="portable fixture",
        code={
            "git_sha": "fixture",
            "dirty": False,
            "branch": "fixture",
            "baseline_commit": None,
        },
        corpus={"source": None, "manifest_sha256": None, "datasets": []},
        seeds={},
        env={
            "python": "3.12",
            "lockfile_sha256": None,
            "gpu": None,
            "key_deps": {},
        },
        cost_usd=0.0,
        base_dir=experiments,
    )
    runner = tmp_path / "runner.py"
    runner.write_text("engine.search(query)\n", encoding="utf-8")
    document, _ = _complete_document(tmp_path)
    document["run_id"] = run_id
    document["source_artifacts"][0].update(
        {
            "locator": str(
                (run_dir / "metrics.json").relative_to(tmp_path)
            ),
            "sha256": _sha(run_dir / "metrics.json"),
        }
    )
    document["classification_id"] = mc.classification_id(document)
    mc.write_classification(
        run_dir, document, repository_root=tmp_path, authority=plan
    )

    manifest_path = experiments / mc.HISTORICAL_MANIFEST_NAME
    _write_json(
        manifest_path,
        {
            "schema_version": mc.HISTORICAL_VERSION,
            "included": [],
            "excluded": [],
        },
    )
    _write_json(
        experiments / mc.POLICY_NAME,
        {
            "schema_version": mc.POLICY_VERSION,
            "classifier_version": "1",
            "index": {
                "path": "experiments/index.jsonl",
                "prefix_bytes": 0,
                "prefix_lines": 0,
                "prefix_sha256": hashlib.sha256(b"").hexdigest(),
            },
            "historical_manifest_path": str(manifest_path.relative_to(tmp_path)),
            "superseded_postcutover_run_ids": [],
        },
    )

    mc.validate_repository(tmp_path)

    (run_dir / mc.SIDECAR_NAME).unlink()
    with pytest.raises(mc.ClassificationError, match="classification_missing"):
        mc.validate_repository(tmp_path)

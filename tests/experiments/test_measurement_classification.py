"""Contract tests for executable experiment measurement classification."""

from __future__ import annotations

import copy
import hashlib
import json
from pathlib import Path

import pytest

from experiments import measurement_classification as mc


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True) + "\n", encoding="utf-8")


def _authority() -> dict:
    return {
        "schema_version": "measurement.plan.v1",
        "plan_id": "native-search-v1",
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
        "schema_version": "measurement.classification.v1",
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
    _write_json(tmp_path / "metrics.json", {"lifecycle": {"erasure": "pass"}})
    document["source_artifacts"][0]["sha256"] = _sha(tmp_path / "metrics.json")
    document["source_artifacts"][0]["measurement_root_json_pointers"] = [
        "/lifecycle"
    ]
    metric = document["metrics"][0]
    metric.update(
        {
            "json_pointer": "/lifecycle/erasure",
            "value_type": "enum",
            "allowed_values": ["pass", "fail"],
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
    document["migration"]["measurement_plan_sha256"] = mc.canonical_sha256(authority)
    document["classification_id"] = mc.classification_id(document)

    mc.validate_classification(document, repository_root=tmp_path, authority=authority)


def test_comparison_sets_are_derived_not_narrated(tmp_path):
    document, authority = _complete_document(tmp_path)
    document["components"].append(
        {"id": "planner", "name": "query planner", "kind": "query_planner"}
    )
    authority["components"] = copy.deepcopy(document["components"])
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
                "execution_witness_ids": [],
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

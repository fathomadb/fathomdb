from __future__ import annotations

import copy
from pathlib import Path

import pytest

from experiments import graph_evidence_01


ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "experiments/configs/graph-evidence-01/exact-evidence.v1.json"


def test_checked_in_config_is_strict_and_complete() -> None:
    config = graph_evidence_01.load_config(CONFIG)

    assert config.program_track == "GRAPH-EVIDENCE-01"
    assert len(config.matrix) == 12
    assert config.measurement == {
        "warmup_operations": 5,
        "measured_operations_per_cell": 20,
        "repetitions": 3,
        "percentile_estimator": "linear-p-n-minus-1",
    }

    document = copy.deepcopy(config.resolved)
    document["unexpected"] = True
    with pytest.raises(graph_evidence_01.GraphEvidenceBenchmarkError, match="unknown"):
        graph_evidence_01.resolve_config(document)


def _expectations() -> dict[str, object]:
    return {
        "positive": {
            "outgoing": {
                "target_revision": "target-r1",
                "edge_revision": "edge-r1",
                "edge_from": "root",
                "edge_to": "target",
                "edge_kind": "supports",
                "source_sha256": "a" * 64,
                "source_body": "canonical source",
            }
        },
        "refusals": {"tampered": "invalid_evidence_reference"},
    }


def _observations() -> list[dict[str, object]]:
    return [
        {
            "case_id": "outgoing",
            "outcome": "resolved",
            "target_revision": "target-r1",
            "edge_revision": "edge-r1",
            "edge_from": "root",
            "edge_to": "target",
            "edge_kind": "supports",
            "source_sha256": "a" * 64,
            "source_body": "canonical source",
            "semantic_digest": "semantic-a",
            "expand_ms": 1.0,
            "resolve_ms": 0.5,
        },
        {
            "case_id": "tampered",
            "outcome": "refused",
            "refusal_class": "invalid_evidence_reference",
        },
    ]


def test_fixture_owned_semantics_reject_self_consistent_sut_drift() -> None:
    metrics = graph_evidence_01.score_observations(_expectations(), _observations())
    assert metrics["fidelity"]["target_revision_exact_rate"] == 1.0
    assert metrics["fidelity"]["typed_refusal_rate"] == 1.0
    assert metrics["denominators"] == {
        "attempted": 2,
        "completed": 1,
        "errors": 0,
        "typed_refusals": 1,
    }

    drifted = _observations()
    drifted[0]["target_revision"] = "sut-invented-r2"
    with pytest.raises(graph_evidence_01.GraphEvidenceBenchmarkError, match="target revision"):
        graph_evidence_01.score_observations(_expectations(), drifted)


def test_opaque_reference_bytes_do_not_define_determinism() -> None:
    repeated = _observations()[:1] * 2
    repeated[0] = dict(repeated[0], evidence_ref="fdbgev1.first")
    repeated[1] = dict(repeated[1], evidence_ref="fdbgev1.second")

    metrics = graph_evidence_01.score_observations(_expectations(), repeated)
    assert metrics["fidelity"]["semantic_identity_consistent"] is True


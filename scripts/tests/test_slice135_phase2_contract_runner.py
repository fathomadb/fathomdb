"""Phase 2 runner records product output without importing fixture answers."""

from __future__ import annotations

import importlib.util
from pathlib import Path
from types import SimpleNamespace


SCRIPT = Path(__file__).resolve().parents[1] / "slice135_phase2_contract_runner.py"
SPEC = importlib.util.spec_from_file_location("slice135_phase2_contract_runner", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


def test_seed_uses_only_corpus_inputs_and_stable_ids() -> None:
    fixture = {
        "corpus": [{"kind": "note", "body": "alpha"}, {"kind": "doc", "body": "beta"}],
        "queries": [{"query": "alpha", "expected_bodies": ["alpha"]}],
    }
    assert RUNNER.seed_records(fixture, source_id="source", id_prefix="id-") == [
        {"kind": "note", "body": "alpha", "logical_id": "id-0", "source_id": "source"},
        {"kind": "doc", "body": "beta", "logical_id": "id-1", "source_id": "source"},
    ]
    assert fixture["corpus"][0] == {"kind": "note", "body": "alpha"}


def test_observation_keeps_unexpected_product_values_for_independent_audit() -> None:
    hit = SimpleNamespace(
        id=SimpleNamespace(space="logical", value="unexpected-id"),
        kind="wrong-kind",
        body="unexpected-body",
        branch="vector",
        source_id="wrong-source",
    )
    assert RUNNER.serialize_hits([hit]) == [
        {
            "id_space": "logical",
            "id_value": "unexpected-id",
            "kind": "wrong-kind",
            "body": "unexpected-body",
            "branch": "vector",
            "source_id": "wrong-source",
        }
    ]

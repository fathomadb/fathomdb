"""A new shared SDK operation must receive an explicit benchmark disposition."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from experiments import sdk_registry


ROOT = Path(__file__).resolve().parents[2]


def test_registry_covers_each_live_governed_operation():
    parity = json.loads(
        (ROOT / "src/conformance/governed-operation-parity.json").read_text()
    )
    live = {
        operation["id"]
        for operation in parity["operations"]
        if operation["state"] == "live"
    }
    registry = sdk_registry.load_registry()
    assert len(live) == 44
    assert live <= set(registry)
    for operation_id in live:
        entry = registry[operation_id]
        assert entry["semantic_version"] == 1
        assert entry["disposition"] in {
            "measure",
            "setup",
            "observe",
            "excluded_with_reason",
        }
        assert set(entry["bindings"]) == {"python", "typescript", "rust"}
        assert (
            entry["reason"] if entry["disposition"] == "excluded_with_reason" else True
        )


def test_registry_rejects_new_operation_without_descriptor(tmp_path: Path):
    parity = json.loads(
        (ROOT / "src/conformance/governed-operation-parity.json").read_text()
    )
    new = dict(parity["operations"][0])
    new["id"] = "engine.future_feature"
    parity["operations"].append(new)
    path = tmp_path / "parity.json"
    path.write_text(json.dumps(parity))
    with pytest.raises(ValueError, match="registry.*drift"):
        sdk_registry.load_registry(path)

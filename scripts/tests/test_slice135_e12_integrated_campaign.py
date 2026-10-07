"""Frozen schedule checks for the integrated E01–E12 paired campaign."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_e12_integrated_campaign as campaign  # noqa: E402


FREEZE = (
    Path(__file__).resolve().parents[2]
    / "dev/plans/0.8.27/features/slice-135/e12-integrated-comparison-protocol.json"
)


def frozen() -> dict:
    """Load the source-bound protocol that the campaign must execute."""
    return json.loads(FREEZE.read_text())


def test_planned_blocks_match_frozen_order_and_samples() -> None:
    blocks = campaign.planned_blocks(frozen())
    assert len(blocks) == 20
    assert [(b["group"], b["version"], b["samples"]) for b in blocks[:4]] == [
        ("query", "baseline", 1000),
        ("query", "candidate", 1000),
        ("query", "candidate", 1000),
        ("query", "baseline", 1000),
    ]
    assert blocks[-1]["name"] == "lifecycle-pair-5-2-candidate"
    assert all(b["cells"] == frozen()["workload"][f"{b['group']}_cells"] for b in blocks)


def test_rejects_missing_version_and_source_drift() -> None:
    changed = frozen()
    changed["workload"]["pair_order"][0] = ["baseline", "baseline"]
    with pytest.raises(ValueError, match="pair"):
        campaign.planned_blocks(changed)
    changed = frozen()
    changed["candidate_product_snapshot"]["source_sha_before_protocol_commit"] = "0" * 40
    with pytest.raises(ValueError, match="candidate"):
        campaign.planned_blocks(changed)


def test_rejects_changed_cell_list_or_sample_rule() -> None:
    changed = deepcopy(frozen())
    changed["workload"]["query_cells"] = changed["workload"]["query_cells"][:-1]
    with pytest.raises(ValueError, match="cells"):
        campaign.planned_blocks(changed)
    changed = frozen()
    changed["workload"]["lifecycle_samples_per_block"] = 7
    with pytest.raises(ValueError, match="samples"):
        campaign.planned_blocks(changed)

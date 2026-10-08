"""Frozen schedule checks for current-candidate installed Python S01."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys
from types import SimpleNamespace

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s01_integrated_campaign as campaign  # noqa: E402


FREEZE = (
    Path(__file__).resolve().parents[2]
    / "dev/plans/0.8.27/features/slice-135/s01-python-integrated-comparison-protocol.json"
)


def frozen() -> dict:
    """Load the committed exact-candidate protocol."""
    return json.loads(FREEZE.read_text())


def test_planned_blocks_follow_frozen_sizes_and_alternation() -> None:
    blocks = campaign.planned_blocks(frozen())
    assert len(blocks) == 20
    assert [(b["rows"], b["pair"], b["role"]) for b in blocks[:6]] == [
        (32, 1, "baseline"),
        (32, 1, "candidate"),
        (32, 2, "candidate"),
        (32, 2, "baseline"),
        (32, 3, "baseline"),
        (32, 3, "candidate"),
    ]
    assert blocks[-1]["name"] == "256-05-candidate"
    assert all(b["samples"] == 1000 for b in blocks)


def test_rejects_wrong_candidate_or_schedule() -> None:
    changed = frozen()
    changed["candidate"]["source_sha"] = "0" * 40
    with pytest.raises(ValueError, match="candidate"):
        campaign.planned_blocks(changed)
    changed = deepcopy(frozen())
    changed["pair_order_each_size"][1] = ["baseline", "baseline"]
    with pytest.raises(ValueError, match="pair"):
        campaign.planned_blocks(changed)


def test_refrozen_candidate_keeps_schedule_and_requires_matching_snapshot() -> None:
    changed = frozen()
    changed["candidate"]["source_sha"] = (
        "3f29d649d0213e595c0dab251a449d92fd625792"
    )
    changed["candidate_product_snapshot"]["source_sha"] = changed["candidate"][
        "source_sha"
    ]
    assert campaign.planned_blocks(changed) == campaign.planned_blocks(frozen())

    changed["candidate_product_snapshot"]["source_sha"] = (
        "cdf253cd223a82e954591db532397a3d78a2027a"
    )
    with pytest.raises(ValueError, match="snapshot"):
        campaign.planned_blocks(changed)


def test_refrozen_protocol_requires_an_expected_byte_hash(tmp_path: Path) -> None:
    path = tmp_path / "refrozen.json"
    path.write_text(json.dumps(frozen()))
    with pytest.raises(ValueError, match="freeze-sha256"):
        campaign.run(SimpleNamespace(freeze=path, freeze_sha256=None))


def test_rejects_sample_and_size_drift() -> None:
    changed = frozen()
    changed["warm_samples_per_cell"] = 100
    with pytest.raises(ValueError, match="samples"):
        campaign.planned_blocks(changed)
    changed = frozen()
    changed["rows"] = [32]
    with pytest.raises(ValueError, match="rows"):
        campaign.planned_blocks(changed)

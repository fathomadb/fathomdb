"""Keep the TypeScript S02 paired order and artifact freeze executable."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_ts_s02_campaign as campaign  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
FREEZE = ROOT / "dev/plans/0.8.27/features/slice-135/s02-ts-comparison-protocol.json"


def test_frozen_s02_plan_has_alternating_ten_blocks() -> None:
    protocol = json.loads(FREEZE.read_text())
    blocks = campaign.planned_blocks(protocol)
    assert [item["role"] for item in blocks] == [
        "baseline", "candidate", "candidate", "baseline", "baseline",
        "candidate", "candidate", "baseline", "baseline", "candidate",
    ]
    assert [item["samples"] for item in blocks] == [20] * 10
    changed = deepcopy(protocol)
    changed["pair_order"][0] = ["candidate", "baseline"]
    with pytest.raises(ValueError, match="pair order"):
        campaign.planned_blocks(changed)


def test_s02_campaign_rejects_shortened_sample_design() -> None:
    protocol = json.loads(FREEZE.read_text())
    protocol["samples_per_block"] = 3
    with pytest.raises(ValueError, match="sample count"):
        campaign.planned_blocks(protocol)


def test_s02_campaign_refuses_changed_independent_pair_auditor() -> None:
    protocol = json.loads(FREEZE.read_text())
    changed = deepcopy(protocol)
    changed["paired_auditor_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="paired_auditor_sha256"):
        campaign.validate_protocol(changed)


def test_s02_campaign_binds_explicit_refreeze_bytes(tmp_path: Path) -> None:
    protocol = json.loads(FREEZE.read_text())
    alternate = tmp_path / "s02-vector-repaired.json"
    alternate.write_text(json.dumps(protocol))
    digest = campaign.sha(alternate)
    assert campaign.resolve_freeze(alternate, digest) == (alternate, digest)
    with pytest.raises(ValueError, match="freeze bytes"):
        campaign.resolve_freeze(alternate, "0" * 64)


def test_s02_campaign_rejects_changed_product_snapshot_source() -> None:
    protocol = json.loads(FREEZE.read_text())
    protocol["candidate_product_snapshot"] = {
        "source_sha": "0" * 40,
        "rust_crates_tree": "1" * 40,
    }
    with pytest.raises(ValueError, match="product snapshot"):
        campaign.validate_protocol(protocol)

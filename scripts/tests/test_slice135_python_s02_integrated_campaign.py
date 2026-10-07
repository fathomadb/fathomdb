"""Integrated S02 campaign refuses source or protocol drift."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_s02_integrated_campaign as campaign  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
PROTOCOL = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/s02-python-integrated-comparison-protocol.json"
)


def test_integrated_protocol_binds_prior_pilot_and_both_wheels() -> None:
    protocol = json.loads(PROTOCOL.read_text())
    campaign.validate_protocol(protocol)
    changed = deepcopy(protocol)
    changed["candidate"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="candidate wheel"):
        campaign.validate_protocol(changed)
    changed = deepcopy(protocol)
    changed["baseline_noise_audit_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="baseline noise"):
        campaign.validate_protocol(changed)


def test_integrated_protocol_preserves_count_and_pair_order() -> None:
    protocol = json.loads(PROTOCOL.read_text())
    changed = deepcopy(protocol)
    changed["samples_per_block"] = 10
    with pytest.raises(ValueError, match="sample count"):
        campaign.validate_protocol(changed)
    changed = deepcopy(protocol)
    changed["pair_order"][1].reverse()
    with pytest.raises(ValueError, match="pair order"):
        campaign.validate_protocol(changed)

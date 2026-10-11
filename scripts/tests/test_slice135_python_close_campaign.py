"""Freeze guards for the installed-Python S02-L comparison."""

from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_campaign as campaign  # noqa: E402


ROOT = Path(__file__).resolve().parents[2]
PROTOCOL = (
    ROOT
    / "dev/plans/0.8.27/features/slice-135/s02-python-lifecycle-comparison-protocol.json"
)


def test_frozen_lifecycle_protocol_binds_runner_and_artifacts() -> None:
    protocol = json.loads(PROTOCOL.read_text())
    campaign.validate_protocol(protocol, ROOT)
    for key in ("block_runner_sha256", "lifecycle_runner_sha256", "audit_sha256"):
        changed = deepcopy(protocol)
        changed[key] = "0" * 64
        with pytest.raises(ValueError, match=key):
            campaign.validate_protocol(changed, ROOT)
    changed = deepcopy(protocol)
    changed["candidate"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="candidate"):
        campaign.validate_protocol(changed, ROOT)


def test_frozen_order_and_sample_count_cannot_change() -> None:
    protocol = json.loads(PROTOCOL.read_text())
    changed = deepcopy(protocol)
    changed["samples_per_block"] = 19
    with pytest.raises(ValueError, match="sample"):
        campaign.validate_protocol(changed, ROOT)
    changed = deepcopy(protocol)
    changed["pair_order"][1].reverse()
    with pytest.raises(ValueError, match="pair order"):
        campaign.validate_protocol(changed, ROOT)

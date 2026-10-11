"""Exact-source lifecycle timing must bind the repaired wheel and auditor."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_python_close_224e_campaign as campaign  # noqa: E402
import slice135_python_close_224e_pair_audit as paired_audit  # noqa: E402


def test_exact_source_lifecycle_runner_and_auditor_use_same_wheel() -> None:
    expected = Path(
        "/tmp/slice135-current-wheel-224e44/"
        "fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl"
    )
    assert campaign.WHEEL["candidate"] == expected
    assert paired_audit.WHEEL["candidate"] == expected
    assert (
        paired_audit.FROZEN.name
        == "s02-python-lifecycle-224e-v2-comparison-protocol.json"
    )


def test_exact_source_protocol_rejects_changed_wheel_hash() -> None:
    root = Path(__file__).resolve().parents[2]
    protocol = json.loads(paired_audit.FROZEN.read_text())
    campaign.validate_protocol(protocol, root)
    protocol["candidate"]["wheel_sha256"] = "0" * 64
    with pytest.raises(ValueError, match="candidate source or wheel changed"):
        campaign.validate_protocol(protocol, root)

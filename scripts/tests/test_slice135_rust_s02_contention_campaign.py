"""Rust SDK contention campaign binds clean source and every executable byte."""

from __future__ import annotations

import hashlib
from pathlib import Path
import sys

import pytest


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import slice135_rust_s02_contention_campaign as campaign  # noqa: E402


def test_freeze_rejects_changed_executable(tmp_path: Path) -> None:
    source = tmp_path / "consumer.rs"
    binary = tmp_path / "consumer"
    source.write_text("source")
    binary.write_bytes(b"binary")
    assets = {"consumer_source": source, "compiled_binary": binary}
    frozen = {
        "status": "FROZEN_RUST_S02_CONTENTION",
        "measured_processes": 10,
        "frozen_sha256": {
            name: hashlib.sha256(path.read_bytes()).hexdigest()
            for name, path in assets.items()
        },
    }
    campaign.validate_freeze(frozen, assets)
    binary.write_bytes(b"different")
    with pytest.raises(ValueError, match="compiled_binary"):
        campaign.validate_freeze(frozen, assets)


def test_freeze_rejects_weaker_sample_rule() -> None:
    with pytest.raises(ValueError, match="sample"):
        campaign.validate_freeze({
            "status": "FROZEN_RUST_S02_CONTENTION", "measured_processes": 1,
            "frozen_sha256": {},
        }, {})
